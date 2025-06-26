use std::collections::HashSet;
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
    NeighboringSwarms(SwarmID, HashSet<(GnomeId, SwarmName)>),
    AddNeighborToSwarm(SwarmID, SwarmName, Neighbor),
    ActiveNeighbors(SwarmID, SwarmName, HashSet<GnomeId>),
    GnomeLeft(SwarmID, SwarmName, GnomeId),
    // PublicAddress(IpAddr, u16, Nat, (PortAllocationRule, i8)),
    ProvidePublicAddress(SwarmID, u8, GnomeId),
    SwarmBusy(SwarmID, bool),
    Disconnected(SwarmID, SwarmName),
}
