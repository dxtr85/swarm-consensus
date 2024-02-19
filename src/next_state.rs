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
    pub fn new(swarm_time_min: SwarmTime) -> Self {
        NextState {
            neighborhood: Neighborhood(0),
            swarm_time: SwarmTime(0),
            swarm_time_min,
            block_id: BlockID(0),
            data: Data(0),
            all_neighbors_same_header: true,
        }
    }

    pub fn update(&mut self, neighbor: &Neighbor) {
        let neighbor_st = neighbor.swarm_time;
        if neighbor_st > self.swarm_time_min && neighbor_st < self.swarm_time {
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

        if self.block_id != block_id_received {
            self.all_neighbors_same_header = false;
        }

        if block_id_received > self.block_id {
            self.block_id = block_id_received;
            self.data = data_received;
            self.neighborhood = Neighborhood(0);
        } else if block_id_received == self.block_id
            && self.neighborhood.0 > neighbor.neighborhood.0
        {
            self.neighborhood = neighbor.neighborhood;
        }
    }

    fn next_swarm_time(&mut self) {
        self.swarm_time = if self.swarm_time.0 == std::u32::MAX {
            self.swarm_time_min.inc()
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

    pub fn reset_for_next_turn(&mut self, new_round: bool) {
        self.next_swarm_time();
        self.all_neighbors_same_header = true;
        if new_round {
            self.neighborhood = Neighborhood(0);
            self.block_id = BlockID(0);
            self.data = Data(0);
        } else {
            self.neighborhood = self.neighborhood.inc();
        }
    }
}
