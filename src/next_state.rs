use std::collections::HashSet;

use crate::gnome::GnomeId;
use crate::Awareness;
use crate::Neighbor;
use crate::ProposalData;
use crate::ProposalID;
use crate::SwarmTime;
use crate::DEFAULT_SWARM_DIAMETER;

#[derive(Debug)]
pub struct NextState {
    pub awareness: Awareness,
    pub become_confused: bool,
    pub awareness_diameter: u8, // = 255;
    pub confusion_diameter: u8, // = 0;
    pub swarm_time: SwarmTime,
    pub swarm_time_min: SwarmTime,
    pub proposal_id: Option<ProposalID>,
    pub proposal_data: ProposalData,
    confused_neighbors: HashSet<GnomeId>,
    aware_neighbors: HashSet<GnomeId>,
    unaware_neighbors: HashSet<GnomeId>,
}

impl NextState {
    pub fn new(swarm_time_min: SwarmTime, awareness: Awareness) -> Self {
        NextState {
            awareness,
            become_confused: false,
            awareness_diameter: 255,
            confusion_diameter: 0,
            swarm_time: SwarmTime(std::u32::MAX),
            swarm_time_min,
            proposal_id: None,
            proposal_data: ProposalData(0),
            confused_neighbors: HashSet::new(),
            aware_neighbors: HashSet::new(),
            unaware_neighbors: HashSet::new(),
        }
    }

    // pub fn from(swarm_time_min: SwarmTime, awareness: Awareness, other_state: NextState) -> Self {
    //     NextState {
    //         awareness,
    //         swarm_time_min,
    //         ..other_state
    //     }
    // }

    pub fn update(&mut self, neighbor: &Neighbor) {
        match neighbor.awareness {
            Awareness::Unaware => {
                self.unaware_neighbors.insert(neighbor.id);
                self.aware_neighbors.remove(&neighbor.id);
                self.confused_neighbors.remove(&neighbor.id);
                if neighbor.swarm_time < self.swarm_time {
                    self.swarm_time = neighbor.swarm_time;
                }
            }
            Awareness::Aware(aware_neighborhood) => {
                if neighbor.swarm_time < self.swarm_time {
                    self.swarm_time = neighbor.swarm_time;
                }
                self.aware_neighbors.insert(neighbor.id);
                self.unaware_neighbors.remove(&neighbor.id);
                self.confused_neighbors.remove(&neighbor.id);
                if let Some(p) = self.proposal_id {
                    if p != neighbor.proposal_id.unwrap() {
                        self.become_confused = true;
                        self.confusion_diameter = DEFAULT_SWARM_DIAMETER * 2;
                        self.awareness = Awareness::Confused(self.confusion_diameter);
                    }
                } else {
                    self.proposal_id = neighbor.proposal_id; //TODO, maybe check if there is Some?
                    self.proposal_data = neighbor.proposal_data;
                    self.awareness = Awareness::Aware(0);
                    if aware_neighborhood < self.awareness_diameter {
                        self.awareness_diameter = aware_neighborhood;
                    }
                }
            }
            Awareness::Confused(confusion_neighborhood) => {
                if neighbor.swarm_time < self.swarm_time {
                    self.swarm_time = neighbor.swarm_time;
                }
                if confusion_neighborhood > self.confusion_diameter {
                    self.confusion_diameter = confusion_neighborhood;
                }
                self.confused_neighbors.insert(neighbor.id);
                self.aware_neighbors.remove(&neighbor.id);
                self.unaware_neighbors.remove(&neighbor.id);
            }
        }
    }

    pub fn next_swarm_time(&self) -> SwarmTime {
        if self.swarm_time.0 == std::u32::MAX {
            self.swarm_time_min.inc()
        } else {
            self.swarm_time.inc()
        }
    }
    pub fn all_confused(&self) -> bool {
        self.unaware_neighbors.is_empty()
            && self.aware_neighbors.is_empty()
            && !self.confused_neighbors.is_empty()
    }
    pub fn all_aware(&self) -> bool {
        !self.aware_neighbors.is_empty()
            && self.unaware_neighbors.is_empty()
            && self.confused_neighbors.is_empty()
    }
    pub fn all_unaware(&self) -> bool {
        !self.unaware_neighbors.is_empty()
            && self.aware_neighbors.is_empty()
            && self.confused_neighbors.is_empty()
    }
    pub fn any_confused(&self) -> bool {
        !self.confused_neighbors.is_empty()
    }
    pub fn any_aware(&self) -> bool {
        !self.aware_neighbors.is_empty()
    }
    // pub fn any_unaware(&self) -> bool {
    //     !self.unaware_neighbors.is_empty()
    // }
}
