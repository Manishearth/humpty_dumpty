#![feature(plugin)]
#![plugin(humpty_dumpty)]

fn main() {
    let x = 1u8;
    let mut y = 2u8;
    y = x;
}