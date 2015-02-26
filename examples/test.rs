#![feature(plugin)]
#![plugin(humpty_dumpty)]

#![allow(unused)]

fn main() {
    let x = 1u8;
    let mut y = 2u8;
    let (a,b,c,d) = (false, false, true, true);
    y = x;
}