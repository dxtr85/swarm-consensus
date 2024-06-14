use crate::message::Header;
use crate::message::Payload;
use crate::neighbor::Neighborhood;
use crate::CastID;
use crate::Configuration;
use crate::GnomeId;
use crate::Message;
use crate::Neighbor;
use crate::SwarmTime;

#[derive(Debug)]
pub enum ChangeConfig {
    None,
    AddBroadcast {
        id: CastID,
        origin: GnomeId,
        source: GnomeId,
        filtered_neighbors: Vec<GnomeId>,
        turn_ended: bool,
    },
}

impl ChangeConfig {
    pub fn end_turn(&mut self) {
        match *self {
            Self::None => {}
            Self::AddBroadcast {
                ref mut turn_ended, ..
            } => *turn_ended = true,
        }
    }

    pub fn add_filtered_neighbor(&mut self, gnome_id: GnomeId) {
        if let Self::AddBroadcast {
            ref mut filtered_neighbors,
            turn_ended,
            ..
        } = *self
        {
            if !turn_ended {
                filtered_neighbors.push(gnome_id);
            }
        }
    }
}

#[derive(Debug)]
pub struct NextState {
    pub neighborhood: Neighborhood,
    pub swarm_time: SwarmTime,
    pub swarm_time_max: SwarmTime,
    pub change_config: ChangeConfig,
    // pub block_id: BlockID,
    // pub last_accepted_block: BlockID,
    // pub last_accepted_reconf: Option<Configuration>,
    // pub data: Data,
    pub header: Header,
    pub last_accepted_message: Message,
    pub payload: Payload,
    all_neighbors_same_header: bool,
}

impl NextState {
    pub fn new() -> Self {
        NextState {
            neighborhood: Neighborhood(0),
            swarm_time: SwarmTime(0),
            swarm_time_max: SwarmTime(std::u32::MAX),
            change_config: ChangeConfig::None,
            // block_id: BlockID(0),
            // last_accepted_block: BlockID(0),
            // last_accepted_reconf: None,
            // data: Data(0),
            header: Header::Sync,
            all_neighbors_same_header: true,
            last_accepted_message: Message::block(),
            payload: Payload::KeepAlive,
        }
    }

    pub fn update(&mut self, neighbor: &Neighbor) {
        // println!("Update {:?}", neighbor);
        let neighbor_st = neighbor.swarm_time;

        if neighbor_st < self.swarm_time_max && neighbor_st < self.swarm_time {
            self.swarm_time = neighbor_st;
        }

        // let block_id_received = match neighbor.header {
        //     Header::Sync => BlockID(0),
        //     Header::Reconfigure => BlockID(0),
        //     Header::Block(b_id) => b_id,
        // };
        // let data_received = match neighbor.payload {
        //     Payload::Block(_b_id, data) => data,
        //     Payload::Reconfigure(config) => Data(config.as_u32()),
        //     _ => Data(0),
        // };
        // println!("id, data: {} {}", block_id_received, data_received);

        // if self.block_id != block_id_received {
        //     self.all_neighbors_same_header = false;
        //     // println!("fols");
        // }
        if self.header != neighbor.header {
            self.all_neighbors_same_header = false;
            // println!("fols");
        }

        // println!("{} > {}", block_id_received, self.block_id);
        // if block_id_received > self.block_id
        //     || (block_id_received == self.block_id && data_received.0 > self.data.0)
        if neighbor.header > self.header {
            // println!("N {} > {}", neighbor.header, self.header);
            // self.block_id = block_id_received;
            self.header = neighbor.header;
            // self.data = data_received;
            self.payload = neighbor.payload.clone();
            self.neighborhood = Neighborhood(0);
            // } else if block_id_received == self.block_id
            if self.header.is_reconfigure() {
                if let Payload::Reconfigure(config) = self.payload {
                    match config {
                        Configuration::StartBroadcast(origin, id) => {
                            self.change_config = ChangeConfig::AddBroadcast {
                                id,
                                origin,
                                source: neighbor.id,
                                filtered_neighbors: vec![],
                                turn_ended: false,
                            };
                        }
                        _ => {
                            println!("Unhandled config");
                        }
                    }
                }
            }
        } else if neighbor.header == self.header && self.neighborhood.0 > neighbor.neighborhood.0 {
            self.neighborhood = neighbor.neighborhood;
            if neighbor.header.is_reconfigure() {
                if let Payload::Reconfigure(config) = neighbor.payload {
                    match config {
                        Configuration::StartBroadcast(_g_id, _c_id) => {
                            self.change_config.add_filtered_neighbor(neighbor.id);
                        }
                        _ => {
                            println!("Unhandled config update");
                        }
                    }
                }
            }
            // TODO: handle reconfig
        }
        // println!("Po wszystkim: {} {}", self.block_id, self.data);
    }

    fn next_swarm_time(&mut self) {
        self.swarm_time = if self.swarm_time.0 == std::u32::MAX {
            SwarmTime(0)
        } else {
            self.swarm_time.inc()
        };
        self.swarm_time_max = self.swarm_time.inc();
    }

    // pub fn next_params(&self) -> (SwarmTime, Neighborhood, BlockID, Data) {
    pub fn next_params(&self) -> (SwarmTime, Neighborhood, Header, Payload) {
        if self.all_neighbors_same_header {
            // println!("next_params 1 {}", self.swarm_time);
            (
                self.swarm_time.inc(),
                self.neighborhood.inc(),
                self.header,
                self.payload.clone(),
            )
        } else {
            // println!("next_params 2 {}", self.swarm_time);
            (
                self.swarm_time.inc(),
                self.neighborhood,
                self.header,
                self.payload.clone(),
            )
        }
    }

    // pub fn reset_for_next_turn(&mut self, new_round: bool, block_id: BlockID, data: Data) {
    pub fn reset_for_next_turn(&mut self, new_round: bool, header: Header, payload: Payload) {
        // println!("reset {}", self.swarm_time);
        self.next_swarm_time();
        // self.block_id = block_id;
        // self.data = data;
        self.header = header;
        self.payload = payload.clone();
        self.all_neighbors_same_header = true;
        self.change_config.end_turn();
        self.neighborhood = if new_round {
            self.change_config = if header.is_reconfigure() {
                if let Payload::Reconfigure(config) = payload {
                    match config {
                        Configuration::StartBroadcast(origin, id) => ChangeConfig::AddBroadcast {
                            id,
                            origin,
                            source: origin,
                            filtered_neighbors: vec![],
                            turn_ended: false,
                        },
                        _ => {
                            // TODO: handle other configs
                            println!("Unhandled config in reset_for_next_turn");
                            ChangeConfig::None
                        }
                    }
                } else {
                    println!("Unexpected payload in reset_for_next_turn");
                    ChangeConfig::None
                }
            } else {
                ChangeConfig::None
            };
            self.all_neighbors_same_header = false;
            Neighborhood(0)
        } else {
            self.neighborhood.inc()
        };
        // println!("bid: {}", self.block_id);
    }
}
