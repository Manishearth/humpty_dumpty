#![feature(plugin)]
#![feature(custom_attribute)]
#![plugin(humpty_dumpty)]

#![allow(unused)]

#![warn(drop_violation)]

fn main() {
    // These should not warn
    let x = 1u8;
    let mut y = 2u8;
    y = x;
    let mut w = Baz(1);
    let z = Bar;
    w = Bar;
    let properly_dropped = Bar;
    not_allowed(w);
    allowed(z);
    drop_properly(properly_dropped);
    // should error here about w/z not being dropped properly
}

use Foo::*;

#[drop_protection]
enum Foo {
    Bar, Baz(u8)
}


fn not_allowed(_: Foo) {

}

#[allowed_on_protected]
fn allowed(_: Foo) {
    
}

#[allowed_drop]
fn drop_properly(_: Foo) {
    
}