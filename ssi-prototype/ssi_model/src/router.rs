use std::thread;
use std::sync::mpsc;
use self::mpsc::{Sender, Receiver};
use rand;
use rand::Rng;
use machine::PacketData as PacketData;

pub struct Router {
    senders: Vec<Sender<Packet>>,
    receivers: Vec<Receiver<Packet>>,
    num_processes: u32,
}

pub struct Packet {
    pub to_id : usize,
    pub from_id : usize,
    pub data : PacketData,
}

impl Router {
    pub fn new(num_processes: u32) -> Self {
        Router {
            senders: vec![],
            receivers: vec![],
            num_processes: num_processes,
        }
    }

    pub fn add_machine(&mut self, id : usize) -> (Sender<Packet>, Receiver<Packet>) {
        // Create channels
        let (router_send, machine_receive) = mpsc::channel();
        let (machine_send, router_recieve) = mpsc::channel();

        // Allow a 2 way connection between router and machine
        self.senders.insert(id, router_send);
        self.receivers.insert(id, router_recieve);
        (machine_send, machine_receive)
    }

    pub fn start_router(mut self){
        thread::spawn(move || {
            let mut rng = rand::thread_rng();
            let mut processes_complete = 0;
            'main: loop {
                rng.shuffle(&mut self.receivers); // Makes network traffic order more unpredictable
                for receiver in self.receivers.iter() {
                    match receiver.try_recv() {
                        Ok(packet) => {
                            match packet.data {
                                // Increase the number of processes complete
                                PacketData::PROCCESS_DONE => {
                                    processes_complete += 1;
                                    println!("[router] {} processes completed", processes_complete);
                                    // Check to see if simulation is over
                                    if processes_complete >= self.num_processes {
                                        println!("[router] killing all machines");
                                        // Kill all machines
                                        for sender in &self.senders {
                                            sender.send(Packet{
                                                to_id: 0,
                                                from_id: 0,
                                                data: PacketData::TERMINATE,
                                            });
                                        }
                                        break 'main;
                                    }
                                }

                                // Forwad packet to machine
                                _ => {
                                    let (from, to) = (packet.from_id, packet.to_id);
                                    // Find the sender channel and forward the packet
                                    let ref send_to = *self.senders.get(packet.to_id).unwrap();
                                    match send_to.send(packet) {
                                        Ok(_) => println!("[router] {} -> {}", from, to),
                                        Err(e) => panic!(e),
                                    };
                                }
                            }
                        },
                        Err(_) => {},// Ignore if no packet received
                    }
                }
            }
        });
    }
}
