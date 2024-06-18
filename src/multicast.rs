use std::{
    collections::HashMap,
    sync::mpsc::{Receiver, Sender},
};

use crate::{Data, GnomeId, Message};

pub struct Multicast {
    origin: GnomeId,
    source: (GnomeId, Receiver<Message>),
    alt_sources: Vec<GnomeId>,
    subscribers: HashMap<GnomeId, Sender<Message>>,
    to_user: Option<Sender<Data>>,
}

impl Multicast {
    pub fn new(
        origin: GnomeId,
        source: (GnomeId, Receiver<Message>),
        alt_sources: Vec<GnomeId>,
        subscribers: HashMap<GnomeId, Sender<Message>>,
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
    pub fn set_source(&mut self, source: (GnomeId, Receiver<Message>)) {
        self.source = source;
    }

    pub fn add_subscriber(&mut self, subscriber: (GnomeId, Sender<Message>)) {
        self.subscribers.insert(subscriber.0, subscriber.1);
    }

    pub fn serve(&mut self) {
        while let Ok(msg) = self.source.1.try_recv() {
            if let Some(sender) = &self.to_user {
                if let Some(data) = msg.payload.data() {
                    let _ = sender.send(data);
                }
            }
            for sender in self.subscribers.values() {
                // println!("Send: {:?}", msg);
                let _ = sender.send(msg.clone());
            }
        }
    }
}
