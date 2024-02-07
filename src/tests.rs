use super::*;
use std::sync::mpsc;

fn make_neighbors(gnome_a: &mut Gnome, gnome_b: &mut Gnome) {
    let (a_sender, a_receiver) = mpsc::channel();
    let (b_sender, b_receiver) = mpsc::channel();
    let a_as_neighbor = Neighbor::from_id_channel(gnome_a.id, a_receiver, b_sender);
    let b_as_neighbor = Neighbor::from_id_channel(gnome_b.id, b_receiver, a_sender);
    gnome_a.add_neighbor(b_as_neighbor);
    gnome_b.add_neighbor(a_as_neighbor);
}
fn build_a_gnome()->(Gnome, Sender<Request>, Receiver<Response>){
    let (s_req, r_req)=mpsc::channel();
    let (s_res, r_res)=mpsc::channel();
    (Gnome::with_diameter(DEFAULT_SWARM_DIAMETER,s_res ,r_req ), s_req, r_res)
}
pub fn three_gnomes_swarm() -> bool {
    let (mut left, _s, _r) = build_a_gnome();
    let (mut middle, _s2, _r2) = build_a_gnome();
    let (mut right, _sender_request, _receiver_response) = build_a_gnome();
    make_neighbors(&mut left, &mut middle);
    make_neighbors(&mut right, &mut middle);

    // middle.send(Message::KeepAlive(middle.id, middle.awareness));
    // right.try_recv();
    // left.try_recv();
    let p1: Proposal = 1;
    // let p2: Proposal = 2;
    left.set_awareness(Awareness::Aware(0, p1));
    // right.set_awareness(Awareness::Aware(0, p1));
    // middle.send(Message::Proposal(middle.id, middle.awareness, 443556));
    // right.try_recv();
    // left.try_recv();
    left.send_all(Message::KeepAlive(left.id, left.awareness));
    right.send_all(Message::KeepAlive(right.id, right.awareness));
    middle.do_your_job();
    true
}
#[test]
fn gnome_message_exchange() {
    let result = three_gnomes_swarm();
    assert!(result);
}
