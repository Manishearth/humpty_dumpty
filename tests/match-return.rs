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
fn match_return() {
    let f = Foo;

    match true {
        true => {
            f.close();
            return;
        }
        _ => {
        }
    }

    f.close();
}

#[test]
fn match_all_return() {
    let f = Foo;

    match true {
        true => {
            f.close();
            return;
        }
        _ => {
            f.close();
            return;
        }
    }
}


#[test]
fn match_one_return() {
    let f = Foo;

    match true {
        true => {
            f.close();
            return;
        }
        _ => {
            f.close();
        }
    }
}

#[test]
fn match_second_return() {
    let f = Foo;

    match true {
        true => {
            f.close();
        }
        _ => {
            f.close();
            return;
        }
    }
}

#[test]
fn match_two_return() {
    let f = Foo;

    match 1 {
        0 => {
            f.close();
            return;
        }
        1 => {
            f.close();
            return;
        }
        _ => {

        }
    }
    f.close();
}
