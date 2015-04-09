#![feature(plugin)]
#![feature(custom_attribute)]
#![plugin(humpty_dumpty)]
#![allow(unused_attributes)]
#![allow(unused_variables)]
#![allow(dead_code)]

#[drop_protect]
struct Foo;

#[drop_protect]
struct Bar;

impl Foo {
    #[allow_drop="Foo"]
    fn close(self) { }
}

impl Bar {
    #[allow_drop="Bar"]
    fn close(self) { }
}

fn main() {
    let x: Result<Foo, Bar> = Ok(Foo);
    match x {
        Ok(y) => y.close(),
        Err(y) => y.close(),
    }
}
