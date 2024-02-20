use crate::gnome_id_dispenser;
use crate::message::BlockID;
use crate::message::Header;
use crate::message::Payload;
use crate::neighbor::Neighborhood;
use crate::Data;
use crate::Message;
use crate::Neighbor;
use crate::NextState;
use crate::Request;
use crate::Response;
use crate::SwarmTime;
use crate::DEFAULT_NEIGHBORS_PER_GNOME;
use crate::DEFAULT_SWARM_DIAMETER;
use std::collections::VecDeque;
use std::fmt;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct GnomeId(pub u32);
impl fmt::Display for GnomeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GID{:9}", self.0)
    }
}

pub struct Gnome {
    pub id: GnomeId,
    pub neighborhood: Neighborhood,
    swarm_time: SwarmTime,
    round_start: SwarmTime,
    swarm_diameter: SwarmTime,
    receiver: Receiver<Request>,
    sender: Sender<Response>,
    neighbors: Vec<Neighbor>,
    new_neighbors: Vec<Neighbor>,
    refreshed_neighbors: Vec<Neighbor>,
    block_id: BlockID,
    data: Data,
    proposals: VecDeque<Data>,
    next_state: NextState,
    timeout_duration: Duration,
}

impl Gnome {
    pub fn new(sender: Sender<Response>, receiver: Receiver<Request>) -> Self {
        Gnome {
            id: gnome_id_dispenser(),
            neighborhood: Neighborhood(0),
            swarm_time: SwarmTime(0),
            round_start: SwarmTime(0),
            swarm_diameter: DEFAULT_SWARM_DIAMETER,
            receiver,
            sender,
            neighbors: Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME),
            new_neighbors: vec![],
            refreshed_neighbors: Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME),
            block_id: BlockID(0),
            data: Data(0),
            proposals: VecDeque::new(),
            next_state: NextState::new(),
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

    fn serve_user_requests(&mut self) -> bool {
        if let Ok(request) = self.receiver.try_recv() {
            match request {
                Request::Disconnect => return true,
                Request::Status => {
                    println!(
                        "Status: {} {} {}\t\t neighbors: {}",
                        self.swarm_time,
                        self.block_id,
                        self.neighborhood,
                        self.neighbors.len()
                    );
                }
                Request::AddData(data) => {
                    self.proposals.push_front(data);
                    println!("vvv USER vvv REQ {}", data);
                }
                Request::AddNeighbor(neighbor) => {
                    println!("{} ADD\tadd a new neighbor", neighbor.id);
                    self.add_neighbor(neighbor);
                }
                Request::SendData(gnome_id, request, data) => {
                    println!("Trying to inform neighbor {} about data", gnome_id);
                    for neighbor in &mut self.neighbors {
                        if neighbor.id == gnome_id {
                            neighbor.add_requested_data(request, data);
                            break;
                        }
                    }
                }
                Request::AskData(gnome_id, request) => {
                    for neighbor in &mut self.neighbors {
                        if neighbor.id == gnome_id {
                            neighbor.request_data(request);
                            break;
                        }
                    }
                }
            }
        }
        false
    }

    fn serve_neighbors_requests(&mut self) {
        for neighbor in &mut self.neighbors {
            if let Some(request) = neighbor.requests.pop_back() {
                println!("Some neighbor request!");
                let _ = self
                    .sender
                    .send(Response::DataInquiry(neighbor.id, request));
            }
        }
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
            self.serve_neighbors_requests();
            let (advance_to_next_turn, new_proposal) = self.try_recv();
            let timeout = timeout_receiver.try_recv().is_ok();
            if advance_to_next_turn
                || timeout && !(self.neighbors.is_empty() || self.refreshed_neighbors.is_empty())
            {
                // println!(
                //     "neigh: {}, ref: {}",
                //     self.neighbors.len(),
                //     self.refreshed_neighbors.len()
                // );
                // println!(
                //     "After traj{}|{}|{}: {} {}",
                //     self.neighbors.len(),
                //     self.refreshed_neighbors.len(),
                //     timeout,
                //     advance_to_next_turn,
                //     new_proposal
                // );
                self.update_state();
                if !new_proposal {
                    // println!("nat nju");
                    self.swap_neighbors();
                    self.send_specialized();
                } else {
                    // println!("nju");
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
        println!("{} >>> {}", self.id, message);
        for neighbor in &mut self.neighbors {
            neighbor.send_out(message);
        }
        for neighbor in &mut self.new_neighbors {
            neighbor.send_out(message);
        }
    }

    pub fn send_specialized(&mut self) {
        let message = self.prepare_message();
        let served_neighbors = Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME);
        let unserved_neighbors = std::mem::replace(&mut self.neighbors, served_neighbors);
        for mut neighbor in unserved_neighbors {
            // println!("Trying specialized for {}", neighbor.id);
            if let Some((_req, resp)) = neighbor.get_specialized_data() {
                // println!("Yes: {}", resp);
                let payload = match resp {
                    crate::neighbor::NeighborResponse::Listing(count, listing) => {
                        Payload::Listing(count, listing)
                    }
                    crate::neighbor::NeighborResponse::Block(id, data) => Payload::Block(id, data),
                };
                let new_message = message.set_payload(payload);
                println!("{} >S> {}", self.id, new_message);
                neighbor.send_out(new_message);
            } else if let Some(request) = neighbor.user_requests.pop_back() {
                let new_message = message.include_request(request);
                println!("{} >S> {}", self.id, new_message);
                neighbor.send_out(new_message);
            } else {
                println!("{} >>> {}", self.id, message);
                neighbor.send_out(message);
            }
            self.neighbors.push(neighbor);
        }
        for neighbor in &mut self.new_neighbors {
            neighbor.send_out(message);
        }
    }

    pub fn prepare_message(&self) -> Message {
        let (header, payload) = if self.block_id.0 == 0 {
            (Header::Sync, Payload::KeepAlive)
        } else {
            (
                Header::Block(self.block_id),
                Payload::Block(self.block_id, self.data),
            )
        };
        Message {
            swarm_time: self.swarm_time,
            neighborhood: self.neighborhood,
            header,
            payload,
        }
    }

    fn swap_neighbors(&mut self) {
        while let Some(neighbor) = self.neighbors.pop() {
            println!(
                "{} DROP\ttimeout \t neighbors: {}",
                neighbor.id,
                self.neighbors.len()
            );
            drop(neighbor);
        }
        self.neighbors = std::mem::replace(
            &mut self.refreshed_neighbors,
            Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME),
        );
        // println!("After słap: {}", self.neighbors.len());
    }

    fn concat_neighbors(&mut self) {
        self.neighbors.append(&mut self.refreshed_neighbors);
        // println!("After konkat: {}", self.neighbors.len());
    }

    fn update_state(&mut self) {
        let (n_st, n_neigh, n_bid, n_data) = self.next_state.next_params();
        // println!("Next params: {} {} {} {}", n_st, n_neigh, n_bid, n_data);
        self.swarm_time = n_st;
        self.neighborhood = n_neigh;
        self.block_id = n_bid;
        self.data = n_data;
        let all_gnomes_aware = n_neigh.0 as u32 >= self.swarm_diameter.0;
        let finish_round =
            self.swarm_time - self.round_start >= self.swarm_diameter + self.swarm_diameter;
        if all_gnomes_aware || finish_round {
            let block_zero = BlockID(0);
            if n_bid > block_zero {
                if all_gnomes_aware {
                    let _ = self.sender.send(Response::Block(n_bid, n_data));
                    println!("^^^ USER ^^^ NEW {} {:075}", n_bid, n_data.0);
                } else {
                    println!(
                        "ERROR: Swarm diameter too small or {} was backdated!",
                        self.block_id
                    );
                }
                if let Some(data) = self.proposals.pop_back() {
                    self.block_id = BlockID(data.0);
                    self.data = data;
                } else {
                    self.block_id = block_zero;
                    self.data = Data(0);
                }
            } else {
                // Sync swarm time
                self.swarm_time = self.round_start + self.swarm_diameter;
                if let Some(data) = self.proposals.pop_back() {
                    self.block_id = BlockID(data.0);
                    self.data = data;
                }
            }
            self.neighborhood = Neighborhood(0);
            self.round_start = self.swarm_time;

            // At start of new round
            // Flush awaiting neighbors
            for neighbor in &mut self.new_neighbors {
                let _ = neighbor.try_recv();
            }
            // Add new_neighbors
            self.refreshed_neighbors.append(&mut self.new_neighbors);
            self.next_state
                .reset_for_next_turn(true, self.block_id, self.data);
            // println!("\n\ntutaj {}\n\n", self.refreshed_neighbors.len());
            for neighbor in &mut self.refreshed_neighbors {
                neighbor.start_new_round(self.swarm_time);
            }
        } else {
            self.next_state
                .reset_for_next_turn(false, self.block_id, self.data);
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
            if let Some(response) = neighbor.user_responses.pop_back() {
                let _ = self.sender.send(response);
            }
            let (served, sanity_passed, new_proposal) = neighbor.try_recv();
            // println!(
            //     "{} srv:{} pass:{} new:{}",
            //     neighbor.id, served, sanity_passed, new_proposal
            // );
            if !new_proposal_received {
                new_proposal_received = new_proposal;
            }
            if served {
                if sanity_passed {
                    // if let Some(proposal) = neighbor.data_id {
                    //     if let Some(existing_proposal) = &self.data_id {
                    //         if proposal.ne(existing_proposal) && self.reject_all_proposals() {
                    //             println!("Droping an unsober neighbor");
                    //             continue;
                    //         }
                    //     }
                    // }

                    // Following will not execute if above `continue` was evaluated
                    // println!("bifor updejt {:?} {:?}", neighbor.header, neighbor.payload);
                    self.next_state.update(&neighbor);
                    // println!("bifor pusz {:?}", self.next_state);
                    self.refreshed_neighbors.push(neighbor);
                } else {
                    println!("Dropping an insane neighbor");
                    continue;
                }
            } else {
                // println!("pusz");
                self.neighbors.push(neighbor);
            }
        }
        // if new_proposal_received {
        //     println!(
        //         "neigh: {} {} {:?}",
        //         self.neighbors.len(),
        //         looped,
        //         new_proposal_received
        //     );
        // }
        (
            self.neighbors.is_empty() && looped || new_proposal_received,
            new_proposal_received,
        )
    }
}
