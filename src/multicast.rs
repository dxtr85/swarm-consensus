use std::{
    collections::HashMap,
    sync::mpsc::{Receiver, Sender},
};

use crate::{CastID, Data, GnomeId, NeighborRequest, NeighborResponse, WrappedMessage};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CastType {
    Unicast,
    Multicast,
    Broadcast,
}

#[derive(Clone, Debug)]
pub enum CastContent {
    Data(Data),
    Request(NeighborRequest),
    Response(NeighborResponse),
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
    pub fn get_data(&self) -> Option<Data> {
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
    pub fn new_unicast(id: CastID, data: Data) -> Self {
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
    pub fn new_multicast(id: CastID, data: Data) -> Self {
        CastMessage {
            c_type: CastType::Multicast,
            id,
            content: CastContent::Data(data),
        }
    }
    pub fn new_broadcast(id: CastID, data: Data) -> Self {
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
}

pub struct Multicast {
    origin: GnomeId,
    source: (GnomeId, Receiver<WrappedMessage>),
    alt_sources: Vec<GnomeId>,
    subscribers: HashMap<GnomeId, Sender<WrappedMessage>>,
    to_user: Option<Sender<Data>>,
}

impl Multicast {
    pub fn new(
        origin: GnomeId,
        source: (GnomeId, Receiver<WrappedMessage>),
        alt_sources: Vec<GnomeId>,
        subscribers: HashMap<GnomeId, Sender<WrappedMessage>>,
        to_user: Option<Sender<Data>>,
    ) -> Self {
        Self {
            origin,
            source,
            alt_sources,
            subscribers,
            to_user,
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
    pub fn set_source(&mut self, source: (GnomeId, Receiver<WrappedMessage>)) {
        self.source = source;
    }

    pub fn add_subscriber(&mut self, subscriber: (GnomeId, Sender<WrappedMessage>)) {
        self.subscribers.insert(subscriber.0, subscriber.1);
    }

    pub fn serve(&mut self) {
        while let Ok(WrappedMessage::Cast(msg)) = self.source.1.try_recv() {
            // println!("Received a casting msg: {:?}", msg);
            if let Some(sender) = &self.to_user {
                let _ = sender.send(msg.get_data().unwrap());
            }
            for sender in self.subscribers.values() {
                // println!("Wrapped send: {:?}", msg);
                let _ = sender.send(WrappedMessage::Cast(msg.clone()));
            }
        }
    }
}
