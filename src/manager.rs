use crate::swarm::Swarm;
use crate::Neighbor;
use crate::Request;
use crate::Response;
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};

pub struct Manager {
    swarms: HashMap<String, Swarm>,
}

impl Manager {
    pub fn new() -> Manager {
        Manager {
            swarms: HashMap::new(),
        }
    }

    pub fn join_a_swarm(
        &mut self,
        name: String,
        neighbors: Option<Vec<Neighbor>>,
    ) -> (Sender<Request>, Receiver<Response>) {
        let mut swarm = Swarm::join(name.clone(), neighbors);
        let sender = swarm.sender.clone();
        let receiver = swarm.receiver.take();
        println!("Joined `{}` swarm", swarm.name);
        self.swarms.insert(name, swarm);
        (sender, receiver.unwrap())
    }

    pub fn print_status(&self, name: &str) {
        if let Some(swarm) = self.swarms.get(name) {
            let _ = swarm.sender.send(Request::Status);
        }
    }

    pub fn finish(mut self) {
        for swarm in self.swarms.values_mut() {
            let _ = swarm.sender.send(Request::Disconnect);
            let jh = swarm.join_handle.take().unwrap();
            let _ = jh.join();
            println!("Leaving `{}` swarm", swarm.name);
        }
        drop(self)
    }
}
