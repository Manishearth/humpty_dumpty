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

#[test]
fn if_return() {
    let f = Foo;

    if true {
        f.close();
        return;
    }

    f.close();
}

#[test]
fn if_return_else() {
    let f = Foo;

    if true {
        f.close();
        return;
    } else {
        // pass
    }

    f.close();
}

#[test]
fn if_return_else_return() {
    let f = Foo;

    if true {
        f.close();
        return;
    } else {
        f.close();
        return;
    }
}

#[test]
fn if_else_return() {
    let f = Foo;

    if true {
        // pass
    } else {
        f.close();
        return;
    }

    f.close();
}

#[test]
fn if_else() {
    let f = Foo;
    if true {
        // pass
    } else {
        // pass
    }

    f.close();
    return;
}
