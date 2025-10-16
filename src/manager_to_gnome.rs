use crate::{ByteSet, Capabilities, CastData, GnomeId, Neighbor, Policy, Requirement, SwarmName};

pub enum ManagerToGnome {
    // ReplyNetworkSettings(Vec<NetworkSettings>, u8, GnomeId),
    ReplyNetworkSettings(Vec<u8>, u8, GnomeId),
    ProvideNeighborsToSwarm(SwarmName, GnomeId),
    AddNeighbor(Neighbor),
    SwarmJoined(SwarmName, Vec<GnomeId>),
    Status,
    SetRunningPolicy(Policy, Requirement),
    SetRunningCapability(Capabilities, Vec<GnomeId>),
    SetRunningByteSet(u8, ByteSet),
    SendCustom(bool, GnomeId, u8, CastData), // bool = is_neighbor_request
    Disconnect,
}
