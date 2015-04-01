#![feature(plugin)]
#![feature(custom_attribute)]
#![plugin(humpty_dumpty)]
#![allow(unused_attributes)]
#![allow(unused_variables)]
#![allow(dead_code)]

#[drop_protect]
struct Foo;

impl Foo {
    fn something(self) -> Self {
        self
    }

    fn dropit(self) {
        // Should err
    }
}

// Should not warn, since we're not dropping anything
fn id<T>(x: T) -> T {
    x
}

// Should not warn
#[allow_drop="Foo"]
fn close(_: Foo) {

}

fn main() {
    let mut x = Foo;
    x = x;
    let y = id(x);
    let z = y.something();
    close(z);
}
