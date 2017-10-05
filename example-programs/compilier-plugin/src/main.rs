#![feature(plugin)]
#![plugin(sample_plugin)]

fn main() {
    assert_eq!(rn!(MMXV), 2015);
}
