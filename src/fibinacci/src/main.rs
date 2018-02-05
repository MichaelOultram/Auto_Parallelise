#![feature(i128_type)]
#![feature(use_external_macros)]
#![feature(conservative_impl_trait)]

#![feature(plugin)]
#![plugin(auto_parallelise)]
extern crate num_cpus;

use std::time::Instant;

#[autoparallelise]
fn main() {
    let now = Instant::now();
    let fibs = fibinacci(40);
    println!("{:?}", fibs);

    let elapsed = now.elapsed();
    let sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1000_000_000.0);
    println!("Seconds: {}", sec);
}


#[autoparallelise]
fn fibinacci(n: u32) -> u128 {
    match n {
        0 | 1 => 1,
        _ => fibinacci(n - 1) + fibinacci(n - 2),
    }
}
