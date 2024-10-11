use crate::GnomeId;
use crate::Neighbor;
use crate::SwarmID;
use crate::SwarmName;

#[derive(Debug)]
pub enum GnomeToManager {
    FounderDetermined(SwarmID, GnomeId),
    NeighboringSwarm(SwarmID, GnomeId, SwarmName),
    AddNeighborToSwarm(SwarmID, SwarmName, Neighbor),
    ActiveNeighbors(SwarmID, Vec<GnomeId>),
    Disconnected,
}
