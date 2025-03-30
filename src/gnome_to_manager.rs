use std::net::IpAddr;

use crate::GnomeId;
use crate::Nat;
use crate::Neighbor;
use crate::PortAllocationRule;
use crate::SwarmID;
use crate::SwarmName;

#[derive(Debug)]
pub enum GnomeToManager {
    FounderDetermined(SwarmID, SwarmName),
    NeighboringSwarm(SwarmID, GnomeId, SwarmName),
    AddNeighborToSwarm(SwarmID, SwarmName, Neighbor),
    ActiveNeighbors(SwarmID, Vec<GnomeId>),
    PublicAddress(IpAddr, u16, Nat, (PortAllocationRule, i8)),
    Disconnected(SwarmID),
}
