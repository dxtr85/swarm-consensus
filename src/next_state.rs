use crate::Awareness;
use crate::Proposal;
use crate::Neighbor;
use crate::DEFAULT_SWARM_DIAMETER;

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
    pub proposal: Option<Proposal>,
}

impl NextState {
    pub fn from_awareness(awareness: Awareness) -> Self {
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
            proposal: None,
        }
    }

    pub fn update(&mut self, neighbor: &Neighbor) {
        match neighbor.awareness {
            Awareness::Unaware => {
                self.any_unaware = true;
            }
            Awareness::Aware(aware_neighborhood, proposal) => {
                self.any_aware = true;
                if let Some(p) = self.proposal {
                    if p != proposal {
                        self.become_confused = true;
                        self.confusion_diameter = DEFAULT_SWARM_DIAMETER * 2;
                        self.awareness = Awareness::Confused(self.confusion_diameter);
                    }
                } else {
                    self.proposal = Some(proposal);
                    self.awareness = Awareness::Aware(0, proposal);
                    if aware_neighborhood < self.awareness_diameter {
                        self.awareness_diameter = aware_neighborhood;
                    }
                }
            }
            Awareness::Confused(_confusion_neighborhood) => {
                self.any_confused = true;
            }
        }
    }
}
