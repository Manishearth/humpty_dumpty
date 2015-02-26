#![feature(plugin)]
#![feature(custom_attribute)]
#![plugin(humpty_dumpty)]

#![allow(unused)]

fn main() {
    // These should not warn
    let x = 1u8;
    let mut y = 2u8;
    let (a,b,c,d) = (false, false, true, true);
    y = x;
    let mut w = Baz(1);
    let z = Bar;
    w = Bar;
}

use Foo::*;

#[drop_protection]
enum Foo {
    Bar, Baz(u8)
}
