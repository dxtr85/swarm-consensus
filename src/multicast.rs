use std::{
    collections::HashMap,
    sync::mpsc::{Receiver, Sender},
};

use crate::{CastData, CastID, GnomeId, NeighborRequest, NeighborResponse, WrappedMessage};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CastType {
    Unicast,
    Multicast,
    Broadcast,
}

#[derive(Clone, Debug)]
pub enum CastContent {
    Data(CastData),
    Request(NeighborRequest),
    Response(NeighborResponse),
}
impl CastContent {
    pub fn len(&self) -> usize {
        match self {
            Self::Data(d) => d.len(),
            Self::Request(nreq) => nreq.len(),
            Self::Response(nresp) => nresp.len(),
        }
    }
}

// TODO: make Unicast CastID = 255 & 254 not available for user
// these will be created automatically for every gnome pair
// in order to quickly exchange Requests and Responses
// For CastType Unicast and CastID = 255 content = Request,
// For CastType Unicast and CastID = 254 content = Response,
// For CastType Unicast and CastID != 254 & != 255 content = Data,
//
#[derive(Clone, Debug)]
pub struct CastMessage {
    pub c_type: CastType,
    pub id: CastID,
    pub content: CastContent,
}

impl CastMessage {
    pub fn id(&self) -> CastID {
        self.id
    }
    pub fn get_data(self) -> Option<CastData> {
        if let CastContent::Data(dat) = self.content {
            Some(dat)
        } else {
            None
        }
    }
    pub fn get_request(&self) -> Option<NeighborRequest> {
        if let CastContent::Request(ref req) = self.content {
            Some(req.clone())
        } else {
            None
        }
    }
    pub fn get_response(&self) -> Option<NeighborResponse> {
        if let CastContent::Response(ref resp) = self.content {
            Some(resp.clone())
        } else {
            None
        }
    }
    pub fn new_unicast(id: CastID, data: CastData) -> Self {
        CastMessage {
            c_type: CastType::Unicast,
            id,
            content: CastContent::Data(data),
        }
    }
    pub fn new_request(neighbor_request: NeighborRequest) -> Self {
        CastMessage {
            c_type: CastType::Unicast,
            id: CastID(255),
            content: CastContent::Request(neighbor_request),
        }
    }
    pub fn new_response(neighbor_response: NeighborResponse) -> Self {
        CastMessage {
            c_type: CastType::Unicast,
            id: CastID(254),
            content: CastContent::Response(neighbor_response),
        }
    }
    pub fn new_multicast(id: CastID, data: CastData) -> Self {
        CastMessage {
            c_type: CastType::Multicast,
            id,
            content: CastContent::Data(data),
        }
    }
    pub fn new_broadcast(id: CastID, data: CastData) -> Self {
        CastMessage {
            c_type: CastType::Broadcast,
            id,
            content: CastContent::Data(data),
        }
    }
    pub fn is_unicast(&self) -> bool {
        matches!(self.c_type, CastType::Unicast)
    }
    pub fn is_multicast(&self) -> bool {
        matches!(self.c_type, CastType::Multicast)
    }
    pub fn is_broadcast(&self) -> bool {
        matches!(self.c_type, CastType::Broadcast)
    }
    pub fn len(&self) -> usize {
        47 + self.content.len()
    }
}

pub struct Multicast {
    origin: GnomeId,
    source: (GnomeId, Receiver<WrappedMessage>),
    alt_sources: Vec<GnomeId>,
    subscribers: HashMap<GnomeId, Sender<WrappedMessage>>,
    to_app: Option<Sender<CastData>>,
}

impl Multicast {
    pub fn new(
        origin: GnomeId,
        source: (GnomeId, Receiver<WrappedMessage>),
        alt_sources: Vec<GnomeId>,
        subscribers: HashMap<GnomeId, Sender<WrappedMessage>>,
        to_app: Option<Sender<CastData>>,
    ) -> Self {
        Self {
            origin,
            source,
            alt_sources,
            subscribers,
            to_app,
        }
    }
    // pub fn subscribers(&self) -> Vec<(GnomeId,Sender<Message>)> {
    //     self.subscribers.clone()
    // }
    pub fn source(&self) -> GnomeId {
        self.source.0
    }
    pub fn origin(&self) -> GnomeId {
        self.origin
    }
    pub fn subscribers(&self) -> Vec<GnomeId> {
        let mut subs = Vec::with_capacity(self.subscribers.len());
        for sub in self.subscribers.keys() {
            subs.push(*sub)
        }
        subs
    }
    pub fn set_source(&mut self, source: (GnomeId, Receiver<WrappedMessage>)) {
        self.source = source;
    }

    pub fn add_subscriber(&mut self, subscriber: (GnomeId, Sender<WrappedMessage>)) {
        self.subscribers.insert(subscriber.0, subscriber.1);
    }

    pub fn remove_subscriber(&mut self, subscriber: &GnomeId) -> Option<Sender<WrappedMessage>> {
        self.subscribers.remove(subscriber)
    }

    pub fn dont_send_to_app(&mut self) {
        self.to_app = None;
    }
    pub fn get_alt_sources(&mut self, old_source: GnomeId) -> Vec<GnomeId> {
        let prev_sources = std::mem::replace(&mut self.alt_sources, vec![]);
        for alt_s in prev_sources {
            if alt_s == old_source {
                continue;
            }
            self.alt_sources.push(alt_s);
        }
        self.alt_sources.clone()
    }

    pub fn serve(&mut self, available_tokens: u64) -> (bool, u64) {
        let mut any_data_processed = false;
        let mut tokens_used = 0;
        // let mut tokens_remaining = available_tokens;
        while let Ok(WrappedMessage::Cast(msg)) = self.source.1.try_recv() {
            eprintln!("Received a casting msg: {:?}", msg);
            let m_size = msg.len() as u64;
            for sender in self.subscribers.values() {
                any_data_processed = true;
                tokens_used += m_size;
                // tokens_remaining = tokens_remaining.saturating_sub(m_size);
                // println!("Wrapped send: {:?}", msg);
                let _ = sender.send(WrappedMessage::Cast(msg.clone()));
            }
            // TODO: we can Unsubscribe from a cast when to_app is None
            //       and subscribers.len()>0 when we want to save bandwith
            if let Some(sender) = &self.to_app {
                let res = sender.send(msg.get_data().unwrap());
                if res.is_err() {
                    //TODO: unsubscribe - or we can keep this cast
                    // since we are not forwarding to anyone it does
                    // not cost us bandwith, only some CPU cycles
                    // if !any_data_processed {
                    // }
                    eprintln!("User not interested in bcast.");
                    self.to_app = None;
                } else {
                    any_data_processed = true;
                }
            }
            // if tokens_remaining == 0 {
            if tokens_used > available_tokens {
                break;
            }
        }
        (any_data_processed, tokens_used)
    }
}
