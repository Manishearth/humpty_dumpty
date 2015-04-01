#![feature(plugin)]
#![feature(custom_attribute)]
#![plugin(humpty_dumpty)]
#![allow(unused_attributes)]
#![allow(unused_variables)]
#![allow(dead_code)]

#[drop_protect]
struct Foo;

#[allow_drop="Foo"]
fn close(_: Foo) { }

fn main() {
    let x = Foo;
    close(x);
}
