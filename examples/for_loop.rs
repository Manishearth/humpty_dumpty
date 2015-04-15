#![feature(plugin)]
#![feature(custom_attribute)]
#![plugin(humpty_dumpty)]
#![allow(unused_attributes)]
#![allow(unused_variables)]
#![allow(dead_code)]

#[drop_protect]
struct Foo;

#[allow_drop="Foo"]
fn close(x: Foo) { }

fn main() {
    let v = vec!(Foo, Foo, Foo);

    for x in v {
        close(x)
    };
}
