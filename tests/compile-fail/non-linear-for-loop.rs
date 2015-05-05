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
    let f = vec!(Foo);
    for i in f {
        //~^ ERROR Non-linear for loop
        if true {
            break;
        }
        i.close();
    }
}

fn main() {}
