use crate::gnome_to_manager::GnomeToManager;
use crate::internal::InternalMsg;
use crate::manager_to_gnome::ManagerToGnome;
use crate::message::BlockID;
use crate::message::Configuration;
use crate::message::Header;
use crate::message::Payload;
use crate::message::Signature;
use crate::multicast::CastType;
use crate::multicast::{CastMessage, Multicast};
use crate::neighbor::NeighborResponse;
use crate::neighbor::Neighborhood;
use crate::neighbor::SwarmSyncRequestParams;
use crate::next_state::ChangeConfig;
use crate::swarm::Swarm;
use crate::CastData;
use crate::CastID;
use crate::GnomeToApp;
use crate::KeyRegistry;
use crate::Message;
use crate::Neighbor;
use crate::NeighborRequest;
use crate::NextState;
use crate::SwarmName;
use crate::SwarmSyncResponse;
use crate::SwarmTime;
use crate::SyncData;
use crate::ToGnome;
use crate::WrappedMessage;
use crate::DEFAULT_NEIGHBORS_PER_GNOME;
use crate::DEFAULT_SWARM_DIAMETER;

use std::collections::HashMap;
use std::collections::VecDeque;
use std::fmt;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::ops::Deref;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;
use std::time::Instant;

#[derive(Clone, Copy, PartialOrd, PartialEq, Ord, Eq, Debug, Hash)]
pub struct GnomeId(pub u64);
impl GnomeId {
    pub fn bytes(&self) -> [u8; 8] {
        self.0.to_be_bytes()
    }
    pub fn any() -> Self {
        GnomeId(0)
    }
    pub fn from(bytes: [u8; 8]) -> Self {
        GnomeId(u64::from_be_bytes(bytes))
    }
    pub fn is_any(&self) -> bool {
        self.0 == 0
    }
}

// impl PartialEq for GnomeId {
//     fn eq(&self, other: &GnomeId) -> bool {
//         self.is_any() || other.is_any() || self.0 == other.0
//     }
// }
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
impl Nat {
    pub fn from(byte: u8) -> Self {
        match byte {
            1 => Self::None,
            2 => Self::FullCone,
            4 => Self::AddressRestrictedCone,
            8 => Self::PortRestrictedCone,
            16 => Self::SymmetricWithPortControl,
            32 => Self::Symmetric,
            _o => Self::Unknown,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortAllocationRule {
    Random = 0,
    FullCone = 1,
    AddressSensitive = 2,
    PortSensitive = 4,
}
impl PortAllocationRule {
    pub fn from(byte: u8) -> Self {
        match byte {
            1 => Self::FullCone,
            2 => Self::AddressSensitive,
            4 => Self::PortSensitive,
            _o => Self::Random,
        }
    }
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
}
#[derive(Clone)]
enum Proposal {
    Block(BlockID, SyncData),
    Config(Configuration),
}
impl Proposal {
    pub fn new(payload: Payload) -> Self {
        if payload.has_data() {
            let (id, data) = payload.id_and_data().unwrap();
            Proposal::Block(id, data)
        } else {
            let conf = payload.config().unwrap();
            Proposal::Config(conf)
        }
    }
    pub fn can_be_extended(&self) -> bool {
        match self {
            Self::Block(_bid, data) => data.len() <= 900,
            Self::Config(conf) => conf.len() <= 900,
        }
    }

    pub fn to_header_payload(
        self,
        sign: &fn(&str, SwarmTime, &mut Vec<u8>) -> Result<Vec<u8>, ()>,
        gnome_id: GnomeId,
        round_start: SwarmTime,
        extend: bool,
        priv_key: &str,
        pubkey_bytes: Vec<u8>,
    ) -> (Header, Payload) {
        // println!("to_header_payload pubkey bytes len: {}", pubkey_bytes.len());
        match self {
            Self::Block(b_id, data) => {
                let mut bytes = data.bytes();
                // TODO: we need to cover case when we are not in swarm's registry
                //       and bytes are longer than 900
                // TODO: we also should not send extended signature when registry
                //       arleady contains our key - this should be a bool argument
                // let extended = bytes.len() <= 900;

                let signature_b = sign(priv_key, round_start, &mut bytes).unwrap();
                let signature = if extend {
                    Signature::Extended(gnome_id, pubkey_bytes, signature_b)
                } else {
                    Signature::Regular(gnome_id, signature_b)
                };

                let data = SyncData::new(bytes).unwrap();
                (Header::Block(b_id), Payload::Block(b_id, signature, data))
            }
            Self::Config(config) => {
                let signature_b = sign(priv_key, round_start, &mut config.bytes()).unwrap();
                // println!("Signature len: {}", signature_b.len());
                let signature = if extend {
                    Signature::Extended(gnome_id, pubkey_bytes, signature_b)
                } else {
                    Signature::Regular(gnome_id, signature_b)
                };
                (
                    // TODO: we need to rework gid
                    Header::Reconfigure(config.as_ct(), config.as_gid(gnome_id)),
                    Payload::Reconfigure(signature, config),
                )
            }
        }
    }
}

pub struct Gnome {
    pub id: GnomeId,
    pub pub_key_bytes: Vec<u8>,
    priv_key_pem: String,
    pub neighborhood: Neighborhood,
    swarm: Swarm,
    swarm_time: SwarmTime,
    round_start: SwarmTime,
    swarm_diameter: SwarmTime,
    receiver: Receiver<ToGnome>,
    band_receiver: Receiver<u64>,
    sender: Sender<GnomeToApp>,
    mgr_sender: Sender<GnomeToManager>,
    mgr_receiver: Receiver<ManagerToGnome>,
    // TODO: make neighbors attrs into HashMap<GnomeId,Neighbor>
    fast_neighbors: Vec<Neighbor>,
    slow_neighbors: Vec<Neighbor>,
    new_neighbors: Vec<Neighbor>,
    refreshed_neighbors: Vec<Neighbor>,
    header: Header,
    payload: Payload,
    my_proposal: Option<Proposal>,
    proposals: VecDeque<Proposal>,
    next_state: NextState,
    timeout_duration: Duration,
    send_immediate: bool,
    ipv6_network_settings: NetworkSettings, //TODO: do we really need those here anymore?
    network_settings: NetworkSettings,      //TODO: do we really need those here anymore?
    net_settings_send: Sender<NetworkSettings>,
    pending_conn_requests: VecDeque<ConnRequest>,
    ongoing_requests: HashMap<u8, OngoingRequest>,
    neighbor_discovery: NeighborDiscovery,
    chill_out: (bool, Instant),
    chill_out_max: Duration,
    data_converters: HashMap<(CastType, CastID), (Receiver<CastData>, Sender<WrappedMessage>)>,
    sign: fn(&str, SwarmTime, &mut Vec<u8>) -> Result<Vec<u8>, ()>,
    send_internal: Sender<InternalMsg>,
    recv_internal: Receiver<InternalMsg>,
}

impl Gnome {
    pub fn new(
        id: GnomeId,
        pub_key_bytes: Vec<u8>,
        priv_key_pem: String,
        swarm: Swarm,
        sender: Sender<GnomeToApp>,
        receiver: Receiver<ToGnome>,
        mgr_sender: Sender<GnomeToManager>,
        mgr_receiver: Receiver<ManagerToGnome>,
        band_receiver: Receiver<u64>,
        network_settings: NetworkSettings,
        net_settings_send: Sender<NetworkSettings>,
        sign: fn(&str, SwarmTime, &mut Vec<u8>) -> Result<Vec<u8>, ()>,
    ) -> Self {
        // println!("DER size: {}", pub_key_bytes.len());
        let (send_internal, recv_internal) = channel();
        let (ipv6_network_settings, network_settings) = if network_settings.pub_ip.is_ipv4() {
            (
                NetworkSettings::new_not_natted(
                    IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0)),
                    0,
                ),
                network_settings,
            )
        } else {
            (
                network_settings,
                NetworkSettings::new_not_natted(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0),
            )
        };
        Gnome {
            id,
            pub_key_bytes,
            priv_key_pem,
            neighborhood: Neighborhood(0),
            swarm,
            swarm_time: SwarmTime(0),
            round_start: SwarmTime(0),
            swarm_diameter: DEFAULT_SWARM_DIAMETER,
            receiver,
            band_receiver,
            sender,
            mgr_sender,
            mgr_receiver,
            fast_neighbors: Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME),
            slow_neighbors: Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME),
            new_neighbors: vec![],
            refreshed_neighbors: Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME),
            header: Header::Sync,
            payload: Payload::KeepAlive(10240),
            my_proposal: None,
            proposals: VecDeque::new(),
            next_state: NextState::new(),
            timeout_duration: Duration::from_millis(500),
            send_immediate: false,
            ipv6_network_settings,
            network_settings,
            net_settings_send,
            pending_conn_requests: VecDeque::new(),
            ongoing_requests: HashMap::new(),
            neighbor_discovery: NeighborDiscovery::default(),
            chill_out: (false, Instant::now()),
            chill_out_max: Duration::from_millis(14500),
            data_converters: HashMap::new(),
            sign,
            send_internal,
            recv_internal,
        }
    }

    pub fn new_with_neighbors(
        id: GnomeId,
        pub_key_bytes: Vec<u8>,
        priv_key_pem: String,
        swarm: Swarm,
        sender: Sender<GnomeToApp>,
        receiver: Receiver<ToGnome>,
        mgr_sender: Sender<GnomeToManager>,
        mgr_receiver: Receiver<ManagerToGnome>,
        band_receiver: Receiver<u64>,
        neighbors: Vec<Neighbor>,
        network_settings: NetworkSettings,
        net_settings_send: Sender<NetworkSettings>,
        sign: fn(&str, SwarmTime, &mut Vec<u8>) -> Result<Vec<u8>, ()>,
    ) -> Self {
        let mut gnome = Gnome::new(
            id,
            pub_key_bytes,
            priv_key_pem,
            swarm,
            sender,
            receiver,
            mgr_sender,
            mgr_receiver,
            band_receiver,
            network_settings,
            net_settings_send,
            sign,
        );
        gnome.fast_neighbors = neighbors;
        gnome
    }

    //TODO: we should probably add identifiers to internal messages
    //      and have them progress on a step by step basis somehow
    fn serve_internal(&mut self) -> bool {
        let mut any_data_processed = false;
        while let Ok(internal) = self.recv_internal.try_recv() {
            any_data_processed = true;
            match internal {
                InternalMsg::SubscribeCast(is_bcast, id, origin) => {
                    // TODO: we have to support also multicast
                    if let Some(source) = self.neighbor_with_enough_bandwith(1) {
                        eprintln!("Subscribing to BCast: {}", id.0);
                        let (send_n, recv_n) = channel();
                        let (send_d, recv_d) = channel();
                        // TODO: bandwith threshold should not be a fixed value
                        if self.send_neighbor_request(
                            source,
                            NeighborRequest::SubscribeRequest(is_bcast, id),
                        ) && self.activate_broadcast_at_neighbor(source, id, send_n)
                        {
                            let b_cast = Multicast::new(
                                origin,
                                (source, recv_n),
                                vec![],
                                HashMap::new(),
                                Some(send_d),
                            );
                            self.swarm.insert_broadcast(id, b_cast);
                            let _res =
                                self.sender
                                    .send(GnomeToApp::Broadcast(self.swarm.id, id, recv_d));
                        }
                    } else {
                        eprintln!("Unable to subscribe a broadcast");
                    }
                }
                InternalMsg::UnsubscribeCast(is_bcast, c_id) => {
                    if let Some((n_id, subs)) =
                        self.swarm.unsubscribe_cast(self.id, is_bcast, &c_id)
                    {
                        self.send_neighbor_request(
                            n_id,
                            NeighborRequest::UnsubscribeRequest(is_bcast, c_id),
                        );
                        for sub in subs {
                            eprintln!("Sending SDrain to: {}", sub);
                            self.send_neighbor_request(
                                sub,
                                NeighborRequest::SourceDrained(is_bcast, c_id),
                            );
                        }
                    }
                }
                InternalMsg::FindNewCastSource(is_bcast, cast_id, old_source) => {
                    eprintln!("Looking for new casting source");
                    let alt_sources = self.swarm.get_alt_sources(is_bcast, &cast_id, old_source);
                    //TODO: pick alternative source
                    if !alt_sources.is_empty() {
                        if let Some(new_source) = self.select_best_alternative(alt_sources) {
                            let _ = self
                                .send_internal
                                .send(InternalMsg::SubscribeCast(is_bcast, cast_id, new_source));
                        }
                    } else {
                        eprintln!("Unable to find alternative source.")
                    }
                }
                InternalMsg::RequestOut(gnome_id, request) => {
                    let neighbor_id = if gnome_id.is_any() {
                        self.neighbor_with_highest_bandwith()
                    } else {
                        gnome_id
                    };
                    if !neighbor_id.is_any() {
                        self.send_neighbor_request(neighbor_id, request);
                    } else {
                        eprintln!("Unable to find a Neighbor to send out CustomRequest");
                    }
                }
                InternalMsg::ResponseOut(gnome_id, response) => {
                    let neighbor_id = if gnome_id.is_any() {
                        self.neighbor_with_highest_bandwith()
                    } else {
                        gnome_id
                    };
                    if !neighbor_id.is_any() {
                        self.send_neighbor_response(neighbor_id, response);
                    } else {
                        eprintln!("Unable to find a Neighbor to send out CustomRequest");
                    }
                }
            }
        }
        any_data_processed
    }

    fn serve_user_data(&self) -> bool {
        let mut any_data_processed = false;
        for ((c_type, c_id), (recv_d, send_m)) in &self.data_converters {
            while let Ok(data) = recv_d.try_recv() {
                // println!("user data: {:?}",data);
                any_data_processed = true;
                let message = match c_type {
                    CastType::Broadcast => {
                        WrappedMessage::Cast(CastMessage::new_broadcast(*c_id, data))
                    }
                    CastType::Multicast => {
                        WrappedMessage::Cast(CastMessage::new_multicast(*c_id, data))
                    }
                    CastType::Unicast => {
                        WrappedMessage::Cast(CastMessage::new_unicast(*c_id, data))
                    }
                };
                let _r = send_m.send(message);
            }
        }
        any_data_processed
    }
    fn serve_user_requests(&mut self) -> (bool, bool) {
        let mut new_user_proposal = false;
        let mut exit_app = false;
        if let Ok(request) = self.receiver.try_recv() {
            match request {
                ToGnome::Disconnect => exit_app = true,
                // ToGnome::UpdateAppRootHash(new_hash) => {
                //     // println!("Gnome updating root hash to {}", new_hash);
                //     *app_root_hash = new_hash;
                // }
                ToGnome::Status => {
                    eprintln!(
                        "Status: {} {} {}\t\t neighbors: {}",
                        self.swarm_time,
                        self.header,
                        self.neighborhood,
                        self.fast_neighbors.len()
                    );
                }
                ToGnome::AddData(data) => {
                    let b_id = data.get_block_id();
                    self.proposals.push_front(Proposal::Block(b_id, data));
                    new_user_proposal = true;
                    // println!("vvv USER vvv REQ {}", data);
                }
                ToGnome::Reconfigure(value, s_data) => {
                    eprintln!("Gnome received reconfigure request");
                    self.proposals
                        .push_front(Proposal::Config(Configuration::UserDefined(value, s_data)));
                    new_user_proposal = true;
                }
                ToGnome::SetFounder(f_id) => {
                    self.swarm.set_founder(f_id);
                    let _res = self.mgr_sender.send(GnomeToManager::FounderDetermined(
                        self.swarm.id,
                        self.swarm.name.founder,
                    ));
                    eprintln!("Set founder to {}, {:?}", f_id, _res);
                }
                ToGnome::AddNeighbor(neighbor) => {
                    eprintln!(
                        "{} ADD\tadd a new neighbor to {} ({})",
                        neighbor.id, self.swarm.name, self.swarm.id
                    );
                    self.add_neighbor(neighbor);
                }
                ToGnome::SwarmNeighbors(swarm_name) => {
                    eprintln!("Neighbors for swarm {} request", swarm_name);
                    // TODO: we need to go trough all of our Neighbors
                    // and ask them to instantiate a new Neighbor
                    // for given swarm_name
                    // Once collected we send back a response
                    self.request_neighbors_for_swarm(swarm_name);
                }
                ToGnome::DropNeighbor(n_id) => {
                    eprintln!("{} DROP\ta neighbor on user request", n_id);
                    self.drop_neighbor(n_id);
                }
                ToGnome::ListNeighbors => {
                    // eprintln!("List neighbors request");
                    let mut n_ids = vec![];
                    for n in &self.fast_neighbors {
                        n_ids.push(n.id);
                    }
                    for n in &self.slow_neighbors {
                        n_ids.push(n.id);
                    }
                    for n in &self.refreshed_neighbors {
                        n_ids.push(n.id);
                    }
                    let _ = self
                        .sender
                        .send(GnomeToApp::Neighbors(self.swarm.id, n_ids));
                }
                ToGnome::StartUnicast(gnome_id) => {
                    // println!("Received StartUnicast {:?}", gnome_id);
                    let mut request_sent = false;
                    let mut avail_ids: [CastID; 256] = [CastID(0); 256];
                    for (added_ids, cast_id) in
                        self.swarm.avail_unicast_ids().into_iter().enumerate()
                    {
                        avail_ids[added_ids] = cast_id;
                    }
                    let request =
                        NeighborRequest::UnicastRequest(self.swarm.id, Box::new(avail_ids));
                    for neighbor in &mut self.fast_neighbors {
                        if neighbor.id == gnome_id {
                            eprintln!("Sending UnicastRequest to neighbor");
                            neighbor.request_data(request.clone());
                            request_sent = true;
                            break;
                        }
                    }
                    if !request_sent {
                        for neighbor in &mut self.slow_neighbors {
                            if neighbor.id == gnome_id {
                                request_sent = true;
                                neighbor.request_data(request.clone());
                                break;
                            }
                        }
                    }
                    if !request_sent {
                        eprintln!("Unable to find gnome with id {}", gnome_id);
                    }
                }
                ToGnome::StartBroadcast => {
                    eprintln!("Received StartBroadcast user request");
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
                ToGnome::EndBroadcast(c_id) => {
                    eprintln!("Received EndBroadcast user request");
                    self.proposals
                        .push_front(Proposal::Config(Configuration::EndBroadcast(self.id, c_id)));
                    new_user_proposal = true;
                }
                ToGnome::UnsubscribeBroadcast(c_id) => {
                    eprintln!("Received UnsubscribeBroadcast user request");
                    let _ = self
                        .send_internal
                        .send(InternalMsg::UnsubscribeCast(true, c_id));
                }
                ToGnome::StartMulticast(_) => {
                    todo!()
                }
                ToGnome::AskData(gnome_id, request) => {
                    self.send_internal
                        .send(InternalMsg::RequestOut(gnome_id, request))
                        .unwrap();
                }
                ToGnome::SendData(gnome_id, response) => {
                    self.send_internal
                        .send(InternalMsg::ResponseOut(gnome_id, response))
                        .unwrap();
                }
                ToGnome::NetworkSettingsUpdate(notify_neighbor, ip_addr, port, nat, port_rule) => {
                    // TODO: now we can receive an IPv6 address in addition to IPv4
                    // we should support both versions in order to maximize number of
                    // potential communication channels
                    if !notify_neighbor {
                        // eprintln!("not notify neighbor");
                        //TODO: if I am founder in this swarm, then I should notify manager
                        // if self.id == self.swarm.name.founder {
                        let _ = self
                            .mgr_sender
                            .send(GnomeToManager::PublicAddress(ip_addr, port, nat, port_rule));
                        // self.sender.send(GnomeToApp::SwarmReady(()))
                        // }
                        if ip_addr.is_ipv6() {
                            self.ipv6_network_settings.pub_ip = ip_addr;
                            self.ipv6_network_settings.set_port(port);
                            self.ipv6_network_settings.nat_type = nat;
                            // self.ipv6_network_settings
                        } else {
                            self.network_settings.pub_ip = ip_addr;
                            self.network_settings.set_port(port);
                            self.network_settings.nat_type = nat;
                            // self.network_settings
                        };
                    } else {
                        let settings_to_send = NetworkSettings {
                            pub_ip: ip_addr,
                            pub_port: port,
                            nat_type: nat,
                            port_allocation: (PortAllocationRule::Random, 0),
                        };
                        eprintln!("Trying to notify neighbor");
                        if let Some(ConnRequest {
                            conn_id,
                            neighbor_id,
                        }) = self.pending_conn_requests.pop_front()
                        {
                            let mut neighbor_informed = false;
                            for neighbor in &mut self.fast_neighbors {
                                if neighbor.id == neighbor_id {
                                    neighbor.add_requested_data(NeighborResponse::ConnectResponse(
                                        conn_id,
                                        settings_to_send,
                                    ));
                                    neighbor_informed = true;
                                    break;
                                }
                            }
                            if !neighbor_informed {
                                for neighbor in &mut self.refreshed_neighbors {
                                    if neighbor.id == neighbor_id {
                                        neighbor.add_requested_data(
                                            NeighborResponse::ConnectResponse(
                                                conn_id,
                                                settings_to_send,
                                            ),
                                        );
                                        neighbor_informed = true;
                                        break;
                                    }
                                }
                            }
                            if !neighbor_informed {
                                for neighbor in &mut self.slow_neighbors {
                                    if neighbor.id == neighbor_id {
                                        neighbor.add_requested_data(
                                            NeighborResponse::ConnectResponse(
                                                conn_id,
                                                settings_to_send,
                                            ),
                                        );
                                        neighbor_informed = true;
                                        break;
                                    }
                                }
                            }
                            if neighbor_informed {
                                eprintln!("Sent back ConnResponse");
                            } else {
                                eprintln!("Failed to send response");
                            }
                        }
                    }
                }
            }
        }
        (exit_app, new_user_proposal)
    }
    fn serve_manager_requests(&mut self) -> (bool, bool) {
        let mut any_data_processed = false;
        let mut bye = false;
        while let Ok(request) = self.mgr_receiver.try_recv() {
            any_data_processed = true;
            match request {
                ManagerToGnome::ProvideNeighborsToSwarm(swarm_name, neighbor_id) => {
                    //TODO: here mgr decides locally to join an existing remote swarm
                    //      and asks existing gnome to clone a neighbor for that swarm
                    // <---- SwarmJoinedInfo from remote Neighbor
                    // ----> CreateNeighbor to remote (WE ARE HERE)
                    //       (----> remote internal CreateNeighbor for new channel creation,
                    //              remote instantiates new channel and clones itself)
                    //
                    eprintln!(
                        "{} received neighbor request for {}. Am I founder?: {}",
                        self.swarm.name,
                        swarm_name,
                        swarm_name.founder.0 == self.id.0
                    );
                    let (s1, r1) = channel();
                    let (s2, r2) = channel();
                    let (s3, r3) = channel();
                    if let Some(shared_sender) = self.get_shared_sender(neighbor_id) {
                        let new_neighbor = Neighbor::from_id_channel_time(
                            neighbor_id,
                            r1,
                            r2,
                            s3,
                            shared_sender,
                            SwarmTime(0),
                            self.swarm_diameter, //TODO
                            vec![],
                        );
                        new_neighbor.clone_to_swarm(swarm_name.clone(), s1, s2, r3);
                        self.send_noop_from_a_neighbor();
                        eprintln!(
                            "{} request add {} to {}",
                            self.swarm.name, new_neighbor.id, swarm_name
                        );
                        let _resp = self.mgr_sender.send(GnomeToManager::AddNeighborToSwarm(
                            self.swarm.id,
                            swarm_name,
                            new_neighbor,
                        ));
                    }
                }
                ManagerToGnome::AddNeighbor(mut neighbor) => {
                    eprintln!(
                        "Gnome adding {} neighbor to {}",
                        neighbor.id, self.swarm.name
                    );
                    eprintln!("Send CN {} foor {}", neighbor.id, self.swarm.name);
                    neighbor.send_out_cast(CastMessage::new_request(
                        NeighborRequest::CreateNeighbor(self.id, self.swarm.name.clone()),
                    ));
                    self.send_noop_from_a_neighbor();
                    self.add_neighbor(neighbor);
                    self.notify_mgr_about_neighbors();
                }
                ManagerToGnome::SwarmJoined(swarm_name, n_ids) => {
                    self.notify_neighbors_about_new_swarm(swarm_name, n_ids);
                }
                ManagerToGnome::Status => {
                    //TODO
                }
                ManagerToGnome::Disconnect => {
                    self.bye_all();
                    let _ = self
                        .mgr_sender
                        .send(GnomeToManager::Disconnected(self.swarm.id));
                    bye = true;
                    return (any_data_processed, bye);
                }
            }
        }
        (any_data_processed, bye)
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
    fn add_ongoing_request(&mut self, origin: GnomeId, network_settings: NetworkSettings) {
        let neighbor_count = self.fast_neighbors.len() + self.slow_neighbors.len();
        if neighbor_count < 2 {
            eprintln!("Not enough neighbors: {}", neighbor_count);
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
        let mut id: u8 = 0;
        eprintln!("Have enough neighbors for ongoing requests");
        while self.ongoing_requests.contains_key(&id) {
            id += 1;
        }
        eprintln!("Ongoing count: {}", id);
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

    fn notify_neighbors_about_new_swarm(&mut self, swarm_name: SwarmName, n_ids: Vec<GnomeId>) {
        let req = NeighborRequest::SwarmJoinedInfo(swarm_name);
        for neighbor in &mut self.fast_neighbors {
            if n_ids.contains(&neighbor.id) {
                neighbor.request_data(req.clone());
            }
        }
        for neighbor in &mut self.slow_neighbors {
            if n_ids.contains(&neighbor.id) {
                neighbor.request_data(req.clone());
            }
        }
        for neighbor in &mut self.refreshed_neighbors {
            if n_ids.contains(&neighbor.id) {
                neighbor.request_data(req.clone());
            }
        }
    }
    fn request_neighbors_for_swarm(&mut self, swarm_name: SwarmName) {
        eprintln!("Send CN 3");
        let req = NeighborRequest::CreateNeighbor(self.id, swarm_name.clone());
        for neighbor in &mut self.fast_neighbors {
            neighbor.request_data(req.clone());
        }
        for neighbor in &mut self.slow_neighbors {
            neighbor.request_data(req.clone());
        }
        for neighbor in &mut self.refreshed_neighbors {
            neighbor.request_data(req.clone());
        }
    }

    fn bye_all(&mut self) {
        eprintln!("Sending bye to all {:?} Neighbors…", self.swarm.id);
        let bye = Message::bye();
        for neighbor in &mut self.fast_neighbors {
            neighbor.send_out(bye.clone());
        }
        for neighbor in &mut self.slow_neighbors {
            neighbor.send_out(bye.clone());
        }
        for neighbor in &mut self.refreshed_neighbors {
            neighbor.send_out(bye.clone());
        }
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
        eprintln!("Trying to notify neighbor");
        //   and also we return a NeighborResponse with our updated
        //   network settings

        self.pending_conn_requests.push_back(ConnRequest {
            conn_id: id,
            neighbor_id: reply_gnome,
        });
        // We send None to notify networking we want it to send us
        // back refreshed NetworkSettings
        let _ = self.net_settings_send.send(network_settings);
        None
    }

    // TODO: find where to apply this function
    fn add_ongoing_reply(&mut self, id: u8, network_settings: NetworkSettings) {
        let opor = self.ongoing_requests.remove(&id);
        if opor.is_some() {
            let mut o_req = opor.unwrap();
            o_req.response = Some(network_settings);
            self.ongoing_requests.insert(id, o_req);
        } else {
            eprintln!("No ongoing request with id: {}", id);
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
                eprintln!("Unable to find more neighbors for {}", id);
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
    fn serve_ongoing_requests(&mut self) -> bool {
        let mut any_data_processed = false;
        let mut keys_to_remove: Vec<u8> = vec![];
        for (k, v) in &mut self.ongoing_requests {
            if v.response.is_some() {
                any_data_processed = true;
                eprintln!("Sending response: {:?}", v.response);
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
                        eprintln!("Unable to find more neighbors for {}", k);
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
        any_data_processed
    }

    fn serve_neighbors_casts(&mut self) -> bool {
        let mut any_data_processed = false;
        for neighbor in &mut self.fast_neighbors {
            any_data_processed |= neighbor.try_recv_cast();
        }
        for neighbor in &mut self.slow_neighbors {
            any_data_processed |= neighbor.try_recv_cast();
        }
        for neighbor in &mut self.refreshed_neighbors {
            any_data_processed |= neighbor.try_recv_cast();
        }
        any_data_processed
    }
    fn serve_sync_requests(&mut self) -> bool {
        let mut any_data_processed = false;
        // TODO: in order to function we need to always have
        //       actual value of app_sync_hash at hand
        //       this should be provided by Manager and stored by Gnome or better Swarm
        if self.new_neighbors.is_empty() {
            return any_data_processed;
        }
        let message = self.prepare_message();
        let mut processed_neighbors = vec![];
        while let Some(mut neighbor) = self.new_neighbors.pop() {
            // eprintln!("Serving Sync Swarm request");
            neighbor.try_recv_cast();
            // eprintln!("SSReq 1");
            if let Some(NeighborRequest::SwarmSyncRequest(SwarmSyncRequestParams {
                sync_key_reg,
                sync_capability,
                sync_policy,
                sync_broadcast,
                sync_multicast,
                // app_root_hash: remote_app_root_hash,
            })) = neighbor.requests.pop_back()
            {
                // eprintln!(
                //     "Remote hash: {}, my hash: {}",
                //     remote_app_root_hash, app_sync_hash
                // );
                any_data_processed = true;
                neighbor.swarm_time = message.swarm_time;
                self.send_sync_responses(
                    // app_sync_hash,
                    sync_key_reg,
                    sync_capability,
                    sync_policy,
                    sync_broadcast,
                    sync_multicast,
                    &mut neighbor,
                );
                // self.fast_neighbors.push(neighbor);
                // } else {
                //     processed_neighbors.push(neighbor);
            }
            processed_neighbors.push(neighbor);
        }
        self.new_neighbors = processed_neighbors;
        any_data_processed
    }

    fn send_sync_responses(
        &self,
        // app_sync_hash: u64,
        sync_key_reg: bool,
        sync_capability: bool,
        sync_policy: bool,
        sync_broadcast: bool,
        sync_multicast: bool,
        neighbor: &mut Neighbor,
    ) {
        // println!("Serving some SyncRequest!");
        let b_count = self.swarm.broadcasts_count();
        let m_count = self.swarm.multicasts_count();

        let chill_phase = if self.chill_out.0 {
            // eprintln!(
            //     "Max: {:?}, elapsed: {:?}",
            //     self.chill_out_max,
            //     self.chill_out.1.elapsed()
            // );
            (self.chill_out_max.checked_sub(self.chill_out.1.elapsed()))
                .unwrap_or(Duration::ZERO)
                .as_millis() as u16
        } else {
            0
            // self.chill_out_max.as_millis() as u16
        };
        let (more_keys, first_key_batch, mut remaining_batches) = if sync_key_reg {
            let mut chunks = self.swarm.key_reg.chunks();
            let more_keys = chunks.len() > 1;
            let first_chunk = if !chunks.is_empty() {
                chunks.pop().unwrap()
            } else {
                vec![]
            };
            eprintln!(
                "SyncResponse  with key reg pairs len: {}",
                // app_sync_hash,
                first_chunk.len()
            );
            (more_keys, first_chunk, chunks)
        } else {
            (false, vec![], vec![])
        };

        let sync_response = SwarmSyncResponse {
            chill_phase,
            founder: self.swarm.name.founder,
            swarm_time: self.swarm_time,
            round_start: self.round_start,
            swarm_type: self.swarm.swarm_type,
            // app_root_hash: app_sync_hash,
            key_reg_size: self.swarm.key_reg.byte(),
            capability_size: self.swarm.capability_reg.len() as u8,
            policy_size: self.swarm.policy_reg.len() as u8,
            broadcast_size: b_count,
            multicast_size: m_count,
            more_key_reg_messages: more_keys,
            key_reg_pairs: first_key_batch,
        };
        let response = NeighborResponse::SwarmSync(sync_response);
        neighbor.start_new_round(self.swarm_time);
        neighbor.send_out_cast(CastMessage::new_response(response));
        let mut i: u8 = 1;
        let total_batches = remaining_batches.len() as u8;
        while let Some(batch) = remaining_batches.pop() {
            let response = NeighborResponse::KeyRegistrySync(i, total_batches, batch);
            neighbor.send_out_cast(CastMessage::new_response(response));
            i += 1;
        }
        if sync_capability {
            let mut chunks = self.swarm.capabilities_chunks();
            let total_chunks = chunks.len();
            if total_chunks > 0 {
                for i in 1..total_chunks + 1 {
                    let response = NeighborResponse::CapabilitySync(
                        i as u8,
                        // TODO: we have to limit maximum size of Capability registry!
                        // max 128 gnomes per Capability
                        total_chunks as u8,
                        chunks.pop().unwrap(),
                    );
                    neighbor.send_out_cast(CastMessage::new_response(response));
                }
            }
        }
        if sync_policy {
            let mut chunks = self.swarm.policy_chunks();
            // println!("Policy chunks: {:?}", chunks);
            // println!("Policy : {:?}", self.swarm.policy_reg);
            let total_chunks = chunks.len();
            if total_chunks > 0 {
                for i in 1..total_chunks + 1 {
                    let response = NeighborResponse::PolicySync(
                        i as u8,
                        total_chunks as u8,
                        chunks.pop().unwrap(),
                    );
                    neighbor.send_out_cast(CastMessage::new_response(response));
                }
            }
        }
        if sync_broadcast {
            let b_casts = if b_count == 0 {
                vec![]
            } else {
                self.swarm.broadcast_ids()
            };
            let response = NeighborResponse::BroadcastSync(1, 1, b_casts);
            neighbor.send_out_cast(CastMessage::new_response(response));
        }
        if sync_multicast {
            let m_casts = if m_count == 0 {
                vec![]
            } else {
                self.swarm.multicast_ids()
            };
            let response = NeighborResponse::MulticastSync(1, 1, m_casts);
            neighbor.send_out_cast(CastMessage::new_response(response));
        }
    }

    fn serve_neighbors_requests(
        &mut self,
        // app_sync_hash: u64,
        refreshed: bool,
        slow: bool,
    ) -> bool {
        let mut any_data_processed = false;
        let mut neighbors = if slow {
            std::mem::take(&mut self.slow_neighbors)
        } else if refreshed {
            std::mem::take(&mut self.refreshed_neighbors)
        } else {
            std::mem::take(&mut self.fast_neighbors)
        };
        let mut pending_ongoing_requests = vec![];
        for neighbor in &mut neighbors {
            if let Some(request) = neighbor.requests.pop_back() {
                any_data_processed = true;
                match request {
                    NeighborRequest::UnicastRequest(_swarm_id, cast_ids) => {
                        let (send_d, recv_d) = channel();
                        for cast_id in cast_ids.deref() {
                            if self.swarm.is_unicast_id_available(*cast_id) {
                                self.swarm.insert_unicast(*cast_id);
                                self.insert_originating_unicast(
                                    *cast_id,
                                    recv_d,
                                    neighbor.sender.clone(),
                                );
                                neighbor.add_requested_data(NeighborResponse::Unicast(
                                    self.swarm.id,
                                    *cast_id,
                                ));
                                let _res = self.sender.send(GnomeToApp::UnicastOrigin(
                                    self.swarm.id,
                                    *cast_id,
                                    send_d,
                                ));
                                break;
                            }
                        }
                    }
                    NeighborRequest::ForwardConnectRequest(network_settings) => {
                        // println!("ForwardConnReq");
                        pending_ongoing_requests.push((neighbor.id, network_settings));
                    }
                    NeighborRequest::ConnectRequest(id, gnome_id, network_settings) => {
                        eprintln!("ConnReq");
                        if let Some(response) =
                            self.serve_connect_request(id, neighbor.id, gnome_id, network_settings)
                        {
                            neighbor.add_requested_data(response);
                        }
                    }
                    NeighborRequest::SwarmSyncRequest(SwarmSyncRequestParams {
                        sync_key_reg,
                        sync_capability,
                        sync_policy,
                        sync_broadcast,
                        sync_multicast,
                        // app_root_hash: _,
                    }) => {
                        // eprintln!("SSReq 2");
                        self.send_sync_responses(
                            // app_sync_hash,
                            sync_key_reg,
                            sync_capability,
                            sync_policy,
                            sync_broadcast,
                            sync_multicast,
                            neighbor,
                        );
                        neighbor.swarm_time = self.swarm_time;
                        neighbor.start_new_round(self.swarm_time);
                    }
                    NeighborRequest::SwarmJoinedInfo(swarm_name) => {
                        eprintln!("SwarmJoinedInfo {}", swarm_name);
                        let _ = self.mgr_sender.send(GnomeToManager::NeighboringSwarm(
                            self.swarm.id,
                            neighbor.id,
                            swarm_name,
                        ));
                    }
                    NeighborRequest::SubscribeRequest(is_bcast, cast_id) => {
                        eprintln!("SubscribeRequest {}", cast_id.0);
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
                            // We can not subscribe so we request our neighbor to
                            // keep searching, excluding us
                            neighbor
                                .request_data(NeighborRequest::SourceDrained(is_bcast, cast_id));
                        }
                    }
                    NeighborRequest::UnsubscribeRequest(is_bcast, cast_id) => {
                        eprintln!("UnsubscribeRequest {}", cast_id.0);
                        self.swarm
                            .remove_subscriber(is_bcast, &cast_id, neighbor.id);
                    }
                    NeighborRequest::SourceDrained(is_bcast, cast_id) => {
                        eprintln!("SourceDrained {}", cast_id.0);
                        let _ = self.send_internal.send(InternalMsg::FindNewCastSource(
                            is_bcast,
                            cast_id,
                            neighbor.id,
                        ));
                    }
                    //Gnome received a request from a neighbor
                    NeighborRequest::CreateNeighbor(gnome_id, swarm_name) => {
                        eprintln!(
                            "Gnome (from {}) Received CreateNeighbor {} for {}",
                            self.swarm.name, gnome_id, swarm_name
                        );
                        // Here we have received a request on existing channel
                        // this request is from a remote neighbor that wants
                        // to join another swarm
                        // TODO: we need to check if we are actually
                        // members of given swarm,
                        // Only when that is the case we create a new communication
                        // logic with remote
                        //
                        // In order to do so, we need to rework manager logic:
                        // Manager should be run as a service with internal loop
                        // Gnome should be able to communicate with Manager in both
                        // directions
                        // Given above we ask Manager to add a given Neighbor to
                        // particular swarm
                        // Manager should respond with either a Success or Failure

                        let (s1, r1) = channel();
                        let (s2, r2) = channel();
                        let (s3, r3) = channel();
                        eprintln!("Send CN 4 +NoOp");
                        let _ = s2.send(CastMessage::new_request(NeighborRequest::CreateNeighbor(
                            self.id,
                            swarm_name.clone(),
                        )));
                        //TODO clone_to_swarm only if Manager confirmed addition
                        let new_neighbor = Neighbor::from_id_channel_time(
                            gnome_id,
                            r1,
                            r2,
                            s3,
                            neighbor.get_shared_sender(),
                            SwarmTime(0),
                            self.swarm_diameter, //TODO
                            neighbor.member_of_swarms.clone(),
                        );
                        new_neighbor.clone_to_swarm(swarm_name.clone(), s1, s2, r3);
                        neighbor.send_no_op();
                        let _ = self.mgr_sender.send(GnomeToManager::AddNeighborToSwarm(
                            self.swarm.id,
                            swarm_name,
                            new_neighbor,
                        ));
                    }
                    NeighborRequest::Custom(m_type, data) => {
                        let _ =
                            self.sender
                                .send(GnomeToApp::Custom(true, m_type, neighbor.id, data));
                    }
                }
            }
        }
        let _ = if slow {
            std::mem::replace(&mut self.slow_neighbors, neighbors)
        } else if refreshed {
            std::mem::replace(&mut self.refreshed_neighbors, neighbors)
        } else {
            std::mem::replace(&mut self.fast_neighbors, neighbors)
        };

        for (id, net_set) in pending_ongoing_requests {
            self.add_ongoing_request(id, net_set);
        }
        any_data_processed
    }

    pub fn do_your_job(mut self) {
        eprintln!(
            "Waiting for user/network to provide some Neighbors for {}...",
            self.swarm.name
        );
        while self.fast_neighbors.is_empty() && self.slow_neighbors.is_empty() {
            // println!("in while");
            let _ = self.serve_user_requests();
            self.serve_manager_requests();
        }
        eprintln!("{} have neighbors!", self.swarm.name);
        self.notify_mgr_about_neighbors();
        let mut available_bandwith = if let Ok(band) = self.band_receiver.try_recv() {
            band
        } else {
            1024
        };
        available_bandwith = 1024;
        eprintln!("Avail bandwith: {}", available_bandwith);
        self.presync_with_swarm(available_bandwith);
        let mut timer = Instant::now();
        self.timeout_duration = Duration::from_secs(16);

        // A gnome's gotta sleep
        let min_sleep_nsec: u64 = 1 << 7; //128nsec min
        let max_sleep_nsec: u64 = 1 << 26; //~64msec max
        let mut sleep_nsec: u64 = 1 << 25;
        // set was_loop_iteration_busy  to true if:
        // - we received a Sync message
        // - we received a Cast message (incl. Neighbor Req/Res)
        // - we received a ManagerRequest
        // - user has sent us a request
        // - we sent a Cast message
        let mut was_loop_iteration_busy;
        let mut loops_with_no_reply = 0;
        loop {
            was_loop_iteration_busy = false;
            let sleep_time = Duration::from_nanos(sleep_nsec);
            // println!("Sleeping for: {:?}", sleep_time);
            std::thread::sleep(sleep_time);
            // println!("Sleep is over");
            let (mut break_the_loop, new_user_proposal) = self.serve_user_requests();
            was_loop_iteration_busy |= self.serve_internal();
            was_loop_iteration_busy |= self.serve_user_data();
            let (mgr_busy, bye) = self.serve_manager_requests();
            break_the_loop |= bye;
            was_loop_iteration_busy |= mgr_busy;
            if self.chill_out.0 {
                was_loop_iteration_busy |= self.serve_sync_requests();
            }
            if !self.refreshed_neighbors.is_empty() {
                was_loop_iteration_busy |= self.serve_neighbors_requests(true, false);
            }
            if !self.fast_neighbors.is_empty() {
                was_loop_iteration_busy |= self.serve_neighbors_requests(false, false);
            }
            if !self.slow_neighbors.is_empty() {
                was_loop_iteration_busy |= self.serve_neighbors_requests(false, true);
            }
            was_loop_iteration_busy |= self.serve_ongoing_requests();
            was_loop_iteration_busy |= self.serve_neighbors_casts();
            was_loop_iteration_busy |= self.swarm.serve_casts();
            // let refr_new_proposal = self.try_recv_refreshed();
            // print!(
            //     "F:{}s:{},r:{},n:{}",
            //     self.fast_neighbors.len(),
            //     self.slow_neighbors.len(),
            //     self.refreshed_neighbors.len(),
            //     self.new_neighbors.len()
            // );
            let (
                _have_responsive_neighbors,
                slow_advance_to_next_turn,
                slow_new_proposal,
                slow_any_data_processed,
            ) = self.try_recv(false);
            was_loop_iteration_busy |= slow_any_data_processed;
            let (
                have_responsive_neighbors,
                fast_advance_to_next_turn,
                fast_new_proposal,
                fast_any_data_processed,
            ) = self.try_recv(true);
            was_loop_iteration_busy |= fast_any_data_processed;

            // We have to send NoOp every 128msec in order to
            // trigger token admission on socket side
            // That is why we can not sleep for longer than 128msec
            if let Ok(band) = self.band_receiver.try_recv() {
                if band == 0 {
                    // print!("R");
                    self.send_noop_from_a_neighbor();
                } else {
                    available_bandwith = band;
                }
                // println!("Avail bandwith: {}", available_bandwith);
                //TODO make use of available_bandwith during multicasting setup
            }
            let advance_to_next_turn = fast_advance_to_next_turn || slow_advance_to_next_turn;
            let new_proposal = new_user_proposal || fast_new_proposal || slow_new_proposal;

            // TODO: when round ends drop slow neighbors with a bye message
            // Those neighbors will need to start over again
            // if !new_proposal && !fast_advance_to_next_turn && !self.slow_neighbors.is_empty() {
            //     eprint!("GSN ");
            //     std::thread::sleep(Duration::from_nanos(sleep_nsec >> 1));
            //     let (
            //         _have_responsive_neighbors,
            //         _slow_advance_to_next_turn,
            //         slow_new_proposal,
            //         slow_any_data_processed,
            //     ) = self.try_recv(app_sync_hash, false);
            //     was_loop_iteration_busy |= slow_any_data_processed;
            //     new_proposal |= slow_new_proposal;
            // }

            // || refr_new_proposal;
            // TODO: here we need to make use of self.chill_out attribute
            // Following needs to be implemented for cases like (Forward)ConnectRequests.
            // Need to find a way to always clear any data we have to send to our neighbors.
            // Maybe we can send that data without updating state...
            // Then only first message will pass sanity @ neighbor, following messages
            // will fail sanity, but requested data should be served...done?

            // maybe self.send_immediate should no longer be...

            // chill out mode may end abruptly in case new_proposal has been received
            if new_proposal
            // || advance_to_next_turn
            //     && self.next_state.last_accepted_message.swarm_time == SwarmTime(0)
            {
                if self.chill_out.0 {
                    // println!("Chill out is terminated abruptly");
                    self.send_immediate = true;
                }
                self.chill_out.0 = false;
            }
            if self.chill_out.0 {
                if self.chill_out.1.elapsed() >= self.chill_out_max {
                    // If self.chill_out.1 reaches 0 self._chill_out.0 =false and it's time to work.
                    // println!(
                    //     "Chill out is over fast:{}, slow:{}, refr:{}",
                    //     self.fast_neighbors.len(),
                    //     self.slow_neighbors.len(),
                    //     self.refreshed_neighbors.len()
                    // );
                    self.chill_out.0 = false;
                    // When we end chill_out mode, we have to start new timer.
                    // println!("Reset timer");
                    self.send_immediate = true;
                    timer = Instant::now();
                    self.timeout_duration = Duration::from_millis(500);
                } else {
                    if was_loop_iteration_busy {
                        // print!("d ");
                        sleep_nsec >>= 2;
                        if sleep_nsec < min_sleep_nsec {
                            sleep_nsec = min_sleep_nsec;
                        }
                    } else {
                        sleep_nsec <<= 1;
                        // print!("i ");
                        if sleep_nsec > max_sleep_nsec {
                            sleep_nsec = max_sleep_nsec;
                        }
                    }
                    continue;
                }
            }

            //TODO: following conditional logic is a terrible mess, it begs for refactoring
            let timeout = timer.elapsed() >= self.timeout_duration;
            if advance_to_next_turn || self.send_immediate || timeout && have_responsive_neighbors {
                loops_with_no_reply = 0;
                self.update_state();
                if !new_proposal && !self.send_immediate {
                    // println!("swap&send");
                    self.swap_neighbors();
                    self.send_all(available_bandwith);
                } else {
                    // println!("konkat&send");
                    self.concat_neighbors();
                    self.send_all(available_bandwith);
                }
                self.send_immediate = false;
                if self.check_if_new_round(available_bandwith)
                    && self.neighbor_discovery.tick_and_check()
                    // TODO: figure out some better algo
                    && available_bandwith > 256
                {
                    // self.query_for_new_neighbors();
                }
                timer = Instant::now();
                self.timeout_duration = Duration::from_millis(500);
            } else if timeout && !have_responsive_neighbors {
                loops_with_no_reply += 1;
                if loops_with_no_reply >= 5 {
                    loops_with_no_reply = 0;
                    break_the_loop = true;
                    if !self.slow_neighbors.is_empty() {
                        eprintln!("Timed out multiple times, droping slow neighbors…");
                        let slow = std::mem::take(&mut self.slow_neighbors);
                        for neighbor in slow {
                            self.drop_neighbor(neighbor.id);
                        }
                    }
                }
            }

            if break_the_loop {
                let _ = self
                    .mgr_sender
                    .send(GnomeToManager::Disconnected(self.swarm.id));
                break;
            };

            if was_loop_iteration_busy {
                sleep_nsec >>= 2;
                // print!("d ");
                if sleep_nsec < min_sleep_nsec {
                    sleep_nsec = min_sleep_nsec;
                }
            } else {
                sleep_nsec <<= 1;
                // print!("i ");
                if sleep_nsec > max_sleep_nsec {
                    sleep_nsec = max_sleep_nsec;
                }
            }
        } //loop
        eprintln!("Gnome is done");
    }
    pub fn has_neighbor(&self, id: GnomeId) -> bool {
        for neighbor in &self.slow_neighbors {
            if neighbor.id == id {
                return true;
            }
        }
        for neighbor in &self.fast_neighbors {
            if neighbor.id == id {
                return true;
            }
        }
        for neighbor in &self.refreshed_neighbors {
            if neighbor.id == id {
                return true;
            }
        }
        false
    }
    fn get_shared_sender(
        &self,
        neighbor_id: GnomeId,
    ) -> Option<
        Sender<(
            SwarmName,
            Sender<Message>,
            Sender<CastMessage>,
            Receiver<WrappedMessage>,
        )>,
    > {
        for neighbor in &self.fast_neighbors {
            if neighbor.id == neighbor_id {
                return Some(neighbor.get_shared_sender());
            }
        }
        for neighbor in &self.refreshed_neighbors {
            if neighbor.id == neighbor_id {
                return Some(neighbor.get_shared_sender());
            }
        }
        for neighbor in &self.slow_neighbors {
            if neighbor.id == neighbor_id {
                return Some(neighbor.get_shared_sender());
            }
        }
        for neighbor in &self.new_neighbors {
            if neighbor.id == neighbor_id {
                return Some(neighbor.get_shared_sender());
            }
        }
        None
    }

    fn notify_mgr_about_neighbors(&self) {
        let s_id = self.swarm.id;
        let mut n_ids = vec![];
        // for neighbor in &self.slow_neighbors {
        //     }
        // }
        for neighbor in &self.fast_neighbors {
            n_ids.push(neighbor.id);
        }
        for neighbor in &self.refreshed_neighbors {
            n_ids.push(neighbor.id);
        }
        //TODO: maybe we should not send to both App & Gnome mgr?
        let _ = self
            .sender
            .send(GnomeToApp::Neighbors(self.swarm.id, n_ids.clone()));
        let _ = self
            .mgr_sender
            .send(GnomeToManager::ActiveNeighbors(s_id, n_ids));
    }
    pub fn add_neighbor(&mut self, neighbor: Neighbor) {
        eprintln!("add_neighbor {:?}", neighbor.member_of_swarms);
        let neighbor_already_exists = self.has_neighbor(neighbor.id);
        for swarm_name in &neighbor.member_of_swarms {
            if !swarm_name.founder.is_any() && swarm_name != &self.swarm.name {
                let _ = self.mgr_sender.send(GnomeToManager::NeighboringSwarm(
                    self.swarm.id,
                    neighbor.id,
                    swarm_name.clone(),
                ));
            }
        }
        if neighbor_already_exists {
            // eprintln!("NOT Replacing a neighbor");
            self.drop_neighbor(neighbor.id);
            // self.fast_neighbors.push(neighbor);
        }
        if self.chill_out.0 || (self.fast_neighbors.is_empty() && self.slow_neighbors.is_empty()) {
            eprintln!(
                "{} ADD {} (chilling or no neighbors around)",
                self.swarm.id, neighbor.id
            );
            self.fast_neighbors.push(neighbor);
        } else {
            self.new_neighbors.push(neighbor);
        }
    }
    fn send_noop_from_a_neighbor(&self) {
        if let Some(neighbor) = self.fast_neighbors.first() {
            neighbor.send_no_op();
        } else if let Some(neighbor) = self.refreshed_neighbors.first() {
            neighbor.send_no_op();
        } else if let Some(neighbor) = self.slow_neighbors.first() {
            neighbor.send_no_op();
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

    pub fn send_all(&mut self, available_bandwith: u64) {
        let message = self.prepare_message();
        // TODO: we need to send something in case policy is not fullfilled
        // now we send an invalid message over the network
        // for all of our peers to drop us. We are also wasting bandwith.
        if !message.header.is_sync() && !self.swarm.verify_policy(&message) {
            eprintln!("Should not send, policy not fulfilled!");
        }
        let keep_alive = message.set_payload(Payload::KeepAlive(available_bandwith));
        for neighbor in &mut self.fast_neighbors {
            if neighbor.header == message.header {
                eprintln!("{} >>> {}", self.swarm.id, keep_alive);
                // println!("Sending KA only");
                neighbor.send_out(keep_alive.clone());
            } else {
                eprintln!("{} >/> {}", self.swarm.id, message);
                neighbor.send_out(message.clone());
            }
        }
        for neighbor in &mut self.slow_neighbors {
            if neighbor.header == message.header {
                eprintln!("{} >s> {}", self.swarm.id, message);
                neighbor.send_out(keep_alive.clone());
            } else {
                eprintln!("{} >S> {}", self.swarm.id, message);
                neighbor.send_out(message.clone());
            }
        }
    }

    pub fn prepare_message(&self) -> Message {
        Message {
            swarm_time: self.swarm_time,
            neighborhood: self.neighborhood,
            header: self.header,
            payload: self.payload.clone(),
        }
    }

    // TODO: first we need to get in sync with rest of the swarm both regarding
    //       SwarmTime and ChillOut phase.
    //       To do so we send sync request and wait for response
    //       If we also receive a sync request then it means we just started
    //       a new swarm or we are isolated from the rest swarm - we simply continue
    //       If we receive a Sync Response it should contain proper SwarmTime
    //       and also how many iterations of ChillOut mode are currently left.
    //       We apply those parameters to our state and continue to loop.
    //       If we receive any other message we set ChillOut to false and continue.
    fn presync_with_swarm(&mut self, available_bandwith: u64) {
        eprintln!("{} In presync", self.swarm.id);
        let mut remote_id = GnomeId(0);
        let response_opt = if let Some(neighbor) = self.fast_neighbors.iter_mut().next() {
            eprintln!(
                "{} {} Sending SyncReq to {} ",
                self.swarm.id, self.swarm.name, neighbor.id,
            );
            neighbor.send_out_cast(CastMessage::new_request(NeighborRequest::SwarmSyncRequest(
                SwarmSyncRequestParams {
                    sync_key_reg: true,
                    sync_capability: true,
                    sync_policy: true,
                    sync_broadcast: true,
                    sync_multicast: true,
                    // app_root_hash,
                },
            )));
            if let Ok(sync_response_option) = neighbor.recv_sync(Duration::from_secs(20)) {
                // TODO: not sure if reversing next_state update with start_new_round
                // inside neighbor.recv_sync is fine
                self.next_state.update(neighbor);
                remote_id = neighbor.id;
                sync_response_option
            } else {
                eprintln!("No response received");
                None
            }
        } else if let Some(neighbor) = self.slow_neighbors.iter_mut().next() {
            eprintln!(
                "SID-{} Slow {} Sending SwarmSyncRequest ",
                self.swarm.id.0, neighbor.id,
            );
            neighbor.send_out_cast(CastMessage::new_request(NeighborRequest::SwarmSyncRequest(
                SwarmSyncRequestParams {
                    sync_key_reg: true,
                    sync_capability: true,
                    sync_policy: true,
                    sync_broadcast: true,
                    sync_multicast: true,
                    // app_root_hash,
                },
            )));
            if let Ok(sync_response_option) = neighbor.recv_sync(Duration::from_secs(20)) {
                self.next_state.update(neighbor);
                remote_id = neighbor.id;
                sync_response_option
            } else {
                None
            }
        } else {
            eprintln!(
                "SID-{} Not Sending SwarmSyncRequest - no neighbors",
                self.swarm.id.0
            );
            None
        };
        // eprintln!("SID-{} Response opt: {:?}", self.swarm.id.0, response_opt);
        // TODO: make use of capability_size, policy_size, b_cast_size, m_cast_size, more_keys_follow
        // TODO: in case we join a swarm which has a *cast originating from our gnome
        //       we need to initialize all necessary piping to be able to send through it
        if let Some(NeighborResponse::SwarmSync(mut swarm_sync_response)) = response_opt {
            eprintln!(
                "App sync ST: {} Round: {} Key#: {}, chill: {}",
                // swarm_sync_response.app_root_hash,
                swarm_sync_response.swarm_time,
                swarm_sync_response.round_start,
                swarm_sync_response.key_reg_pairs.len(),
                swarm_sync_response.chill_phase
            );
            // eprintln!(
            //     "1 Setting founder from: {} to {}",
            //     self.swarm.name.founder, swarm_sync_response.founder
            // );
            eprintln!("Legacy founder set");
            self.swarm.set_founder(swarm_sync_response.founder);
            let _ = self.sender.send(GnomeToApp::SwarmReady(
                self.swarm.name.clone(),
                self.id == self.swarm.name.founder,
            ));
            self.swarm.swarm_type = swarm_sync_response.swarm_type;
            self.swarm.key_reg =
                KeyRegistry::from(&mut vec![swarm_sync_response.key_reg_size, 0, 0]);
            self.swarm_time = swarm_sync_response.swarm_time;
            self.round_start = swarm_sync_response.round_start;
            self.next_state.swarm_time = swarm_sync_response.swarm_time;
            // let _ = self.mgr_sender.send(GnomeToManager::FounderDetermined(
            //     self.swarm.id,
            //     self.swarm.name.founder,
            // ));
            while let Some((g_id, pubkey)) = swarm_sync_response.key_reg_pairs.pop() {
                self.swarm.key_reg.insert(g_id, pubkey);
            }
            if swarm_sync_response.chill_phase > 0 {
                eprintln!("Into chill {}", swarm_sync_response.chill_phase);
                self.chill_out.0 = true;
                self.chill_out.1 = Instant::now() - self.chill_out_max
                    + Duration::from_millis(swarm_sync_response.chill_phase as u64);
            } else {
                // eprintln!("no chill ");
                self.chill_out.0 = false;
            }
        } else if response_opt.is_none() {
            // let synced = app_root_hash == 0;
            let _ = self.sender.send(GnomeToApp::SwarmReady(
                self.swarm.name.clone(),
                self.id == self.swarm.name.founder, //TODO: not sure if this is ok
            ));
            // if remote_id.0 > 0 && self.swarm.name.founder.is_any() {
            //     // TODO: both of us want to Sync to empty Swarm
            //     //       we need to determine who is Founder
            //     if self.id > remote_id {
            //         // eprintln!(
            //         //     "2 {} Setting founder from: {} to {}",
            //         //     synced, self.swarm.name.founder, self.id
            //         // );
            //         self.swarm.set_founder(self.id);
            //     } else {
            //         // eprintln!(
            //         //     "3 Setting founder from: {} to {}",
            //         //     self.swarm.name.founder, remote_id
            //         // );
            //         self.swarm.set_founder(remote_id);
            //     }
            //     let _ = self.mgr_sender.send(GnomeToManager::FounderDetermined(
            //         self.swarm.id,
            //         self.swarm.name.founder,
            //     ));
            // } else if !self.swarm.name.founder.is_any() {
            //     self.swarm.set_founder(self.swarm.name.founder);
            // } else {
            //     // eprintln!("Unable to determine Founder");
            // }
            // println!("Sync response: {}", response);
            self.send_all(available_bandwith);
            return;
        } else {
            eprintln!("unexpected Response opt: {:?}", response_opt);
            let _ = self.sender.send(GnomeToApp::SwarmReady(
                self.swarm.name.clone(),
                self.id == self.swarm.name.founder, //TODO: not sure if this is ok
            ));
        }
    }

    fn swap_neighbors(&mut self) {
        // println!("Swapping neighbors");
        // TODO: we do not want to drop slow neighbors,
        //       only increase some counters to indicate what % of time
        //       they could not keep up
        //       user should decide to drop a neighbor
        //       or set a policy to drop a neighbor when certain dgram loss threashold
        //       is crossed
        for neighbor in &mut self.slow_neighbors {
            // TODO: here we should also check if given neighbor
            //       is a source for any multicast and
            //       maybe change it to some other neighbor
            neighbor.add_timeout();
        }
        let fast_n = std::mem::take(&mut self.fast_neighbors);
        for neighbor in fast_n {
            self.slow_neighbors.push(neighbor);
        }
        self.fast_neighbors = std::mem::replace(
            &mut self.refreshed_neighbors,
            Vec::with_capacity(DEFAULT_NEIGHBORS_PER_GNOME),
        );
        // TODO: if we have only slow_neighbors and no fast/refreshed
        //       neighbors - this is an indication that something is
        //       wrong with our network connection
        // println!(
        //     "After swap fast: {}, slow: {}, refr: {}",
        //     self.fast_neighbors.len(),
        //     self.slow_neighbors.len(),
        //     self.refreshed_neighbors.len()
        // );
    }

    fn send_neighbor_response(&mut self, neighbor_id: GnomeId, response: NeighborResponse) -> bool {
        for neighbor in &mut self.refreshed_neighbors {
            if neighbor.id == neighbor_id {
                neighbor.add_requested_data(response);
                return true;
            }
        }
        for neighbor in &mut self.fast_neighbors {
            if neighbor.id == neighbor_id {
                neighbor.add_requested_data(response);
                return true;
            }
        }
        for neighbor in &mut self.slow_neighbors {
            if neighbor.id == neighbor_id {
                neighbor.add_requested_data(response);
                return true;
            }
        }
        eprintln!("Failed to send response");
        false
    }

    fn send_neighbor_request(&mut self, id: GnomeId, request: NeighborRequest) -> bool {
        for neighbor in &mut self.fast_neighbors {
            if neighbor.id == id {
                neighbor.request_data(request);
                return true;
            }
        }
        for neighbor in &mut self.slow_neighbors {
            if neighbor.id == id {
                neighbor.request_data(request);
                return true;
            }
        }
        false
    }

    fn select_best_alternative(&self, alt_sources: Vec<GnomeId>) -> Option<GnomeId> {
        if alt_sources.is_empty() {
            return None;
        }
        let mut curr_pick = alt_sources[0];
        let mut curr_bandwith = 0;
        for neighbor in &self.refreshed_neighbors {
            let n_id = neighbor.id;
            if alt_sources.contains(&n_id) {
                let n_bandwith = neighbor.available_bandwith;
                if n_bandwith > curr_bandwith {
                    curr_pick = n_id;
                    curr_bandwith = n_bandwith;
                }
            }
        }
        for neighbor in &self.fast_neighbors {
            let n_id = neighbor.id;
            if alt_sources.contains(&n_id) {
                let n_bandwith = neighbor.available_bandwith;
                if n_bandwith > curr_bandwith {
                    curr_pick = n_id;
                    curr_bandwith = n_bandwith;
                }
            }
        }
        Some(curr_pick)
    }

    fn neighbor_with_highest_bandwith(&self) -> GnomeId {
        // eprintln!("Searching among {} neighbors", self.fast_neighbors.len());
        let mut gnome_id = GnomeId::any();
        let mut curr_max_band = 0;
        for neighbor in &self.fast_neighbors {
            // eprintln!("N band: {}", neighbor.available_bandwith);
            if neighbor.available_bandwith >= curr_max_band {
                gnome_id = neighbor.id;
                curr_max_band = neighbor.available_bandwith;
            }
        }
        gnome_id
    }

    fn neighbor_with_enough_bandwith(&self, min_bandwith: u64) -> Option<GnomeId> {
        // eprintln!("Searching among {} neighbors", self.fast_neighbors.len());
        for neighbor in &self.fast_neighbors {
            eprintln!("N band: {}", neighbor.available_bandwith);
            if neighbor.available_bandwith >= min_bandwith {
                return Some(neighbor.id);
            }
        }
        None
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
                eprintln!("Not updating neighborhood when catching up with swarm");
            } else {
                self.neighborhood = n_neigh;
            }
        } else {
            self.neighborhood = n_neigh;
        }
        self.swarm_time = n_st;
        self.header = n_head;

        self.payload = n_payload;
        if self.header == Header::Sync
            && self.round_start > SwarmTime(0)
            && self.neighborhood.0 <= 1
        {
            if let Some(mut proposal) = self.proposals.pop_back() {
                // We are submitting new proposal, so we have to reset NHood
                self.neighborhood = Neighborhood(0);
                let extend = !self.swarm.key_reg.has_key(self.id);
                let proposal_can_extend = proposal.can_be_extended();
                // Now we need to cover cases
                // 1. extend = false
                if !extend {
                    eprintln!("No need extending");
                    (self.header, self.payload) = proposal.clone().to_header_payload(
                        &self.sign,
                        self.id,
                        self.round_start,
                        extend,
                        &self.priv_key_pem,
                        self.pub_key_bytes.clone(),
                    );
                    // println!("No need extending: {}", self.payload.has_signature());
                } else if !proposal_can_extend {
                    // 2. extend = true, can not be extended
                    // 2 requires us to send a special Reconfiguration message
                    //   that adds given pubkey to swarm's key_reg
                    //   and we need to push back existing proposal
                    let configuration =
                        Configuration::InsertPubkey(self.id, self.pub_key_bytes.clone());
                    let new_proposal = Proposal::Config(configuration);
                    let orig_proposal = std::mem::replace(&mut proposal, new_proposal);
                    self.proposals.push_back(orig_proposal);
                    (self.header, self.payload) = proposal.clone().to_header_payload(
                        &self.sign,
                        self.id,
                        self.round_start,
                        extend,
                        &self.priv_key_pem,
                        self.pub_key_bytes.clone(),
                    );
                } else {
                    // 3. extend = true, can be extended
                    (self.header, self.payload) = proposal.clone().to_header_payload(
                        &self.sign,
                        self.id,
                        self.round_start,
                        extend,
                        &self.priv_key_pem,
                        self.pub_key_bytes.clone(),
                    );
                }
                self.next_state.header = self.header;
                self.next_state.payload = self.payload.clone();

                if self.header.is_reconfigure() {
                    if let Payload::Reconfigure(
                        _signature,
                        Configuration::StartBroadcast(g_id, c_id),
                    ) = &self.payload
                    {
                        self.next_state.change_config = ChangeConfig::AddBroadcast {
                            id: *c_id,
                            origin: *g_id,
                            source: *g_id,
                            filtered_neighbors: vec![],
                            turn_ended: false,
                        };
                        // }
                    } else if let Payload::Reconfigure(
                        _signature,
                        Configuration::InsertPubkey(id, key),
                    ) = &self.payload
                    {
                        self.next_state.change_config = ChangeConfig::InsertPubkey {
                            id: *id,
                            key: key.clone(),
                            turn_ended: false,
                        };
                    } else if let Payload::Reconfigure(
                        _sign,
                        Configuration::EndBroadcast(_g_id, c_id),
                    ) = &self.payload
                    {
                        self.next_state.change_config = ChangeConfig::RemoveBroadcast {
                            id: *c_id,
                            turn_ended: false,
                        };
                    } else if let Payload::Reconfigure(
                        _sign,
                        Configuration::UserDefined(id, s_data),
                    ) = &self.payload
                    {
                        self.next_state.change_config = ChangeConfig::Custom {
                            id: *id,
                            s_data: s_data.clone(),
                            turn_ended: false,
                        };
                    }
                }
                self.my_proposal = Some(proposal);
                self.send_immediate = true;
            }
        }
    }

    fn check_if_new_round(&mut self, available_bandwith: u64) -> bool {
        let all_gnomes_aware = self.neighborhood.0 as u32 >= self.swarm_diameter.0;
        let finish_round =
            self.swarm_time - self.round_start >= self.swarm_diameter + self.swarm_diameter;
        if all_gnomes_aware || finish_round {
            // println!("New round");
            if !self.slow_neighbors.is_empty() {
                // TODO: we need to store it as gnome's attribute and allow for
                //       user to change it (default is 87.5%)
                // let drop_treshold: u8 = 7 * self.swarm_diameter.0 as u8;
                let drop_treshold: u8 = 0;
                let mut slow_neighbors = std::mem::take(&mut self.slow_neighbors);
                while let Some(mut neighbor) = slow_neighbors.pop() {
                    neighbor.shift_timeout();
                    if neighbor.timeouts_count() < drop_treshold {
                        self.slow_neighbors.push(neighbor);
                    } else {
                        eprintln!(
                            "{} DROP {} as drop threshold exceeded",
                            self.swarm.name, neighbor.id
                        );
                    }
                }
            }
            for neighbor in &mut self.fast_neighbors {
                neighbor.shift_timeout();
            }
            let block_proposed = self.header.non_zero_block();
            let reconfig = self.header.is_reconfigure();
            if block_proposed || reconfig {
                self.send_immediate = true;
                if all_gnomes_aware {
                    self.next_state.last_accepted_message = self.prepare_message();
                    let payload = std::mem::replace(
                        &mut self.payload,
                        Payload::KeepAlive(available_bandwith),
                    );
                    if block_proposed {
                        // println!("Block proposed");
                        if let Payload::Block(block_id, signature, data) = payload {
                            if let Some((g_id, pub_key)) = signature.pubkey() {
                                self.swarm.key_reg.insert(g_id, pub_key)
                            }
                            let _res = self.sender.send(GnomeToApp::Block(block_id, data));
                            // println!("^^^ USER ^^^ NEW {} {:075}", block_id, data.0);
                        } else {
                            eprintln!("Can not send to user, payload not matching\n {:?}", payload);
                        }
                    } else {
                        eprintln!("We have got a Reconfig to parse");
                        if let Payload::Reconfigure(signature, _conf) = payload {
                            if let Some((g_id, pub_key)) = signature.pubkey() {
                                self.swarm.key_reg.insert(g_id, pub_key)
                            }
                        }
                        let change_config = std::mem::replace(
                            &mut self.next_state.change_config,
                            ChangeConfig::None,
                        );
                        // println!("Me: {}, c-k: {:?}", self.id, change_config);
                        match change_config {
                            ChangeConfig::AddBroadcast {
                                id,
                                origin,
                                source,
                                filtered_neighbors,
                                ..
                            } => {
                                let mut subscribers: HashMap<GnomeId, Sender<WrappedMessage>> =
                                    self.get_neighbor_ids_and_senders();
                                // println!("origin: {}", origin);
                                // println!("subs before filter: {:?}", subscribers);
                                subscribers.retain(|&gid, _s| {
                                    !filtered_neighbors.contains(&gid)
                                        &&gid != origin //This is covered
                                        &&gid != source
                                });
                                // println!("subs after filter: {:?}", subscribers);
                                // println!("filtered neighbors: {:?}", filtered_neighbors);
                                let (wrapped_message_sender, wrapped_message_receiver) = channel();
                                let (cast_data_sender, cast_data_receiver) = channel();
                                if self.id == origin {
                                    eprintln!("I am origin!");
                                    let (internal_cast_data_sender, internal_cast_data_receiver) =
                                        channel();
                                    let b_cast = Multicast::new(
                                        origin,
                                        (source, wrapped_message_receiver),
                                        filtered_neighbors,
                                        subscribers,
                                        Some(internal_cast_data_sender),
                                    );

                                    self.insert_originating_broadcast(
                                        id,
                                        cast_data_receiver,
                                        wrapped_message_sender,
                                    );
                                    self.swarm.insert_broadcast(id, b_cast);
                                    let _res = self.sender.send(GnomeToApp::BroadcastOrigin(
                                        self.swarm.id,
                                        id,
                                        cast_data_sender,
                                        internal_cast_data_receiver,
                                    ));
                                } else if self.activate_broadcast_at_neighbor(
                                    source,
                                    id,
                                    wrapped_message_sender,
                                ) {
                                    let b_cast = Multicast::new(
                                        origin,
                                        (source, wrapped_message_receiver),
                                        filtered_neighbors,
                                        subscribers,
                                        Some(cast_data_sender),
                                    );
                                    self.swarm.insert_broadcast(id, b_cast);
                                    let _res = self.sender.send(GnomeToApp::Broadcast(
                                        self.swarm.id,
                                        id,
                                        cast_data_receiver,
                                    ));
                                } else {
                                    eprintln!("Unable to activate broadcast");
                                }
                            }
                            ChangeConfig::RemoveBroadcast { id, .. } => {
                                let _ = self.swarm.remove_cast(true, &id);
                                self.remove_originating_broadcast(id);
                            }
                            ChangeConfig::InsertPubkey { id, key, .. } => {
                                // This is done automatically
                                // self.swarm.key_reg.insert(id, key);
                            }
                            ChangeConfig::Custom { id, s_data, .. } => {
                                // This is done automatically
                                eprintln!("Custom Reconfig-{} to serve", id);
                                let _res = self.sender.send(GnomeToApp::Reconfig(id, s_data));
                            }
                            ChangeConfig::None => {}
                        }
                    }
                    let my_proposal = self.my_proposal.take();
                    if let Some(my_proposed_data) = my_proposal {
                        // TODO: here we need to distinguish between Block and Reconfig
                        let extend = self
                            .next_state
                            .last_accepted_message
                            .payload
                            .is_signature_extended();
                        let (head, payload) = my_proposed_data.to_header_payload(
                            &self.sign,
                            self.id,
                            self.round_start,
                            extend,
                            &self.priv_key_pem,
                            self.pub_key_bytes.clone(),
                        );
                        if (&head, &payload)
                            != (
                                &self.next_state.last_accepted_message.header,
                                &self.next_state.last_accepted_message.payload,
                            )
                        {
                            self.proposals.push_back(Proposal::new(payload));
                            // panic!(
                            //     "Not the same proposal!{:?} != {:?}",
                            //     head, self.next_state.last_accepted_message.header
                            // );
                        }
                        self.my_proposal = None;
                    }

                    self.round_start = self.swarm_time;
                    eprintln!("New round start: {}", self.round_start);
                } else {
                    eprintln!(
                        "ERROR: Swarm diameter too small or {} was backdated! Rstart:{}",
                        self.header, self.round_start
                    );
                }
                if let Some(mut proposal) = self.proposals.pop_back() {
                    let extend = !self.swarm.key_reg.has_key(self.id);
                    let proposal_can_extend = proposal.can_be_extended();
                    // TODO: insert logic here ?
                    // probably yes, but include my_proposal update
                    // Now we need to cover cases
                    // 1. extend = false
                    if !extend {
                        (self.header, self.payload) = proposal.clone().to_header_payload(
                            &self.sign,
                            self.id,
                            self.round_start,
                            extend,
                            &self.priv_key_pem,
                            self.pub_key_bytes.clone(),
                        );
                    } else if !proposal_can_extend {
                        // 2. extend = true, can not be extended
                        // 2 requires us to send a special Reconfiguration message
                        //   that adds given pubkey to swarm's key_reg
                        //   and we need to push back existing proposal
                        let configuration =
                            Configuration::InsertPubkey(self.id, self.pub_key_bytes.clone());
                        let new_proposal = Proposal::Config(configuration);
                        let orig_proposal = std::mem::replace(&mut proposal, new_proposal);
                        self.proposals.push_back(orig_proposal);
                        (self.header, self.payload) = proposal.clone().to_header_payload(
                            &self.sign,
                            self.id,
                            self.round_start,
                            extend,
                            &self.priv_key_pem,
                            self.pub_key_bytes.clone(),
                        );
                    } else {
                        // 3. extend = true, can be extended
                        (self.header, self.payload) = proposal.clone().to_header_payload(
                            &self.sign,
                            self.id,
                            self.round_start,
                            extend,
                            &self.priv_key_pem,
                            self.pub_key_bytes.clone(),
                        );
                    }
                    self.my_proposal = Some(proposal);
                    self.send_immediate = true;
                } else {
                    // eprintln!("Starting to chill… 1");
                    self.header = Header::Sync;
                    self.payload = Payload::KeepAlive(available_bandwith);
                    self.chill_out.0 = true;
                    // TODO: probably we can merge chillout and timeout into one
                    self.chill_out.1 = Instant::now();
                }
            // println!("We have got a Reconfig to parse");
            } else {
                // Sync swarm time
                // self.swarm_time = self.round_start + self.swarm_diameter;
                eprintln!("{} Sync swarm time {}", self.swarm.id, self.swarm_time);
                self.round_start = self.swarm_time;
                self.next_state.last_accepted_message = self.prepare_message();
                // println!("set N-0");
                self.neighborhood = Neighborhood(0);
                // println!("--------round start to: {}", self.swarm_time);
                if let Some(mut proposal) = self.proposals.pop_back() {
                    self.my_proposal = Some(proposal.clone());
                    let extend = !self.swarm.key_reg.has_key(self.id);
                    let proposal_can_extend = proposal.can_be_extended();
                    // TODO: insert logic here ?
                    // probably yes, but include my_proposal update
                    // Now we need to cover cases
                    // 1. extend = false
                    if !extend {
                        (self.header, self.payload) = proposal.clone().to_header_payload(
                            &self.sign,
                            self.id,
                            self.round_start,
                            extend,
                            &self.priv_key_pem,
                            self.pub_key_bytes.clone(),
                        );
                    } else if !proposal_can_extend {
                        // 2. extend = true, can not be extended
                        // 2 requires us to send a special Reconfiguration message
                        //   that adds given pubkey to swarm's key_reg
                        //   and we need to push back existing proposal
                        let configuration =
                            Configuration::InsertPubkey(self.id, self.pub_key_bytes.clone());
                        let new_proposal = Proposal::Config(configuration);
                        let orig_proposal = std::mem::replace(&mut proposal, new_proposal);
                        self.proposals.push_back(orig_proposal);
                        (self.header, self.payload) = proposal.clone().to_header_payload(
                            &self.sign,
                            self.id,
                            self.round_start,
                            extend,
                            &self.priv_key_pem,
                            self.pub_key_bytes.clone(),
                        );
                    } else {
                        // 3. extend = true, can be extended
                        (self.header, self.payload) = proposal.clone().to_header_payload(
                            &self.sign,
                            self.id,
                            self.round_start,
                            extend,
                            &self.priv_key_pem,
                            self.pub_key_bytes.clone(),
                        );
                    }
                    self.my_proposal = Some(proposal);
                    self.send_immediate = true;
                } else {
                    // eprintln!("Starting to chill… 2");
                    self.chill_out.0 = true;
                    self.chill_out.1 = Instant::now();
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
                            &mut self.swarm,
                        );
                        eprintln!("{} ADD {}", self.swarm.id, neighbor.id);
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

    fn try_recv(&mut self, fast: bool) -> (bool, bool, bool, bool) {
        let mut any_data_processed = false;
        let n_len = if fast {
            self.fast_neighbors.len()
        } else {
            self.slow_neighbors.len()
        };
        if n_len == 0 {
            return (false, false, false, any_data_processed);
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
                any_data_processed = true;
                match response {
                    GnomeToApp::ToGnome(neighbor_response) => match neighbor_response {
                        NeighborResponse::ConnectResponse(id, net_set) => {
                            self.add_ongoing_reply(id, net_set);
                        }
                        NeighborResponse::AlreadyConnected(id) => {
                            self.skip_neighbor(id);
                        }
                        NeighborResponse::ForwardConnectResponse(net_set) => {
                            eprintln!("ForwardConnResponse: {:?}", net_set);
                            let _ = self.net_settings_send.send(net_set);
                        }
                        NeighborResponse::ForwardConnectFailed => {
                            // TODO build querying mechanism
                            // TODO inform gnome's mechanism to ask another neighbor
                        }
                        // TODO make use af app_sync_hash
                        // We should notify application layer about received
                        // app_sync_hash and it should act accordingly
                        // If our hash is the same as received we do nothing
                        // if it is different we need to sync,
                        // if we have just joined then this means we are behind
                        NeighborResponse::SwarmSync(mut swarm_sync_response) => {
                            if swarm_sync_response.chill_phase > 0 {
                                eprintln!("Into chill {}", swarm_sync_response.chill_phase);
                                self.chill_out.0 = true;
                                self.chill_out.1 = Instant::now() - self.chill_out_max
                                    + Duration::from_millis(swarm_sync_response.chill_phase as u64);
                            } else {
                                // eprintln!("no chill ");
                                self.chill_out.0 = false;
                            }
                            eprintln!(
                                "4 Setting founder from {} to {}",
                                self.swarm.name.founder, swarm_sync_response.founder
                            );
                            self.swarm_time = swarm_sync_response.swarm_time;
                            self.round_start = swarm_sync_response.round_start;
                            self.swarm.set_founder(swarm_sync_response.founder);
                            self.swarm.swarm_type = swarm_sync_response.swarm_type;
                            self.swarm.key_reg = KeyRegistry::from(&mut vec![
                                swarm_sync_response.key_reg_size,
                                0,
                                0,
                            ]);
                            self.next_state.swarm_time = swarm_sync_response.swarm_time;
                            while let Some((id, pubkey)) = swarm_sync_response.key_reg_pairs.pop() {
                                self.swarm.key_reg.insert(id, pubkey);
                            }
                            let _ = self.sender.send(GnomeToApp::SwarmReady(
                                self.swarm.name.clone(),
                                self.id == self.swarm.name.founder,
                            ));
                            // if swarm_sync_response.app_root_hash != app_sync_hash {
                            //     // println!("rem: {:?}, our: {:?}", rem_app_sync_hash, app_sync_hash);
                            //     let _ = self.sender.send(GnomeToApp::AppDataSynced(false));
                            // } else {
                            //     let _ = self.sender.send(GnomeToApp::AppDataSynced(true));
                            // }
                        }
                        NeighborResponse::Subscribed(is_bcast, cast_id, origin, source) => {
                            let source = source.unwrap();
                            let (send_d, recv_d) = channel();
                            let (send_m, recv_m) = channel();
                            let activated = if is_bcast {
                                self.activate_broadcast_at_neighbor(source, cast_id, send_m)
                            } else {
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
                                    let _res = self.sender.send(GnomeToApp::Broadcast(
                                        self.swarm.id,
                                        cast_id,
                                        recv_d,
                                    ));
                                } else {
                                    self.swarm.insert_multicast(cast_id, m_cast);
                                    let _res = self.sender.send(GnomeToApp::Multicast(
                                        self.swarm.id,
                                        cast_id,
                                        recv_d,
                                    ));
                                }
                            }
                        }
                        NeighborResponse::KeyRegistrySync(chunk_no, total_chunks, mut pairs) => {
                            // TODO: we need to preserve key registry order!
                            while let Some((gnome_id, pubkey)) = pairs.pop() {
                                self.swarm.key_reg.insert(gnome_id, pubkey);
                            }
                        }
                        NeighborResponse::CapabilitySync(chunk_no, total_chunks, mut pairs) => {
                            // TODO: we also need to cover case when a capability
                            //       is split into two or more chunks
                            while let Some((capability, ids)) = pairs.pop() {
                                self.swarm.insert_capability(capability, ids);
                            }
                        }
                        NeighborResponse::PolicySync(_chunk_no, _total_chunks, mut pairs) => {
                            while let Some((policy, req)) = pairs.pop() {
                                self.swarm.policy_reg.insert(policy, req);
                            }
                        }
                        NeighborResponse::BroadcastSync(chunk_no, total_chunks, mut pairs) => {
                            while let Some((c_id, origin)) = pairs.pop() {
                                self.send_internal
                                    .send(InternalMsg::SubscribeCast(true, c_id, origin))
                                    .unwrap();
                            }
                        }
                        NeighborResponse::MulticastSync(chunk_no, total_chunks, mut pairs) => {
                            while let Some((c_id, origin)) = pairs.pop() {
                                // TODO: do we really want to subscribe to all multicasts?
                                // self.send_internal
                                //     .send(InternalMsg::SubscribeCast(false, c_id, origin))
                                //     .unwrap();
                            }
                        }
                        other => {
                            eprintln!("Uncovered NeighborResponse: {:?}\nother", other);
                        }
                    },
                    _ => {
                        // eprintln!("Got response: {:?}\nunderscore", response);
                        let _ = self.sender.send(response);
                    }
                }
            }
            let (served, sanity_passed, new_proposal, drop_me) = neighbor.try_recv(
                self.next_state.last_accepted_message.clone(),
                &mut self.swarm,
            );
            // println!("snd {}", pass);
            //     "{} srv:{} pass:{} new:{}",
            //     neighbor.id, served, sanity_passed, new_proposal
            // );
            if !new_proposal_received {
                new_proposal_received = new_proposal;
            }
            if served {
                // println!("Served!");
                any_data_processed = true;
                if sanity_passed {
                    //TODO: this is wacky
                    if self.round_start.0 == 0 {
                        self.next_state.swarm_time = neighbor.swarm_time;
                    }
                    self.next_state.update(&mut neighbor);
                }
                if !drop_me {
                    self.refreshed_neighbors.push(neighbor);
                } else {
                    eprintln!("{} Dropping a neighbor {}", self.swarm.name, neighbor.id);
                }
            } else if !drop_me {
                if fast {
                    self.fast_neighbors.push(neighbor);
                } else {
                    self.slow_neighbors.push(neighbor);
                }
            } else {
                eprintln!("{} Dropping  neighbor {}", self.swarm.name, neighbor.id);
            }
        }
        let refreshed_empty = self.refreshed_neighbors.is_empty();
        let fast_empty = self.fast_neighbors.is_empty();
        let slow_empty = self.slow_neighbors.is_empty();
        let have_responsive_neighbors = !refreshed_empty || !fast_empty;
        if refreshed_empty && fast_empty && slow_empty {
            eprintln!("Can not advance with no neighbors around\n\n\n\n");
            self.chill_out.0 = false;
            (
                have_responsive_neighbors,
                false,
                new_proposal_received,
                any_data_processed,
            )
        } else {
            let advance_to_next_turn = if fast {
                fast_empty && looped || new_proposal_received
            } else {
                slow_empty && looped || new_proposal_received
            };
            (
                have_responsive_neighbors,
                advance_to_next_turn,
                new_proposal_received,
                any_data_processed,
            )
        }
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
        }
    }
    fn get_neighbor_ids_and_senders(&self) -> HashMap<GnomeId, Sender<WrappedMessage>> {
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

    fn insert_originating_unicast(
        &mut self,
        id: CastID,
        recv_d: Receiver<CastData>,
        send_n: Sender<WrappedMessage>,
    ) {
        self.data_converters
            .insert((CastType::Unicast, id), (recv_d, send_n));
    }

    fn insert_originating_broadcast(
        &mut self,
        id: CastID,
        recv_d: Receiver<CastData>,
        send_n: Sender<WrappedMessage>,
    ) {
        self.data_converters
            .insert((CastType::Broadcast, id), (recv_d, send_n));
    }

    fn remove_originating_broadcast(&mut self, id: CastID) {
        let _ = self.data_converters.remove(&(CastType::Broadcast, id));
    }

    fn activate_broadcast_at_neighbor(
        &mut self,
        id: GnomeId,
        cast_id: CastID,
        sender: Sender<WrappedMessage>,
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
