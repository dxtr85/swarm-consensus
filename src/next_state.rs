use crate::message::BlockID;
use crate::message::Header;
use crate::message::Payload;
use crate::neighbor::Neighborhood;
use crate::proposal::Data;
use crate::Neighbor;
use crate::SwarmTime;

#[derive(Debug)]
pub struct NextState {
    pub neighborhood: Neighborhood,
    pub swarm_time: SwarmTime,
    pub swarm_time_min: SwarmTime,
    pub block_id: BlockID,
    pub data: Data,
    all_neighbors_same_header: bool,
}

impl NextState {
    pub fn new() -> Self {
        NextState {
            neighborhood: Neighborhood(0),
            swarm_time: SwarmTime(0),
            swarm_time_min: SwarmTime(0),
            block_id: BlockID(0),
            data: Data(0),
            all_neighbors_same_header: true,
        }
    }

    pub fn update(&mut self, neighbor: &Neighbor) {
        // println!("Update {:?}", neighbor);
        let neighbor_st = neighbor.swarm_time;
        if neighbor_st > self.swarm_time_min && neighbor_st <= self.swarm_time {
            self.swarm_time = neighbor_st;
        }

        let block_id_received = match neighbor.header {
            Header::Sync => BlockID(0),
            Header::Block(b_id) => b_id,
        };
        let data_received = match neighbor.payload {
            Payload::Block(_b_id, data) => data,
            _ => Data(0),
        };
        // println!("id, data: {} {}", block_id_received, data_received);

        if self.block_id != block_id_received {
            self.all_neighbors_same_header = false;
            // println!("fols");
        }

        // println!("{} > {}", block_id_received, self.block_id);
        if block_id_received > self.block_id {
            self.block_id = block_id_received;
            self.data = data_received;
            self.neighborhood = Neighborhood(0);
        } else if block_id_received == self.block_id
            && self.neighborhood.0 > neighbor.neighborhood.0
        {
            self.neighborhood = neighbor.neighborhood;
        }
        // println!("Po wszystkim: {} {}", self.block_id, self.data);
    }

    fn next_swarm_time(&mut self) {
        self.swarm_time = if self.swarm_time.0 == std::u32::MAX {
            SwarmTime(0)
        } else {
            self.swarm_time.inc()
        };
        self.swarm_time_min = self.swarm_time;
    }

    pub fn next_params(&self) -> (SwarmTime, Neighborhood, BlockID, Data) {
        if self.all_neighbors_same_header {
            (
                self.swarm_time.inc(),
                self.neighborhood.inc(),
                self.block_id,
                self.data,
            )
        } else {
            (
                self.swarm_time.inc(),
                self.neighborhood,
                self.block_id,
                self.data,
            )
        }
    }

    pub fn reset_for_next_turn(&mut self, new_round: bool, block_id: BlockID, data: Data) {
        self.next_swarm_time();
        self.block_id = block_id;
        self.data = data;
        self.all_neighbors_same_header = true;
        self.neighborhood = if new_round {
            self.all_neighbors_same_header = false;
            Neighborhood(0)
        } else {
            self.neighborhood.inc()
        };
    }
}
