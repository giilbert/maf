use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAdminView {
    pub id: Uuid,
    pub username: String,
    pub name: String,
    pub permissions: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgAdminView {
    pub name: String,
    pub slug: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserWithOrgsAdminView {
    #[serde(flatten)]
    pub user: UserAdminView,
    pub orgs: Vec<OrgAdminView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUser {
    pub name: String,
    pub username: String,
}
