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
    round_start: SwarmTime,
    sender: Sender<Message>,
    pub swarm_time: SwarmTime,
    pub swarm_diameter: SwarmTime,
    pub neighborhood: Neighborhood,
    pub prev_neighborhood: Option<Neighborhood>,
    pub header: Header,
    pub payload: Payload,
    pub user_requests: VecDeque<NeighborRequest>,
    pub user_responses: VecDeque<Response>,
    requests: VecDeque<NeighborRequest>,
    requested_data: VecDeque<(NeighborRequest, NeighborResponse)>,
    gnome_header: Header,
    gnome_neighborhood: Neighborhood,
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
        swarm_diameter: SwarmTime,
    ) -> Self {
        Neighbor {
            id,
            receiver,
            round_start: SwarmTime(0),
            sender,
            swarm_time,
            swarm_diameter,
            neighborhood: Neighborhood(0),
            prev_neighborhood: None,
            header: Header::Sync,
            payload: Payload::KeepAlive,
            user_requests: VecDeque::new(),
            user_responses: VecDeque::new(),
            requests: VecDeque::new(),
            requested_data: VecDeque::new(),
            gnome_header: Header::Sync,
            gnome_neighborhood: Neighborhood(0),
        }
    }

    pub fn start_new_round(&mut self, swarm_time: SwarmTime) {
        self.swarm_time = swarm_time;
        self.round_start = swarm_time;
        self.header = Header::Sync;
        self.payload = Payload::KeepAlive;
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
            // println!(
            //     "{} got {} {} {:?}",
            //     self.id, swarm_time, neighborhood, header
            // );
            if self.sanity_check(&swarm_time, &neighborhood, &header) {
                message_recvd = true;
            } else {
                // println!("Coś nie poszło");
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
                                self.user_responses
                                    .push_back(Response::Block(block_id, data));
                            }
                        }
                        Header::Sync => {
                            self.user_responses
                                .push_back(Response::Block(block_id, data));
                        }
                    };
                }
                Payload::Request(request) => {
                    self.requests.push_back(request);
                }
                Payload::Listing(count, listing) => {
                    self.user_responses
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
        neighborhood: &Neighborhood,
        header: &Header,
    ) -> bool {
        if self.swarm_time > *swarm_time {
            println!("Received a message with older swarm_time than previous!");
            return false;
        }
        // A neighbor can not announce a number greater than the number
        // we announced to him, plus one
        let hood_inc_limited = if self.gnome_header == *header {
            neighborhood.0 <= self.gnome_neighborhood.0 + 1
        } else {
            true
        };
        // println!("hood_inc_limited: {}", hood_inc_limited);

        if self.header == *header {
            // A gnome can not stay at the same neighborhood number for more than
            // 2 turns
            let hood_increased = if let Some(prev_neighborhood) = self.prev_neighborhood {
                // println!(
                //     "{} new: {:?} > prev: {:?}",
                //     swarm_time, neighborhood, prev_neighborhood
                // );
                neighborhood.0 > prev_neighborhood.0
            } else {
                // A gnome can not backtrack by announcing a smaller neighborhood
                // number than before
                // println!(
                //     "{} current: {:?} <= new: {:?}",
                //     swarm_time, self.neighborhood, neighborhood
                // );
                self.neighborhood.0 <= neighborhood.0
            };
            // println!("hood_increased: {}", hood_increased);
            hood_increased && hood_inc_limited
        } else {
            let no_backdating = self.swarm_time - self.round_start < self.swarm_diameter;
            // println!("no_backdating: {}", no_backdating);
            if let Header::Block(id) = self.header {
                match header {
                    Header::Block(new_id) => new_id >= &id && no_backdating && hood_inc_limited,
                    Header::Sync => false,
                }
            } else {
                // Backdating check
                no_backdating && hood_inc_limited
            }
        }
    }

    pub fn send_out(&mut self, message: Message) {
        let _ = self.sender.send(message);
        self.gnome_header = message.header;
        self.gnome_neighborhood = message.neighborhood;
    }

    pub fn get_specialized_data(&mut self) -> Option<(NeighborRequest, NeighborResponse)> {
        self.requested_data.pop_back()
    }

    pub fn add_requested_data(&mut self, request: NeighborRequest, data: NeighborResponse) {
        self.requested_data.push_front((request, data));
    }
    pub fn request_data(&mut self, request: NeighborRequest) {
        self.user_requests.push_front(request);
    }
}
