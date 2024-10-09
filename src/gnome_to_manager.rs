use crate::GnomeId;
use crate::Neighbor;
use crate::SwarmID;

#[derive(Debug)]
pub enum GnomeToManager {
    NeighboringSwarm(SwarmID, GnomeId, String),
    AddNeighborToSwarm(SwarmID, String, Neighbor),
    Disconnected,
}
