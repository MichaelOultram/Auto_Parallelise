#![feature(i128_type)]
#![feature(use_external_macros)]
#![feature(conservative_impl_trait)]

extern crate num_cpus;

extern crate auto_parallelise;
use auto_parallelise::noqueue_threadpool::NoQueueThreadPool;
use std::sync::mpsc::Receiver;

use std::time::Instant;

//#[auto_parallelize]
fn main() {
    let threadpool = NoQueueThreadPool::new(num_cpus::get() - 1);
    let now = Instant::now();
    let n = 20;

    let fibs_task = fibinacci_par(threadpool.clone(), n);
    println!("num_cpus: {}", num_cpus::get());
    let fibs = threadpool.result(fibs_task);
    println!("{:?}", fibs);

    let elapsed = now.elapsed();
    let sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1000_000_000.0);
    println!("Seconds: {}", sec);
}


fn fibinacci_par(threadpool: NoQueueThreadPool, n: u32) -> Receiver<u128> {
    let pool = threadpool.clone();
    threadpool.task_block(move || {
        match n {
            0 | 1 => 1,
            _ => {
                let a = fibinacci_par(pool.clone(), n - 1);
                let b = fibinacci_par(pool.clone(), n - 2);
                pool.result(a) + pool.result(b)
            },
        }
    })
}
