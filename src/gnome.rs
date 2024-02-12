extern crate chrono;
extern crate timer;
use crate::gnome_id_dispenser;
use crate::Awareness;
use crate::Message;
use crate::Neighbor;
use crate::NextState;
use crate::Proposal;
use crate::Request;
use crate::Response;
use crate::SwarmTime;
use crate::DEFAULT_NEIGHBORS_PER_GNOME;
use crate::DEFAULT_SWARM_DIAMETER;
use std::sync::mpsc::{channel, Receiver, Sender};

#[derive(Clone, Copy, Debug)]
pub struct GnomeId(pub u32);

pub struct Gnome {
    pub id: GnomeId,
    pub awareness: Awareness,
    swarm_time: SwarmTime,
    receiver: Receiver<Request>,
    sender: Sender<Response>,
    neighbors: Vec<Neighbor>,
    new_neighbors: Vec<Neighbor>,
    refreshed_neighbors: Vec<Neighbor>,
    swarm_diameter: u8,
    proposal: Option<Proposal>,
    next_state: NextState,
}

impl Gnome {
    pub fn new(sender: Sender<Response>, receiver: Receiver<Request>) -> Self {
        Gnome {
            id: gnome_id_dispenser(),
            awareness: Awareness::Unaware(SwarmTime(0)),
            swarm_time: SwarmTime(0),
            receiver,
            sender,
            neighbors: Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME),
            new_neighbors: vec![],
            refreshed_neighbors: Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME),
            swarm_diameter: DEFAULT_SWARM_DIAMETER,
            proposal: None,
            next_state: NextState::from_awareness(Awareness::Unaware(SwarmTime(0))),
        }
    }

    pub fn new_with_neighbors(
        sender: Sender<Response>,
        receiver: Receiver<Request>,
        neighbors: Vec<Neighbor>,
    ) -> Self {
        let mut gnome = Gnome::new(sender, receiver);
        gnome.neighbors = neighbors;
        gnome
    }
    // pub fn with_diameter(
    //     swarm_diameter: u8,
    //     sender: Sender<Response>,
    //     receiver: Receiver<Request>,
    // ) -> Self {
    //     Gnome {
    //         id: gnome_id_dispenser(),
    //         awareness: Awareness::Unaware(0),
    //         swarm_time: 0,
    //         receiver,
    //         sender,
    //         neighbors: Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME),
    //         refreshed_neighbors: Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME),
    //         swarm_diameter,
    //         proposal: None,
    //         next_state: NextState::from_awareness(Awareness::Unaware(0)),
    //     }
    // }

    fn serve_user_requests(&mut self) -> bool {
        if let Ok(request) = self.receiver.try_recv() {
            match request {
                Request::Disconnect => return true,
                Request::Status => {
                    println!("{:?}, proposal: {:?}", self.awareness, self.proposal);
                }
                Request::MakeProposal(_proposal) => {}
                Request::AddNeighbor(neighbor) => {
                    self.add_neighbor(neighbor);
                }
            }
        }
        false
    }

    pub fn do_your_job(mut self) {
        let timer = timer::Timer::new();
        let (tx, timeout_receiver) = channel();
        let mut txc = tx.clone();
        let mut _guard = timer.schedule_with_delay(chrono::Duration::seconds(1), move || {
            // This closure is executed on the scheduler thread,
            // so we want to move it away asap.
            let _ignored = txc.send(()); // Avoid unwrapping here.
        });

        // let mut loop_breaker = 10;
        loop {
            // println!("In loop");
            if self.serve_user_requests() {
                // println!("EXIT on user request.");
                break;
            };
            // println!("In loop {loop_breaker}");
            // loop_breaker -= 1;
            // if loop_breaker == 0 {
            // break;
            // }
            let advance_to_next_turn = self.try_recv();
            let timeout = timeout_receiver.try_recv().is_ok();
            if advance_to_next_turn | timeout {
                println!("Advancing to next turn!");
                self.swap_neighbors();
                let sync_neighbors_st = self.update_state();
                // println!("New self: {:?}", self.awareness);
                let message_to_send = self.prepare_message();
                self.send_all(message_to_send);
                if sync_neighbors_st {
                    self.sync_neighbors_time();
                }
                drop(_guard);
                txc = tx.clone();
                _guard = timer.schedule_with_delay(chrono::Duration::seconds(1), move || {
                    let _ignored = txc.send(()); // Avoid unwrapping here.
                });
            } else {
                // println!("Not advancing to next turn.");
            }
        }
        // println!("out of loop.");
    }

    pub fn add_neighbor(&mut self, neighbor: Neighbor) {
        self.new_neighbors.push(neighbor);
    }

    pub fn send_all(&mut self, value: Message) {
        self.neighbors.append(&mut self.new_neighbors);
        for neighbor in &mut self.neighbors {
            let _ = neighbor.sender.send(value);
        }
    }

    fn sync_neighbors_time(&mut self) {
        for neighbor in &mut self.neighbors {
            neighbor.set_swarm_time(self.swarm_time);
        }
    }

    #[inline]
    pub fn set_awareness(&mut self, awareness: Awareness) {
        self.awareness = awareness;
    }

    pub fn prepare_message(&self) -> Message {
        if let Awareness::Aware(_swarm_time, 0, proposal) = self.awareness {
            Message::Proposal(self.awareness, proposal)
        } else {
            Message::KeepAlive(self.awareness)
        }
    }

    fn swap_neighbors(&mut self) {
        // Drop neighbors that are late
        while let Some(neighbor) = self.neighbors.pop() {
            drop(neighbor);
        }
        self.neighbors = std::mem::replace(
            &mut self.refreshed_neighbors,
            Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME),
        );
    }

    fn update_state(&mut self) -> bool {
        let mut sync_neighbors_swarm_time = false;
        self.swarm_time = self.next_state.swarm_time.inc();
        if self.next_state.become_confused {
            self.set_awareness(Awareness::Confused(
                self.swarm_time,
                2 * self.swarm_diameter + 1,
            ));
            self.proposal = None;
            return sync_neighbors_swarm_time;
        }
        if self.next_state.any_confused || self.next_state.any_unaware {
            self.next_state.all_aware = false;
        }
        if self.next_state.any_aware || self.next_state.any_unaware {
            self.next_state.all_confused = false;
        }
        if self.next_state.any_aware || self.next_state.any_confused {
            self.next_state.all_unaware = false;
        }
        if self.next_state.all_unaware {
            if let Awareness::Unaware(_swarm_time) = self.awareness {
                if let Some(proposal) = self.proposal {
                    self.set_awareness(Awareness::Aware(self.swarm_time, 0, proposal));
                }
            } else {
                // Can not update awareness state when all neighbors are unaware
                // return;
            }
        } else if self.next_state.all_aware {
            if self.proposal.is_none() {
                self.proposal = self.next_state.proposal;
            }
            if self.next_state.awareness_diameter >= 2 * self.swarm_diameter - 1 {
                let (proposal_time, proposal_data) = self.proposal.unwrap();
                self.swarm_time =
                    SwarmTime(proposal_time.0 + self.next_state.awareness_diameter as u32 + 1);
                self.set_awareness(Awareness::Unaware(self.swarm_time));
                let _ = self
                    .sender
                    .send(Response::Data(Box::new([proposal_data; 1024])));
                self.proposal = None;
                sync_neighbors_swarm_time = true;
            } else {
                self.set_awareness(Awareness::Aware(
                    self.swarm_time,
                    self.next_state.awareness_diameter + 1,
                    self.proposal.unwrap(),
                ));
            }
        } else if self.next_state.all_confused {
            if self.next_state.confusion_diameter == 1 {
                self.set_awareness(Awareness::Unaware(self.swarm_time));
            } else {
                self.set_awareness(Awareness::Confused(
                    self.swarm_time,
                    self.next_state.confusion_diameter - 1,
                ));
            }
        } else if self.next_state.any_confused {
        } else if self.next_state.any_aware {
            if let Awareness::Unaware(_swarm_time) = self.awareness {
                self.set_awareness(Awareness::Aware(
                    self.swarm_time,
                    0,
                    self.next_state.proposal.unwrap(),
                ))
            }
        }
        self.next_state = NextState::from_awareness(self.awareness);
        sync_neighbors_swarm_time
    }

    fn try_recv(&mut self) -> bool {
        let mut looped = false;
        let loop_neighbors = std::mem::replace(
            &mut self.neighbors,
            Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME),
        );
        for mut neighbor in loop_neighbors {
            looped = true;
            let (served, sanity_passed) = neighbor.try_recv();
            if served {
                if sanity_passed {
                    if let Some(proposal) = neighbor.proposal {
                        if let Some(existing_proposal) = &self.proposal {
                            if proposal.ne(existing_proposal) && self.reject_all_proposals() {
                                // Droping an unsober neighbor
                                continue;
                            }
                        }
                    }

                    // Following will not execute if above `continue` was evaluated
                    self.next_state.update(&neighbor);
                    self.refreshed_neighbors.push(neighbor);
                } else {
                    // Dropping an insane neighbor
                    continue;
                }
            } else {
                self.neighbors.push(neighbor);
            }
        }
        self.neighbors.is_empty() && looped
    }

    fn reject_all_proposals(&self) -> bool {
        match self.awareness {
            Awareness::Unaware(_st) => false,
            Awareness::Confused(_st, _n) => true,
            Awareness::Aware(_st, n, _p) => n >= self.swarm_diameter,
        }
    }
}
