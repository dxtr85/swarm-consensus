use crate::{GnomeId, Neighbor, SwarmName};

pub enum ManagerToGnome {
    // ListNeighboringSwarms,
    ProvideNeighborsToSwarm(SwarmName, GnomeId),
    AddNeighbor(Neighbor),
    SwarmJoined(SwarmName, Vec<GnomeId>),
    Status,
    Disconnect,
}
