#![feature(plugin)]
#![feature(custom_attribute)]
#![plugin(humpty_dumpty)]
#![allow(unused_attributes)]
#![allow(unused_variables)]
#![allow(dead_code)]

#![deny(dropped_linear)]

#[drop_protect]
struct Foo;

#[allow_drop="Foo"]
fn close(_: Foo) { }

fn main() {
    loop {
        let y = Foo; //~ ERROR dropped var
    }
}

fn test2() {
    let x = Foo;
    loop {
        match 2 {
            1 => {
                close(x);
                break;
            }
            2 => {
                break; //~ERROR Non-linear break
            }
            _ => {

            }
        }
    }
}
