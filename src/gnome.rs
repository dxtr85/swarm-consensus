use crate::gnome_id_dispenser;
use crate::Awareness;
use crate::Data;
use crate::Message;
use crate::Neighbor;
use crate::NextState;
use crate::ProposalData;
use crate::Request;
use crate::Response;
use crate::SwarmTime;
use crate::DEFAULT_NEIGHBORS_PER_GNOME;
use crate::DEFAULT_SWARM_DIAMETER;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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
    proposal_id: Option<(SwarmTime, GnomeId)>,
    proposal_data: ProposalData,
    next_state: NextState,
    timeout_duration: Duration,
}

impl Gnome {
    pub fn new(sender: Sender<Response>, receiver: Receiver<Request>) -> Self {
        Gnome {
            id: gnome_id_dispenser(),
            awareness: Awareness::Unaware,
            swarm_time: SwarmTime(0),
            receiver,
            sender,
            neighbors: Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME),
            new_neighbors: vec![],
            refreshed_neighbors: Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME),
            swarm_diameter: DEFAULT_SWARM_DIAMETER,
            proposal_id: None,
            proposal_data: ProposalData(0),
            next_state: NextState::from(SwarmTime(0), Awareness::Unaware),
            timeout_duration: Duration::from_millis(500),
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
                    let mut propid = "".to_string();
                    if let Some((p_time, p_gid)) = self.proposal_id {
                        propid = format!("PropID-{}-{}", p_time.0, p_gid.0);
                    }
                    println!(
                        "{:10} {:?} {:?}, neighbors: {}",
                        self.swarm_time.0,
                        self.awareness,
                        propid,
                        self.neighbors.len()
                    );
                }
                Request::MakeProposal(_proposal) => {}
                Request::AddNeighbor(neighbor) => {
                    self.add_neighbor(neighbor);
                }
                Request::SendData(gnome_id, request, data) => {
                    for neighbor in &mut self.neighbors {
                        if neighbor.id == gnome_id {
                            neighbor.add_requested_data(request, data);
                            break;
                        }
                    }
                }
            }
        }
        false
    }

    fn start_new_timer(
        &self,
        duration: Duration,
        sender: Sender<String>,
        old_handle: Option<JoinHandle<()>>,
    ) -> JoinHandle<()> {
        if let Some(old_handle) = old_handle {
            drop(old_handle);
        }

        thread::spawn(move || {
            thread::sleep(duration);
            if let Ok(()) = sender.send("Time out".to_string()) {}
        })
    }
    pub fn do_your_job(mut self) {
        let (timer_sender, timeout_receiver) = channel();
        let mut _guard = self.start_new_timer(self.timeout_duration, timer_sender.clone(), None);

        loop {
            if self.serve_user_requests() {
                // println!("EXIT on user request.");
                break;
            };
            let (advance_to_next_turn, new_proposal) = self.try_recv();
            let timeout = timeout_receiver.try_recv().is_ok();
            if advance_to_next_turn | timeout {
                self.update_state();
                if !new_proposal {
                    self.swap_neighbors();
                    self.send_specialized();
                } else {
                    self.concat_neighbors();
                    self.send_all();
                }
                _guard =
                    self.start_new_timer(self.timeout_duration, timer_sender.clone(), Some(_guard));
            }
        }
        // println!("out of loop.");
    }

    pub fn add_neighbor(&mut self, neighbor: Neighbor) {
        self.new_neighbors.push(neighbor);
    }

    pub fn send_all(&mut self) {
        let message = self.prepare_message();
        println!("{:?} >  {}", self.id, message);
        for neighbor in &self.neighbors {
            let _ = neighbor.sender.send(message);
        }
    }

    pub fn send_specialized(&mut self) {
        let message = self.prepare_message();
        println!("{:?} >  {}", self.id, message);
        let served_neighbors = Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME);
        let unserved_neighbors = std::mem::replace(&mut self.neighbors, served_neighbors);
        for mut neighbor in unserved_neighbors {
            if let Some((request, response)) = neighbor.get_specialized_data() {
                let new_message = message.include(request, response);
                let _ = neighbor.sender.send(new_message);
            } else {
                let _ = neighbor.sender.send(message);
            }
            self.neighbors.push(neighbor);
        }
        self.neighbors.append(&mut self.new_neighbors);
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
        // println!("Preparing message: {:?}", self.awareness);
        let data: Data = if let Awareness::Aware(_) = self.awareness {
            Data::Proposal(
                self.proposal_id.unwrap().0,
                self.proposal_id.unwrap().1,
                self.proposal_data,
            )
        } else {
            Data::KeepAlive
        };
        Message {
            swarm_time: self.swarm_time,
            awareness: self.awareness,
            data,
        }
    }

    fn swap_neighbors(&mut self) {
        while let Some(neighbor) = self.neighbors.pop() {
            println!("Drop neighbor {:?} that was late", neighbor.id);
            drop(neighbor);
        }
        self.neighbors = std::mem::replace(
            &mut self.refreshed_neighbors,
            Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME),
        );
    }

    fn concat_neighbors(&mut self) {
        self.neighbors.append(&mut self.refreshed_neighbors);
    }

    fn update_state(&mut self) {
        let mut sync_neighbors_swarm_time = false;
        self.swarm_time = self.next_state.next_swarm_time();
        // println!("Next swarm time: {:?}", self.swarm_time);
        if self.next_state.become_confused {
            self.set_awareness(Awareness::Confused(self.swarm_diameter));
            self.proposal_id = None;
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
            if let Awareness::Unaware = self.awareness {
                // TODO add check if there was confusion just before
                if self.proposal_id.is_some() {
                    self.set_awareness(Awareness::Aware(0));
                }
            } else {
                println!("All neighbors unaware, but me: {:?}", self.awareness);
                // Can not update awareness state when all neighbors are unaware
                // return;
            }
        } else if self.next_state.all_aware {
            // println!("All aware!");
            if self.awareness.is_unaware() {
                self.proposal_id = self.next_state.proposal_id;
                self.proposal_data = self.next_state.proposal_data;
            }
            self.set_awareness(Awareness::Aware(
                // self.swarm_time,
                self.next_state.awareness_diameter + 1,
                // self.next_state.proposal.unwrap(),
            ));
            let proposal_time = self.proposal_id.unwrap().0;
            let neighborhood = self.awareness.neighborhood().unwrap();
            if neighborhood >= self.swarm_diameter
                || self.swarm_time - proposal_time >= (2 * self.swarm_diameter as u32)
            {
                if neighborhood >= self.swarm_diameter {
                    // SYNC swarm_time
                    self.swarm_time = SwarmTime(
                        proposal_time.0 + 2 * (self.next_state.awareness_diameter as u32 + 1),
                    );
                    self.set_awareness(Awareness::Unaware);
                    let _ = self.sender.send(Response::Data(self.proposal_data));
                    // self.proposal_data = ProposalData(0);
                    self.proposal_id = None;
                    sync_neighbors_swarm_time = true;
                } else {
                    println!("ERROR: Swarm diameter too small or Proposal was backdated!");
                    self.proposal_id = None;
                    self.set_awareness(Awareness::Unaware);
                }
            } else {
                self.set_awareness(Awareness::Aware(
                    // self.swarm_time,
                    self.next_state.awareness_diameter + 1,
                    // self.proposal.unwrap(),
                ));
            }
        } else if self.next_state.all_confused {
            if self.next_state.confusion_diameter == 0 {
                self.set_awareness(Awareness::Unaware);
            } else {
                self.set_awareness(Awareness::Confused(
                    // self.swarm_time,
                    self.next_state.confusion_diameter - 1,
                ));
            }
        } else if self.next_state.any_confused {
            self.set_awareness(Awareness::Confused(self.swarm_diameter));
        } else if self.next_state.any_aware {
            if let Awareness::Unaware = self.awareness {
                self.set_awareness(Awareness::Aware(0));
                // self.next_state.proposal.unwrap()))
            }
        }
        self.next_state = NextState::from(self.swarm_time, self.awareness);
        if sync_neighbors_swarm_time {
            self.sync_neighbors_time();
        }
    }

    fn try_recv(&mut self) -> (bool, bool) {
        let mut looped = false;
        let mut new_proposal_received = false;
        let loop_neighbors = std::mem::replace(
            &mut self.neighbors,
            Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME),
        );
        for mut neighbor in loop_neighbors {
            looped = true;
            let (served, sanity_passed, new_proposal) =
                neighbor.try_recv(self.awareness, self.proposal_id);
            if !new_proposal_received {
                new_proposal_received = new_proposal;
            }
            if served {
                if sanity_passed {
                    if let Some(proposal) = neighbor.proposal_id {
                        if let Some(existing_proposal) = &self.proposal_id {
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
        (
            self.neighbors.is_empty() && looped || new_proposal_received,
            new_proposal_received,
        )
    }

    fn reject_all_proposals(&self) -> bool {
        match self.awareness {
            Awareness::Unaware => false,
            Awareness::Confused(_) => true,
            Awareness::Aware(n) => n >= self.swarm_diameter,
        }
    }
}
