use crate::message::BlockID;
use crate::message::Header;
use crate::message::Payload;
use crate::message::WrappedMessage;
use crate::multicast::CastMessage;
use crate::multicast::CastType;
use crate::policy::Policy;
use crate::requirement::Requirement;
use crate::Capabilities;
use crate::CastContent;
use crate::CastData;
use crate::CastID;
use crate::GnomeId;
use crate::GnomeToApp;
use crate::Message;
use crate::NetworkSettings;
use crate::Signature;
use crate::Swarm;
use crate::SwarmID;
use crate::SwarmName;
use crate::SwarmTime;
use crate::SwarmType;
use crate::SyncData;
use std::collections::HashMap;
use std::fmt::Display;

use std::collections::VecDeque;
use std::sync::mpsc::channel;
use std::sync::mpsc::SendError;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    cast_receiver: Receiver<CastMessage>,
    pub sender: Sender<WrappedMessage>,
    shared_sender: Sender<(
        SwarmName,
        Sender<Message>,
        Sender<CastMessage>,
        Receiver<WrappedMessage>,
    )>,
    round_start: SwarmTime,
    pub swarm_time: SwarmTime,
    pub swarm_diameter: SwarmTime,
    pub neighborhood: Neighborhood,
    pub prev_neighborhood: Option<Neighborhood>,
    pub header: Header,
    pub payload: Payload,
    pub user_requests: VecDeque<NeighborRequest>,
    pub user_responses: VecDeque<GnomeToApp>,
    pub requests: VecDeque<NeighborRequest>,
    pub requested_data: VecDeque<NeighborResponse>,
    gnome_header: Header,
    gnome_neighborhood: Neighborhood,
    active_unicasts: HashMap<CastID, Sender<CastData>>,
    active_broadcasts: HashMap<CastID, Sender<WrappedMessage>>,
    pub available_bandwith: u64,
    pub member_of_swarms: Vec<SwarmName>,
    timeouts: [u8; 8],
    pub new_message_recieved: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwarmSyncRequestParams {
    pub sync_key_reg: bool,
    pub sync_capability: bool,
    pub sync_policy: bool,
    pub sync_broadcast: bool,
    pub sync_multicast: bool,
    // pub app_root_hash: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwarmSyncResponse {
    pub chill_phase: u16,
    pub founder: GnomeId,
    pub swarm_time: SwarmTime,
    pub round_start: SwarmTime,
    pub swarm_type: SwarmType,
    // pub app_root_hash: u64,
    pub key_reg_size: u8,
    pub capability_size: u8,
    pub policy_size: u8,
    pub broadcast_size: u8,
    pub multicast_size: u8,
    pub more_key_reg_messages: bool,
    pub key_reg_pairs: Vec<(GnomeId, Vec<u8>)>,
}
//TODO: Move all upper layer Requests Responses into Custom wrap
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NeighborRequest {
    UnicastRequest(SwarmID, Box<[CastID; 256]>),
    ForwardConnectRequest(NetworkSettings),
    ConnectRequest(u8, GnomeId, NetworkSettings),
    SwarmSyncRequest(SwarmSyncRequestParams),
    SubscribeRequest(bool, CastID),
    UnsubscribeRequest(bool, CastID), // We send this when no longer interested in bcast
    SourceDrained(bool, CastID),      // We send this to our subscribers to indicate they
    // have to find another source for given cast, give them some time to do so
    CreateNeighbor(GnomeId, SwarmName),
    SwarmJoinedInfo(SwarmName),
    //TODO: implement ListNeighboringSwarms,
    Custom(u8, CastData),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NeighborResponse {
    BroadcastSync(u8, u8, Vec<(CastID, GnomeId)>),
    MulticastSync(u8, u8, Vec<(CastID, GnomeId)>),
    Unicast(SwarmID, CastID),
    ForwardConnectResponse(NetworkSettings),
    ForwardConnectFailed,
    ConnectResponse(u8, NetworkSettings),
    AlreadyConnected(u8),
    SwarmSync(SwarmSyncResponse),
    KeyRegistrySync(u8, u8, Vec<(GnomeId, Vec<u8>)>),
    CapabilitySync(u8, u8, Vec<(Capabilities, Vec<GnomeId>)>),
    PolicySync(u8, u8, Vec<(Policy, Requirement)>),
    Subscribed(bool, CastID, GnomeId, Option<GnomeId>),
    //TODO: implement NeighboringSwarms(u8, u8, Vec<SwarmName>),
    Custom(u8, CastData),
}

impl Neighbor {
    pub fn from_id_channel_time(
        id: GnomeId,
        receiver: Receiver<Message>,
        cast_receiver: Receiver<CastMessage>,
        sender: Sender<WrappedMessage>,
        shared_sender: Sender<(
            SwarmName,
            Sender<Message>,
            Sender<CastMessage>,
            Receiver<WrappedMessage>,
        )>,
        swarm_time: SwarmTime,
        swarm_diameter: SwarmTime,
        member_of_swarms: Vec<SwarmName>,
    ) -> Self {
        Neighbor {
            id,
            receiver,
            round_start: SwarmTime(0),
            cast_receiver,
            sender,
            shared_sender,
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
            member_of_swarms,
            timeouts: [0; 8],
            new_message_recieved: false,
        }
    }
    pub fn get_shared_sender(
        &self,
    ) -> Sender<(
        SwarmName,
        Sender<Message>,
        Sender<CastMessage>,
        Receiver<WrappedMessage>,
    )> {
        self.shared_sender.clone()
    }

    // clone_to_swarm is used to notify networking about
    // new swarm that Gnome want's to share with Neighbor
    pub fn clone_to_swarm(
        &self,
        swarm_name: SwarmName,
        send: Sender<Message>,
        c_send: Sender<CastMessage>,
        recv: Receiver<WrappedMessage>,
    ) {
        // println!("clone to swarm");
        let _r = self.shared_sender.send((swarm_name, send, c_send, recv));
        if let Some(err) = _r.err() {
            eprintln!("failed to clone to swarm: {:?}", err);
        }
    }

    pub fn clone_state(&mut self, neighbor: Neighbor) {
        self.round_start = neighbor.round_start;
        self.swarm_time = neighbor.swarm_time;
        self.swarm_diameter = neighbor.swarm_diameter;
        self.neighborhood = neighbor.neighborhood;
        self.prev_neighborhood = neighbor.prev_neighborhood;
        self.header = neighbor.header;
        self.payload = neighbor.payload;
        self.user_requests = neighbor.user_requests;
        self.user_responses = neighbor.user_responses;
        self.requests = neighbor.requests;
        self.requested_data = neighbor.requested_data;
        self.gnome_header = neighbor.gnome_header;
        self.gnome_neighborhood = neighbor.gnome_neighborhood;
        self.active_unicasts = neighbor.active_unicasts;
        self.active_broadcasts = neighbor.active_broadcasts;
        self.available_bandwith = neighbor.available_bandwith;
        self.new_message_recieved = neighbor.new_message_recieved;
    }
    pub fn start_new_round(&mut self, swarm_time: SwarmTime) {
        // eprintln!("\n\nN{} Starting new round @{}", self.id, swarm_time);
        self.round_start = swarm_time;
        self.header = Header::Sync;
        self.payload = Payload::KeepAlive(self.available_bandwith);
        self.prev_neighborhood = None;
        self.neighborhood = Neighborhood(0);
    }

    pub fn recv(&mut self, timeout: Duration) -> Option<Message> {
        let recv_result = self.receiver.recv_timeout(timeout);
        if let Ok(response) = recv_result {
            // TODO: we should update Neighbor state according to
            self.header = response.header;
            self.payload = response.payload.clone();
            self.neighborhood = response.neighborhood;
            self.swarm_time = response.swarm_time;
            self.new_message_recieved = true;
            return Some(response);
        }
        None
    }
    pub fn recv_sync(&mut self, timeout: Duration) -> Result<Option<NeighborResponse>, String> {
        let recv_result = self.cast_receiver.recv_timeout(timeout);
        if let Ok(CastMessage {
            c_type,
            id,
            content,
        }) = recv_result
        {
            if c_type == CastType::Unicast && CastID(254) == id {
                if let CastContent::Response(NeighborResponse::SwarmSync(swarm_sync_response)) =
                    content
                {
                    self.swarm_time = swarm_sync_response.swarm_time;
                    eprintln!("SNR1");
                    self.start_new_round(swarm_sync_response.round_start);
                    return Ok(Some(NeighborResponse::SwarmSync(swarm_sync_response)));
                }
            } else if c_type == CastType::Unicast && CastID(255) == id {
                if let CastContent::Request(NeighborRequest::SwarmSyncRequest(sync_req_params)) =
                    content
                {
                    eprintln!("Neighbor Received SwarmSyncRequest {:?} ", sync_req_params);
                    // if sync_req_params.app_root_hash != 0 {
                    //     // TODO we need a more sophisticated sync method
                    //     // once we enable storing swarm data on disk this will run
                    //     // probably we should return either NeighborResponse or NeighborRequest?
                    //     // -> Result<Option<NeighborResponse>, Option<NeighborRequest>>
                    //     // lol
                    //     panic!("Received SwarmSyncRequest with non zero app_sync_hash!");
                    // }
                } else {
                    eprintln!("Received unexpected SSRequest content:\n {:?}", content);
                }
                return Ok(None);
            };
            // TODO: we should update Neighbor state according to
            // self.header = response.header;
            // self.payload = response.payload.clone();
            // self.neighborhood = response.neighborhood;
            // self.swarm_time = response.swarm_time;

            // return Some(self.swarm_time);
        }
        Err("Unexpected Cast message during SwarmSync".to_string())
    }

    pub fn try_recv_cast(&mut self) -> bool {
        let mut any_data_processed = false;
        while let Ok(c_msg @ CastMessage { c_type, id, .. }) = self.cast_receiver.try_recv() {
            any_data_processed = true;
            match c_type {
                CastType::Broadcast => {
                    if let Some(sender) = self.active_broadcasts.get(&id) {
                        let _ = sender.send(WrappedMessage::Cast(c_msg));
                    }
                }
                CastType::Multicast => {
                    // TODO
                }
                CastType::Unicast => {
                    match id {
                        CastID(255) => {
                            //Request
                            // println!("Some NReq received: {:?}", c_type);
                            if let Some(request) = c_msg.get_request() {
                                self.requests.push_front(request.clone());
                            }
                        }
                        CastID(254) => {
                            //Response
                            if let Some(response) = c_msg.get_response() {
                                self.serve_neighbor_response(response);
                            }
                        }
                        _ => {
                            if let Some(sender) = self.active_unicasts.get(&id) {
                                let _ = sender.send(c_msg.get_data().unwrap());
                            } else {
                                eprintln!("Could not find Unicast with id: {:?}", id);
                            }
                        }
                    }
                }
            }
        }
        any_data_processed
    }

    fn verify_payload(
        &self,
        round_start: SwarmTime,
        swarm: &Swarm,
        signature: &Signature,
        bytes: &mut Vec<u8>,
    ) -> bool {
        match *signature {
            Signature::Regular(gid, ref sign) => {
                // println!(
                //     "Regular signature verification... gid: {}, blen: {}\nVerify head: ",
                //     gid,
                //     bytes.len()
                // );
                // for i in 0..20 {
                //     if let Some(byte) = bytes.get(i) {
                //         print!("{}-", byte);
                //     }
                // }
                // println!("Verify tail: ");
                // for i in bytes.len() - 20..bytes.len() {
                //     if let Some(byte) = bytes.get(i) {
                //         print!("{}-", byte);
                //     }
                // }
                if let Some(pubkey_bytes) = swarm.key_reg.get(gid) {
                    eprintln!("key found in reg {}", round_start);
                    (swarm.verify)(gid, &pubkey_bytes, round_start, bytes, sign)
                } else {
                    eprintln!("no key found in reg!");
                    false
                }
            } //TODO
            Signature::Extended(gid, ref pubkey_bytes, ref sign) => {
                // eprintln!("Extended signature verification...");
                (swarm.verify)(gid, pubkey_bytes, round_start, bytes, sign)
            }
        }
    }

    pub fn try_recv(
        &mut self,
        last_accepted_message: Message,
        swarm: &mut Swarm,
    ) -> (bool, bool, bool, bool) {
        let mut message_recvd = false;
        let mut sanity_passed = false;
        let mut new_proposal = false;
        let mut drop_me = false;
        let mut force_break = false;
        while let Ok(
            mut message @ Message {
                swarm_time,
                neighborhood,
                header,
                ..
            },
        ) = self.receiver.try_recv()
        {
            eprintln!("{}  <  {}", self.id, message);

            if message.swarm_time.0 < last_accepted_message.swarm_time.0 {
                eprintln!("Old message, ignoring");
                continue;
            }
            if message.header == last_accepted_message.header
                && message.swarm_time.0 - last_accepted_message.swarm_time.0 <= 2
                && message.neighborhood.0 as u32 >= self.swarm_diameter.0
            // && message.neighborhood == Neighborhood(7)
            {
                // eprintln!("Ignoring: {}", message);
                if let Payload::KeepAlive(avail_bandwith) = message.payload {
                    self.available_bandwith = avail_bandwith;
                }
                // TODO: without this message verification fails...
                // self.round_start = message.swarm_time;
                continue;
            }

            message_recvd = true;
            // eprintln!("{}  <  {}", self.id, message);
            if message.is_bye() {
                drop_me = true;
                return (message_recvd, sanity_passed, new_proposal, drop_me);
            }
            // TODO:
            //       if there is signature then unpack payload into pieces
            //           verify it
            //           if verification failed
            //               return
            //       else
            //           continue
            if message.payload.has_signature() {
                let tested_message = std::mem::replace(&mut message, Message::bye());
                let (r_st, r_n, r_header, is_config, sign_bytes_opt) = tested_message.unpack();
                let (signature, mut bytes) = sign_bytes_opt.unwrap();
                eprintln!("N{} verify_payload ST: {}", self.id, self.round_start);
                if !self.verify_payload(self.round_start, swarm, &signature, &mut bytes) {
                    eprintln!("Verification failed {}", self.round_start);
                    // TODO: maybe it's too drastic of a measure?
                    drop_me = true;
                    return (message_recvd, false, new_proposal, drop_me);
                } else {
                    message.pack(r_st, r_n, r_header, is_config, Some((signature, bytes)));
                }
                if !swarm.verify_policy(&message) {
                    eprintln!(
                        "Policy not fulfilled for {} {}",
                        message.header, message.payload
                    );
                    drop_me = true;
                    return (message_recvd, false, new_proposal, drop_me);
                }
            }
            // println!("Verification success");
            if !self.sanity_check(&swarm_time, &neighborhood, &header) {
                // TODO: sanity might fail for casting messages, but we still
                // need to put them throught for user to receive

                // TODO: maybe return instead of continue?
                return (message_recvd, sanity_passed, new_proposal, drop_me);
            }
            sanity_passed = true;
            self.new_message_recieved = true;
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
                        if new_ct > ct || new_gid > gid {
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
                        force_break = true;
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
            match message.payload {
                Payload::KeepAlive(bandwith) => {
                    self.available_bandwith = bandwith;
                    // println!("KeepAlive");
                }
                Payload::Bye => {
                    println!("Bye");
                    drop_me = true;
                }
                Payload::Block(block_id, signature, data) => {
                    match self.header {
                        Header::Block(id) => {
                            if id == block_id {
                                self.payload = Payload::Block(id, signature, data)
                            } else {
                                eprintln!(
                                    "This is not possible\nheader id: {}, block id: {}",
                                    id, block_id
                                );
                            }
                        }
                        Header::Sync => {
                            eprintln!("This is not possible too");
                        }
                        Header::Reconfigure(_ct, _gid) => {
                            eprintln!("Sending Block in Reconfigure header is not allowed");
                        }
                    };
                }
                Payload::Reconfigure(sign, conf) => {
                    //TODO: this can not be a Sync header, since we can not distinguish
                    // two Sync messages
                    // Probably we need to introduce new header, Reconfigure
                    // Priority:  Block > Reconfigure > Sync
                    // Reconfigure is when we have first bit in a Header set to '0'
                    // and three Payload bits are all ones: '111'
                    self.payload = Payload::Reconfigure(sign, conf);
                } // println!("Served casting 3");
            }
            // println!("returning: {} {:?}", new_proposal, self.neighborhood);
            if force_break {
                break;
            }
        }
        (message_recvd, sanity_passed, new_proposal, drop_me)
    }

    fn serve_neighbor_response(&mut self, response: NeighborResponse) {
        match response {
            NeighborResponse::Unicast(swarm_id, cast_id) => {
                let (sender, receiver) = channel();
                self.active_unicasts.insert(cast_id, sender);
                self.user_responses
                    .push_front(GnomeToApp::Unicast(swarm_id, cast_id, receiver));
            }
            NeighborResponse::ForwardConnectResponse(_network_settings) => {
                //TODO send this to networking
                self.user_responses
                    .push_front(GnomeToApp::ToGnome(response));
            }
            NeighborResponse::ForwardConnectFailed => {
                self.user_responses
                    .push_front(GnomeToApp::ToGnome(response));
            }
            NeighborResponse::AlreadyConnected(_id) => {
                self.user_responses
                    .push_front(GnomeToApp::ToGnome(response));
            }
            NeighborResponse::ConnectResponse(_id, _network_settings) => {
                self.user_responses
                    .push_front(GnomeToApp::ToGnome(response));
                //TODO send this to gnome.ongoing_requests
            }
            NeighborResponse::SwarmSync(swarm_sync_response) => {
                self.user_responses
                    .push_front(GnomeToApp::ToGnome(NeighborResponse::SwarmSync(
                        swarm_sync_response,
                    )));
            }
            NeighborResponse::Subscribed(is_bcast, cast_id, origin_id, _none) => {
                self.user_responses
                    .push_front(GnomeToApp::ToGnome(NeighborResponse::Subscribed(
                        is_bcast,
                        cast_id,
                        origin_id,
                        Some(self.id),
                    )));
            }
            NeighborResponse::KeyRegistrySync(part_no, total_parts_count, id_key_pairs) => {
                self.user_responses.push_front(GnomeToApp::ToGnome(
                    NeighborResponse::KeyRegistrySync(part_no, total_parts_count, id_key_pairs),
                ));
            }
            NeighborResponse::CapabilitySync(part_no, total_parts_count, capa_ids_pairs) => {
                self.user_responses.push_front(GnomeToApp::ToGnome(
                    NeighborResponse::CapabilitySync(part_no, total_parts_count, capa_ids_pairs),
                ));
            }
            NeighborResponse::PolicySync(part_no, total_parts_count, policy_req_pairs) => {
                self.user_responses
                    .push_front(GnomeToApp::ToGnome(NeighborResponse::PolicySync(
                        part_no,
                        total_parts_count,
                        policy_req_pairs,
                    )));
            }
            NeighborResponse::BroadcastSync(part_no, total_parts_count, cast_id_source_pairs) => {
                self.user_responses.push_front(GnomeToApp::ToGnome(
                    NeighborResponse::BroadcastSync(
                        part_no,
                        total_parts_count,
                        cast_id_source_pairs,
                    ),
                ));
            }
            NeighborResponse::MulticastSync(part_no, total_parts_count, cast_id_source_pairs) => {
                self.user_responses.push_front(GnomeToApp::ToGnome(
                    NeighborResponse::MulticastSync(
                        part_no,
                        total_parts_count,
                        cast_id_source_pairs,
                    ),
                ));
            }
            NeighborResponse::Custom(id, data) => self
                .user_responses
                .push_front(GnomeToApp::Custom(false, id, self.id, data)),
        }
    }

    pub fn activate_broadcast(&mut self, cast_id: CastID, sender: Sender<WrappedMessage>) {
        self.active_broadcasts.insert(cast_id, sender);
    }

    pub fn add_timeout(&mut self) {
        self.timeouts[0] += 1;
    }

    pub fn shift_timeout(&mut self) {
        // println!("shift_timeout");
        for i in (0..=6).rev() {
            self.timeouts[i + 1] = self.timeouts[i];
        }
        self.timeouts[0] = 0;
    }

    pub fn timeouts_count(&self) -> u8 {
        // println!("timeouts_count");
        self.timeouts.iter().sum()
    }

    fn send_casting(&self, message: CastMessage) {
        if message.is_broadcast() {
            if let Some(sender) = self.active_broadcasts.get(&message.id()) {
                let _ = sender.send(WrappedMessage::Cast(message));
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
            eprintln!(
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
                // self.neighborhood.0 < neighborhood.0
                true
            };
            if !hood_increased {
                eprintln!(
                    "{} fail hood_increased prev:{:?} curr:{} recv:{} ",
                    swarm_time, self.prev_neighborhood, self.neighborhood, neighborhood
                );
            }
            if !hood_inc_limited {
                eprintln!(
                    "{} fail hood_inc_limited neighbor {} <= {} gnome",
                    swarm_time,
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
                eprintln!(
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
                        if *new_ct > ct || *new_gid > gid {
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

    pub fn send_no_op(&self) {
        let _ = self.sender.send(WrappedMessage::NoOp);
    }
    pub fn send_out_cast(&mut self, message: CastMessage) -> Result<(), SendError<WrappedMessage>> {
        // println!("Sending: {:?}", message);
        let _res = self.sender.send(WrappedMessage::Cast(message));
        // if let Some(err) = _res.err() {
        //     eprintln!("Unable to send cast: {:?}", err);
        // }
        _res
    }

    pub fn send_out(&mut self, message: Message) {
        self.gnome_header = message.header;
        // println!("new gn: {}", message.neighborhood.0);
        self.gnome_neighborhood = message.neighborhood;
        let _ = self.sender.send(WrappedMessage::Regular(message));
    }

    pub fn get_specialized_data(&mut self) -> Option<NeighborResponse> {
        // println!("Getting specialized data");
        self.requested_data.pop_back()
    }

    // TODO: find all instances where this is used and replace with
    //       gnome.send_neighbor_response
    pub fn add_requested_data(&mut self, response: NeighborResponse) {
        // TODO: maybe move it somewhere else?
        if let NeighborResponse::Unicast(swarm_id, cast_id) = response {
            let (sender, receiver) = channel();
            self.active_unicasts.insert(cast_id, sender);
            self.user_responses
                .push_front(GnomeToApp::Unicast(swarm_id, cast_id, receiver));
        }
        self.send_out_cast(CastMessage::new_response(response));
    }

    pub fn request_data(&mut self, request: NeighborRequest) {
        self.send_out_cast(CastMessage::new_request(request));
    }
}
