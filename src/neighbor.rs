use crate::proposal::ProposalID;
use crate::Awareness;
use crate::Data;
use crate::GnomeId;
use crate::Message;
use crate::ProposalData;
use crate::Response;
use crate::SwarmTime;

use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, Sender};

#[derive(Debug)]
pub struct Neighbor {
    pub id: GnomeId,
    receiver: Receiver<Message>,
    pub sender: Sender<Message>,
    pub swarm_time: SwarmTime,
    pub awareness: Awareness,
    pub prev_awareness: Option<Awareness>,
    pub proposal_id: Option<ProposalID>,
    pub proposal_data: ProposalData,
    pub our_requests: VecDeque<NeighborRequest>,
    pub our_responses: VecDeque<Response>,
    requests: VecDeque<NeighborRequest>,
    requested_data: VecDeque<(NeighborRequest, NeighborResponse)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NeighborRequest {
    ListingFrom(SwarmTime),
    Proposal(ProposalID),
}

#[derive(Clone, Copy, Debug)]
pub enum NeighborResponse {
    Listing(u8, [ProposalID; 128]),
    Proposal(ProposalData),
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
            proposal_data: ProposalData(0),
            our_requests: VecDeque::new(),
            our_responses: VecDeque::new(),
            requests: VecDeque::new(),
            requested_data: VecDeque::new(),
        }
    }

    pub fn set_swarm_time(&mut self, swarm_time: SwarmTime) {
        self.awareness = Awareness::Unaware;
        self.swarm_time = swarm_time;
    }

    pub fn try_recv(
        &mut self,
        gnome_awareness: Awareness,
        proposal_id: Option<ProposalID>,
    ) -> (bool, bool, bool) {
        let mut message_recvd = false;
        let mut sanity_passed = true;
        let mut new_proposal = false;
        while let Ok(
            m @ Message {
                swarm_time,
                awareness,
                data,
            },
        ) = self.receiver.try_recv()
        {
            if self.sanity_check(&awareness, &swarm_time, &gnome_awareness) {
                message_recvd = true;
            } else {
                continue;
            }

            self.swarm_time = swarm_time;
            if !awareness.is_aware() {
                self.prev_awareness = None;
            }
            self.prev_awareness = Some(self.awareness);
            self.awareness = awareness;
            match data {
                Data::KeepAlive => println!("{} << {}", self.id, m),
                Data::Proposal(ProposalID(proposal_time, proposer), data) => {
                    // A gnome can not announce a different action unless the previous
                    // action was completed or the confusion timeout has passed
                    if !gnome_awareness.is_unaware() {
                        let ProposalID(known_proposal_time, known_proposer) = proposal_id.unwrap();
                        if known_proposal_time != proposal_time && known_proposer != proposer {
                            println!("Gnome is not unaware(1), sanity fail!");
                            sanity_passed = false;
                        }
                    } else {
                        new_proposal = true;
                    }

                    self.prev_awareness = None;
                    self.proposal_id = Some(ProposalID(proposal_time, proposer));
                    self.proposal_data = data;
                    println!("{} << {}", self.id, m);
                }
                Data::ProposalId(ProposalID(proposal_time, proposer)) => {
                    // A gnome can not announce a different action unless the previous
                    // action was completed or the confusion timeout has passed
                    if !gnome_awareness.is_unaware() {
                        if let Some(ProposalID(known_proposal_time, known_proposer)) = proposal_id {
                            if known_proposal_time != proposal_time && known_proposer != proposer {
                                println!("Gnome is not unaware(2), sanity fail!");
                                sanity_passed = false;
                            }
                        }
                    } else {
                        println!("Got new proposal without data, this should not happen...");
                        // new_proposal = true;
                    }
                    println!("{} << {}", self.id, m);
                }
                Data::Request(request) => self.requests.push_back(request),
                Data::Response(request, response) => match (request, response) {
                    (
                        NeighborRequest::ListingFrom(_swarm_time),
                        NeighborResponse::Listing(count, listing),
                    ) => {
                        self.our_responses
                            .push_back(Response::Listing(count, listing));
                    }
                    (
                        NeighborRequest::Proposal(ProposalID(swarm_time, gnome_id)),
                        NeighborResponse::Proposal(data),
                    ) => {
                        self.our_responses.push_back(Response::Data(
                            crate::proposal::ProposalID(swarm_time, gnome_id),
                            data,
                        ));
                    }
                    (_, _) => {
                        println!("ERROR: Unexpected neighbor request/response pair!");
                    }
                },
            }
        }
        if !sanity_passed {
            println!("Sanity passed: {}", sanity_passed);
        };
        (message_recvd, sanity_passed, new_proposal)
    }

    fn sanity_check(
        &self,
        awareness: &Awareness,
        swarm_time: &SwarmTime,
        gnome_awareness: &Awareness,
    ) -> bool {
        if self.swarm_time > *swarm_time {
            println!("Received a message with older swarm_time than previous!");
            return false;
        }
        // A gnome can not keep progressing after being told
        // that there is a conflicting proposal (must become confused)
        if gnome_awareness.is_confused() && awareness.is_aware() {
            println!("Neighbor should become confused, but did not.");
            return false;
        }
        if gnome_awareness.is_aware() && awareness.is_unaware() {
            println!("Neighbor should become aware, but did not.");
            return false;
        }
        if self.awareness.is_aware() && awareness.is_aware() {
            if let Awareness::Aware(gnome_neighborhood) = gnome_awareness {
                let new_neighborhood = awareness.neighborhood().unwrap();
                let current_neighborhood = self.awareness.neighborhood().unwrap();
                let newer_than_two_turns_before = if let Some(prev_awareness) = self.prev_awareness
                {
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
            } else {
                true
            }
        } else if self.awareness.is_unaware() {
            return true;
        } else {
            println!(
                "Uncovered sanity_check case: {} {}",
                self.awareness, awareness
            );
            true
        }
    }

    pub fn get_specialized_data(&mut self) -> Option<(NeighborRequest, NeighborResponse)> {
        self.requested_data.pop_back()
    }

    pub fn add_requested_data(&mut self, request: NeighborRequest, data: NeighborResponse) {
        self.requested_data.push_front((request, data));
    }
    pub fn request_data(&mut self, request: NeighborRequest) {
        self.our_requests.push_front(request);
    }
}
