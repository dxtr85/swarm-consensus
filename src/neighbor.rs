use crate::Awareness;
use crate::GnomeId;
use crate::Message;
use crate::Proposal;
use crate::SwarmTime;

use std::sync::mpsc::{Receiver, Sender};

#[derive(Debug)]
pub struct Neighbor {
    id: GnomeId,
    receiver: Receiver<Message>,
    pub sender: Sender<Message>,
    pub awareness: Awareness,
    pub prev_awareness: Option<Awareness>,
    pub proposal: Option<Proposal>,
}

impl Neighbor {
    pub fn from_id_channel_time(
        id: GnomeId,
        receiver: Receiver<Message>,
        sender: Sender<Message>,
        swarm_time: SwarmTime,
    ) -> Self {
        Neighbor {
            id,
            receiver,
            sender,
            awareness: Awareness::Unaware(swarm_time),
            prev_awareness: None,
            proposal: None,
        }
    }
    fn swarm_time(&self) -> SwarmTime {
        match self.awareness {
            Awareness::Unaware(st) => st,
            Awareness::Aware(st, _p, _pd) => st,
            Awareness::Confused(st, _cd) => st,
        }
    }

    pub fn set_swarm_time(&mut self, swarm_time: SwarmTime) {
        self.awareness = Awareness::Unaware(swarm_time);
    }

    pub fn try_recv(&mut self) -> (bool, bool) {
        let mut message_recvd = false;
        let mut sanity_passed = true;
        if let Ok(data) = self.receiver.try_recv() {
            message_recvd = true;
            let swarm_time = self.swarm_time();
            match data {
                ka @ Message::KeepAlive(awareness) => {
                    self.awareness = awareness;
                    println!("{:?} << {:?}", self.id, ka);
                }
                p @ Message::Proposal(awareness, value) => {
                    self.awareness = awareness;
                    self.proposal = Some(value as Proposal);
                    println!("{:?} << {:?}", self.id, p);
                }
            }
            let new_swarm_time = self.swarm_time();
            if new_swarm_time < swarm_time {
                println!(
                    "Received a message with ST{:?}  when previous was {:?}",
                    new_swarm_time, swarm_time
                );
                sanity_passed = false;
            }
        }
        (message_recvd, sanity_passed)
    }
}
