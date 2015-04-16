#![feature(plugin)]
#![feature(custom_attribute)]
#![plugin(humpty_dumpty)]
#![allow(unused_attributes)]
#![allow(unused_variables)]
#![allow(dead_code)]

#[drop_protect]
struct Foo;

unsafe trait Closeable {
    fn close(self);
}

unsafe impl Closeable for Vec<Foo> {
    #[allow_drop="collections::vec::Vec<Foo>"]
    fn close(self) { }
}


fn main() {
    let v: Vec<Foo> = Vec::new();
    v.close();
}
