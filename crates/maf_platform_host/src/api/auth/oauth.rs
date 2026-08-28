//! An implementation of OAuth2 login flow for Google.
//!
//! This module provides the necessary functionality to initiate the OAuth2 login process with
//! Google and handles the callback after the user has authenticated.

use anyhow::Context;
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Redirect};
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::{Cookie, SameSite};
use base64::Engine;
use chrono::Utc;
use maf_schemas::ErrorResponse;
use migrations::entity::account::OAuthProvider;
use migrations::entity::user::Permissions;
use migrations::entity::{account, session, user};
use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointNotSet, EndpointSet,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use rand::Rng;
use rand::distr::Alphanumeric;
use reqwest::redirect::Policy;
use sea_orm::ActiveValue::*;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, QuerySelect,
    TransactionTrait,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::api::{AppState, Environment};
use crate::storage::repos::utils::DbErrorExt;

const CSRF_COOKIE: &str = "maf_oauth_csrf_state";
const PKCE_VERIFIER_COOKIE: &str = "maf_oauth_pkce_verifier";

/// Represents an initialized OAuth client with all required endpoints set. (The `oauth2` crate uses
/// this weird builder pattern to ensure that all variables are set before the client can be used.)
type InitializedClient = BasicClient<
    EndpointSet,    // auth_url
    EndpointNotSet, // device_auth_url
    EndpointNotSet, // introspection_url
    EndpointNotSet, // revocation_url
    EndpointSet,    // token_url
>;

/// Collects all OAuth2 clients for this application.
#[derive(Debug)]
pub struct OAuthClients {
    reqwest: reqwest::Client,
    google: Option<InitializedClient>,
}

impl OAuthClients {
    fn create_google_oauth_client() -> anyhow::Result<InitializedClient> {
        let client_id = std::env::var("GOOGLE_CLIENT_ID")
            .map(ClientId::new)
            .context("failed to read GOOGLE_CLIENT_ID")?;
        let client_secret = std::env::var("GOOGLE_CLIENT_SECRET")
            .map(ClientSecret::new)
            .context("failed to read GOOGLE_CLIENT_SECRET")?;
        let redirect_url = std::env::var("AUTH_REDIRECT_URL")
            .map(|url| format!("{url}/api/v1/auth/callback/google"))
            .map(RedirectUrl::new)
            .context("failed to read AUTH_REDIRECT_URL")?
            .context("failed to parse AUTH_REDIRECT_URL")?;

        let client = BasicClient::new(client_id)
            .set_client_secret(client_secret)
            .set_redirect_uri(redirect_url)
            .set_auth_uri(AuthUrl::new(
                "https://accounts.google.com/o/oauth2/v2/auth".into(),
            )?)
            .set_token_uri(TokenUrl::new(
                "https://www.googleapis.com/oauth2/v3/token".into(),
            )?);

        Ok(client)
    }

    pub fn new() -> Self {
        let reqwest = reqwest::Client::builder()
            .redirect(Policy::none())
            .build()
            .expect("failed to build reqwest client");

        let google = match Self::create_google_oauth_client() {
            Ok(client) => Some(client),
            Err(err) => {
                tracing::warn!(?err, "google oauth is not configured");
                None
            }
        };

        OAuthClients { reqwest, google }
    }

    pub fn google(&self) -> &InitializedClient {
        self.google
            .as_ref()
            .expect("google oauth client is not configured")
    }
}

const GOOGLE_SCOPES: &[&str] = &[
    "openid",
    "https://www.googleapis.com/auth/userinfo.email",
    "https://www.googleapis.com/auth/userinfo.profile",
];

/// **GET** `/api/v1/auth/login`
///
/// Initiates the OAuth login process with Google.
pub async fn oauth_login(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, Redirect), ErrorResponse> {
    let (pkce_code_challenge, pkce_code_verifier) = PkceCodeChallenge::new_random_sha256();

    let (authorize_url, csrf_state) = state
        .oauth_clients()
        .google()
        .authorize_url(CsrfToken::new_random)
        .add_scopes(GOOGLE_SCOPES.iter().map(|s| Scope::new(s.to_string())))
        .set_pkce_challenge(pkce_code_challenge)
        .add_extra_param("access_type", "offline")
        .add_extra_param("prompt", "consent")
        .url();

    const COOKIE_MAX_AGE: time::Duration = time::Duration::minutes(3);
    const COOKIE_PATH: &str = "/api/v1/auth";

    // Store verifier + csrf state in a short-lived, HttpOnly, Secure cookie.
    let verifier_cookie =
        Cookie::build((PKCE_VERIFIER_COOKIE, pkce_code_verifier.secret().clone()))
            .http_only(true)
            .secure(true)
            .same_site(SameSite::Lax)
            .path(COOKIE_PATH)
            .max_age(COOKIE_MAX_AGE)
            .build();

    let csrf_cookie = Cookie::build((CSRF_COOKIE, csrf_state.secret().clone()))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Lax)
        .path(COOKIE_PATH)
        .max_age(COOKIE_MAX_AGE)
        .build();

    let jar = jar.add(verifier_cookie).add(csrf_cookie);

    Ok((jar, Redirect::to(authorize_url.as_str())))
}

#[derive(Debug, Deserialize)]
pub struct GoogleOauthCallbackQuery {
    /// Authorization code that was sent to Google during the initial login request. Used to
    /// exchange for an access token and retrieve the user's information.
    code: String,
    /// CSRF state parameter that was sent to Google during the initial login request. This is used
    /// to make sure that the request is coming from a legitimate source (e.g. us).
    state: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GoogleUserInfo {
    /// This is the unique ID that Google uses to identify the user.
    pub sub: String,
    pub name: String,
    pub email: String,
    pub email_verified: bool,
    pub picture: String,
}

/// Represents an error that occurred during the OAuth2 login process.
#[derive(Debug, Default, Serialize)]
#[serde(untagged)]
pub enum OAuthErrorResponse {
    #[default]
    Unknown,
    /// The user's email is not verified, so we cannot allow them to log in.
    UnverifiedEmail,
    /// The application is running in a "waitlist" mode, and the user does not have an account yet.
    /// Account creation by the user is not allowed in this mode and the user must be invited by an
    /// administrator to create an account.
    InWaitlistMode,
    /// The user tried to connect an account to an existing user, but we cannot verify that the
    /// account belongs to the same person as the existing user.
    CannotLinkAccount,
    #[serde(skip_serializing)]
    Other(ErrorResponse),
}

impl IntoResponse for OAuthErrorResponse {
    fn into_response(self) -> axum::response::Response {
        if let OAuthErrorResponse::Other(other) = self {
            return other.into_response();
        }

        // TODO: This really should become some kind of redirect to a page that explains the error
        // to the user, but for now we just return a 400 with a message.
        ErrorResponse::bad_request(Some(
            serde_json::to_string(&self)
                .unwrap_or_else(|_| "Unknown".to_string())
                .as_str(),
        ))
        .into_response()
    }
}

impl From<anyhow::Error> for OAuthErrorResponse {
    fn from(err: anyhow::Error) -> Self {
        OAuthErrorResponse::Other(ErrorResponse::from(err))
    }
}

#[derive(Debug)]
struct UpdateDatabaseWithUserInfoOptions {
    access_token: String,
    refresh_token: Option<String>,
    google_user_info: GoogleUserInfo,
    expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug)]
enum UpdateDatabaseResult {
    Success(user::Model),
    /// Retry occurs when two requests try to create the same user or other record at the same time,
    /// and one of them fails due to a unique constraint violation. In this case, we allow the
    /// request to rollback the transaction and retry the login process.
    Retry,
}

/// Updates the database with user information inside a transaction. This function returning `Err`
/// means that the transaction should be rolled back, `Ok(UpdateDatabaseResult::Retry)` means that
/// the transaction should be rolled back and the function should be retried, and
/// `Ok(UpdateDatabaseResult::Success(user))` means that everything is good.
async fn update_database_with_user_info_inner(
    txn: &DatabaseTransaction,
    state: &AppState,
    options: &UpdateDatabaseWithUserInfoOptions,
) -> Result<UpdateDatabaseResult, OAuthErrorResponse> {
    // If this is set to `false`, the user will only be able to log in if their account already
    // exists in the database (i.e. if the app is running in a "waitlist" mode). If set to `true`,
    // the user will be able to create a new account if it doesn't exist yet.
    let support_creating_users = state.environment() == Environment::Development;
    let user_info = options.google_user_info.clone();

    let user = user::Entity::find()
        .filter(user::Column::Email.eq(&user_info.email))
        // Lock the row as if we are going to update it, so that we don't have a race condition
        // where two requests try to create multiple accounts for the same user at the same
        // time.
        .lock_exclusive()
        .all(txn)
        .await
        .context("failed to find user with accounts")?
        .into_iter()
        .next();

    let user = match user {
        Some(user) => user,
        None => {
            // The user does not exist and we cannot create new users, so we cannot allow them
            // to log in.
            if !support_creating_users {
                return Err(OAuthErrorResponse::InWaitlistMode);
            }

            // The user does not exist yet, so we need to create a new user record.
            let user = user::ActiveModel {
                id: Set(Uuid::new_v4()),
                email: Set(user_info.email.clone()),
                name: Set(user_info.name.clone()),
                username: Set(None),
                email_verified: Set(user_info.email_verified),
                avatar_url: Set(Some(user_info.picture.clone())),
                created_at: Set(chrono::Utc::now()),
                updated_at: Set(chrono::Utc::now()),
                permissions: Set(Permissions::empty()),
            }
            .insert(txn)
            .await;

            if let Err(err) = &user
                && err.is_unique_violation()
            {
                // The lock does not prevent two requests from trying to create the same user at the
                // same time, so we allow the request to rollback the transaction and retry the
                // login process.
                return Ok(UpdateDatabaseResult::Retry);
            } else {
                user.context("failed to create new user")?
            }
        }
    };

    tracing::debug!(?user, "created user");

    let accounts = account::Entity::find()
        .filter(account::Column::UserId.eq(user.id))
        .all(txn)
        .await
        .context("failed to find accounts for user")?;

    // The user has an account associated with Google, so we can allow them to log in.
    if accounts.iter().any(|a| a.provider == OAuthProvider::Google) {
        // Update their account with the new access token and refresh token and check if the
        // account is actually the same user as the one we are trying to log in with.
        let update_account_result = account::Entity::update(account::ActiveModel {
            access_token: Set(options.access_token.clone()),
            refresh_token: Set(options.refresh_token.clone()),
            updated_at: Set(chrono::Utc::now()),
            expires_at: Set(Some(options.expires_at)),
            ..Default::default()
        })
        .filter(
            account::Column::UserId
                .eq(user.id)
                .and(account::Column::Provider.eq(OAuthProvider::Google)),
        )
        .exec(txn)
        .await;

        tracing::debug!(?update_account_result, "updated account for user");
    } else {
        // The user does not have a Google account linked.

        if !accounts.is_empty() {
            // The user has an account, but it is not linked to Google. We cannot allow them to log
            // in with Google, because we cannot verify that they are the same person.
            return Err(OAuthErrorResponse::CannotLinkAccount);
        }

        // The user does not have an existing Google account, we can log them in by creating a new
        // account record for them.
        let create_account_result = account::ActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(user.id),
            provider: Set(OAuthProvider::Google),
            provider_user_id: Set(user_info.sub),
            access_token: Set(options.access_token.clone()),
            refresh_token: Set(options.refresh_token.clone()),
            expires_at: Set(Some(options.expires_at)),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
            scopes: Set(GOOGLE_SCOPES.iter().map(|s| s.to_string()).collect()),
        }
        .insert(txn)
        .await;

        tracing::debug!(?create_account_result, "created account for user");

        if let Err(err) = &create_account_result
            && err.is_unique_violation()
        {
            // The user tried to create a new account, but it already exists. This can happen if
            // two requests try to create the same account at the same time as the lock does not
            // prevent this from happening. In this case, we allow them to retry the login.
            return Ok(UpdateDatabaseResult::Retry);
        } else {
            create_account_result.context("failed to create new account")?;
        }
    }

    Ok(UpdateDatabaseResult::Success(user))
}

/// Updates the database with the user's information from Google. If the user does not exist yet, a
/// new user record will be created (if allowed). If the user already exists, their information will
/// be updated with the latest data from Google.
///
/// TODO: more oauth providers, account linking
#[async_recursion::async_recursion]
#[tracing::instrument(skip(state, options), fields(email = ?options.google_user_info.email))]
async fn update_database_with_user_info(
    state: &AppState,
    options: UpdateDatabaseWithUserInfoOptions,
) -> Result<user::Model, OAuthErrorResponse> {
    let user = {
        let txn = state
            .db()
            .begin()
            .await
            .context("failed to start transaction")?;

        match update_database_with_user_info_inner(&txn, state, &options).await {
            Ok(UpdateDatabaseResult::Success(user)) => {
                txn.commit().await.context("failed to commit transaction")?;
                user
            }
            Ok(UpdateDatabaseResult::Retry) => {
                txn.rollback()
                    .await
                    .context("failed to rollback transaction")?;
                tracing::debug!("retrying update_database_with_user_info");
                return update_database_with_user_info(state, options).await;
            }
            Err(err) => {
                txn.rollback()
                    .await
                    .context("failed to rollback transaction")?;

                return Err(err);
            }
        }
    };

    Ok(user)
}

/// Creates a random string of 128 alphanumeric characters to be used as a session token.
fn generate_session_token() -> String {
    const SESSION_TOKEN_LENGTH: usize = 128;
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(SESSION_TOKEN_LENGTH)
        .map(char::from)
        .collect()
}

/// Hashes the given string using SHA256 and returns the base64-encoded result. This is used to
/// store the session token in the database in a secure way.
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let result = hasher.finalize();
    base64::engine::general_purpose::STANDARD.encode(result)
}

const SESSION_LENGTH: chrono::Duration = chrono::Duration::days(1);
const SESSION_COOKIE_NAME: &str = "maf_session_token";

/// **GET** `/api/v1/auth/callback/google`
///
/// Handles the callback from Google after the user has authenticated. Exchanges the authorization
/// code for an access token and retrieves the user's information.
pub async fn oauth_callback_google(
    State(state): State<AppState>,
    jar: CookieJar,
    query: Query<GoogleOauthCallbackQuery>,
) -> Result<(CookieJar, Redirect), OAuthErrorResponse> {
    // Validate CSRF state before doing anything else with the code.
    let csrf_cookie_value = jar
        .get(CSRF_COOKIE)
        .map(|c| c.value().to_string())
        .ok_or_else(|| {
            tracing::warn!("missing CSRF cookie in oauth callback");
            OAuthErrorResponse::Unknown
        })?;

    if csrf_cookie_value != query.state {
        tracing::warn!("CSRF state mismatch on oauth callback");
        return Err(OAuthErrorResponse::Unknown);
    }

    let token_response = state
        .oauth_clients()
        .google()
        .exchange_code(AuthorizationCode::new(query.code.clone()))
        .set_pkce_verifier(
            jar.get(PKCE_VERIFIER_COOKIE)
                .map(|c| PkceCodeVerifier::new(c.value().to_string()))
                .ok_or_else(|| {
                    tracing::warn!("missing PKCE verifier cookie in oauth callback");
                    OAuthErrorResponse::Unknown
                })?,
        )
        .request_async(&state.oauth_clients().reqwest)
        .await
        .context("failed to exchange code for token")?;

    let access_token_string = token_response.access_token().secret().clone();
    let refresh_token_string = token_response.refresh_token().map(|t| t.secret().clone());

    let expires_in_sec = token_response
        .expires_in()
        .map(|d| d.as_secs())
        .context("google oauth did not send back an expires_in value")?;
    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(expires_in_sec as i64);

    // Clean up the short-lived cookies now that they've served their purpose.
    let jar = jar
        .remove(Cookie::from(CSRF_COOKIE))
        .remove(Cookie::from(PKCE_VERIFIER_COOKIE));

    tracing::debug!(?token_response, "received token response from google");

    // Get the user's information from Google using the access token.
    let user_info = state
        .oauth_clients()
        .reqwest
        .get("https://www.googleapis.com/oauth2/v3/userinfo")
        .bearer_auth(token_response.access_token().secret())
        .send()
        .await
        .context("failed to get user info from google")?
        .json::<GoogleUserInfo>()
        .await
        .context("failed to parse user info from google")?;

    tracing::debug!(?user_info, "received user info from google");

    // If the user's email is not verified, we cannot allow them to log in.
    if !user_info.email_verified {
        return Err(OAuthErrorResponse::UnverifiedEmail);
    }

    // Update the database with the user's account information (creating the record if it doesn't
    // exist).
    let user = update_database_with_user_info(
        &state,
        UpdateDatabaseWithUserInfoOptions {
            access_token: access_token_string,
            refresh_token: refresh_token_string,
            google_user_info: user_info,
            expires_at,
        },
    )
    .await?;

    let session_token = generate_session_token();
    // Create a new session for the user and set a cookie with the session ID.
    let session = session::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(user.id),
        created_at: Set(Utc::now()),
        expires_at: Set(Utc::now() + SESSION_LENGTH),
        token_hash: Set(hash_token(&session_token)),
    }
    .insert(state.db())
    .await
    .context("failed to create new session")?;

    let jar = jar.add(
        Cookie::build((SESSION_COOKIE_NAME, session_token))
            .http_only(true)
            .secure(true)
            .same_site(SameSite::Lax)
            .path("/")
            .max_age(time::Duration::seconds(SESSION_LENGTH.num_seconds()))
            .build(),
    );

    tracing::debug!(
        user_id =? user.id,
        session_id =? session.id,
        "created new session for user"
    );

    Ok((jar, Redirect::to("/")))
}

/// **POST** `/api/v1/auth/session`
///
/// Refreshes the user's session by creating a new session token and updating the existing session's
/// expiration time. Refreshes the session cookie with the new token and returns the user's
/// information.
///
/// If the user is not logged in, this endpoint will return a 401 Unauthorized response.
pub async fn update_session(state: State<AppState>) {}
