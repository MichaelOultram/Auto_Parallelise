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
    vector_clock: Vec<u32>,
}

pub struct Packet {
    pub to_id : usize,
    pub vector_clock : Vec<u32>,
    pub data : PacketData,
}

impl Router {
    pub fn new(num_processes: u32) -> Self {
        Router {
            senders: vec![],
            receivers: vec![],
            vector_clock: vec![],
            num_processes: num_processes,
        }
    }

    pub fn add_machine(&mut self, id : usize) -> (Sender<Packet>, Receiver<Packet>) {
        // Create channels
        let (router_send, machine_receive) = mpsc::channel();
        let (machine_send, router_recieve) = mpsc::channel();

        // Add a space in the vector clock
        self.vector_clock.push(0);

        // Allow a 2 way connection between router and machine
        self.senders.insert(id, router_send);
        self.receivers.insert(id, router_recieve);
        (machine_send, machine_receive)
    }

    pub fn start_router(mut self) -> Box<Fn() -> ()> {
        let (external_send, router_receive) = mpsc::channel();
        self.receivers.push(router_receive);
        let num_machines = self.senders.len();
        thread::spawn(move || {
            let mut rng = rand::thread_rng();
            let mut processes_complete = 0;
            'main: loop {
                rng.shuffle(&mut self.receivers); // Makes network traffic order more unpredictable
                for receiver in self.receivers.iter() {
                    match receiver.try_recv() {
                        Ok(packet) => {
                            combine_vector_clocks(&mut self.vector_clock, &packet.vector_clock);
                            //println!("[router] {:?}", self.vector_clock);
                            match packet.data {
                                // Increase the number of processes complete
                                PacketData::ProcessDone => {
                                    processes_complete += 1;
                                    println!("[router] {} processes completed", processes_complete);
                                    // Check to see if simulation is over
                                    if processes_complete >= self.num_processes {
                                        self.terminate();
                                        break 'main;
                                    }
                                }

                                // External terminate packet
                                PacketData::Terminate => {
                                    self.terminate();
                                    break 'main;
                                }

                                // Forwad packet to machine
                                _ => {
                                    // Find the sender channel and forward the packet
                                    let ref send_to = *self.senders.get(packet.to_id).unwrap();
                                    match send_to.send(packet) {
                                        Ok(_) => {},
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

        Box::new(move || {
            external_send.send(Packet {
                to_id: 0,
                vector_clock: vec![0;num_machines as usize], // Unable to use actual vector clock
                data: PacketData::Terminate,
            });
        })
    }

    fn terminate(&self) {
        println!("[router] killing all machines");
        // Kill all machines
        for sender in &self.senders {
            sender.send(Packet{
                to_id: 0,
                vector_clock: self.vector_clock.clone(),
                data: PacketData::Terminate,
            });
        }
    }
}

pub fn combine_vector_clocks(original: &mut Vec<u32>, new: &Vec<u32>) {
    let length = original.len();
    assert_eq!(length, new.len());

    for i in 0..length {
        if new[i] > original[i] {
            original[i] = new[i];
        }
    }
}
