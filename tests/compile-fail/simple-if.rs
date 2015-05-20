#![feature(plugin)]
#![feature(custom_attribute)]
#![plugin(humpty_dumpty)]
#![allow(unused_attributes)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![deny(dropped_linear)]

#[drop_protect]
struct Foo;

impl Foo {
    #[allow_drop="Foo"]
    fn close(self) { }
}

fn test1() {
    let f = Foo;
    if false {
        //~^ ERROR If branch is not linear
        f.close();
    }
}

fn test2() {
    let f = Foo; //~ ERROR dropped var
    if false {
        //~^ ERROR If branches are not linear
        f.close();
    } else {

    }
}

fn test3() {
    let f = Foo;
    if false {
        //~^ ERROR If branches are not linear
    } else {
        f.close();
    }
}

fn main() {}
