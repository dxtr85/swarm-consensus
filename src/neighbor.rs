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

    pub fn try_recv(&mut self, gnome_awareness: Awareness) -> (bool, bool) {
        let mut message_recvd = false;
        let mut sanity_passed = true;
        if let Ok(data) = self.receiver.try_recv() {
            message_recvd = true;
            let swarm_time = self.swarm_time();
            match data {
                ka @ Message::KeepAlive(new_awareness) => {
                    if !new_awareness.is_aware(){
                        self.prev_awareness = None;
                    }
                    // A gnome can not keep progressing after being told 
                    // that there is a conflicting proposal (must become confused)
                    if gnome_awareness.is_confused() && new_awareness.is_aware(){
                        sanity_passed = false;
                    }
                    let gnome_neighborhood = if gnome_awareness.is_aware(){
                        gnome_awareness.neighborhood().unwrap()
                    }else{0};
                    if self.awareness.is_aware() && new_awareness.is_aware() {
                        sanity_passed = sanity_passed && self.sanity_check(&new_awareness, gnome_neighborhood);
                    }
                    self.prev_awareness = Some(self.awareness);
                    self.awareness = new_awareness;
                    println!("{:?} << {:?}", self.id, ka);
                }
                p @ Message::Proposal(new_awareness, value) => {
                    // A gnome can not announce a different action unless the previous
                    // action was completed or the confusion timeout has passed
                    if !gnome_awareness.is_unaware(){
                        sanity_passed = false;
                    }
                    self.prev_awareness = None;
                    self.awareness = new_awareness;
                    self.proposal = Some(value as Proposal);
                    println!("{:?} << {:?}", self.id, p);
                }
            }
            let new_swarm_time = self.swarm_time();
            // TODO: with below if we receive messages in reversed order,
            // we drop a neighbor, but sanity will fail anyway so...
            if new_swarm_time < swarm_time {
                println!(
                    "Received a message with {:?}  when previous was {:?}",
                    new_swarm_time, swarm_time
                );
                sanity_passed = false;
            }
        }
        (message_recvd, sanity_passed)
    }

    fn sanity_check(&self, awareness: &Awareness, gnome_neighborhood: u8) -> bool {
        let current_neighborhood = awareness.neighborhood().unwrap();
        let new_neighborhood = self.awareness.neighborhood().unwrap();
        let newer_than_two_turns_before = if let Some(prev_awareness) = self.prev_awareness{
            let prev_neighborhood = prev_awareness.neighborhood().unwrap();
            new_neighborhood > prev_neighborhood
        }else{true};
       let backtrack_sanity = current_neighborhood > new_neighborhood;
       let neighborhood_increase_sanity = new_neighborhood > current_neighborhood || newer_than_two_turns_before;
       let not_too_aware = new_neighborhood <= gnome_neighborhood +1;
       backtrack_sanity && neighborhood_increase_sanity&& not_too_aware
    }
}
