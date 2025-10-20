use crate::ByteSet;
use crate::Capabilities;
use crate::Policy;
use crate::Requirement;
use std::collections::HashSet;

use crate::GnomeId;
// use crate::Nat;
use crate::Neighbor;
// use crate::PortAllocationRule;
use crate::SwarmID;
use crate::SwarmName;

#[derive(Debug)]
pub enum GnomeToManager {
    FounderDetermined(SwarmID, SwarmName),
    NeighboringSwarms(SwarmID, HashSet<(GnomeId, SwarmName)>),
    AddNeighborToSwarm(SwarmID, SwarmName, Neighbor),
    ActiveNeighbors(SwarmID, SwarmName, HashSet<GnomeId>),
    GnomeLeft(SwarmID, SwarmName, GnomeId),
    RunningPolicies(Vec<(Policy, Requirement)>),
    RunningCapabilities(Vec<(Capabilities, Vec<GnomeId>)>),
    RunningByteSets(Vec<(u8, ByteSet)>),
    ProvidePublicAddress(SwarmID, u8, GnomeId),
    SwarmBusy(SwarmID, bool),
    Disconnected(SwarmID, SwarmName),
}
