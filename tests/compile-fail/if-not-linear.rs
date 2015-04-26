#![feature(plugin)]
#![feature(custom_attribute)]
#![plugin(humpty_dumpty)]
#![allow(unused_attributes)]
#![allow(unused_variables)]
#![allow(dead_code)]

#[drop_protect]
struct Foo;

impl Foo {
    #[allow_drop="Foo"]
    fn close(self) { }
}

fn test1() {
    let f = Foo; //~ ERROR dropped var
    if false { return } else { f.close(); }
    //~^ ERROR Branch arms are not linear
}

fn test2() {
    let f = Foo;
    if true { f.close(); } else { return }
    //~^ ERROR Branch arms are not linear
}

fn test3() {
    let f = Foo; //~ ERROR dropped var
    if true { f.close(); }
    //~^ ERROR If branch is not linear
}

fn main() {
    test1();
    test2();
}
