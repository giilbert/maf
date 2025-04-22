use uuid::Uuid;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct App {
    pub id: Uuid,
    pub name: String,
}
