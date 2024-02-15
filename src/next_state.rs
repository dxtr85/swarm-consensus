use crate::gnome::GnomeId;
use crate::Awareness;
use crate::Neighbor;
use crate::ProposalData;
use crate::SwarmTime;
use crate::DEFAULT_SWARM_DIAMETER;

#[derive(Debug)]
pub struct NextState {
    pub awareness: Awareness,
    pub become_confused: bool,
    pub all_confused: bool,     // = true;
    pub all_unaware: bool,      // = true;
    pub all_aware: bool,        // = true;
    pub any_confused: bool,     // = false;
    pub any_unaware: bool,      // = false;
    pub any_aware: bool,        //= false;
    pub awareness_diameter: u8, // = 255;
    pub confusion_diameter: u8, // = 0;
    pub swarm_time: SwarmTime,
    pub swarm_time_min: SwarmTime,
    pub proposal_id: Option<(SwarmTime, GnomeId)>,
    pub proposal_data: ProposalData,
}

impl NextState {
    pub fn from(swarm_time_min: SwarmTime, awareness: Awareness) -> Self {
        NextState {
            awareness,
            become_confused: false,
            all_confused: true,
            all_unaware: true,
            all_aware: true,
            any_confused: false,
            any_unaware: false,
            any_aware: false,
            awareness_diameter: 255,
            confusion_diameter: 0,
            swarm_time: SwarmTime(std::u32::MAX),
            swarm_time_min,
            proposal_id: None,
            proposal_data: ProposalData(0),
        }
    }

    pub fn update(&mut self, neighbor: &Neighbor) {
        match neighbor.awareness {
            Awareness::Unaware => {
                self.any_unaware = true;
                if neighbor.swarm_time < self.swarm_time {
                    self.swarm_time = neighbor.swarm_time;
                }
            }
            Awareness::Aware(
                // swarm_time,
                aware_neighborhood, // , proposal
            ) => {
                if neighbor.swarm_time < self.swarm_time {
                    self.swarm_time = neighbor.swarm_time;
                }
                self.any_aware = true;
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
                self.any_confused = true;
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
}
