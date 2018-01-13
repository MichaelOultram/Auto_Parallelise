#![feature(plugin)]
#![plugin(auto_parallelise)]

#[autoparallelise]
fn main() {
    let mut a = 4;
    let mut b = 3;
    a += 1;
    b += 1;
}
