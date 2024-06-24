use crate::message::BlockID;
use crate::message::Configuration;
use crate::message::Header;
use crate::message::Payload;
use crate::multicast::Multicast;
use crate::neighbor::NeighborResponse;
use crate::neighbor::Neighborhood;
use crate::next_state::ChangeConfig;
use crate::swarm::Swarm;
use crate::CastID;
use crate::Data;
use crate::Message;
use crate::Neighbor;
use crate::NeighborRequest;
use crate::NextState;
use crate::Request;
use crate::Response;
use crate::SwarmTime;
use crate::DEFAULT_NEIGHBORS_PER_GNOME;
use crate::DEFAULT_SWARM_DIAMETER;

// use std::cmp::{max, min};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::fmt;
use std::net::IpAddr;
use std::ops::Deref;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Debug, Hash)]
pub struct GnomeId(pub u64);
impl fmt::Display for GnomeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GID-{:x}", self.0)
    }
}

struct OngoingRequest {
    origin: GnomeId,
    queried_neighbors: Vec<GnomeId>,
    timestamp: SwarmTime,
    response: Option<NetworkSettings>,
    network_settings: NetworkSettings,
}

struct NeighborDiscovery {
    counter: u16,
    treshold: u16,
    attempts: u8,
    try_next: bool,
    queried_neighbors: Vec<GnomeId>,
}
impl NeighborDiscovery {
    // Every new round we increment a counter.
    // Once counter reaches defined threashold
    // function returns true.
    // Function may also return true if recent
    // neighbor search ended with a failure, up to n-tries.
    // If neither of above is true, then function returns false.
    //
    // TODO: modify treshold & attempts to depend on available bandwith
    fn tick_and_check(&mut self) -> bool {
        self.counter += 1;
        let treshold_reached = self.counter >= self.treshold;
        if treshold_reached {
            self.counter = 0;
            self.attempts = 3;
            self.try_next = false;
            true
        } else if self.try_next && self.attempts > 0 {
            self.attempts -= 1;
            self.try_next = false;
            true
        } else {
            false
        }
    }
}

impl Default for NeighborDiscovery {
    fn default() -> Self {
        NeighborDiscovery {
            counter: 1000,
            treshold: 1000,
            attempts: 3,
            try_next: true,
            queried_neighbors: vec![],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Nat {
    Unknown = 0,
    None = 1,
    FullCone = 2,
    AddressRestrictedCone = 4,
    PortRestrictedCone = 8,
    SymmetricWithPortControl = 16,
    Symmetric = 32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortAllocationRule {
    Random = 0,
    FullCone = 1,
    AddressSensitive = 2,
    PortSensitive = 4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NetworkSettings {
    pub pub_ip: IpAddr,
    pub pub_port: u16,
    pub nat_type: Nat,
    pub port_allocation: (PortAllocationRule, i8),
}

impl NetworkSettings {
    pub fn new_not_natted(pub_ip: IpAddr, pub_port: u16) -> Self {
        Self {
            pub_ip,
            pub_port,
            nat_type: Nat::None,
            port_allocation: (PortAllocationRule::FullCone, 0),
        }
    }
    pub fn update(&mut self, other: Self) {
        self.pub_ip = other.pub_ip;
        self.pub_port = other.pub_port;
        self.nat_type = other.nat_type;
        self.port_allocation = other.port_allocation;
    }

    pub fn set_port(&mut self, port: u16) {
        self.pub_port = port;
    }

    pub fn get_predicted_addr(&self, mut iter: u8) -> (IpAddr, u16) {
        let mut port = self.port_increment(self.pub_port);
        while iter > 0 {
            iter -= 1;
            port = self.port_increment(port);
        }
        (self.pub_ip, port)
    }

    pub fn refresh_required(&self) -> bool {
        self.port_allocation.0 != PortAllocationRule::FullCone
    }
    pub fn nat_at_most_address_sensitive(&self) -> bool {
        self.nat_type == Nat::None
            || self.nat_type == Nat::FullCone
            || self.nat_type == Nat::AddressRestrictedCone
    }
    pub fn no_nat(&self) -> bool {
        self.nat_type == Nat::None
    }
    pub fn nat_port_restricted(&self) -> bool {
        self.nat_type == Nat::PortRestrictedCone
    }
    pub fn nat_symmetric(&self) -> bool {
        self.nat_type == Nat::Symmetric
    }
    pub fn nat_symmetric_with_port_control(&self) -> bool {
        self.nat_type == Nat::SymmetricWithPortControl
    }
    pub fn nat_unknown(&self) -> bool {
        self.nat_type == Nat::Unknown
    }
    pub fn port_allocation_predictable(&self) -> bool {
        self.port_allocation.0 == PortAllocationRule::FullCone
            || self.port_allocation.0 == PortAllocationRule::AddressSensitive
            || self.port_allocation.0 == PortAllocationRule::PortSensitive
    }
    pub fn port_sensitive_allocation(&self) -> bool {
        self.port_allocation.0 == PortAllocationRule::PortSensitive
    }
    pub fn port_increment(&self, port: u16) -> u16 {
        match self.port_allocation {
            (PortAllocationRule::AddressSensitive | PortAllocationRule::PortSensitive, value) => {
                if value > 0 {
                    port + (value as u16)
                } else {
                    port - (value.unsigned_abs() as u16)
                }
            }
            _ => port,
        }
    }
}

impl Default for NetworkSettings {
    fn default() -> Self {
        NetworkSettings {
            pub_ip: IpAddr::from([0, 0, 0, 0]),
            pub_port: 1026,
            nat_type: Nat::Unknown,
            port_allocation: (PortAllocationRule::Random, 0),
        }
    }
}
struct ConnRequest {
    conn_id: u8,
    neighbor_id: GnomeId,
    // network_settings: NetworkSettings,
}
#[derive(Clone, Copy)]
enum Proposal {
    Block(BlockID, Data),
    Config(Configuration),
}
impl Proposal {
    pub fn as_header_payload(&self) -> (Header, Payload) {
        match *self {
            Self::Block(b_id, data) => (Header::Block(b_id), Payload::Block(b_id, data)),
            Self::Config(config) => (
                Header::Reconfigure(config.as_ct(), config.as_gid()),
                Payload::Reconfigure(config),
            ),
        }
    }
}

pub struct Gnome {
    pub id: GnomeId,
    pub neighborhood: Neighborhood,
    swarm: Swarm,
    swarm_time: SwarmTime,
    round_start: SwarmTime,
    swarm_diameter: SwarmTime,
    receiver: Receiver<Request>,
    band_receiver: Receiver<u64>,
    sender: Sender<Response>,
    fast_neighbors: Vec<Neighbor>,
    slow_neighbors: Vec<Neighbor>,
    new_neighbors: Vec<Neighbor>,
    refreshed_neighbors: Vec<Neighbor>,
    // block_id: BlockID,
    // data: Data,
    header: Header,
    payload: Payload,
    my_proposal: Option<Proposal>,
    proposals: VecDeque<Proposal>,
    next_state: NextState,
    timeout_duration: Duration,
    send_immediate: bool,
    network_settings: NetworkSettings, //TODO: do we really need those here anymore?
    net_settings_send: Sender<NetworkSettings>,
    pending_conn_requests: VecDeque<ConnRequest>,
    ongoing_requests: HashMap<u8, OngoingRequest>,
    neighbor_discovery: NeighborDiscovery,
    chill_out: (bool, u8, Duration),
    data_converters: HashMap<(bool, CastID), (Receiver<Data>, Sender<Message>)>,
}

impl Gnome {
    pub fn new(
        id: GnomeId,
        swarm: Swarm,
        sender: Sender<Response>,
        receiver: Receiver<Request>,
        band_receiver: Receiver<u64>,
        network_settings: NetworkSettings,
        net_settings_send: Sender<NetworkSettings>,
    ) -> Self {
        Gnome {
            id,
            neighborhood: Neighborhood(0),
            swarm,
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
            // block_id: BlockID(0),
            // data: Data(0),
            // header: Header::Block(BlockID(0)),
            header: Header::Sync,
            // payload: Payload::Block(BlockID(0), Data(0)),
            payload: Payload::KeepAlive(1024),
            my_proposal: None,
            proposals: VecDeque::new(),
            next_state: NextState::new(),
            timeout_duration: Duration::from_millis(500),
            send_immediate: false,
            network_settings,
            net_settings_send,
            pending_conn_requests: VecDeque::new(),
            ongoing_requests: HashMap::new(),
            neighbor_discovery: NeighborDiscovery::default(),
            chill_out: (false, 250, Duration::from_millis(59)), // chill time is 250*59ms = just under 15sec
            data_converters: HashMap::new(),
        }
    }

    pub fn new_with_neighbors(
        id: GnomeId,
        swarm: Swarm,
        sender: Sender<Response>,
        receiver: Receiver<Request>,
        band_receiver: Receiver<u64>,
        neighbors: Vec<Neighbor>,
        network_settings: NetworkSettings,
        net_settings_send: Sender<NetworkSettings>,
    ) -> Self {
        let mut gnome = Gnome::new(
            id,
            swarm,
            sender,
            receiver,
            band_receiver,
            network_settings,
            net_settings_send,
        );
        gnome.fast_neighbors = neighbors;
        gnome
    }

    fn serve_user_data(&self) {
        for ((is_bcast, c_id), (recv_d, send_m)) in &self.data_converters {
            while let Ok(data) = recv_d.try_recv() {
                let payload = if *is_bcast {
                    Payload::Broadcast(*c_id, data)
                } else {
                    Payload::Multicast(*c_id, data)
                };
                let message = Message {
                    swarm_time: self.swarm_time,
                    neighborhood: self.neighborhood,
                    header: self.header,
                    payload,
                };
                let _r = send_m.send(message);
                // println!("Converted data send result: {:?}", r);
            }
        }
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
                        self.header,
                        self.neighborhood,
                        self.fast_neighbors.len()
                    );
                }
                Request::AddData(data) => {
                    let b_id = data.get_block_id();
                    self.proposals.push_front(Proposal::Block(b_id, data));
                    new_user_proposal = true;
                    // println!("vvv USER vvv REQ {}", data);
                }
                Request::AddNeighbor(neighbor) => {
                    println!("{} ADD\tadd a new neighbor", neighbor.id);
                    self.add_neighbor(neighbor);
                }
                Request::DropNeighbor(n_id) => {
                    println!("{} DROP\ta neighbor on user request", n_id);
                    self.drop_neighbor(n_id);
                }
                Request::SendData(gnome_id, _request, data) => {
                    println!("Trying to inform neighbor {} about data", gnome_id);
                    let mut data_sent = false;
                    for neighbor in &mut self.fast_neighbors {
                        if neighbor.id == gnome_id {
                            neighbor.add_requested_data(data.clone());
                            data_sent = true;
                            break;
                        }
                    }
                    if !data_sent {
                        for neighbor in &mut self.slow_neighbors {
                            if neighbor.id == gnome_id {
                                neighbor.add_requested_data(data.clone());
                                break;
                            }
                        }
                    }
                }
                Request::AskData(gnome_id, request) => {
                    let mut data_asked = false;
                    for neighbor in &mut self.fast_neighbors {
                        if neighbor.id == gnome_id {
                            neighbor.request_data(request.clone());
                            data_asked = true;
                            break;
                        }
                    }
                    if !data_asked {
                        for neighbor in &mut self.slow_neighbors {
                            if neighbor.id == gnome_id {
                                neighbor.request_data(request.clone());
                                break;
                            }
                        }
                    }
                }
                Request::StartUnicast(gnome_id) => {
                    // println!("Received StartUnicast {:?}", gnome_id);
                    let mut request_sent = false;
                    let mut avail_ids: [CastID; 256] = [CastID(0); 256];
                    // let mut added_ids = 0;
                    for (added_ids, cast_id) in
                        self.swarm.avail_unicast_ids().into_iter().enumerate()
                    {
                        avail_ids[added_ids] = cast_id;
                        // added_ids += 1;
                    }
                    let request =
                        NeighborRequest::UnicastRequest(self.swarm.id, Box::new(avail_ids));
                    for neighbor in &mut self.fast_neighbors {
                        if neighbor.id == gnome_id {
                            println!("Sending UnicastRequest to neighbor");
                            neighbor.request_data(request.clone());
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
                                neighbor.request_data(request.clone());
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
                Request::StartBroadcast => {
                    println!("Received StartBroadcast user request");
                    let cast_id_opt = self.swarm.next_broadcast_id();
                    if cast_id_opt.is_some() {
                        self.proposals
                            .push_front(Proposal::Config(Configuration::StartBroadcast(
                                self.id,
                                cast_id_opt.unwrap().to_owned(),
                            )));
                        new_user_proposal = true;
                    }
                    // println!("vvv USER vvv REQ {}", data);
                }
                Request::StartMulticast(_) => {
                    todo!()
                }
                Request::Custom(_id, _data) => {
                    todo!()
                }
                Request::NetworkSettingsUpdate(notify_neighbor, ip_addr, port, nat) => {
                    self.network_settings.pub_ip = ip_addr;
                    self.network_settings.set_port(port);
                    self.network_settings.nat_type = nat;

                    if notify_neighbor {
                        println!("Trying to notify neighbor");
                        if let Some(ConnRequest {
                            conn_id,
                            neighbor_id,
                            // network_settings,
                        }) = self.pending_conn_requests.pop_front()
                        {
                            let mut neighbor_informed = false;
                            for neighbor in &mut self.fast_neighbors {
                                if neighbor.id == neighbor_id {
                                    neighbor.add_requested_data(NeighborResponse::ConnectResponse(
                                        conn_id,
                                        self.network_settings,
                                    ));
                                    neighbor_informed = true;
                                    break;
                                }
                            }
                            if !neighbor_informed {
                                for neighbor in &mut self.slow_neighbors {
                                    if neighbor.id == neighbor_id {
                                        neighbor.add_requested_data(
                                            NeighborResponse::ConnectResponse(
                                                conn_id,
                                                self.network_settings,
                                            ),
                                        );
                                        neighbor_informed = true;
                                        break;
                                    }
                                }
                            }
                            if neighbor_informed {
                                // let _ = self.net_settings_send.send(network_settings);
                                println!("Sent back ConnResponse");
                            } else {
                                println!("Failed to send response");
                            }
                        }
                    }
                }
            }
        }
        (exit_app, new_user_proposal)
    }

    // ongoing requests should be used to track which neighbor is currently selected for
    // making a connection with another neighbor.
    // it should contain gnome_id to identify which gnome is requesting to connect
    // another gnome_id should identify currently queried neighbor,
    // an u8 value should be used as an identifier of messages received from neighbors
    // so that we can help multiple neighbors simultaneously
    // Also each ongoing request should contain a SwarmTime timestamp marking when given
    // query was sent, in order to timeout on unresponsive gnome
    //
    // if there is only one neighbor we simply send back a failure message to originating
    // neighbor, since we do not have other neighbors to connect to.
    fn add_ongoing_request(
        &mut self,
        // neighbor_count: usize,
        origin: GnomeId,
        network_settings: NetworkSettings,
    ) {
        let neighbor_count = self.fast_neighbors.len() + self.slow_neighbors.len();
        if neighbor_count < 2 {
            println!("Not enough neighbors: {}", neighbor_count);
            let mut response_sent = false;
            for neighbor in &mut self.fast_neighbors {
                if neighbor.id == origin {
                    neighbor.add_requested_data(NeighborResponse::ForwardConnectFailed);
                    response_sent = true;
                    break;
                }
            }
            if !response_sent {
                for neighbor in &mut self.slow_neighbors {
                    if neighbor.id == origin {
                        neighbor.add_requested_data(NeighborResponse::ForwardConnectFailed);
                        break;
                    }
                }
            }
            return;
        }
        // if self.ongoing_requests.len() == 256{
        //     println!("Maximum amount of ongoing requests");
        //     return;
        // }
        let mut id: u8 = 0;
        while self.ongoing_requests.contains_key(&id) {
            id += 1;
        }
        let mut queried_neighbor: Option<GnomeId> = None;
        for neigh in &mut self.fast_neighbors {
            if neigh.id != origin {
                queried_neighbor = Some(neigh.id);
                neigh.request_data(NeighborRequest::ConnectRequest(
                    id,
                    origin,
                    network_settings,
                ));
                break;
            }
        }
        if queried_neighbor.is_none() {
            for neigh in &mut self.slow_neighbors {
                if neigh.id != origin {
                    queried_neighbor = Some(neigh.id);
                    neigh.request_data(NeighborRequest::ConnectRequest(
                        id,
                        origin,
                        network_settings,
                    ));
                    break;
                }
            }
        }
        let queried_neighbors = if queried_neighbor.is_none() {
            vec![]
        } else {
            vec![queried_neighbor.unwrap()]
        };
        let or = OngoingRequest {
            origin,
            queried_neighbors,
            timestamp: self.swarm_time,
            response: None,
            network_settings,
        };
        self.ongoing_requests.insert(id, or);
    }

    fn serve_connect_request(
        &mut self,
        id: u8,
        reply_gnome: GnomeId,
        origin: GnomeId,
        network_settings: NetworkSettings,
    ) -> Option<NeighborResponse> {
        for neighbor in &self.fast_neighbors {
            if neighbor.id == origin {
                return Some(NeighborResponse::AlreadyConnected(id));
            }
        }
        for neighbor in &self.slow_neighbors {
            if neighbor.id == origin {
                return Some(NeighborResponse::AlreadyConnected(id));
            }
        }
        // TODO: vary response depending on available bandwith
        // TODO: In some cases we need to split this procedure in two:
        // - first we ask networking about our current network settings
        // - second once our network settings are refreshed we send
        //   another message to networking to connect to neighbor
        println!("Trying to notify neighbor");
        //   and also we return a NeighborResponse with our updated
        //   network settings

        // if self.network_settings.refresh_required() {
        self.pending_conn_requests.push_back(ConnRequest {
            conn_id: id,
            neighbor_id: reply_gnome,
            // network_settings,
        });
        // We send None to notify networking we want it to send us
        // back refreshed NetworkSettings
        //     let _ = self.net_settings_send.send(None);
        //     None
        // } else {
        let _ = self
            .net_settings_send
            // .send((self.network_settings, Some(network_settings)));
            .send(network_settings);
        None
        // Some(NeighborResponse::ConnectResponse(id, self.network_settings))
        // }
    }

    // TODO: find where to apply this function
    fn add_ongoing_reply(&mut self, id: u8, network_settings: NetworkSettings) {
        let opor = self.ongoing_requests.remove(&id);
        if opor.is_some() {
            let mut o_req = opor.unwrap();
            o_req.response = Some(network_settings);
            self.ongoing_requests.insert(id, o_req);
        } else {
            println!("No ongoing request with id: {}", id);
        }
    }

    fn skip_neighbor(&mut self, id: u8) {
        let mut key_to_remove = None;
        let option_req = self.ongoing_requests.get_mut(&id);
        if let Some(v) = option_req {
            let mut neighbor_found = false;
            for neighbor in &mut self.fast_neighbors {
                if !v.queried_neighbors.contains(&neighbor.id) {
                    neighbor_found = true;
                    neighbor.request_data(NeighborRequest::ConnectRequest(
                        id,
                        v.origin,
                        v.network_settings,
                    ));
                    break;
                }
            }
            if !neighbor_found {
                for neighbor in &mut self.slow_neighbors {
                    if !v.queried_neighbors.contains(&neighbor.id) {
                        neighbor_found = true;
                        neighbor.request_data(NeighborRequest::ConnectRequest(
                            id,
                            v.origin,
                            v.network_settings,
                        ));
                        break;
                    }
                }
            }
            if !neighbor_found {
                println!("Unable to find more neighbors for {}", id);
                let mut response_sent = false;
                for neighbor in &mut self.fast_neighbors {
                    if neighbor.id == v.origin {
                        neighbor.add_requested_data(NeighborResponse::ForwardConnectFailed);
                        response_sent = true;
                        break;
                    }
                }
                if !response_sent {
                    for neighbor in &mut self.slow_neighbors {
                        if neighbor.id == v.origin {
                            neighbor.add_requested_data(NeighborResponse::ForwardConnectFailed);
                            break;
                        }
                    }
                }
                key_to_remove = Some(id);
            } else {
                v.timestamp = self.swarm_time;
            }
        }
        if let Some(key) = key_to_remove {
            self.ongoing_requests.remove(&key);
        }
    }

    // Here we iterate over every element in ongoing requests
    // If a neighbor sends back a response to our query,
    // this response is also inserted into ongoing requests.
    // When we iterate over ongoing requests and see that given item
    // contains a neighbor response
    // we take that response and send it back to originating gnome
    // for direct connection establishment
    // If we find that there is no response, we can do two things:
    // if a request was sent to a particular gnome long ago,
    // we can select another neighbor and send him a query, resetting timestamp.
    // if a request is fairly recent, we simply move on
    //
    // once we find out all neighbors have been queried we no longer need given item in
    // ongoing requests, so we drop it and put back it's u8 identifier to available set
    // we also send back a reply to originating neighbor
    //
    // TODO: cover a case when queried neighbor already is connected to origin
    fn serve_ongoing_requests(&mut self) {
        let mut keys_to_remove: Vec<u8> = vec![];
        for (k, v) in &mut self.ongoing_requests {
            if v.response.is_some() {
                println!("Sending response: {:?}", v.response);
                let mut response_sent = false;
                for neighbor in &mut self.fast_neighbors {
                    if neighbor.id == v.origin {
                        neighbor.add_requested_data(NeighborResponse::ForwardConnectResponse(
                            v.response.unwrap(),
                        ));
                        response_sent = true;
                        break;
                    }
                }
                if !response_sent {
                    for neighbor in &mut self.slow_neighbors {
                        if neighbor.id == v.origin {
                            neighbor.add_requested_data(NeighborResponse::ForwardConnectResponse(
                                v.response.unwrap(),
                            ));
                            break;
                        }
                    }
                }
                keys_to_remove.push(*k);
            } else {
                let time_delta = self.swarm_time - v.timestamp;
                if time_delta > SwarmTime(100) {
                    let mut neighbor_found = false;
                    for neighbor in &mut self.fast_neighbors {
                        if !v.queried_neighbors.contains(&neighbor.id) {
                            neighbor_found = true;
                            neighbor.request_data(NeighborRequest::ConnectRequest(
                                *k,
                                v.origin,
                                v.network_settings,
                            ));
                            break;
                        }
                    }
                    if !neighbor_found {
                        for neighbor in &mut self.slow_neighbors {
                            if !v.queried_neighbors.contains(&neighbor.id) {
                                neighbor_found = true;
                                neighbor.request_data(NeighborRequest::ConnectRequest(
                                    *k,
                                    v.origin,
                                    v.network_settings,
                                ));
                                break;
                            }
                        }
                    }
                    if !neighbor_found {
                        println!("Unable to find more neighbors for {}", k);
                        let mut response_sent = false;
                        for neighbor in &mut self.fast_neighbors {
                            if neighbor.id == v.origin {
                                neighbor.add_requested_data(NeighborResponse::ForwardConnectFailed);
                                response_sent = true;
                                break;
                            }
                        }
                        if !response_sent {
                            for neighbor in &mut self.slow_neighbors {
                                if neighbor.id == v.origin {
                                    neighbor
                                        .add_requested_data(NeighborResponse::ForwardConnectFailed);
                                    break;
                                }
                            }
                        }
                        keys_to_remove.push(*k);
                    } else {
                        v.timestamp = self.swarm_time;
                    }
                }
            }
        }
        for key in keys_to_remove {
            self.ongoing_requests.remove(&key);
        }
    }

    fn serve_sync_requests(&mut self) {
        if self.new_neighbors.is_empty() {
            return;
        }
        // println!("Serving Sync Swarm request");
        let message = self.prepare_message();
        let mut processed_neighbors = vec![];
        // for neighbor in self.new_neighbors {
        while let Some(mut neighbor) = self.new_neighbors.pop() {
            let _ = neighbor.try_recv(self.next_state.last_accepted_message.clone());
            if let Some(request) = neighbor.requests.pop_back() {
                if let NeighborRequest::SwarmSyncRequest = request {
                    println!("Serving some SyncRequest!");
                    let b_count = self.swarm.broadcasts_count();
                    let m_count = self.swarm.multicasts_count();
                    let b_casts = if b_count == 0 {
                        vec![]
                    } else {
                        self.swarm.broadcast_ids()
                    };

                    let m_casts = if m_count == 0 {
                        vec![]
                    } else {
                        self.swarm.multicast_ids()
                    };

                    let chill_phase = if self.chill_out.0 {
                        self.chill_out.1
                    } else {
                        0
                    };
                    let response = NeighborResponse::SwarmSync(chill_phase, b_casts, m_casts);
                    // neighbor.add_requested_data(response);
                    // let message = self.prepare_message();
                    neighbor.swarm_time = message.swarm_time;
                    neighbor.start_new_round(self.swarm_time);
                    neighbor.send_out(message.include_response(response));
                    // self.refreshed_neighbors.push(neighbor);
                    self.fast_neighbors.push(neighbor);
                } else {
                    processed_neighbors.push(neighbor);
                }
            } else {
                processed_neighbors.push(neighbor);
            }
        }
        self.new_neighbors = processed_neighbors;
    }

    fn serve_neighbors_requests(&mut self, refreshed: bool) {
        let mut neighbors = if refreshed {
            std::mem::take(&mut self.refreshed_neighbors)
        } else {
            std::mem::take(&mut self.fast_neighbors)
        };
        // let n_count = neighbors.len();
        let mut pending_ongoing_requests = vec![];
        for neighbor in &mut neighbors {
            if let Some(request) = neighbor.requests.pop_back() {
                println!("Some neighbor request! {:?}", request);
                // println!("Some neighbor request!");
                match request {
                    NeighborRequest::UnicastRequest(_swarm_id, cast_ids) => {
                        for cast_id in cast_ids.deref() {
                            if self.swarm.is_unicast_id_available(*cast_id) {
                                self.swarm.insert_unicast(*cast_id, neighbor.id);
                                neighbor.add_requested_data(NeighborResponse::Unicast(
                                    self.swarm.id,
                                    *cast_id,
                                ));
                                break;
                            }
                        }
                    }
                    NeighborRequest::ForwardConnectRequest(network_settings) => {
                        println!("ForwardConnReq");
                        pending_ongoing_requests.push((neighbor.id, network_settings));
                    }
                    NeighborRequest::ConnectRequest(id, gnome_id, network_settings) => {
                        println!("ConnReq");
                        if let Some(response) =
                            self.serve_connect_request(id, neighbor.id, gnome_id, network_settings)
                        {
                            neighbor.add_requested_data(response);
                        }
                    }
                    NeighborRequest::SwarmSyncRequest => {
                        println!("SwarmSyncRequest");
                        let b_count = self.swarm.broadcasts_count();
                        let m_count = self.swarm.multicasts_count();
                        let b_casts = if b_count == 0 {
                            vec![]
                        } else {
                            self.swarm.broadcast_ids()
                        };

                        let m_casts = if m_count == 0 {
                            vec![]
                        } else {
                            self.swarm.multicast_ids()
                        };

                        let chill_phase = if self.chill_out.0 {
                            self.chill_out.1
                        } else {
                            0
                        };
                        let response = NeighborResponse::SwarmSync(chill_phase, b_casts, m_casts);
                        let message = Message::new(
                            self.swarm_time,
                            self.header,
                            Payload::Response(response),
                            self.neighborhood,
                        );
                        neighbor.swarm_time = self.swarm_time;
                        neighbor.start_new_round(self.swarm_time);
                        neighbor.send_out(message);
                    }
                    NeighborRequest::SubscribeRequest(is_bcast, cast_id) => {
                        if let Some(origin) = self.swarm.add_subscriber(
                            is_bcast,
                            &cast_id,
                            neighbor.id,
                            neighbor.sender.clone(),
                        ) {
                            neighbor.add_requested_data(NeighborResponse::Subscribed(
                                is_bcast, cast_id, origin, None,
                            ));
                        } else {
                            // TODO: send response with failure indication
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
        let _ = if refreshed {
            std::mem::replace(&mut self.refreshed_neighbors, neighbors)
        } else {
            std::mem::replace(&mut self.fast_neighbors, neighbors)
        };

        for (id, net_set) in pending_ongoing_requests {
            self.add_ongoing_request(id, net_set);
        }
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
        self.sync_with_swarm();
        // let mut _guard = self.start_new_timer(self.timeout_duration, timer_sender.clone(), None);
        let mut _guard = self.start_new_timer(Duration::from_secs(16), timer_sender.clone(), None);

        loop {
            let (exit_app, new_user_proposal) = self.serve_user_requests();
            self.serve_user_data();
            self.serve_sync_requests();
            self.serve_neighbors_requests(true);
            self.serve_neighbors_requests(false);
            self.serve_ongoing_requests();
            self.swarm.serve_casts();
            // let refr_new_proposal = self.try_recv_refreshed();
            // print!(
            //     "F:{}s:{},r:{},n:{}",
            //     self.fast_neighbors.len(),
            //     self.slow_neighbors.len(),
            //     self.refreshed_neighbors.len(),
            //     self.new_neighbors.len()
            // );
            let (fast_advance_to_next_turn, fast_new_proposal) = self.try_recv(true);
            let (slow_advance_to_next_turn, slow_new_proposal) = self.try_recv(false);
            // if fast_new_proposal || slow_new_proposal {
            //     println!("New neighbor proposal");
            // }
            if let Ok(band) = self.band_receiver.try_recv() {
                available_bandwith = band;
                // println!("Avail bandwith: {}", available_bandwith);
                //TODO make use of available_bandwith during multicasting setup
            }
            let timeout = timeout_receiver.try_recv().is_ok();
            let advance_to_next_turn = fast_advance_to_next_turn || slow_advance_to_next_turn;
            let new_proposal = new_user_proposal || fast_new_proposal || slow_new_proposal;
            // || refr_new_proposal;
            // TODO: here we need to make use of self.chill_out attribute
            // Following needs to be implemented for cases like (Forward)ConnectRequests.
            // Need to find a way to always clear any data we have to send to our neighbors.
            // Maybe we can send that data without updating state...
            // Then only first message will pass sanity @ neighbor, following messages
            // will fail sanity, but requested data should be served...done?

            // maybe self.send_immediate should no longer be...

            // chill out mode may end abruptly in case new_proposal has been received
            // if new_user_proposal {
            //     println!("new user proposal");
            // }
            // if fast_new_proposal {
            //     println!("fast new proposal");
            // }
            // if slow_new_proposal {
            //     println!("slow new proposal");
            // }
            if new_proposal
            // || advance_to_next_turn
            //     && self.next_state.last_accepted_message.swarm_time == SwarmTime(0)
            {
                if self.chill_out.0 {
                    // println!("Chill out is terminated abruptly");
                    self.send_immediate = true;
                }
                self.chill_out.0 = false;
                self.chill_out.1 = 250;
            }
            if self.chill_out.0 {
                if self.chill_out.1 == 250 {
                    // println!("Let's chill out");
                    // We need to communicate to the timeout mechanism to pause his countdown.
                    // We can do this by droping guard.
                    // TODO: most probably there is a better way...
                    let (s, _r) = channel();
                    _guard = s;
                    drop(_r);
                } else if self.chill_out.1 == 0 {
                    // If self.chill_out.1 reaches 0 self._chill_out.0 =false and it's time to work.
                    println!(
                        "Chill out is over fast:{}, slow:{}, refr:{}",
                        self.fast_neighbors.len(),
                        self.slow_neighbors.len(),
                        self.refreshed_neighbors.len()
                    );
                    self.chill_out.0 = false;
                    // When we end chill_out mode, we have to start new timer.
                    _guard = self.start_new_timer(
                        self.timeout_duration,
                        timer_sender.clone(),
                        Some(_guard),
                    );
                    continue;
                }
                // in case we are in chill out mode we go to sleep for self.chill_out.2
                // and we decrese self.chill_out.1 by 1 and continue loop.
                self.chill_out.1 -= 1;
                // print!("c");
                thread::sleep(self.chill_out.2);
                continue;
            }
            // We still need a way to suspend chill_out when we have non-critical data to send
            // and later resume from suspension;
            // We also need a place where we initialize chill mode.

            // in other case we execute following code
            if advance_to_next_turn
                || self.send_immediate
                || timeout
                    && !(self.fast_neighbors.is_empty()
                        && self.slow_neighbors.is_empty()
                        && self.refreshed_neighbors.is_empty())
            {
                self.update_state();
                if !new_proposal && !self.send_immediate {
                    // println!("swap&send");
                    self.swap_neighbors();
                    self.send_specialized(true);
                } else {
                    // println!("konkat&send");
                    self.concat_neighbors();
                    self.send_all();
                }
                self.send_immediate = false;
                if self.check_if_new_round(available_bandwith)
                    && self.neighbor_discovery.tick_and_check()
                    // TODO: figure out some better algo
                    && available_bandwith > 256
                {
                    self.query_for_new_neighbors();
                }
                _guard =
                    self.start_new_timer(self.timeout_duration, timer_sender.clone(), Some(_guard));
            } else if self.chill_out.0 {
                // If we have some specialized data to send to one or more of our neighbors
                // we suspend chill mode, send data, then continue to chill -- needs analysis
                // If we send specialized data to our neighbors, they receive it, but they
                // do not end their chill_out mode, since no new proposal has been sent.
                self.send_specialized(false);
            }
            if exit_app {
                break;
            };
            // TODO: remove this once chill_out logic is implemented
            // thread::sleep(Duration::from_millis(250));
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
            // println!("f");
            neighbor.send_out(message.clone());
        }
        for neighbor in &mut self.slow_neighbors {
            // println!("s");
            neighbor.send_out(message.clone());
        }
        // for neighbor in &mut self.new_neighbors {
        //     neighbor.send_out(message);
        // }
    }

    pub fn send_specialized(&mut self, send_default: bool) {
        let message = self.prepare_message();
        // let served_neighbors = Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME);
        // let unserved_neighbors = if fast {
        // std::mem::replace(&mut self.fast_neighbors, served_neighbors)
        //     &mut self.fast_neighbors
        // } else {
        // std::mem::replace(&mut self.slow_neighbors, served_neighbors)
        //     &mut self.slow_neighbors
        // };
        // let mut generic_info_printed = false;
        // println!("Sending out specialized message to neighbors");
        if !send_default {
            for neighbor in &mut self.refreshed_neighbors {
                neighbor.send_out_specialized_message(&message, self.id, send_default);
                // if fast {
                //     self.fast_neighbors.push(neighbor);
                // } else {
                //     self.slow_neighbors.push(neighbor);
                // }
            }
        }
        eprintln!("{} >>> {}", self.id, message);
        for neighbor in &mut self.fast_neighbors {
            neighbor.send_out_specialized_message(&message, self.id, send_default);
        }
        for neighbor in &mut self.slow_neighbors {
            neighbor.send_out_specialized_message(&message, self.id, send_default);
        }
        // for neighbor in &mut self.new_neighbors {
        //     neighbor.send_out(message);
        // }
    }

    pub fn prepare_message(&self) -> Message {
        // let (header, payload) = if self.block_id.0 == 0 {
        //     if self.data == Data(0) {
        //         (Header::Sync, Payload::KeepAlive)
        //     } else {
        //         let data_bytes = Vec::from(self.data.0.to_be_bytes());
        //         let first_byte = data_bytes.iter().next().unwrap();
        //         if *first_byte == 255 {
        //             (Header::Sync, Payload::Bye)
        //         } else {
        //             let config = match *first_byte {
        //                 // TODO: cover more cases, redefine Data as Vec<u8>
        //                 // first byte defines config type, following bytes define payload
        //                 // may also include first byte
        //                 254 => Configuration::StartBroadcast,
        //                 253 => Configuration::ChangeBroadcastSource,
        //                 252 => Configuration::EndBroadcast,
        //                 251 => Configuration::StartMulticast,
        //                 250 => Configuration::ChangeMulticastSource,
        //                 249 => Configuration::EndMulticast,
        //                 248 => Configuration::CreateGroup,
        //                 247 => Configuration::DeleteGroup,
        //                 246 => Configuration::ModifyGroup,
        //                 other => Configuration::UserDefined(other),
        //             };
        //             (Header::Reconfigure, Payload::Reconfigure(config))
        //         }
        //     }
        // } else {
        //     (
        //         Header::Block(self.block_id),
        //         Payload::Block(self.block_id, self.data),
        //     )
        // };
        // println!("prep msg nhood: {}", self.neighborhood);
        Message {
            swarm_time: self.swarm_time,
            neighborhood: self.neighborhood,
            header: self.header,
            payload: self.payload.clone(),
        }
    }

    // TODO: first we need to get in sync with rest of the swarm both regarding
    //       SwarmTime and ChillOut phase.
    //       To do so we send sync request and await for response
    //       If we also receive a sync request then it means we just started
    //       a new swarm or an isolated from the rest swarm - we simply continue
    //       If we receive a Sync Response it should contain proper SwarmTime
    //       and also how many iterations of ChillOut mode are currently left.
    //       We apply those parameters to our state and continue to loop.
    //       If we receive any other message we set ChillOut to false and continue.
    fn sync_with_swarm(&mut self) {
        // let request = ;
        let message = self
            .prepare_message()
            .include_request(NeighborRequest::SwarmSyncRequest);
        let response_opt = if let Some(neighbor) = self.fast_neighbors.iter_mut().next() {
            // neighbor.request_data(request);
            // let new_message = message
            println!("{} >S> {}", neighbor.id, message);
            neighbor.send_out(message);
            if let Some(response) = neighbor.recv(Duration::from_secs(20)) {
                self.next_state.update(neighbor);
                // self.next_state
                //     .reset_for_next_turn(true, Header::Sync, Payload::KeepAlive(1024));
                neighbor.swarm_time = response.swarm_time;
                neighbor.start_new_round(response.swarm_time);
                // self.update_state();
                Some(response)
            } else {
                None
            }
        } else if let Some(neighbor) = self.slow_neighbors.iter_mut().next() {
            // neighbor.request_data(request);
            neighbor.send_out(message);
            if let Some(response) = neighbor.recv(Duration::from_secs(20)) {
                self.next_state.update(neighbor);
                // self.next_state
                //     .reset_for_next_turn(true, Header::Sync, Payload::KeepAlive(1024));
                neighbor.swarm_time = response.swarm_time;
                neighbor.start_new_round(response.swarm_time);
                // self.update_state();
                Some(response)
            } else {
                None
            }
        } else {
            None
        };
        if let Some(response) = response_opt {
            println!("Sync response: {}", response);
            if matches!(
                response.payload,
                Payload::Request(NeighborRequest::SwarmSyncRequest)
            ) {
                self.send_all();
                return;
            }
            self.swarm_time = response.swarm_time;
            self.round_start = response.swarm_time;
            self.next_state.swarm_time = response.swarm_time;
            self.next_state.last_accepted_message = response.clone();
            if let Payload::Response(NeighborResponse::SwarmSync(chill_phase, _b_casts, _m_casts)) =
                response.payload
            {
                if chill_phase > 0 {
                    println!("Into chill {}", chill_phase);
                    self.chill_out.0 = true;
                    self.chill_out.1 = chill_phase;
                } else {
                    println!("no chill ");
                    self.chill_out.0 = false;
                    self.chill_out.1 = chill_phase;
                }
                // self.send_all();
                // TODO: include b_casts and m_casts in self.swarm
            }
        }
    }

    fn swap_neighbors(&mut self) {
        // println!("Swapping neighbors");
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
        // println!(
        //     "After swap fast: {}, slow: {}, refr: {}",
        //     self.fast_neighbors.len(),
        //     self.slow_neighbors.len(),
        //     self.refreshed_neighbors.len()
        // );
    }

    fn concat_neighbors(&mut self) {
        self.fast_neighbors.append(&mut self.refreshed_neighbors);
    }

    fn update_state(&mut self) {
        // TODO: before we modify current params, we need to check if
        // we are inside an reconfigure round.
        // If that is the case, we need to modify self.pending_casting
        // with proper data.
        // Pending casting should be moved to active one at the end of round
        // if it is the same data we are accepting
        // Once active, a gnome will send broadcast messages to all his
        // neighbors except the source gnome.
        // Other neighbors will send direct Unsubscribe message to his
        // neighbors in order to filter out all gnomes but the one selected to
        // be it's source.
        // Gnome receiving Unsubscribe message will remove requesting neighbor from
        // broadcasting subscribers list.
        // If a gnome wishes to change a broadcasting source he should send
        // a direct Subscribe message to selected neighbor, with optional
        // Unsubscribe message to current source. Source gnome should
        // respond with Subscribed message.
        let (n_st, n_neigh, n_head, n_payload) = self.next_state.next_params();
        // println!("Next params: {} {} {} {}", n_st, n_neigh, n_head, n_payload);
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
        // self.block_id = n_bid;
        // self.data = n_data;
        self.header = n_head;
        self.payload = n_payload;
        // if n_bid == BlockID(0) && n_data == Data(0) {
        if self.header == Header::Sync {
            if let Some(proposal) = self.proposals.pop_back() {
                let (head, payload) = proposal.as_header_payload();
                self.header = head;
                self.payload = payload;
                if self.header.is_reconfigure() {
                    if let Payload::Reconfigure(config) = self.payload {
                        if let Configuration::StartBroadcast(g_id, c_id) = config {
                            self.next_state.change_config = ChangeConfig::AddBroadcast {
                                id: c_id,
                                origin: g_id,
                                source: g_id,
                                filtered_neighbors: vec![],
                                turn_ended: false,
                            };
                        }
                    }
                }
                // println!("some");
                // match proposal {
                //     Proposal::Block(b_id, data) => {
                //         // self.block_id = BlockID(data.0);
                //         // self.data = data;
                //         self.header = Header::Block(b_id);
                //         self.payload = Payload::Block(b_id, data);
                //     }
                //     Proposal::Config(config) => {
                //         // self.block_id = BlockID(0);
                //         // self.data = Data(config.as_u32())
                //         self.header = Header::Reconfigure;
                //         self.payload = Payload::Reconfigure(config);
                //     }
                // }
                self.my_proposal = Some(proposal);
                self.send_immediate = true;
            }
        } else if self.header.is_reconfigure() {
            match self.payload {
                Payload::Reconfigure(config) => match config {
                    Configuration::StartBroadcast(_gid, _cid) => {
                        // if self.pending_casting.
                    }
                    _ => {
                        println!("Unhandled config payload");
                    }
                },
                _ => {
                    println!("Unexpected payload while for Reconfigure header");
                }
            }
        }
    }

    fn check_if_new_round(&mut self, available_bandwith: u64) -> bool {
        let all_gnomes_aware = self.neighborhood.0 as u32 >= self.swarm_diameter.0;
        let finish_round =
            self.swarm_time - self.round_start >= self.swarm_diameter + self.swarm_diameter;
        if all_gnomes_aware || finish_round {
            // println!("New round");
            // let block_non_zero = self.block_id > BlockID(0);
            let block_proposed = self.header.non_zero_block();
            let reconfig = self.header.is_reconfigure();
            if block_proposed || reconfig {
                self.send_immediate = true;
                if all_gnomes_aware {
                    self.next_state.last_accepted_message = self.prepare_message();
                    if block_proposed {
                        if let Payload::Block(block_id, data) = self.payload {
                            let _res = self.sender.send(Response::Block(block_id, data));
                            // println!("^^^ USER ^^^ NEW {} {:075}", block_id, data.0);
                        } else {
                            println!("Can not send to user, payload not matching");
                        }
                    } else {
                        println!("We have got a Reconfig to parse");
                        let change_config = std::mem::replace(
                            &mut self.next_state.change_config,
                            ChangeConfig::None,
                        );
                        println!("Me: {}, c-k: {:?}", self.id, change_config);
                        match change_config {
                            ChangeConfig::AddBroadcast {
                                id,
                                origin,
                                source,
                                filtered_neighbors,
                                ..
                            } => {
                                let mut subscribers: HashMap<GnomeId, Sender<Message>> =
                                    self.get_neighbor_ids_and_senders();
                                subscribers.retain(|&gid, _s| !filtered_neighbors.contains(&gid));
                                let (send_n, recv_n) = channel();
                                let (send_d, recv_d) = channel();
                                if self.id == origin {
                                    println!("I am origin!");
                                    // TODO: need to handle Data from recv_d and
                                    //       push it into send_n
                                    let b_cast = Multicast::new(
                                        origin,
                                        (source, recv_n),
                                        filtered_neighbors,
                                        subscribers,
                                        None,
                                    );

                                    self.insert_originating_broadcast(id, recv_d, send_n);
                                    self.swarm.insert_broadcast(id, b_cast);
                                    let _res = self.sender.send(Response::BroadcastOrigin(
                                        self.swarm.id,
                                        id,
                                        send_d,
                                    ));
                                } else if self.activate_broadcast_at_neighbor(source, id, send_n) {
                                    // TODO: we need to create channel to actually pass messages
                                    //       to user
                                    //       and to subscribers
                                    let b_cast = Multicast::new(
                                        origin,
                                        (source, recv_n),
                                        filtered_neighbors,
                                        subscribers,
                                        Some(send_d),
                                    );
                                    self.swarm.insert_broadcast(id, b_cast);
                                    let _res = self.sender.send(Response::Broadcast(
                                        self.swarm.id,
                                        id,
                                        recv_d,
                                    ));
                                } else {
                                    println!("Unable to activate broadcast");
                                }
                            }
                            ChangeConfig::None => {}
                        }
                        // self.next_state.last_accepted_reconf =
                        //     Some(Configuration::from_u32(self.data.0));
                    }
                    if let Some(my_proposed_data) = self.my_proposal {
                        // TODO: here we need to distinguish between Block and Reconfig
                        let (head, payload) = my_proposed_data.as_header_payload();
                        if (head, payload)
                            != (
                                self.next_state.last_accepted_message.header,
                                self.next_state.last_accepted_message.payload.clone(),
                            )
                        {
                            self.proposals.push_back(my_proposed_data);
                        }
                        // match my_proposed_data {
                        //     Proposal::Block(b_id, data) => {
                        //         // if data.0 != self.block_id.0 {
                        //         if data.0 != self.block_id.0 {
                        //             self.proposals.push_back(my_proposed_data);
                        //         }
                        //     }
                        //     Proposal::Config(config) => {
                        //         if self.block_id.0 != 0 || self.data.0 != config.as_u32() {
                        //             self.proposals.push_back(my_proposed_data);
                        //         }
                        //     }
                        // }
                        self.my_proposal = None;
                    }

                    self.round_start = self.swarm_time;
                    // println!("New round start: {}", self.round_start);
                    // self.next_state.last_accepted_block = self.block_id;
                } else {
                    panic!(
                        "ERROR: Swarm diameter too small or {} was backdated! Rstart:{}",
                        self.header, self.round_start
                    );
                }
                if let Some(proposal) = self.proposals.pop_back() {
                    // println!("some");
                    // match proposal {
                    //     Proposal::Block(data) => {
                    //         self.block_id = BlockID(data.0);
                    //         self.data = data;
                    //     }
                    //     Proposal::Config(config) => {
                    //         self.block_id = BlockID(0);
                    //         self.data = Data(config.as_u32());
                    //     }
                    // }
                    let (head, payload) = proposal.as_header_payload();
                    self.header = head;
                    self.payload = payload;
                    self.my_proposal = Some(proposal);
                    self.send_immediate = true;
                } else {
                    // println!("none");
                    // self.block_id = BlockID(0);
                    // self.data = Data(0);
                    self.header = Header::Sync;
                    self.payload = Payload::KeepAlive(available_bandwith);
                    self.chill_out.0 = true;
                    self.chill_out.1 = 250;
                }
            // } else if self.block_id == BlockID(0) && self.data.0 > 0 {
            // println!("We have got a Reconfig to parse");
            } else {
                // Sync swarm time
                // self.swarm_time = self.round_start + self.swarm_diameter;
                // println!("Sync swarm time {}", self.swarm_time);
                // self.next_state.swarm_time = self.swarm_time;
                self.round_start = self.swarm_time;
                self.next_state.last_accepted_message = self.prepare_message();
                // if self.neighborhood.0 as u32 >= self.swarm_diameter.0 {
                // println!("set N-0");
                self.neighborhood = Neighborhood(0);
                // }
                // self.next_state.all_neighbors_same_header = false;
                // println!("--------round start to: {}", self.swarm_time);
                if let Some(proposal) = self.proposals.pop_back() {
                    // match proposal {
                    //     Proposal::Block(data) => {
                    //         self.block_id = BlockID(data.0);
                    //         self.data = data;
                    //     }
                    //     Proposal::Config(config) => {
                    //         self.block_id = BlockID(0);
                    //         self.data = Data(config.as_u32());
                    //     }
                    // }
                    let (head, payload) = proposal.as_header_payload();
                    self.header = head;
                    self.payload = payload;
                    self.my_proposal = Some(proposal);
                    self.send_immediate = true;
                } else {
                    self.chill_out.0 = true;
                    self.chill_out.1 = 250;
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
                        let _ = neighbor.try_recv(
                            self.next_state.last_accepted_message.clone(),
                            // self.next_state.last_accepted_reconf,
                        );
                        // println!("snd2 {}", msg);
                        neighbor.send_out(msg.clone());
                        self.fast_neighbors.push(neighbor);
                    }
                }
            }

            self.next_state
                .reset_for_next_turn(true, self.header, self.payload.clone());
            for neighbor in &mut self.fast_neighbors {
                // println!("fast");
                neighbor.start_new_round(self.swarm_time);
            }
            for neighbor in &mut self.slow_neighbors {
                // println!("slow");
                neighbor.start_new_round(self.swarm_time);
            }
            true
        } else {
            self.next_state
                .reset_for_next_turn(false, self.header, self.payload.clone());
            false
        }
    }

    fn try_recv_refreshed(&mut self) -> bool {
        let mut new_proposal_received = false;
        let refr_len = self.refreshed_neighbors.len();
        if refr_len == 0 {
            return false;
        }
        let loop_neighbors =
            std::mem::replace(&mut self.refreshed_neighbors, Vec::with_capacity(refr_len));
        for mut neighbor in loop_neighbors {
            while let Some(response) = neighbor.user_responses.pop_back() {
                match response {
                    Response::ToGnome(neighbor_response) => match neighbor_response {
                        NeighborResponse::ConnectResponse(id, net_set) => {
                            self.add_ongoing_reply(id, net_set);
                        }
                        NeighborResponse::AlreadyConnected(id) => {
                            self.skip_neighbor(id);
                        }
                        NeighborResponse::ForwardConnectResponse(net_set) => {
                            println!("ForwardConnResponse: {:?}", net_set);
                            let _ = self
                                .net_settings_send
                                // .send((self.network_settings, Some(net_set)));
                                .send(net_set);
                        }
                        NeighborResponse::ForwardConnectFailed => {
                            // TODO build querying mechanism
                            // TODO inform gnome's mechanism to ask another neighbor
                        }
                        NeighborResponse::SwarmSync(chill_phase, b_casts, m_casts) => {
                            // TODO serve this
                            println!("Received SwarmSync response");
                            if let Some(neighbor) = self
                                .fast_neighbors
                                .iter_mut()
                                .max_by(|n, m| n.available_bandwith.cmp(&m.available_bandwith))
                            {
                                println!("Do something");
                                for b_cast_id in b_casts {
                                    neighbor.request_data(NeighborRequest::SubscribeRequest(
                                        true, b_cast_id,
                                    ))
                                }
                                for m_cast_id in m_casts {
                                    neighbor.request_data(NeighborRequest::SubscribeRequest(
                                        false, m_cast_id,
                                    ))
                                }
                            }
                        }
                        NeighborResponse::Subscribed(is_bcast, cast_id, origin_id, source_opt) => {
                            // TODO: handle this
                        }
                        other => {
                            println!("Uncovered NeighborResponse: {:?}\nuncovered", other);
                        }
                    },
                    _ => {
                        println!("Got response: {:?}\nunderscore", response);
                        let _ = self.sender.send(response);
                    }
                }
            }
            let (served, sanity_passed, new_proposal, drop_me) = neighbor.try_recv(
                self.next_state.last_accepted_message.clone(),
                // self.next_state.last_accepted_reconf,
            );
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
                }
            }
            if !drop_me {
                self.refreshed_neighbors.push(neighbor);
            } else {
                println!("Dropping neighbor");
            }
        }
        new_proposal_received
    }

    fn try_recv(&mut self, fast: bool) -> (bool, bool) {
        let n_len = if fast {
            self.fast_neighbors.len()
        } else {
            self.slow_neighbors.len()
        };
        if n_len == 0 {
            return (false, false);
        }
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
                match response {
                    Response::ToGnome(neighbor_response) => match neighbor_response {
                        NeighborResponse::ConnectResponse(id, net_set) => {
                            self.add_ongoing_reply(id, net_set);
                        }
                        NeighborResponse::AlreadyConnected(id) => {
                            self.skip_neighbor(id);
                        }
                        NeighborResponse::ForwardConnectResponse(net_set) => {
                            println!("ForwardConnResponse: {:?}", net_set);
                            let _ = self
                                .net_settings_send
                                // .send((self.network_settings, Some(net_set)));
                                .send(net_set);
                        }
                        NeighborResponse::ForwardConnectFailed => {
                            // TODO build querying mechanism
                            // TODO inform gnome's mechanism to ask another neighbor
                        }
                        NeighborResponse::SwarmSync(chill_phase, b_casts, m_casts) => {
                            if chill_phase > 0 {
                                println!("Into chill {}", chill_phase);
                                self.chill_out.0 = true;
                                self.chill_out.1 = chill_phase;
                            } else {
                                println!("no chill ");
                                self.chill_out.0 = false;
                                self.chill_out.1 = chill_phase;
                            }
                        }
                        NeighborResponse::Subscribed(is_bcast, cast_id, origin, source) => {
                            let source = source.unwrap();
                            let (send_d, recv_d) = channel();
                            let (send_m, recv_m) = channel();
                            let activated = if is_bcast {
                                self.activate_broadcast_at_neighbor(source, cast_id, send_m)
                            } else {
                                //     self.activate_multicast(cast_id, send);
                                false
                            };
                            if activated {
                                let m_cast = Multicast::new(
                                    origin,
                                    (source, recv_m),
                                    vec![],
                                    HashMap::new(),
                                    Some(send_d),
                                );
                                if is_bcast {
                                    self.swarm.insert_broadcast(cast_id, m_cast);
                                    let _res = self.sender.send(Response::Broadcast(
                                        self.swarm.id,
                                        cast_id,
                                        recv_d,
                                    ));
                                } else {
                                    self.swarm.insert_multicast(cast_id, m_cast);
                                    let _res = self.sender.send(Response::Multicast(
                                        self.swarm.id,
                                        cast_id,
                                        recv_d,
                                    ));
                                }
                            }
                        }
                        other => {
                            println!("Uncovered NeighborResponse: {:?}\nother", other);
                        }
                    },
                    _ => {
                        println!("Got response: {:?}\nunderscore", response);
                        let _ = self.sender.send(response);
                    }
                }
            }
            let (served, sanity_passed, new_proposal, drop_me) = neighbor.try_recv(
                self.next_state.last_accepted_message.clone(),
                // self.next_state.last_accepted_reconf,
            );
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
                }
                if !drop_me {
                    self.refreshed_neighbors.push(neighbor);
                } else {
                    println!("Dropping disconnected neighbor");
                    continue;
                }
            } else if !drop_me {
                if fast {
                    self.fast_neighbors.push(neighbor);
                } else {
                    self.slow_neighbors.push(neighbor);
                }
            } else {
                println!("Dropping neighbor");
            }
        }
        (
            self.fast_neighbors.is_empty() && self.slow_neighbors.is_empty() && looped
                || new_proposal_received,
            new_proposal_received,
        )
    }

    // We send a query to a single neighbor asking to provide us with a new neighbor.
    // We need to track what neighbors have been queried so that we do not query the
    // same neighbor over again, if all our neighbors have been asked we simply clean
    // the list of queried neighbors and start over.
    fn query_for_new_neighbors(&mut self) {
        let request = NeighborRequest::ForwardConnectRequest(self.network_settings);
        let mut request_sent = false;
        for neighbor in &mut self.fast_neighbors {
            if !self
                .neighbor_discovery
                .queried_neighbors
                .contains(&neighbor.id)
            {
                neighbor.request_data(request.clone());
                request_sent = true;
                self.neighbor_discovery.queried_neighbors.push(neighbor.id);
                break;
            }
        }
        if !request_sent {
            for neighbor in &mut self.slow_neighbors {
                if !self
                    .neighbor_discovery
                    .queried_neighbors
                    .contains(&neighbor.id)
                {
                    neighbor.request_data(request);
                    request_sent = true;
                    self.neighbor_discovery.queried_neighbors.push(neighbor.id);
                    break;
                }
            }
        }
        if !request_sent {
            self.neighbor_discovery.queried_neighbors = vec![];
            // let opt_neighbor = self.fast_neighbors.iter_mut().next();
            // if let Some(neighbor) = opt_neighbor {
            //     neighbor.request_data(request);
            //     self.neighbor_discovery.queried_neighbors.push(neighbor.id);
            // } else {
            //     let opt_neighbor = self.slow_neighbors.iter_mut().next();
            //     if let Some(neighbor) = opt_neighbor {
            //         neighbor.request_data(request);
            //         self.neighbor_discovery.queried_neighbors.push(neighbor.id);
            //     }
            // }
        }
    }
    fn get_neighbor_ids_and_senders(&self) -> HashMap<GnomeId, Sender<Message>> {
        let mut ids = HashMap::new();
        for neighbor in &self.fast_neighbors {
            ids.insert(neighbor.id, neighbor.sender.clone());
        }
        for neighbor in &self.slow_neighbors {
            ids.insert(neighbor.id, neighbor.sender.clone());
        }
        for neighbor in &self.refreshed_neighbors {
            ids.insert(neighbor.id, neighbor.sender.clone());
        }
        for neighbor in &self.new_neighbors {
            ids.insert(neighbor.id, neighbor.sender.clone());
        }
        ids
    }

    fn insert_originating_broadcast(
        &mut self,
        id: CastID,
        // b_cast: Multicast,
        recv_d: Receiver<Data>,
        send_n: Sender<Message>,
    ) {
        self.data_converters.insert((true, id), (recv_d, send_n));
    }

    fn activate_broadcast_at_neighbor(
        &mut self,
        id: GnomeId,
        cast_id: CastID,
        sender: Sender<Message>,
    ) -> bool {
        for neighbor in &mut self.fast_neighbors {
            if neighbor.id == id {
                neighbor.activate_broadcast(cast_id, sender);
                return true;
            }
        }
        for neighbor in &mut self.refreshed_neighbors {
            if neighbor.id == id {
                neighbor.activate_broadcast(cast_id, sender);
                return true;
            }
        }
        for neighbor in &mut self.slow_neighbors {
            if neighbor.id == id {
                neighbor.activate_broadcast(cast_id, sender);
                return true;
            }
        }
        for neighbor in &mut self.new_neighbors {
            if neighbor.id == id {
                neighbor.activate_broadcast(cast_id, sender);
                return true;
            }
        }
        false
    }
}
