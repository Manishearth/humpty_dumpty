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
fn loop_continue1() {
    let foo = Foo;

    loop {
        if true {
            continue;
        } else {
            foo.close();
            break;
        }
    }
}


#[test]
fn loop_continue2() {
    let foo = Foo;

    loop {
        if true {
            continue;
        } else {
            foo.close();
        }
        break;
    }
}

#[test]
fn loop_continue3() {
    let foo = Foo;

    loop {
        if true {
            continue;
        } else {
        }
        foo.close();
        break;
    }
}

#[test]
fn loop_continue4() {
    let foo = Foo;

    loop {
        if true {
            continue;
        }
        foo.close();
        break;
    }
}
