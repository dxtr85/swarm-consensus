use crate::{GnomeId, Neighbor, NetworkSettings, SwarmName};

pub enum ManagerToGnome {
    ReplyNetworkSettings(Vec<NetworkSettings>, u8, GnomeId),
    ProvideNeighborsToSwarm(SwarmName, GnomeId),
    AddNeighbor(Neighbor),
    SwarmJoined(SwarmName, Vec<GnomeId>),
    Status,
    Disconnect,
}
