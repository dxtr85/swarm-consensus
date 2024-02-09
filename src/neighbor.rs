use crate::Awareness;
use crate::GnomeId;
use crate::Message;
use crate::Proposal;

use std::sync::mpsc::{Receiver, Sender};

#[derive(Debug)]
pub struct Neighbor {
    id: GnomeId,
    receiver: Receiver<Message>,
    pub sender: Sender<Message>,
    pub awareness: Awareness,
    pub proposal: Option<Proposal>,
}

impl Neighbor {
    pub fn from_id_channel(
        id: GnomeId,
        receiver: Receiver<Message>,
        sender: Sender<Message>,
    ) -> Self {
        Neighbor {
            id,
            receiver,
            sender,
            awareness: Awareness::Unaware,
            proposal: None,
        }
    }

    pub fn try_recv(&mut self) -> bool {
        let mut message_recvd = false;
        if let Ok(data) = self.receiver.try_recv() {
            message_recvd = true;
            match data {
                ka @ Message::KeepAlive(_id, awareness) => {
                    self.awareness = awareness;
                    println!("<< {:?}", ka);
                }
                p @ Message::Proposal(_id, awareness, value) => {
                    self.awareness = awareness;
                    self.proposal = Some(value as Proposal);
                    println!("[{:?},{:?}] << {:?} received", &self.id, &self.awareness, p);
                }
            }
        }
        message_recvd
    }
}
