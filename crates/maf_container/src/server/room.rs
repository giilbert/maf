use anyhow::Context;
use biscuit::{ClaimsSet, JWT, jwa::SignatureAlgorithm, jws::Secret};
use maf_schemas::apps::{MetaEntryMap, RoomId, generate_room_secret};
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::{
    Connection, Container, ContainerRuntime,
    container::{ContainerHandle, ContainerResourceLimit, CreateContainerOptions},
    wasi::{
        HookRequest,
        bindings::{self, HookRequestCaller, HookRequestInit},
    },
};

use super::Bundle;

#[derive(Debug, Clone)]
pub struct RoomInner {
    id: Uuid,
    pub secret: String,
    pub container: ContainerHandle,
    pub bundle: Bundle,
}

pub struct CreateRoomInnerOptions {
    pub bundle: Bundle,
    pub resource_limit: ContainerResourceLimit,
    pub meta: Option<MetaEntryMap>,
}

impl RoomInner {
    pub async fn new(
        container_runtime: &ContainerRuntime,
        options: CreateRoomInnerOptions,
    ) -> anyhow::Result<(Self, Container)> {
        let room_id = Uuid::new_v4();
        let container = Container::load_from_binary(
            &container_runtime,
            room_id,
            CreateContainerOptions {
                bytes: &options.bundle.wasm_module_bytes,
                resource_limit: options.resource_limit,
                meta: options.meta,
            },
        )
        .await?;

        Ok((
            Self {
                id: room_id,
                secret: generate_room_secret(),
                container: container.handle(),
                bundle: options.bundle,
            },
            container,
        ))
    }

    pub async fn replace_container(&mut self, container: Container) -> anyhow::Result<()> {
        self.container = container.handle();
        Ok(())
    }

    /// Returns the unique identifier of the room.
    pub fn id(&self) -> RoomId {
        self.id
    }

    pub async fn add_connection(&self, connection: impl Connection) -> anyhow::Result<()> {
        match self.container.add_connection(Box::new(connection)).await {
            Ok(_) => tracing::info!("connection added to room {}", self.id),
            Err(_) => anyhow::bail!("failed to add connection to room {}", self.id),
        }

        Ok(())
    }

    pub async fn call_hook(
        &self,
        caller: HookRequestCaller,
        method: String,
        data: bindings::HookBody,
    ) -> anyhow::Result<bindings::HookBody> {
        let (message_tx, message_rx) = oneshot::channel::<bindings::HookBody>();

        let request = HookRequest::new(
            HookRequestInit {
                caller,
                method,
                data,
            },
            message_tx,
        );
        self.container.send_hook_request(request).await?;

        match tokio::time::timeout(std::time::Duration::from_secs(5), message_rx).await? {
            Ok(response) => {
                tracing::info!("hook response: {:?}", response);
                Ok(response)
            }
            Err(_) => {
                tracing::info!("hook response timed out");
                anyhow::bail!("failed to receive hook response");
            }
        }
    }

    /// Decodes and verifies a JWT signed with the room's secret.
    ///
    /// If the token is valid, returns the decoded claims as a [`serde_json::Value`]. If the token
    /// is invalid or verification fails, returns `Err`.
    pub fn decode_token(&self, token: &str) -> Result<serde_json::Value, anyhow::Error> {
        let mut verified_jwt: JWT<ClaimsSet<serde_json::Value>, serde_json::Value> =
            JWT::new_encoded(&token)
                .decode(
                    &Secret::bytes_from_str(&self.secret),
                    SignatureAlgorithm::HS256,
                )
                .context("failed to decode JWT")?;

        let payload = verified_jwt.payload_mut().expect("JWT should be decoded");

        // Check that the audience matches the room ID
        let audience = payload.registered.audience.clone();
        if !audience.is_some_and(|aud| aud.contains(&self.id.to_string())) {
            anyhow::bail!("invalid audience in JWT");
        }

        Ok(serde_json::to_value(payload).context("failed to reencode JWT")?)
    }
}
