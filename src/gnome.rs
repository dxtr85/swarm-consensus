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
use crate::DEFAULT_NEIGHBORS_PER_GNOME;
use crate::DEFAULT_SWARM_DIAMETER;
use std::sync::mpsc::{channel, Receiver, Sender};

#[derive(Clone, Copy, Debug)]
pub struct GnomeId(pub u32);

pub struct Gnome {
    pub id: GnomeId,
    pub awareness: Awareness,
    turn_number: u32,
    receiver: Receiver<Request>,
    sender: Sender<Response>,
    neighbors: Vec<Neighbor>,
    refreshed_neighbors: Vec<Neighbor>,
    swarm_diameter: u8,
    proposal: Option<Proposal>,
    next_state: NextState,
}

impl Gnome {
    pub fn new(sender: Sender<Response>, receiver: Receiver<Request>) -> Self {
        Gnome {
            id: gnome_id_dispenser(),
            awareness: Awareness::Unaware,
            turn_number: 0,
            receiver,
            sender,
            neighbors: Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME),
            refreshed_neighbors: Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME),
            swarm_diameter: DEFAULT_SWARM_DIAMETER,
            proposal: None,
            next_state: NextState::from_awareness(Awareness::Unaware),
        }
    }

    pub fn with_diameter(
        swarm_diameter: u8,
        sender: Sender<Response>,
        receiver: Receiver<Request>,
    ) -> Self {
        Gnome {
            id: gnome_id_dispenser(),
            awareness: Awareness::Unaware,
            turn_number: 0,
            receiver,
            sender,
            neighbors: Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME),
            refreshed_neighbors: Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME),
            swarm_diameter,
            proposal: None,
            next_state: NextState::from_awareness(Awareness::Unaware),
        }
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
            if let Ok(request) = self.receiver.try_recv() {
                match request {
                    Request::Disconnect => break,
                    Request::MakeProposal(_proposal) => {}
                    Request::AddNeighbor(_neighbor) => {}
                }
            }
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
                self.update_state();
                let message_to_send = self.prepare_message();
                self.send_all(message_to_send);
                drop(_guard);
                txc = tx.clone();
                _guard = timer.schedule_with_delay(chrono::Duration::seconds(1), move || {
                    let _ignored = txc.send(()); // Avoid unwrapping here.
                });
                // TODO: remove below break when time comes
                println!("[{:?},{:?}", &self.id, &self.awareness);
                break;
            }
        }
    }

    pub fn add_neighbor(&mut self, neighbor: Neighbor) {
        self.neighbors.push(neighbor);
    }

    pub fn send_all(&mut self, value: Message) {
        println!("[{:?},{:?}] >> {:?} ", self.id, self.awareness, value);
        // TODO: Construct new Message before sending
        for neighbor in &mut self.neighbors {
            let _ = neighbor.sender.send(value);
        }
        self.turn_number += 1;
    }

    pub fn set_awareness(&mut self, awareness: Awareness) {
        self.awareness = awareness;
    }

    pub fn prepare_message(&self) -> Message {
        if let Awareness::Aware(0, proposal) = self.awareness {
            Message::Proposal(self.id, self.awareness, proposal)
        } else {
            Message::KeepAlive(self.id, self.awareness)
        }
    }

    fn swap_neighbors(&mut self) {
        self.neighbors = std::mem::replace(
            &mut self.refreshed_neighbors,
            Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME),
        );
    }

    fn update_state(&mut self) {
        // let mut proposal_option = Option<M
        // let mut neighbor_awareness = Vec::new();
        if self.next_state.become_confused {
            self.awareness = Awareness::Confused(2 * self.swarm_diameter + 1);
            self.proposal = None;
            return;
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
            if self.awareness == Awareness::Unaware {
                if let Some(proposal) = self.proposal {
                    self.awareness = Awareness::Aware(0, proposal);
                }
            } else {
                // Can not update awareness state when all neighbors are unaware
                // return;
            }
        } else if self.next_state.all_aware {
            // self.proposal = self.next_state.proposal;
            if self.next_state.awareness_diameter >= 2 * self.swarm_diameter {
                self.awareness = Awareness::Unaware;
                self.sender.send(Response::ApprovedProposal(Box::new(
                    [self.proposal.unwrap(); 1024],
                )));
            }
            self.awareness = Awareness::Aware(
                self.next_state.awareness_diameter + 1,
                self.proposal.unwrap(),
            );
        } else if self.next_state.all_confused {
            if self.next_state.confusion_diameter == 1 {
                self.awareness = Awareness::Unaware;
            } else {
                self.awareness = Awareness::Confused(self.next_state.confusion_diameter - 1);
            }
        } else if self.next_state.any_confused {
        } else if self.next_state.any_aware && self.awareness == Awareness::Unaware {
            self.awareness = Awareness::Aware(0, self.next_state.proposal.unwrap())
        }
    }

    fn try_recv(&mut self) -> bool {
        let mut looped = false;
        let loop_neighbors = std::mem::replace(
            &mut self.neighbors,
            Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME),
        );
        for mut neighbor in loop_neighbors {
            looped = true;
            let served = neighbor.try_recv();
            if served {
                if let Some(proposal) = neighbor.proposal {
                    if let Some(existing_proposal) = &self.proposal {
                        if proposal.ne(existing_proposal) && self.reject_all_proposals() {
                            // Droping a neighbor
                            continue;
                        }
                    }
                }
                // Following will not execute if above `continue` was evaluated
                self.next_state.update(&neighbor);
                self.refreshed_neighbors.push(neighbor);
            } else {
                self.neighbors.push(neighbor);
            }
        }
        self.neighbors.is_empty() && looped
    }

    fn reject_all_proposals(&self) -> bool {
        match self.awareness {
            Awareness::Unaware => false,
            Awareness::Confused(_n) => true,
            Awareness::Aware(n, _p) => n >= self.swarm_diameter,
        }
    }
}
