use std::sync::{Arc,Mutex};
use std::thread::{self, Thread};
use std::sync::mpsc::{channel, Receiver, Sender};

// Only one thread can access this at once
struct ThreadPoolInner {
    perm_threads: Vec<TaskRunner>, // Normal threadpool threads
    temp_threads: Vec<TaskRunner>, // Threads that are blocked could run another task
}

// Each thread has access to this
#[derive(Clone)]
pub struct ThreadPool {
    pool: Arc<Mutex<ThreadPoolInner>>,
}

//static THREADPOOL: Option<ThreadPool> = None;

struct TaskRunner {
    sender: Sender<ToThreadMsg>,
    receiver: Receiver<FromThreadMsg>,
    status: ThreadStatus,
}

#[derive(PartialEq)]
enum ThreadStatus {
    Waiting, Running
}

enum ToThreadMsg {
    NewTask(Box<TaskFunction>),
    Shutdown,
}

#[derive(PartialEq)]
enum FromThreadMsg {
    NewTask,
    Exited,
}

// From: https://github.com/reem/rust-scoped-pool/blob/master/src/lib.rs
trait TaskFunction: Send {
    fn run(self: Box<Self>);
}
impl<F: FnOnce() + Send> TaskFunction for F {
    fn run(self: Box<Self>) { (*self)() }
}

impl ThreadPool {
    pub fn new(num_threads: usize) -> Self {
        // TODO: Make it so only one true threadpool exists
        let pool = Arc::new(Mutex::new(ThreadPoolInner {
            perm_threads: vec![],
            temp_threads: vec![],
        }));
        let mut threadpool = ThreadPool {
            pool: pool
        };
        threadpool.add_perm_threads(num_threads);
        threadpool
    }

    pub fn add_perm_threads(&self, num_threads: usize) {
        let mut pool = self.pool.lock().unwrap();
        for _ in 0..num_threads {
            // Create two-way channel
            let (sender, thread_receiver) = channel::<ToThreadMsg>();
            let (thread_sender, receiver) = channel::<FromThreadMsg>();

            // Spawn the permanent thread
            thread::spawn(move || ThreadPool::perm_thread_worker(thread_sender, thread_receiver));

            // Add this TaskRunner to the list of permanent threads
            pool.perm_threads.push(TaskRunner{
                sender: sender,
                receiver: receiver,
                status: ThreadStatus::Waiting,
            });
        }
    }

    fn perm_thread_worker(thread_sender: Sender<FromThreadMsg>, thread_receiver: Receiver<ToThreadMsg>) {
        loop {
            // Request a new task
            thread_sender.send(FromThreadMsg::NewTask);

            // Wait for a response
            if let Ok(msg) = thread_receiver.recv() {
                match msg {
                    ToThreadMsg::Shutdown => break,
                    ToThreadMsg::NewTask(task) => task.run(),
                }
            }
        }
        thread_sender.send(FromThreadMsg::Exited);
    }

    pub fn clear_perm_threads(&self) {
        let mut pool = self.pool.lock().unwrap();
        for ref thread in &pool.perm_threads {
            thread.sender.send(ToThreadMsg::Shutdown);
        }
        pool.perm_threads.clear();
    }

    pub fn num_threads_free(&self) -> u32 {
        self.num_perm_threads_free() + self.num_temp_threads_free()
    }

    pub fn num_perm_threads_free(&self) -> u32 {
        let pool = self.pool.lock().unwrap();
        let mut counter: u32 = 0;
        for thread in &pool.perm_threads {
            if thread.status == ThreadStatus::Waiting {
                counter += 1;
            }
        }
        counter
    }

    pub fn num_temp_threads_free(&self) -> u32 {
        let pool = self.pool.lock().unwrap();
        let mut counter: u32 = 0;
        for thread in &pool.temp_threads {
            if thread.status == ThreadStatus::Waiting {
                counter += 1;
            }
        }
        counter
    }

    pub fn task<F, T>(&self, f: F) -> Receiver<T>
    where F: FnOnce() -> T, F: Send + 'static, T: Send + 'static
    {
        // Create a task for the function
        let (sender, receiver) = channel::<T>();
        let task_function = Box::new(move || {
            let result = f();
            sender.send(result);
        });

        // Check to see if any threads are waiting
        let mut chosen_thread: Option<usize> = None;
        {
            let mut pool = self.pool.lock().unwrap();
            for thread_id in 0..pool.perm_threads.len() {
                let thread = &mut pool.perm_threads[thread_id];
                if thread.status == ThreadStatus::Waiting {
                    chosen_thread = Some(thread_id);
                    break;
                }
            }
            // TODO: Include temp_threads
        }

        if let Some(thread_id) = chosen_thread {
            // Assign to the waiting thread
            let mut pool = self.pool.lock().unwrap();
            let thread = &mut pool.perm_threads[thread_id];
            thread.sender.send(ToThreadMsg::NewTask(task_function));
            thread.status = ThreadStatus::Running;
        } else {
            // Run the task in the current thread
            task_function();
        }

        // Return the receiver, so the result can be requested
        receiver
    }

}
