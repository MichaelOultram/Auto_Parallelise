use std::sync::{Arc,Mutex};
use std::thread;
use std::sync::mpsc::{channel, Receiver, Sender};

// Only one thread can access this at once
struct NoQueueThreadPoolInner {
    perm_threads: Vec<TaskRunner>, // Normal ThreadPool threads
    temp_threads: Vec<TaskRunner>, // Threads that are blocked could run another task
}

// Each thread has access to this
#[derive(Clone)]
pub struct NoQueueThreadPool {
    pool: Arc<Mutex<NoQueueThreadPoolInner>>,
}

//static THREADPOOL: Option<NoQueueThreadPool> = None;

struct TaskRunner {
    sender: Sender<ThreadMsg>,
    receiver: Receiver<ThreadStatus>,
    status: ThreadStatus,
}

// Consumer messages
#[derive(PartialEq, Debug, Clone, Copy)]
enum ThreadStatus {
    Waiting,  // No task, and could exit itself down
    Reserved, // No task yet, assumed a new task very soon, and will not exit unless told to.
    Running,  // Running a task
    Exited,   // Thread has exited
}

// Producer messages
enum ThreadMsg {
    ReserveTask,                // Waiting             -> ReserveTask
    NewTask(Box<TaskFunction>), // Waiting/ReserveTask -> Running
    Shutdown,                   // Waiting/ReserveTask -> Exited
}

// From: https://github.com/reem/rust-scoped-pool/blob/master/src/lib.rs
trait TaskFunction: Send {
    fn run(self: Box<Self>);
}
impl<F: FnOnce() + Send> TaskFunction for F {
    fn run(self: Box<Self>) { (*self)() }
}

impl NoQueueThreadPool {
    pub fn new(num_threads: usize) -> Self {
        // TODO: Make it so only one true NoQueueThreadPool exists
        let pool = Arc::new(Mutex::new(NoQueueThreadPoolInner {
            perm_threads: vec![],
            temp_threads: vec![],
        }));
        let NoQueueThreadPool = NoQueueThreadPool {
            pool: pool
        };
        NoQueueThreadPool.add_perm_threads(num_threads);
        NoQueueThreadPool
    }

    pub fn add_perm_threads(&self, num_threads: usize) {
        let mut pool = self.pool.lock().unwrap();
        for thread_id in 0..num_threads {
            // Create two-way channel
            let (sender, thread_receiver) = channel::<ThreadMsg>();
            let (thread_sender, receiver) = channel::<ThreadStatus>();

            // Spawn the permanent thread
            thread::spawn(move || NoQueueThreadPool::perm_thread_worker(thread_id, thread_sender, thread_receiver));
            receiver.recv(); // Ignore first Waiting message

            // Add this TaskRunner to the list of permanent threads
            pool.perm_threads.push(TaskRunner{
                sender: sender,
                receiver: receiver,
                status: ThreadStatus::Waiting,
            });
        }
    }

    fn perm_thread_worker(thread_id: usize, thread_sender: Sender<ThreadStatus>, thread_receiver: Receiver<ThreadMsg>) {
        let mut is_waiting = true;
        loop {
            // Request a new task
            if is_waiting {
                //eprintln!("[perm_thread_{}] Waiting", thread_id);
                thread_sender.send(ThreadStatus::Waiting);
            }

            // Wait for a response
            match thread_receiver.recv() {
                Ok(ThreadMsg::Shutdown) => break,
                Ok(ThreadMsg::ReserveTask) => {
                    // A perm_thread will never try to shut itself down
                    //eprintln!("[perm_thread_{}] Reserved", thread_id);
                    thread_sender.send(ThreadStatus::Reserved);
                    is_waiting = false;
                },
                Ok(ThreadMsg::NewTask(task)) => {
                    //eprintln!("[perm_thread_{}] Running", thread_id);
                    thread_sender.send(ThreadStatus::Running);
                    task.run();
                    is_waiting = true;
                },
                Err(_) => {}, //eprintln!("perm_thread_worker errored on recv()"),
            }
        }
        //eprintln!("[perm_thread_{}] Exited", thread_id);
        thread_sender.send(ThreadStatus::Exited);
    }

    pub fn clear_perm_threads(&self) {
        let mut pool = self.pool.lock().unwrap();
        for ref thread in &pool.perm_threads {
            thread.sender.send(ThreadMsg::Shutdown);
        }
        //TODO: Wait for threads to confirm exit?
        pool.perm_threads.clear();
    }

    fn receive_threads(pool: &mut NoQueueThreadPoolInner) {
        let mut delete_list: Vec<usize> = vec![];
        for thread_id in 0..pool.perm_threads.len() {
            let thread = &mut pool.perm_threads[thread_id];
            // Only check if running threads have finished
            if thread.status == ThreadStatus::Running {
                if let Ok(msg) = thread.receiver.try_recv() {
                    match msg {
                        ThreadStatus::Waiting => thread.status = msg,
                        ThreadStatus::Exited => {
                            thread.status = msg;
                            //delete_list.push(thread_id);
                        }
                        _ => panic!("Running thread gave wrong thing"),
                    }
                }
            }
        }
        let mut counter = 0;
        for delete_id in &delete_list {
            pool.perm_threads.remove(delete_id - counter);
            counter += 1;
        }

        //TODO: Repeated code for a different list!
        delete_list.clear();
        for thread_id in 0..pool.temp_threads.len() {
            let thread = &mut pool.temp_threads[thread_id];
            // Only check if running threads have finished
            if thread.status == ThreadStatus::Running {
                if let Ok(msg) = thread.receiver.try_recv() {
                    match msg {
                        ThreadStatus::Waiting => thread.status = msg,
                        ThreadStatus::Exited => {
                            thread.status = msg;
                            //delete_list.push(thread_id);
                        }
                        _ => panic!("Running thread gave wrong thing"),
                    }
                }
            }
        }
        let mut counter = 0;
        for delete_id in &delete_list {
            pool.temp_threads.remove(delete_id - counter);
            counter += 1;
        }
    }

    pub fn num_threads_free(&self) -> u32 {
        //self.receive_threads();
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

    pub fn print(&self) {
        let pool = self.pool.lock().unwrap();
        let mut perm_threads = vec![];
        let mut temp_threads = vec![];
        for thread in &pool.perm_threads {
            perm_threads.push(thread.status);
        }
        for thread in &pool.temp_threads {
            temp_threads.push(thread.status);
        }
        eprintln!("perm_threads: {:?}\ntemp_threads: {:?}", perm_threads, temp_threads);
    }

    pub fn task_block<F, T>(&self, f: F) -> Receiver<T>
    where F: FnOnce() -> T, F: Send + 'static, T: Send + 'static
    {
        // Create a task for the function
        let (sender, receiver) = channel::<T>();
        let task_function = Box::new(move || {
            let result = f();
            sender.send(result);
        });

        let mut pool = self.pool.lock().unwrap();

        // Check on Running threads to see if they are finished
        NoQueueThreadPool::receive_threads(&mut pool);

        // Check to see if any threads are waiting
        // true = permanent, false = temporary
        let mut chosen_thread: Option<(bool,usize)> = None;
        for thread_id in 0..pool.perm_threads.len() {
            let thread = &mut pool.perm_threads[thread_id];
            if thread.status == ThreadStatus::Waiting {
                chosen_thread = Some((true, thread_id));
                break;
            }
        }
        if chosen_thread == None {
            // No permant threads available, try temporary threads
            for thread_id in 0..pool.temp_threads.len() {
                let thread = &mut pool.temp_threads[thread_id];
                if thread.status == ThreadStatus::Waiting {
                    chosen_thread = Some((false, thread_id));
                    break;
                }
            }
        }


        // Get the chosen thread, if there is one
        let mut delete_thread = false;
        if let Some((permanent_thread, thread_id)) = chosen_thread {
            // Assign to the waiting thread
            let thread = if permanent_thread {
                &mut pool.perm_threads[thread_id]
            } else {
                &mut pool.temp_threads[thread_id]
            };

            // Try to reserve the thread
            thread.sender.send(ThreadMsg::ReserveTask); // TODO: May fail, isn't explicitly checked
            let reserved_msg = thread.receiver.recv();
            if let Ok(ThreadStatus::Reserved) = reserved_msg {
                // Thread successfully reserved, send the function
                thread.status = ThreadStatus::Reserved;
                thread.sender.send(ThreadMsg::NewTask(task_function));

                // Make sure task is Running
                let recv = thread.receiver.recv();
                if let Ok(ThreadStatus::Running) = recv {
                    thread.status = ThreadStatus::Running;
                    // Task is running in another thread, so return the receiver.
                    return receiver;
                } else {
                    panic!("Reserved thread did not accept NewTask: {:?}", recv.unwrap());
                }
            } else if let Ok(ThreadStatus::Exited) = reserved_msg {
                // Could not reserve thread, going to have to run the task in current thread
                // But first, cleanup the thread that rejected us
                thread.status = ThreadStatus::Exited;
                delete_thread = true;
            } else if let Err(_) = reserved_msg {
                panic!("Channel closed without exiting...");
            } else {
                // TODO: Shouldn't receive these messages, cleanup
                panic!("Waiting thread unexpectedly didn't return Reserved or Exited");
            }
        }

        if delete_thread {
            // Delete chosen_thread
            if let Some((permanent_thread, thread_id)) = chosen_thread {
                if permanent_thread {
                    pool.perm_threads.remove(thread_id);
                } else {
                    pool.temp_threads.remove(thread_id);
                }
            }
        }

        // Either no Thread was available or it did not accept
        // Run the task in the current thread
        drop(pool); // Unlocking mutex
        task_function();

        // Return the receiver, so the result can be requested
        receiver
    }

    // Returns result of a Task, but may run other tasks whilst waiting
    pub fn result<T>(&self, r: Receiver<T>) -> T {
        // Check to see if Task has terminated
        if let Ok(result) = r.try_recv() {
            return result;
        }

        // Create two-way channel
        let (sender, thread_receiver) = channel::<ThreadMsg>();
        let (thread_sender, receiver) = channel::<ThreadStatus>();

        // Add ourselves as a temp_thread
        {
            // Add this TaskRunner to the list of temporary threads
            let mut pool = self.pool.lock().unwrap();
            pool.temp_threads.push(TaskRunner{
                sender: sender,
                receiver: receiver,
                status: ThreadStatus::Waiting,
            });
        }

        // Whilst waiting, request and execute Tasks
        loop {
            // Check to see if we have our result
            if let Ok(result) = r.try_recv() {
                // Alert NoQueueThreadPool that we are exiting
                thread_sender.send(ThreadStatus::Exited);
                return result;
            }

            // Check to see if we have a task to execute
            if let Ok(msg) = thread_receiver.try_recv() {
                if let ThreadMsg::ReserveTask = msg {
                    // Accept the Reserve request, and block wait for the NewTask
                    thread_sender.send(ThreadStatus::Reserved);
                    if let Ok(ThreadMsg::NewTask(task)) = thread_receiver.recv() {
                        thread_sender.send(ThreadStatus::Running);
                        task.run();
                        thread_sender.send(ThreadStatus::Waiting);
                    } else {
                        panic!("temporary thread didn't receive a NewTask")
                    }
                } else {
                    panic!("temporary thread didn't receive following the protocol")
                }

            }
        }
    }

}
