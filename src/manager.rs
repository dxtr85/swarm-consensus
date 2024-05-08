use crate::gnome::NetworkSettings;
use crate::swarm::{Swarm, SwarmID};
use crate::Request;
use crate::Response;
use crate::{GnomeId, Neighbor};
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};

pub struct Manager {
    gnome_id: GnomeId,
    swarms: HashMap<SwarmID, Swarm>,
    network_settings: NetworkSettings,
    to_networking: Sender<(
        String,
        Sender<Request>,
        Sender<u32>,
        Receiver<(NetworkSettings, Option<NetworkSettings>)>,
    )>,
}

impl Manager {
    pub fn new(
        gnome_id: GnomeId,
        network_settings: Option<NetworkSettings>,
        to_networking: Sender<(
            String,
            Sender<Request>,
            Sender<u32>,
            Receiver<(NetworkSettings, Option<NetworkSettings>)>,
        )>,
    ) -> Manager {
        let network_settings = network_settings.unwrap_or_default();
        Manager {
            gnome_id,
            swarms: HashMap::new(),
            network_settings,
            to_networking, // Send a message to networking about new swarm subscription, and where to send Neighbors
        }
    }

    pub fn add_neighbor_to_a_swarm(&mut self, id: SwarmID, neighbor: Neighbor) {
        if let Some(swarm) = self.swarms.get_mut(&id) {
            match swarm.sender.send(Request::AddNeighbor(neighbor)) {
                Ok(()) => println!("Added neighbor to existing swarm"),
                Err(e) => println!("Failed adding neighbor to existing swarm: {:?}", e),
            }
        } else {
            println!("No swarm with id: {:?}", id);
        }
    }

    fn next_avail_swarm_id(&self) -> Option<SwarmID> {
        for i in 0..=255 {
            let swarm_id = SwarmID(i);
            if !self.swarms.contains_key(&swarm_id) {
                return Some(swarm_id);
            }
        }
        None
    }

    pub fn join_a_swarm(
        &mut self,
        name: String,
        neighbors: Option<Vec<Neighbor>>,
    ) -> Result<(SwarmID, (Sender<Request>, Receiver<Response>)), String> {
        if let Some(swarm_id) = self.next_avail_swarm_id() {
            let (band_send, band_recv) = channel();
            let (net_settings_send, net_settings_recv) = channel();
            let mut swarm = Swarm::join(
                name.clone(),
                swarm_id,
                self.gnome_id,
                neighbors,
                band_recv,
                net_settings_send,
                self.network_settings,
            );
            println!("swarm '{}' created ", name);
            let sender = swarm.sender.clone();
            let receiver = swarm.receiver.take();
            println!("Joined `{}` swarm", swarm.name);
            self.notify_networking(name.clone(), sender.clone(), band_send, net_settings_recv);
            println!("inserting swarm");
            self.swarms.insert(swarm_id, swarm);
            Ok((swarm_id, (sender, receiver.unwrap())))
        } else {
            Err("Could not connect to a swarm, all SwarmIDs taken".to_string())
        }
    }

    pub fn notify_networking(
        &mut self,
        swarm_name: String,
        sender: Sender<Request>,
        avail_bandwith_sender: Sender<u32>,
        network_settings_receiver: Receiver<(NetworkSettings, Option<NetworkSettings>)>,
    ) {
        // println!("About to send notification");
        let r = self.to_networking.send((
            swarm_name,
            sender,
            avail_bandwith_sender,
            network_settings_receiver,
        ));
        println!("notification sent: {:?}", r);
    }

    pub fn print_status(&self, id: &SwarmID) {
        if let Some(swarm) = self.swarms.get(id) {
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
