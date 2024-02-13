use crate::Awareness;
use crate::Neighbor;
use crate::Proposal;
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
    pub proposal: Option<Proposal>,
}

impl NextState {
    pub fn from_awareness(awareness: Awareness) -> Self {
        let swarm_time_min = match awareness {
            Awareness::Unaware(st) => st,
            Awareness::Aware(st, _pd, _p) => st,
            Awareness::Confused(st, _p) => st,
        };
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
            proposal: None,
        }
    }

    pub fn update(&mut self, neighbor: &Neighbor) {
        match neighbor.awareness {
            Awareness::Unaware(swarm_time) => {
                self.any_unaware = true;
                if swarm_time < self.swarm_time {
                    self.swarm_time = swarm_time;
                }
            }
            Awareness::Aware(swarm_time, aware_neighborhood, proposal) => {
                if swarm_time < self.swarm_time {
                    self.swarm_time = swarm_time;
                }
                self.any_aware = true;
                if let Some(p) = self.proposal {
                    if p != proposal {
                        self.become_confused = true;
                        self.confusion_diameter = DEFAULT_SWARM_DIAMETER * 2;
                        self.awareness =
                            Awareness::Confused(self.swarm_time, self.confusion_diameter);
                    }
                } else {
                    self.proposal = Some(proposal);
                    self.awareness = Awareness::Aware(self.swarm_time, 0, proposal);
                    if aware_neighborhood < self.awareness_diameter {
                        self.awareness_diameter = aware_neighborhood;
                    }
                }
            }
            Awareness::Confused(swarm_time, confusion_neighborhood) => {
                if swarm_time < self.swarm_time {
                    self.swarm_time = swarm_time;
                }
                if confusion_neighborhood > self.confusion_diameter {
                    self.confusion_diameter = confusion_neighborhood;
                }
                self.any_confused = true;
            }
        }
    }

    pub fn next_swarm_time(&self)-> SwarmTime{
        if self.swarm_time.0 == std::u32::MAX{
            self.swarm_time_min.inc()
        }else{
            self.swarm_time.inc()
        }
    }
}
