use std;
use std::thread;
use std::sync::mpsc::{Receiver, channel};

pub struct Worker<T>
where T: Send + 'static {
    pub receiver : Option<Receiver<T>>,
    pub working : bool,
}

impl<T> Worker<T>
where T: std::marker::Send {

    // Starts a worker
    pub fn start<F>(f : F) -> Self
    where F: (FnOnce() -> T) + Send + 'static {
        // Create a channel
        let (send, recv) = channel();
        thread::spawn(move || {
            let ret = f();
            match send.send(ret) {
                Ok(_) => {}
                Err(e) => panic!(e),
            }
        });
        Worker{
            receiver: Some(recv),
            working: true,
        }
    }

    pub fn dummy() -> Self {
        Worker {
            receiver: None,
            working: false,
        }
    }

    // WARNING: Will only return the result once
    pub fn result(&mut self) -> Option<T> {
        assert!(self.working);
        match self.receiver {
            None => None,
            Some(ref receiver) => match receiver.try_recv() {
                Ok(r) => {
                    self.working = false;
                    Some(r)
                },
                Err(_) => None,
            },
        }

    }
}
