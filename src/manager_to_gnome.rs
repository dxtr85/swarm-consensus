use crate::{GnomeId, Neighbor, SwarmName};

pub enum ManagerToGnome {
    // ReplyNetworkSettings(Vec<NetworkSettings>, u8, GnomeId),
    ReplyNetworkSettings(Vec<u8>, u8, GnomeId),
    ProvideNeighborsToSwarm(SwarmName, GnomeId),
    AddNeighbor(Neighbor),
    SwarmJoined(SwarmName, Vec<GnomeId>),
    Status,
    Disconnect,
}
