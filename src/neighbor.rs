use crate::message::BlockID;
use crate::message::Header;
use crate::message::Payload;
use crate::Data;
use crate::GnomeId;
use crate::Message;
use crate::Response;
use crate::SwarmTime;
use std::fmt::Display;

use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, Sender};

#[derive(Clone, Copy, Debug)]
pub struct Neighborhood(pub u8);

impl Neighborhood {
    pub fn inc(&self) -> Self {
        Neighborhood(self.0 + 1)
    }
}

impl Display for Neighborhood {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "N-{}", self.0)
    }
}

#[derive(Debug)]
pub struct Neighbor {
    pub id: GnomeId,
    receiver: Receiver<Message>,
    pub sender: Sender<Message>,
    pub swarm_time: SwarmTime,
    pub neighborhood: Neighborhood,
    pub prev_neighborhood: Option<Neighborhood>,
    pub header: Header,
    pub payload: Payload,
    pub our_requests: VecDeque<NeighborRequest>,
    pub our_responses: VecDeque<Response>,
    requests: VecDeque<NeighborRequest>,
    requested_data: VecDeque<(NeighborRequest, NeighborResponse)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NeighborRequest {
    ListingRequest(SwarmTime),
    PayloadRequest(u8, [BlockID; 128]),
}

#[derive(Clone, Copy, Debug)]
pub enum NeighborResponse {
    Listing(u8, [BlockID; 128]),
    Block(BlockID, Data),
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
            neighborhood: Neighborhood(0),
            prev_neighborhood: None,
            header: Header::Sync,
            payload: Payload::KeepAlive,
            our_requests: VecDeque::new(),
            our_responses: VecDeque::new(),
            requests: VecDeque::new(),
            requested_data: VecDeque::new(),
        }
    }

    pub fn set_swarm_time(&mut self, swarm_time: SwarmTime) {
        self.swarm_time = swarm_time;
    }

    pub fn try_recv(&mut self) -> (bool, bool, bool) {
        let mut message_recvd = false;
        let sanity_passed = true;
        let mut new_proposal = false;
        while let Ok(Message {
            swarm_time,
            neighborhood,
            header,
            payload,
        }) = self.receiver.try_recv()
        {
            // println!("{} got {}", self.id, m);
            if self.sanity_check(&swarm_time, &neighborhood, &header) {
                message_recvd = true;
            } else {
                continue;
            }
            // println!("Sanity passed");

            self.swarm_time = swarm_time;
            match header {
                Header::Sync => {
                    if self.header == Header::Sync {
                        self.prev_neighborhood = Some(self.neighborhood);
                    } else {
                        self.prev_neighborhood = None;
                        self.header = Header::Sync;
                    }
                    self.neighborhood = neighborhood;
                }
                Header::Block(id) => {
                    if let Header::Block(current_id) = self.header {
                        if current_id == id {
                            self.prev_neighborhood = Some(self.neighborhood);
                        } else {
                            new_proposal = true;
                            self.prev_neighborhood = None;
                            self.header = header;
                        }
                        self.neighborhood = neighborhood;
                    } else {
                        self.header = header;
                        new_proposal = true;
                        self.prev_neighborhood = None;
                        self.neighborhood = neighborhood;
                    }
                }
            };
            match payload {
                Payload::KeepAlive => {
                    // println!("KeepAlive");
                }
                Payload::Block(block_id, data) => {
                    match self.header {
                        Header::Block(id) => {
                            if id == block_id {
                                self.payload = payload;
                            } else {
                                self.our_responses
                                    .push_back(Response::Block(block_id, data));
                            }
                        }
                        Header::Sync => {
                            self.our_responses
                                .push_back(Response::Block(block_id, data));
                        }
                    };
                }
                Payload::Request(request) => {
                    self.requests.push_back(request);
                }
                Payload::Listing(count, listing) => {
                    self.our_responses
                        .push_back(Response::Listing(count, listing));
                }
            }
            // println!("returning: {} {:?}", new_proposal, self.neighborhood);
        }
        (message_recvd, sanity_passed, new_proposal)
    }

    fn sanity_check(
        &self,
        swarm_time: &SwarmTime,
        _neighborhood: &Neighborhood,
        header: &Header,
    ) -> bool {
        if self.swarm_time > *swarm_time {
            println!("Received a message with older swarm_time than previous!");
            return false;
        }
        if let Header::Block(id) = self.header {
            return match header {
                Header::Block(new_id) => new_id <= &id, //&& self.neighborhood <= neighborhood,
                Header::Sync => true, //TODO: maybe check here if neighborhood was big enough to swich
            };
        } else {
            return true;
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
