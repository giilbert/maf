// TODO:
pub struct UserApp {
    pub room_creation_strategy: RoomCreationStrategy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomCreationStrategy {
    /// Auto-create a room and put everyone in it
    AutoCreate,
}
