use crate::{GnomeId, Neighbor};

pub enum ManagerToGnome {
    ProvideNeighborsToSwarm(String, GnomeId),
    AddNeighbor(Neighbor),
    SwarmJoined(String),
    Status,
    Disconnect,
}
