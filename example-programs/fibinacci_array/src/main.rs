#![feature(i128_type)]
#![feature(use_external_macros)]
#![feature(conservative_impl_trait)]

//#![feature(plugin)]
//#![plugin(auto_parallelize)]
extern crate num_cpus;
#[macro_use]
extern crate lazy_static;

use std::time::Instant;

mod pool;
use std::sync::mpsc::Receiver;
use pool::ThreadPool;

//#[auto_parallelize]
fn main() {
    let threadpool = ThreadPool::new(num_cpus::get() - 1);
    let now = Instant::now();

    println!("num_cpus: {}", num_cpus::get());

    let fibs = fibinacci_par(threadpool.clone(), 20).recv().unwrap();
    //let fibs = fibinacci(10);
    println!("{:?}", fibs);

    let elapsed = now.elapsed();
    let sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1000_000_000.0);
    println!("Seconds: {}", sec);
}

//#[auto_parallelize]
fn fibinacci_array(length: usize) -> Vec<u128> {
    let mut output = vec![1,1];
    for i in 2..length {
        let next = {
            let a = output.get(i-1).unwrap();
            let b = output.get(i-2).unwrap();
            a+b
        };
        output.push(next);
    }
    output
}

//#[auto_parallelize]
fn fibinacci(n: u32) -> u128 {
    match n {
        0 | 1 => 1,
        _ => fibinacci(n - 1) + fibinacci(n - 2),
    }
}

//#[auto_parallelize]
fn fibinacci_par(threadpool: ThreadPool, n: u32) -> Receiver<u128> {
    let pool = threadpool.clone();
    threadpool.task(move || {
        match n {
            0 | 1 => 1,
            _ => {
                let a = fibinacci_par(pool.clone(), n - 1);
                let b = fibinacci_par(pool.clone(), n - 2);
                a.recv().unwrap() + b.recv().unwrap()
            },
        }
    })
}
