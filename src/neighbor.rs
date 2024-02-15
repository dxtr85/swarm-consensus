use crate::Awareness;
use crate::Data;
use crate::GnomeId;
use crate::Message;
use crate::ProposalData;
use crate::SwarmTime;

use std::sync::mpsc::{Receiver, Sender};

#[derive(Debug)]
pub struct Neighbor {
    id: GnomeId,
    receiver: Receiver<Message>,
    pub sender: Sender<Message>,
    pub swarm_time: SwarmTime,
    pub awareness: Awareness,
    pub prev_awareness: Option<Awareness>,
    pub proposal_id: Option<(SwarmTime, GnomeId)>,
    pub proposal_data: ProposalData
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
            swarm_time,
            awareness: Awareness::Unaware,
            prev_awareness: None,
            proposal_id: None,
            proposal_data: ProposalData(0)
        }
    }
    // fn swarm_time(&self) -> SwarmTime {
    //     match self.awareness {
    //         Awareness::Unaware => st,
    //         Awareness::Aware( _p, _pd) => st,
    //         Awareness::Confused(st, _cd) => st,
    //     }
    // }

    pub fn set_swarm_time(&mut self, swarm_time: SwarmTime) {
        self.awareness = Awareness::Unaware;
        self.swarm_time = swarm_time; 
    }


   pub fn try_recv(&mut self, gnome_awareness: Awareness, proposal_id: Option<(SwarmTime, GnomeId)>) -> (bool, bool) {
        let mut message_recvd = false;
        let mut sanity_passed = true;
        if let Ok(
            m @ Message {
                swarm_time,
                awareness,
                data,
            },
        ) = self.receiver.try_recv()
        {
            message_recvd = true;
            if self.swarm_time > swarm_time{
                println!("Received a message with older swarm_time than previous!");
            }
            self.swarm_time = swarm_time;
            // let old_swarm_time = self.swarm_time;
            match data {
                Data::KeepAlive => {
                    if !awareness.is_aware() {
                        self.prev_awareness = None;
                    }
                    // A gnome can not keep progressing after being told
                    // that there is a conflicting proposal (must become confused)
                    if gnome_awareness.is_confused() && awareness.is_aware() {
                        println!("Gnome is confused, sanity fail!");
                        sanity_passed = false;
                    }
                    let gnome_neighborhood = if gnome_awareness.is_aware() {
                        gnome_awareness.neighborhood().unwrap()
                    } else {
                        0
                    };
                    if self.awareness.is_aware() && awareness.is_aware() {
                        sanity_passed =
                            sanity_passed && self.sanity_check(&awareness, gnome_neighborhood);
                    }
                    self.prev_awareness = Some(self.awareness);
                    self.awareness = awareness;
                    println!("{:?} {}", self.id, m);
                }
                Data::Proposal(proposal_time, proposer, data) => {
                    // A gnome can not announce a different action unless the previous
                    // action was completed or the confusion timeout has passed
                    if !gnome_awareness.is_unaware() {
                         let (known_proposal_time, known_proposer) = proposal_id.unwrap();
                         if known_proposal_time != proposal_time && known_proposer != proposer{
                        println!("Gnome is not unaware(1), sanity fail!");
                        sanity_passed = false;
                    }}

                    self.prev_awareness = None;
                    self.awareness = awareness;
                    self.proposal_id = Some((proposal_time, proposer));
                    self.proposal_data = data;
                    println!("{:?} {}", self.id, m);
                }
                Data::ProposalId(proposal_time, proposer) => {
                    // A gnome can not announce a different action unless the previous
                    // action was completed or the confusion timeout has passed
                    if !gnome_awareness.is_unaware() {
                         let (known_proposal_time, known_proposer) = proposal_id.unwrap();
                         if known_proposal_time != proposal_time && known_proposer != proposer{
                        println!("Gnome is not unaware(2), sanity fail!");
                        sanity_passed = false;
                    }}
                    self.prev_awareness = None;
                    self.awareness = awareness;
                    println!("{:?} {}", self.id, m);
                }
                _ => {}
            }
            let new_swarm_time = self.swarm_time;

            // TODO: with below if we receive messages in reversed order,
            // we drop a neighbor, but sanity will fail anyway so...
            if new_swarm_time < swarm_time {
                println!(
                    "Received a message \n{:?}\n with {:?}  when previous was {:?}",
                    m, new_swarm_time, swarm_time
                );
                sanity_passed = false;
            }
        }
        if ! sanity_passed{        println!("Sanity passed: {}", sanity_passed);
        };
        (message_recvd, sanity_passed)
    }

    fn sanity_check(&self, awareness: &Awareness, gnome_neighborhood: u8) -> bool {
        let current_neighborhood = awareness.neighborhood().unwrap();
        let new_neighborhood = self.awareness.neighborhood().unwrap();
        let newer_than_two_turns_before = if let Some(prev_awareness) = self.prev_awareness {
            let prev_neighborhood = prev_awareness.neighborhood().unwrap();
            new_neighborhood > prev_neighborhood
        } else {
            true
        };

        let backtrack_sanity = current_neighborhood > new_neighborhood;
        let neighborhood_increase_sanity =
            new_neighborhood > current_neighborhood || newer_than_two_turns_before;
        let not_too_aware = new_neighborhood <= gnome_neighborhood + 1;

        backtrack_sanity && neighborhood_increase_sanity && not_too_aware
    }
}
