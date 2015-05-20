#![feature(plugin)]
#![feature(custom_attribute)]
#![plugin(humpty_dumpty)]
#![allow(unused_attributes)]
#![allow(unused_variables)]
#![allow(dead_code)]

#![deny(dropped_linear)]

#[drop_protect]
struct Foo;

fn main() {
    let foo = Foo; //~ ERROR dropped var
}
