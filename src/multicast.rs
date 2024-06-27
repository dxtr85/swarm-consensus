use std::{
    collections::HashMap,
    sync::mpsc::{Receiver, Sender},
};

use crate::{CastID, Data, GnomeId, WrappedMessage};

#[derive(Clone, Copy, Debug)]
pub enum CastType {
    Unicast,
    Multicast,
    Broadcast,
}
#[derive(Clone, Copy, Debug)]
pub struct CastMessage {
    pub c_type: CastType,
    pub id: CastID,
    pub data: Data,
}
impl CastMessage {
    pub fn id(&self) -> CastID {
        self.id
    }
    pub fn data(&self) -> Data {
        self.data
    }
    pub fn new_unicast(id: CastID, data: Data) -> Self {
        CastMessage {
            c_type: CastType::Unicast,
            id,
            data,
        }
    }
    pub fn new_multicast(id: CastID, data: Data) -> Self {
        CastMessage {
            c_type: CastType::Multicast,
            id,
            data,
        }
    }
    pub fn new_broadcast(id: CastID, data: Data) -> Self {
        CastMessage {
            c_type: CastType::Broadcast,
            id,
            data,
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
    source: (GnomeId, Receiver<CastMessage>),
    alt_sources: Vec<GnomeId>,
    subscribers: HashMap<GnomeId, Sender<WrappedMessage>>,
    to_user: Option<Sender<Data>>,
}

impl Multicast {
    pub fn new(
        origin: GnomeId,
        source: (GnomeId, Receiver<CastMessage>),
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
    pub fn set_source(&mut self, source: (GnomeId, Receiver<CastMessage>)) {
        self.source = source;
    }

    pub fn add_subscriber(&mut self, subscriber: (GnomeId, Sender<WrappedMessage>)) {
        self.subscribers.insert(subscriber.0, subscriber.1);
    }

    pub fn serve(&mut self) {
        while let Ok(msg) = self.source.1.try_recv() {
            // println!("Received a casting msg: {:?}", msg);
            if let Some(sender) = &self.to_user {
                let _ = sender.send(msg.data);
            }
            for sender in self.subscribers.values() {
                // println!("Wrapped send: {:?}", msg);
                let _ = sender.send(WrappedMessage::Cast(msg));
            }
        }
    }
}
