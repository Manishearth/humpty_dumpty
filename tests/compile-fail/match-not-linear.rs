#![feature(plugin)]
#![feature(custom_attribute)]
#![plugin(humpty_dumpty)]
#![allow(unused_attributes)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![deny(dropped_linear)]

fn main() {}

#[drop_protect]
struct Foo;

impl Foo {
    fn new() -> Foo { Foo }
    fn one_or_the_other(self, b: bool) -> Result<Foo, Foo> {
        if b { Ok(self) } else { Err(self) }
    }
    #[allow_drop="Foo"]
    fn drop(self) {}
}

fn one() {
    let foo = Foo::new();
    match foo.one_or_the_other(true) {
        //~^ ERROR Match arms are not linear
        Ok(foo) => foo.drop(),
        Err(foo) => {} //~ ERROR dropped var
    }
}

fn the_other() {
    let foo = Foo::new();
    match foo.one_or_the_other(false) {
        //~^ ERROR Match arms are not linear
        Ok(foo) => {},
        Err(foo) => foo.drop()
    }
}
