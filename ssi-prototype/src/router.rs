use std::thread;
use std::sync::{mpsc};
use rand;
use rand::Rng;
use machine::PacketData as PacketData;

pub type Router = (Vec<mpsc::Sender<Packet>>, Vec<mpsc::Receiver<Packet>>);

pub struct Packet {
    pub to_id : usize,
    pub from_id : usize,
    pub data : PacketData,
}

pub fn add_machine(router : &mut Router, id : usize)
    -> (mpsc::Sender<Packet>, mpsc::Receiver<Packet>) {

    let (ref mut senders, ref mut receivers) = *router;
    // Create channels
    let (router_send, machine_receive) = mpsc::channel();
    let (machine_send, router_recieve) = mpsc::channel();

    // Allow a 2 way connection between router and machine
    senders.insert(id, router_send);
    receivers.insert(id, router_recieve);
    (machine_send, machine_receive)
}

pub fn start_router(router : Router){
    thread::spawn(move || {
        let (senders, mut receivers) = router;
        let mut rng = rand::thread_rng();
        loop {
            rng.shuffle(&mut receivers); // Makes network traffic order more unpredictable
            for receiver in receivers.iter() {
                match receiver.try_recv() {
                    Ok(packet) => {
                        let (from, to) = (packet.from_id, packet.to_id);
                        // Find the sender channel and forward the packet
                        let ref send_to = *senders.get(packet.to_id).unwrap();
                        match send_to.send(packet) {
                            Ok(_) => println!("[router] {} -> {}", from, to),
                            Err(e) => panic!(e),
                        };
                    },
                    Err(_) => {},// Ignore if no packet received
                }
            }
        }
    });
}
