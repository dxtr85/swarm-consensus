// use crate::gnome_id_dispenser;
use crate::message::BlockID;
use crate::message::Header;
use crate::message::Payload;
use crate::neighbor::NeighborResponse;
use crate::neighbor::Neighborhood;
use crate::CastID;
use crate::Data;
use crate::Message;
use crate::Neighbor;
use crate::NeighborRequest;
use crate::NextState;
use crate::Request;
use crate::Response;
use crate::SwarmID;
use crate::SwarmTime;
use crate::DEFAULT_NEIGHBORS_PER_GNOME;
use crate::DEFAULT_SWARM_DIAMETER;

use std::collections::HashSet;
use std::collections::VecDeque;
use std::fmt;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct GnomeId(pub u64);
impl fmt::Display for GnomeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GID-{:x}", self.0)
    }
}

pub struct Gnome {
    pub id: GnomeId,
    pub neighborhood: Neighborhood,
    swarm_id: SwarmID,
    swarm_time: SwarmTime,
    round_start: SwarmTime,
    swarm_diameter: SwarmTime,
    receiver: Receiver<Request>,
    band_receiver: Receiver<u32>,
    sender: Sender<Response>,
    fast_neighbors: Vec<Neighbor>,
    slow_neighbors: Vec<Neighbor>,
    new_neighbors: Vec<Neighbor>,
    refreshed_neighbors: Vec<Neighbor>,
    block_id: BlockID,
    data: Data,
    my_proposed_block: Option<Data>,
    proposals: VecDeque<Data>,
    next_state: NextState,
    timeout_duration: Duration,
    active_unicasts: HashSet<CastID>,
    send_immediate: bool,
}

impl Gnome {
    pub fn new(
        id: GnomeId,
        swarm_id: SwarmID,
        sender: Sender<Response>,
        receiver: Receiver<Request>,
        band_receiver: Receiver<u32>,
    ) -> Self {
        Gnome {
            id,
            neighborhood: Neighborhood(0),
            swarm_id,
            swarm_time: SwarmTime(0),
            round_start: SwarmTime(0),
            swarm_diameter: DEFAULT_SWARM_DIAMETER,
            receiver,
            band_receiver,
            sender,
            fast_neighbors: Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME),
            slow_neighbors: Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME),
            new_neighbors: vec![],
            refreshed_neighbors: Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME),
            block_id: BlockID(0),
            data: Data(0),
            my_proposed_block: None,
            proposals: VecDeque::new(),
            next_state: NextState::new(),
            timeout_duration: Duration::from_millis(500),
            active_unicasts: HashSet::new(),
            send_immediate: false,
        }
    }

    pub fn new_with_neighbors(
        id: GnomeId,
        swarm_id: SwarmID,
        sender: Sender<Response>,
        receiver: Receiver<Request>,
        band_receiver: Receiver<u32>,
        neighbors: Vec<Neighbor>,
    ) -> Self {
        let mut gnome = Gnome::new(id, swarm_id, sender, receiver, band_receiver);
        gnome.fast_neighbors = neighbors;
        gnome
    }

    fn serve_user_requests(&mut self) -> (bool, bool) {
        let mut new_user_proposal = false;
        let mut exit_app = false;
        if let Ok(request) = self.receiver.try_recv() {
            match request {
                Request::Disconnect => exit_app = true,
                Request::Status => {
                    println!(
                        "Status: {} {} {}\t\t neighbors: {}",
                        self.swarm_time,
                        self.block_id,
                        self.neighborhood,
                        self.fast_neighbors.len()
                    );
                }
                Request::AddData(data) => {
                    self.proposals.push_front(data);
                    new_user_proposal = true;
                    println!("vvv USER vvv REQ {}", data);
                }
                Request::AddNeighbor(neighbor) => {
                    println!("{} ADD\tadd a new neighbor", neighbor.id);
                    self.add_neighbor(neighbor);
                }
                Request::DropNeighbor(n_id) => {
                    println!("{} DROP\ta neighbor on user request", n_id);
                    self.drop_neighbor(n_id);
                }
                Request::SendData(gnome_id, request, data) => {
                    println!("Trying to inform neighbor {} about data", gnome_id);
                    let mut data_sent = false;
                    for neighbor in &mut self.fast_neighbors {
                        if neighbor.id == gnome_id {
                            neighbor.add_requested_data(request, data);
                            data_sent = true;
                            break;
                        }
                    }
                    if !data_sent {
                        for neighbor in &mut self.slow_neighbors {
                            if neighbor.id == gnome_id {
                                neighbor.add_requested_data(request, data);
                                break;
                            }
                        }
                    }
                }
                Request::AskData(gnome_id, request) => {
                    let mut data_asked = false;
                    for neighbor in &mut self.fast_neighbors {
                        if neighbor.id == gnome_id {
                            neighbor.request_data(request);
                            data_asked = true;
                            break;
                        }
                    }
                    if !data_asked {
                        for neighbor in &mut self.slow_neighbors {
                            if neighbor.id == gnome_id {
                                neighbor.request_data(request);
                                break;
                            }
                        }
                    }
                }
                Request::StartUnicast(gnome_id) => {
                    // println!("Received StartUnicast {:?}", gnome_id);
                    let mut request_sent = false;
                    let mut avail_ids: [CastID; 256] = [CastID(0); 256];
                    let mut added_ids = 0;
                    for cast_id in self.avail_unicast_ids().into_iter() {
                        avail_ids[added_ids] = cast_id;
                        added_ids += 1;
                    }
                    let request = NeighborRequest::UnicastRequest(self.swarm_id, avail_ids);
                    for neighbor in &mut self.fast_neighbors {
                        if neighbor.id == gnome_id {
                            println!("Sending UnicastRequest to neighbor");
                            neighbor.request_data(request);
                            // let (sender, receiver) = channel();
                            // neighbor.add_unicast(cast_id, sender);
                            // self.pending_unicasts.insert(gnome_id, (cast_id, receiver));
                            request_sent = true;
                            break;
                        }
                    }
                    if !request_sent {
                        for neighbor in &mut self.slow_neighbors {
                            if neighbor.id == gnome_id {
                                request_sent = true;
                                neighbor.request_data(request);
                                // let (sender, receiver) = channel();
                                // neighbor.add_unicast(cast_id, sender);
                                // self.pending_unicasts.insert(gnome_id, (cast_id, receiver));
                                break;
                            }
                        }
                    }
                    if !request_sent {
                        println!("Unable to find gnome with id {}", gnome_id);
                    }
                }
                Request::StartMulticast(_) | Request::StartBroadcast => {
                    todo!()
                }
                Request::Custom(_id, _data) => {
                    todo!()
                }
            }
        }
        (exit_app, new_user_proposal)
    }

    fn is_unicast_id_available(&self, cast_id: CastID) -> bool {
        !self.active_unicasts.contains(&cast_id)
    }

    fn all_possible_unicast_ids(&self) -> HashSet<CastID> {
        let mut ids = HashSet::new();
        for i in 0..=255 {
            let cast_id = CastID(i);
            ids.insert(cast_id);
        }
        ids
    }

    fn avail_unicast_ids(&self) -> HashSet<CastID> {
        let mut available = self.all_possible_unicast_ids();
        for occupied in &self.active_unicasts {
            available.remove(occupied);
        }
        available
    }

    fn serve_neighbors_requests(&mut self) {
        let mut neighbors = std::mem::replace(&mut self.fast_neighbors, Vec::new());
        for neighbor in &mut neighbors {
            if let Some(request) = neighbor.requests.pop_back() {
                println!("Some neighbor request! {:?}", request);
                match request {
                    NeighborRequest::UnicastRequest(_swarm_id, cast_ids) => {
                        for cast_id in cast_ids {
                            if self.is_unicast_id_available(cast_id) {
                                self.active_unicasts.insert(cast_id);
                                neighbor.add_requested_data(
                                    request,
                                    NeighborResponse::Unicast(self.swarm_id, cast_id),
                                );
                                // println!("To user: {:?}", res);
                                // } else {
                                //     if let Some(cast_id) = next_uni_id {
                                //         neighbor.add_requested_data(
                                //             request,
                                //             NeighborResponse::Unicast(self.swarm_id, cast_id),
                                //         );
                                //     } else {
                                //         println!("Unable to find free Unicast ID");
                                //     }
                                break;
                            }
                        }
                    }
                    _ => {
                        let _ = self
                            .sender
                            .send(Response::DataInquiry(neighbor.id, request));
                    }
                }
            }
        }
        let _ = std::mem::replace(&mut self.fast_neighbors, neighbors);
        for neighbor in &mut self.slow_neighbors {
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
        sender: Sender<()>,
        terminator: Option<Sender<()>>,
        // old_handle: Option<JoinHandle<()>>,
    ) -> Sender<()> {
        if let Some(terminator) = terminator {
            // println!("Droping old timeout handle, starting {:?}", duration);
            drop(terminator);
        }
        // println!("droped");
        let (tx, rx) = channel();
        let mut send = true;

        thread::spawn(move || {
            let dur10th = duration / 10;
            for _i in 0..10 {
                thread::yield_now();
                thread::sleep(dur10th);
                match rx.try_recv() {
                    Ok(_) | Err(TryRecvError::Disconnected) => {
                        // println!("Terminating. ");
                        send = false;
                        break;
                    }
                    Err(TryRecvError::Empty) => {
                        // print!(".")
                    }
                }
            }

            if send {
                let _ = sender.send(());
            }
        });
        tx
    }

    pub fn do_your_job(mut self) {
        println!("Waiting for user/network to provide some Neighbors...");
        while self.fast_neighbors.is_empty() && self.slow_neighbors.is_empty() {
            // println!("in while");
            let _ = self.serve_user_requests();
        }
        println!("Have neighbors!");
        let (timer_sender, timeout_receiver) = channel();
        let mut available_bandwith = if let Ok(band) = self.band_receiver.try_recv() {
            band
        } else {
            0
        };
        println!("Avail bandwith: {}", available_bandwith);
        let mut _guard = self.start_new_timer(self.timeout_duration, timer_sender.clone(), None);
        self.send_all();

        loop {
            let (exit_app, _new_user_proposal) = self.serve_user_requests();
            self.serve_neighbors_requests();
            let (fast_advance_to_next_turn, fast_new_proposal) = self.try_recv(true);
            let (slow_advance_to_next_turn, slow_new_proposal) = self.try_recv(false);
            if let Ok(band) = self.band_receiver.try_recv() {
                available_bandwith = band;
                println!("Avail bandwith: {}", available_bandwith);
                //TODO make use of available_bandwith during multicasting setup
            }
            let timeout = timeout_receiver.try_recv().is_ok();
            let advance_to_next_turn = fast_advance_to_next_turn || slow_advance_to_next_turn;
            let new_proposal = fast_new_proposal || slow_new_proposal;
            if advance_to_next_turn
                || self.send_immediate
                || timeout
                    && !(self.fast_neighbors.is_empty()
                        && self.slow_neighbors.is_empty()
                        && self.refreshed_neighbors.is_empty())
            {
                self.update_state();
                if !new_proposal && !self.send_immediate {
                    self.swap_neighbors();
                    self.send_specialized(true);
                    self.send_specialized(false);
                } else {
                    self.concat_neighbors();
                    self.send_all();
                }
                self.send_immediate = false;
                self.check_if_new_round();
                _guard =
                    self.start_new_timer(self.timeout_duration, timer_sender.clone(), Some(_guard));
            }
            if exit_app {
                break;
            };
            thread::sleep(Duration::from_millis(25));
        }
    }

    pub fn add_neighbor(&mut self, neighbor: Neighbor) {
        if self.fast_neighbors.is_empty() && self.slow_neighbors.is_empty() {
            self.fast_neighbors.push(neighbor);
        } else {
            self.new_neighbors.push(neighbor);
        }
    }

    pub fn drop_neighbor(&mut self, neighbor_id: GnomeId) {
        if let Some(index) = self.fast_neighbors.iter().position(|x| x.id == neighbor_id) {
            self.fast_neighbors.remove(index);
        }
        if let Some(index) = self.slow_neighbors.iter().position(|x| x.id == neighbor_id) {
            self.slow_neighbors.remove(index);
        }
        if let Some(index) = self
            .refreshed_neighbors
            .iter()
            .position(|x| x.id == neighbor_id)
        {
            self.refreshed_neighbors.remove(index);
        }
        if let Some(index) = self.new_neighbors.iter().position(|x| x.id == neighbor_id) {
            self.new_neighbors.remove(index);
        }
    }

    pub fn send_all(&mut self) {
        let message = self.prepare_message();
        eprintln!("{} >>> {}", self.id, message);
        for neighbor in &mut self.fast_neighbors {
            neighbor.send_out(message);
        }
        for neighbor in &mut self.slow_neighbors {
            neighbor.send_out(message);
        }
        // for neighbor in &mut self.new_neighbors {
        //     neighbor.send_out(message);
        // }
    }

    pub fn send_specialized(&mut self, fast: bool) {
        let message = self.prepare_message();
        let served_neighbors = Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME);
        let unserved_neighbors = if fast {
            std::mem::replace(&mut self.fast_neighbors, served_neighbors)
        } else {
            std::mem::replace(&mut self.slow_neighbors, served_neighbors)
        };
        let mut generic_info_printed = false;
        for mut neighbor in unserved_neighbors {
            // println!("Trying specialized for {}", neighbor.id);
            if let Some((_req, resp)) = neighbor.get_specialized_data() {
                // println!("Yes: {:?}", resp);
                let payload = Payload::Response(resp);
                // match resp {
                // NeighborResponse::Listing(count, listing) => Payload::Listing(count, listing),
                // NeighborResponse::Block(id, data) => Payload::Block(id, data),
                // NeighborResponse::Unicast(gnome_id, cast_id) => {
                //     println!("{:?} pending: {:?}", gnome_id, self.pending_unicasts);
                //     if let Some((_cast_id_local, receiver)) =
                //         self.pending_unicasts.remove(&gnome_id)
                //     {
                //         //     if cast_id_local.0 == cast_id.0 {
                //         println!("Sending Unicast response to user and neighbor");
                //         let _ = self.sender.send(Response::Unicast(
                //             self.swarm_id,
                //             cast_id,
                //             receiver,
                //         ));
                //         Payload::Unicast(cast_id, Data(0))
                //     // } else if self.is_unicast_id_available(cast_id){
                //     //         println!("CastID {:?}, reserved for {:?}", cast_id, g_id);
                //     //         self.pending_unicasts.insert(cast_id, (swarm_id, g_id));
                //     //         Payload::KeepAlive
                //     //     }
                //     } else {
                //         println!("No pending unicast found");
                //         Payload::KeepAlive
                //     }
                // }
                // };
                let new_message = message.set_payload(payload);
                println!("{} >S> {}", self.id, new_message);
                neighbor.send_out(new_message);
            } else if let Some(request) = neighbor.user_requests.pop_back() {
                let new_message = message.include_request(request);
                println!("{} >S> {}", self.id, new_message);
                neighbor.send_out(new_message);
            } else {
                if !generic_info_printed {
                    eprintln!("{} >>> {}", self.id, message);
                    generic_info_printed = true;
                }
                // println!("snd {}", message);
                neighbor.send_out(message);
            }
            if fast {
                self.fast_neighbors.push(neighbor);
            } else {
                self.slow_neighbors.push(neighbor);
            }
        }
        // for neighbor in &mut self.new_neighbors {
        //     neighbor.send_out(message);
        // }
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
        // println!("prep msg nhood: {}", self.neighborhood);
        Message {
            swarm_time: self.swarm_time,
            neighborhood: self.neighborhood,
            header,
            payload,
        }
    }

    fn swap_neighbors(&mut self) {
        while let Some(neighbor) = self.slow_neighbors.pop() {
            println!(
                "{} DROP\ttimeout \t neighbors: {}, {}",
                neighbor.id,
                self.fast_neighbors.len(),
                self.refreshed_neighbors.len()
            );
            drop(neighbor);
        }
        self.slow_neighbors = std::mem::replace(
            &mut self.fast_neighbors,
            Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME),
        );
        self.fast_neighbors = std::mem::replace(
            &mut self.refreshed_neighbors,
            Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME),
        );
    }

    fn concat_neighbors(&mut self) {
        self.fast_neighbors.append(&mut self.refreshed_neighbors);
    }

    fn update_state(&mut self) {
        let (n_st, n_neigh, n_bid, n_data) = self.next_state.next_params();
        // println!("Next params: {} {} {} {}", n_st, n_neigh, n_bid, n_data);
        if let Some(sub) = n_st.0.checked_sub(self.swarm_time.0) {
            if sub >= self.swarm_diameter.0 + self.swarm_diameter.0 {
                println!("Not updating neighborhood when catching up with swarm");
            } else {
                self.neighborhood = n_neigh;
            }
        } else {
            self.neighborhood = n_neigh;
        }
        self.swarm_time = n_st;
        self.block_id = n_bid;
        self.data = n_data;
    }

    fn check_if_new_round(&mut self) {
        let all_gnomes_aware = self.neighborhood.0 as u32 >= self.swarm_diameter.0;
        let finish_round =
            self.swarm_time - self.round_start >= self.swarm_diameter + self.swarm_diameter;
        if all_gnomes_aware || finish_round {
            let block_zero = BlockID(0);
            if self.block_id > block_zero {
                self.send_immediate = true;
                if all_gnomes_aware {
                    let res = self.sender.send(Response::Block(self.block_id, self.data));
                    println!(
                        "^^^ USER ^^^ {:?} NEW {} {:075}",
                        res, self.block_id, self.data.0
                    );
                    if let Some(my_proposed_data) = self.my_proposed_block {
                        if my_proposed_data.0 != self.block_id.0 {
                            self.proposals.push_back(my_proposed_data);
                        }
                    }
                    self.my_proposed_block = None;

                    self.round_start = self.swarm_time;
                    // println!("New round start: {}", self.round_start);
                    self.next_state.last_accepted_block = self.block_id;
                } else {
                    println!(
                        "ERROR: Swarm diameter too small or {} was backdated!",
                        self.block_id
                    );
                }
                if let Some(data) = self.proposals.pop_back() {
                    // println!("some");
                    self.block_id = BlockID(data.0);
                    self.data = data;
                    self.my_proposed_block = Some(data);
                    self.send_immediate = true;
                } else {
                    // println!("none");
                    self.block_id = block_zero;
                    self.data = Data(0);
                }
            } else {
                // Sync swarm time
                // self.swarm_time = self.round_start + self.swarm_diameter;
                // println!("Sync swarm time {}", self.swarm_time);
                // self.next_state.swarm_time = self.swarm_time;
                self.round_start = self.swarm_time;
                // if self.neighborhood.0 as u32 >= self.swarm_diameter.0 {
                // println!("set N-0");
                self.neighborhood = Neighborhood(0);
                // }
                // self.next_state.all_neighbors_same_header = false;
                // println!("--------round start to: {}", self.swarm_time);
                if let Some(data) = self.proposals.pop_back() {
                    self.block_id = BlockID(data.0);
                    self.data = data;
                    self.my_proposed_block = Some(data);
                    self.send_immediate = true;
                }
                // At start of new round
                // Flush awaiting neighbors
                // We ignore msgs from new neighbors until start of next round

                if !self.new_neighbors.is_empty() {
                    let new_neighbors = std::mem::replace(
                        &mut self.new_neighbors,
                        Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME),
                    );

                    let msg = self.prepare_message();
                    for mut neighbor in new_neighbors {
                        let _ = neighbor.try_recv(self.next_state.last_accepted_block);
                        // println!("snd2 {}", msg);
                        neighbor.send_out(msg);
                        self.fast_neighbors.push(neighbor);
                    }
                }
            }

            self.next_state
                .reset_for_next_turn(true, self.block_id, self.data);
            for neighbor in &mut self.fast_neighbors {
                // println!("fast");
                neighbor.start_new_round(self.swarm_time);
            }
            for neighbor in &mut self.slow_neighbors {
                // println!("slow");
                neighbor.start_new_round(self.swarm_time);
            }
        } else {
            self.next_state
                .reset_for_next_turn(false, self.block_id, self.data);
        }
    }

    fn try_recv(&mut self, fast: bool) -> (bool, bool) {
        let mut looped = false;
        let mut new_proposal_received = false;
        let loop_neighbors = if fast {
            std::mem::replace(
                &mut self.fast_neighbors,
                Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME),
            )
        } else {
            std::mem::replace(
                &mut self.slow_neighbors,
                Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME),
            )
        };
        for mut neighbor in loop_neighbors {
            looped = true;
            while let Some(response) = neighbor.user_responses.pop_back() {
                println!("Got response: {:?}", response);
                let _ = self.sender.send(response);
            }
            let (served, sanity_passed, new_proposal, drop_me) =
                neighbor.try_recv(self.next_state.last_accepted_block);
            // println!(
            // println!("snd {}", pass);
            //     "{} srv:{} pass:{} new:{}",
            //     neighbor.id, served, sanity_passed, new_proposal
            // );
            if !new_proposal_received {
                new_proposal_received = new_proposal;
            }
            if served {
                // println!("Served!");
                if sanity_passed {
                    //TODO: this is wacky
                    if self.round_start.0 == 0 {
                        self.next_state.swarm_time = neighbor.swarm_time;
                    }
                    self.next_state.update(&neighbor);
                    // println!("bifor pusz {:?}", self.next_state);
                    if !drop_me {
                        self.refreshed_neighbors.push(neighbor);
                    } else {
                        println!("Droping disconnected neighbor");
                    }
                } else {
                    println!("Dropping an insane neighbor");
                    continue;
                }
            } else {
                // println!("pusz");
                if !drop_me {
                    if fast {
                        self.fast_neighbors.push(neighbor);
                    } else {
                        self.slow_neighbors.push(neighbor);
                    }
                } else {
                    println!("Dropping neighbor");
                }
            }
        }
        (
            self.fast_neighbors.is_empty() && self.slow_neighbors.is_empty() && looped
                || new_proposal_received,
            new_proposal_received,
        )
    }
}
