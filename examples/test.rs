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
    {
        let mut w = Baz(1);
        let z = Bar;
        w = Bar;
        let properly_dropped = Bar;
        not_allowed(w);
        allowed(z);
        drop_properly(properly_dropped);
        // should error here about w/z not being dropped properly
    }

    {
        let w = Bar;
        let properly_dropped = Bar;
        let z = Bar;

        w.not_allowed();
        properly_dropped.proper_drop();
        z.allowed();
    }
}

use Foo::*;

#[drop_protection]
enum Foo {
    Bar, Baz(u8)
}

impl Foo {
    fn not_allowed(self) {

    }

    #[allowed_on_protected]
    fn allowed(self) {

    }

    #[allowed_drop]
    fn proper_drop(self) {

    }
}


fn not_allowed(_: Foo) {

}

#[allowed_on_protected]
fn allowed(_: Foo) {

}

#[allowed_drop]
fn drop_properly(_: Foo) {

}
