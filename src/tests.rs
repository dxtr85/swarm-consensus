use super::*;
use std::sync::mpsc::{channel, Receiver, Sender};

// fn make_neighbors(gnome_a: &mut Gnome, gnome_b: &mut Gnome) {
//     let (a_sender, a_receiver) = mpsc::channel();
//     let (b_sender, b_receiver) = mpsc::channel();
//     let a_as_neighbor = Neighbor::from_id_channel(gnome_a.id, a_receiver, b_sender);
//     let b_as_neighbor = Neighbor::from_id_channel(gnome_b.id, b_receiver, a_sender);
//     gnome_a.add_neighbor(b_as_neighbor);
//     gnome_b.add_neighbor(a_as_neighbor);
// }
// fn build_a_gnome() -> (Gnome, Sender<Request>, Receiver<Response>) {
//     let (s_req, r_req) = mpsc::channel();
//     let (s_res, r_res) = mpsc::channel();
//     (
//         Gnome::with_diameter(DEFAULT_SWARM_DIAMETER, s_res, r_req),
//         s_req,
//         r_res,
//     )
// }
fn build_a_neighbor(id: GnomeId) -> (Neighbor, Sender<Message>, Receiver<Message>) {
    let (s_req, r_req) = channel::<Message>();
    let (s_res, r_res) = channel::<Message>();
    (Neighbor::from_id_channel(id, r_req, s_res), s_req, r_res)
}

#[test]
fn gnome_message_exchange() {
    let mut manager = Manager::new();
    let left_id = gnome::GnomeId(1);
    let right_id = gnome::GnomeId(2);
    let (left, left_s, _r) = build_a_neighbor(left_id);
    let (right, right_s, _receiver_response) = build_a_neighbor(right_id);

    let neighbors: Vec<Neighbor> = vec![left, right];
    let (_req_sender, resp_receiver) =
        manager.join_a_swarm("test swarm".to_string(), Some(neighbors));
    println!("Joined `test swarm!`");

    let p1: Proposal = 1;
    let bp1 = Box::new([p1; 1024]);
    let left_awareness = Awareness::Aware(12, p1);
    let right_awareness = Awareness::Aware(12, p1);
    println!("Neighbors sent KeepAlive!");

    let _ = left_s.send(Message::KeepAlive(left_id, left_awareness));
    let _ = right_s.send(Message::KeepAlive(right_id, right_awareness));

    let left_awareness = Awareness::Aware(13, p1);
    let right_awareness = Awareness::Aware(13, p1);
    println!("Neighbors sent another KeepAlive!");

    let _ = left_s.send(Message::KeepAlive(left_id, left_awareness));
    let _ = right_s.send(Message::KeepAlive(right_id, right_awareness));

    let rcvd = resp_receiver.recv();

    assert!(rcvd.is_ok(), "User received invalid response!");
    let unwrapped = rcvd.unwrap();
    assert_eq!(
        unwrapped,
        Response::ApprovedProposal(bp1),
        "User received unexpected response!"
    );

    println!("User received: {:?}", unwrapped);
    manager.finish();
}

#[test]
fn exit_loop_on_request() {
    let mut manager = Manager::new();
    let (_req_sender, _resp_receiver) = manager.join_a_swarm("test swarm".to_string(), None);
    manager.finish();
}
