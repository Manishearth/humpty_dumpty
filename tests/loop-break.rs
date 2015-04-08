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
fn loop_break() {
    let foo = Foo;

    loop {
        foo.close();
        break;
    }
}

#[test]
fn loop_if_break() {
    let foo = Foo;

    loop {
        if true {
            foo.close();
            break;
        }

        foo.close();
        break;
    }
}

#[test]
fn loop_match_break_else_break() {
    let foo = Foo;

    loop {
        match true {
            true => {
                foo.close();
                return;
            }
            _ => {
                break;
            }
        }
    }
    foo.close();
}

#[test]
fn loop_if_break_else() {
    let foo = Foo;

    loop {
        if true {
            foo.close();
            break;
        } else {
            foo.close();
        }
        break;
    }
}

#[test]
fn loop_if_break_else_break() {
    let foo = Foo;

    loop {
        if true {
            foo.close();
            break;
        } else {
            foo.close();
            break;
        }
    }
}
