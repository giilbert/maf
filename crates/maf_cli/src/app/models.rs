use uuid::Uuid;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct App {
    pub id: Uuid,
    pub name: String,
    pub config: Option<String>,
    pub api_client_id: String,
    pub api_secret: String,
}
