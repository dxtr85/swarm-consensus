use crate::message::BlockID;
use crate::message::Header;
use crate::message::Payload;
use crate::CastID;
use crate::Data;
use crate::GnomeId;
use crate::Message;
use crate::NetworkSettings;
use crate::Response;
use crate::SwarmID;
use crate::SwarmTime;
use std::collections::HashMap;
use std::fmt::Display;

use std::collections::VecDeque;
use std::sync::mpsc::channel;
use std::sync::mpsc::{Receiver, Sender};

#[derive(Clone, Copy, Debug)]
pub struct Neighborhood(pub u8);

impl Neighborhood {
    pub fn inc(&self) -> Self {
        Neighborhood(self.0 + 1)
    }
}

impl Display for Neighborhood {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "N-{}", self.0)
    }
}

#[derive(Debug)]
pub struct Neighbor {
    pub id: GnomeId,
    receiver: Receiver<Message>,
    pub sender: Sender<Message>,
    round_start: SwarmTime,
    pub swarm_time: SwarmTime,
    pub swarm_diameter: SwarmTime,
    pub neighborhood: Neighborhood,
    pub prev_neighborhood: Option<Neighborhood>,
    pub header: Header,
    pub payload: Payload,
    pub user_requests: VecDeque<NeighborRequest>,
    pub user_responses: VecDeque<Response>,
    pub requests: VecDeque<NeighborRequest>,
    pub requested_data: VecDeque<NeighborResponse>,
    gnome_header: Header,
    gnome_neighborhood: Neighborhood,
    active_unicasts: HashMap<CastID, Sender<Data>>,
    active_broadcasts: HashMap<CastID, Sender<Message>>,
    pub available_bandwith: u64,
    // pending_unicasts: HashMap<CastID, Sender<Data>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NeighborRequest {
    ListingRequest(SwarmTime),
    BlockRequest(u8, Box<[BlockID; 128]>),
    UnicastRequest(SwarmID, Box<[CastID; 256]>),
    ForwardConnectRequest(NetworkSettings),
    ConnectRequest(u8, GnomeId, NetworkSettings),
    SwarmSyncRequest,
    SubscribeRequest(bool, CastID),
    CustomRequest(u8, Data),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NeighborResponse {
    Listing(u8, Vec<BlockID>),
    Block(BlockID, Data),
    Unicast(SwarmID, CastID),
    ForwardConnectResponse(NetworkSettings),
    ForwardConnectFailed,
    ConnectResponse(u8, NetworkSettings),
    AlreadyConnected(u8),
    SwarmSync(Vec<CastID>, Vec<CastID>),
    Subscribed(bool, CastID, GnomeId, Option<GnomeId>),
    CustomResponse(u8, Data),
}

impl Neighbor {
    pub fn from_id_channel_time(
        id: GnomeId,
        receiver: Receiver<Message>,
        sender: Sender<Message>,
        swarm_time: SwarmTime,
        swarm_diameter: SwarmTime,
    ) -> Self {
        Neighbor {
            id,
            receiver,
            round_start: SwarmTime(0),
            sender,
            swarm_time,
            swarm_diameter,
            neighborhood: Neighborhood(0),
            prev_neighborhood: None,
            header: Header::Sync,
            payload: Payload::KeepAlive(0),
            user_requests: VecDeque::new(),
            user_responses: VecDeque::new(),
            requests: VecDeque::new(),
            requested_data: VecDeque::new(),
            gnome_header: Header::Sync,
            gnome_neighborhood: Neighborhood(0),
            active_unicasts: HashMap::new(),
            active_broadcasts: HashMap::new(),
            available_bandwith: 1024,
            // pending_unicasts: HashMap::new(),
        }
    }

    pub fn start_new_round(&mut self, swarm_time: SwarmTime) {
        // println!("\n\nStarting new round!");
        // self.swarm_time = swarm_time;
        self.round_start = swarm_time;
        self.header = Header::Sync;
        self.payload = Payload::KeepAlive(self.available_bandwith);
        self.prev_neighborhood = None;
        self.neighborhood = Neighborhood(0);
    }

    // pub fn add_unicast(&mut self, cast_id: CastID, sender: Sender<Data>) {
    //     self.pending_unicasts.insert(cast_id, sender);
    // }

    pub fn try_recv(
        &mut self,
        last_accepted_message: Message,
        // last_accepted_block: BlockID,
        // last_accepted_reconf: Option<Configuration>,
    ) -> (bool, bool, bool, bool) {
        let mut message_recvd = false;
        let sanity_passed = true;
        let mut new_proposal = false;
        let mut drop_me = false;
        while let Ok(
            ref message @ Message {
                swarm_time,
                neighborhood,
                header,
                ref payload,
            },
        ) = self.receiver.try_recv()
        {
            println!("{} < {}", self.id, message);
            if message.is_cast() {
                // println!("Unserved casting 1");
                self.send_casting(message.clone());
                continue;
            }
            message_recvd = true;
            if message.header == last_accepted_message.header
                && message.payload == last_accepted_message.payload
            {
                continue;
            }
            // if let Some(config) = last_accepted_reconf {
            //     if message.header == Header::Reconfigure
            //         && message.payload == Payload::Reconfigure(config)
            //     {
            //         // TODO: here we might be droping casting messages
            //         if message.is_cast() {
            //             println!("Unserved casting 2");
            //         }
            //         continue;
            //     }
            // }
            // eprintln!("{}  <  {}", self.id, message);
            if message.is_bye() {
                drop_me = true;
                return (message_recvd, sanity_passed, new_proposal, drop_me);
            }
            if !self.sanity_check(&swarm_time, &neighborhood, &header) {
                //     message_recvd = true;
                // } else {
                // println!("Coś nie poszło {}", message);
                // TODO: sanity might fail for casting messages, but we still
                // need to put them throught for user to receive
                if message.is_cast() {
                    self.send_casting(message.clone());
                } else if message.is_request() || message.is_response() {
                    // println!("Unserved requests");
                } else {
                    continue;
                }
            }
            // } else {
            //     message_recvd = true;
            // }
            // println!("Sanity passed {}", message);

            self.swarm_time = swarm_time;
            match header {
                Header::Sync => {
                    if self.header == Header::Sync {
                        if neighborhood.0 > 0 {
                            self.prev_neighborhood = Some(self.neighborhood);
                        } else {
                            self.prev_neighborhood = None;
                        }
                    } else {
                        self.prev_neighborhood = None;
                        self.header = Header::Sync;
                    }
                    self.neighborhood = neighborhood;
                }
                Header::Reconfigure(new_ct, new_gid) => {
                    if let Header::Reconfigure(ct, gid) = self.header {
                        if new_ct > ct ||
                        // {
                        //     new_proposal = true;
                        //     self.prev_neighborhood = None;
                        //     self.header = header;
                        // } else if
                         new_gid > gid
                        {
                            new_proposal = true;
                            self.prev_neighborhood = None;
                            self.header = header;
                        } else if neighborhood.0 > 0 {
                            self.prev_neighborhood = Some(self.neighborhood);
                        } else {
                            self.prev_neighborhood = None;
                        }
                    } else {
                        new_proposal = true;
                        self.prev_neighborhood = None;
                        // println!("Neighbor got new reconfigure proposal");
                        self.header = header;
                    }
                    self.neighborhood = neighborhood;
                }
                Header::Block(id) => {
                    // println!("Neighbor proposal recv: {:?}", id);
                    if let Header::Block(current_id) = self.header {
                        if current_id == id {
                            self.prev_neighborhood = Some(self.neighborhood);
                        } else {
                            new_proposal = true;
                            self.prev_neighborhood = None;
                            self.header = header;
                        }
                        self.neighborhood = neighborhood;
                    } else {
                        self.header = header;
                        new_proposal = true;
                        self.prev_neighborhood = None;
                        self.neighborhood = neighborhood;
                    }
                }
            };
            match payload {
                Payload::KeepAlive(bandwith) => {
                    self.available_bandwith = *bandwith;
                    // println!("KeepAlive");
                }
                Payload::Bye => {
                    println!("Bye");
                    drop_me = true;
                }
                Payload::Block(block_id, data) => {
                    match self.header {
                        Header::Block(id) => {
                            if id == *block_id {
                                self.payload = payload.clone();
                            } else {
                                self.user_responses
                                    .push_front(Response::Block(*block_id, *data));
                            }
                        }
                        Header::Sync => {
                            self.user_responses
                                .push_front(Response::Block(*block_id, *data));
                        }
                        Header::Reconfigure(_ct, _gid) => {
                            println!("Sending Block in Reconfigure header is not allowed");
                        }
                    };
                }
                Payload::Reconfigure(_config) => {
                    //TODO: this can not be a Sync header, since we can not distinguish
                    // two Sync messages
                    // Probably we need to introduce new header, Reconfigure
                    // Priority:  Block > Reconfigure > Sync
                    // Reconfigure is when we have first bit in a Header set to '0'
                    // and three Payload bits are all ones: '111'
                    self.payload = payload.clone();
                }
                Payload::Request(request) => {
                    // println!("Pushing riquest");
                    self.requests.push_front(request.clone());
                }
                Payload::Response(response) => match response {
                    NeighborResponse::Listing(_count, listing) => {
                        // let mut list_ver = vec![];
                        // let mut i: usize = 0;
                        // let count: usize = *count as usize;
                        // while i < count {
                        //     i += 1;
                        //     list_ver.push(listing[i]);
                        // }
                        self.user_responses
                            .push_front(Response::Listing(listing.clone()))
                    }
                    NeighborResponse::Block(block_id, data) => {
                        self.user_responses
                            .push_front(Response::Block(*block_id, *data));
                    }
                    NeighborResponse::Unicast(swarm_id, cast_id) => {
                        let (sender, receiver) = channel();
                        self.active_unicasts.insert(*cast_id, sender);
                        self.user_responses
                            .push_front(Response::Unicast(*swarm_id, *cast_id, receiver));
                    }
                    resp @ NeighborResponse::ForwardConnectResponse(_network_settings) => {
                        //TODO send this to networking
                        self.user_responses
                            .push_front(Response::ToGnome(resp.clone()));
                    }
                    resp @ NeighborResponse::ForwardConnectFailed => {
                        self.user_responses
                            .push_front(Response::ToGnome(resp.clone()));
                        //TODO notify gnome
                    }
                    resp @ NeighborResponse::AlreadyConnected(_id) => {
                        self.user_responses
                            .push_front(Response::ToGnome(resp.clone()));
                        //TODO notify gnome
                    }
                    resp @ NeighborResponse::ConnectResponse(_id, _network_settings) => {
                        self.user_responses
                            .push_front(Response::ToGnome(resp.clone()));
                        //TODO send this to gnome.ongoing_requests
                    }
                    resp @ NeighborResponse::SwarmSync(_bcasts, _mcasts) => {
                        self.user_responses
                            .push_front(Response::ToGnome(resp.clone()));
                    }
                    NeighborResponse::Subscribed(is_bcast, cast_id, origin_id, _none) => {
                        self.user_responses.push_front(Response::ToGnome(
                            NeighborResponse::Subscribed(
                                *is_bcast,
                                *cast_id,
                                *origin_id,
                                Some(self.id),
                            ),
                        ));
                    }
                    NeighborResponse::CustomResponse(id, data) => {
                        self.user_responses.push_front(Response::Custom(*id, *data))
                    }
                },
                Payload::Unicast(cid, data) => {
                    // println!("Served casting 3");
                    if let Some(sender) = self.active_unicasts.get(&cid) {
                        let _ = sender.send(*data);
                    }
                }
                Payload::Multicast(_mid, _data) => {
                    // TODO: serve this
                    // self.send_casting(message);
                }
                Payload::Broadcast(_bid, _data) => {
                    // TODO: serve this
                    self.send_casting(message.clone());
                }
            }
            // println!("returning: {} {:?}", new_proposal, self.neighborhood);
        }
        (message_recvd, sanity_passed, new_proposal, drop_me)
    }

    pub fn activate_broadcast(&mut self, cast_id: CastID, sender: Sender<Message>) {
        self.active_broadcasts.insert(cast_id, sender);
    }

    fn send_casting(&self, message: Message) {
        match message.payload {
            Payload::Broadcast(cast_id, _data) => {
                if let Some(sender) = self.active_broadcasts.get(&cast_id) {
                    let _ = sender.send(message);
                }
            }
            // Payload::Multicast(cast_id, _data) => {
            //     if let Some(sender) = self.active_multicasts.get(&cast_id) {
            //         let _ = sender.send(message);
            //     }
            // }
            _ => {
                println!("send_casting: unexpected message: {:?}", message);
            }
        }
    }

    fn sanity_check(
        &self,
        swarm_time: &SwarmTime,
        neighborhood: &Neighborhood,
        header: &Header,
    ) -> bool {
        if self.swarm_time > *swarm_time {
            println!(
                "Received a message with older swarm_time {} than previous {}!",
                swarm_time, self.swarm_time
            );
            return false;
        }
        // A neighbor can not announce a number greater than the number
        // we announced to him, plus one
        let hood_inc_limited = if self.gnome_header == *header {
            // println!("{} <= {}", neighborhood.0, self.gnome_neighborhood.0 + 1);
            neighborhood.0 <= self.gnome_neighborhood.0 + 1
        } else {
            true
        };
        if self.header == *header {
            // println!("same header");
            // A gnome can not stay at the same neighborhood number for more than
            // 2 turns
            let hood_increased = if let Some(prev_neighborhood) = self.prev_neighborhood {
                // println!(
                //     "{} new: {:?} > pprev: {:?}",
                //     swarm_time, neighborhood, prev_neighborhood
                // );
                if neighborhood.0 == 0
                    && u32::from(self.neighborhood.0 + 1) >= self.swarm_diameter.0
                {
                    true
                } else {
                    neighborhood.0 > prev_neighborhood.0 || neighborhood.0 > self.neighborhood.0
                }
            } else {
                // A gnome can not backtrack by announcing a smaller neighborhood
                // number than before
                // println!(
                //     "{} current: {:?} <= new: {:?}",
                //     swarm_time, self.neighborhood, neighborhood
                // );
                self.neighborhood.0 <= neighborhood.0
            };
            if !hood_increased {
                println!(
                    "fail hood_increased {:?} {} {} ",
                    self.prev_neighborhood, self.neighborhood, neighborhood
                );
            }
            if !hood_inc_limited {
                println!(
                    "fail hood_inc_limited   {} <= {}",
                    neighborhood.0,
                    self.gnome_neighborhood.0 + 1
                );
            }
            hood_increased && hood_inc_limited
        } else {
            // TODO: Need to think this through
            let no_backdating =
                *swarm_time - self.round_start < (self.swarm_diameter + self.swarm_diameter);
            if !no_backdating {
                println!(
                    "backdating: {}-{}<{}",
                    swarm_time, self.round_start, self.swarm_diameter
                );
            }
            if let Header::Block(id) = self.header {
                match header {
                    Header::Block(new_id) => new_id >= &id && no_backdating && hood_inc_limited,
                    Header::Reconfigure(_ct, _gid) => false,
                    Header::Sync => false,
                }
            } else if let Header::Reconfigure(ct, gid) = self.header {
                match header {
                    Header::Block(_id) => no_backdating && hood_inc_limited,
                    Header::Reconfigure(new_ct, new_gid) => {
                        //TODO: not sure if this is ok
                        if *new_ct > ct ||
                            // no_backdating
                        // } else if 
                            *new_gid > gid
                        {
                            no_backdating
                        } else {
                            no_backdating && hood_inc_limited
                        }
                    }
                    Header::Sync => false,
                }
            } else {
                // Backdating check
                no_backdating && hood_inc_limited
            }
        }
    }

    pub fn send_out(&mut self, message: Message) {
        let _ = self.sender.send(message.clone());
        self.gnome_header = message.header;
        // println!("new gn: {}", message.neighborhood.0);
        self.gnome_neighborhood = message.neighborhood;
    }

    pub fn get_specialized_data(&mut self) -> Option<NeighborResponse> {
        // println!("Getting specialized data");
        self.requested_data.pop_back()
    }

    pub fn send_out_specialized_message(
        &mut self,
        message: &Message,
        id: GnomeId,
        send_default: bool,
    ) {
        if let Some(resp) = self.get_specialized_data() {
            let payload = Payload::Response(resp);
            let new_message = message.set_payload(payload);
            println!("{} >S> {}", id, new_message);
            self.send_out(new_message);
        } else if let Some(request) = self.user_requests.pop_back() {
            let new_message = message.include_request(request);
            println!("{} >S> {}", id, new_message);
            self.send_out(new_message);
        } else if send_default {
            // if !generic_info_printed {
            // eprintln!("{} >>> {}", id, message);
            //     generic_info_printed = true;
            // }
            // println!("snd {}", message);
            self.send_out(message.to_owned());
        }
    }

    pub fn add_requested_data(&mut self, data: NeighborResponse) {
        // self.requested_data.push_front((request, data));
        self.requested_data.push_front(data.clone());

        // TODO: maybe move it somewhere else?
        if let NeighborResponse::Unicast(swarm_id, cast_id) = data {
            let (sender, receiver) = channel();
            self.active_unicasts.insert(cast_id, sender);
            self.user_responses
                .push_front(Response::Unicast(swarm_id, cast_id, receiver));
        }
    }

    pub fn request_data(&mut self, request: NeighborRequest) {
        self.user_requests.push_front(request);
    }
}
