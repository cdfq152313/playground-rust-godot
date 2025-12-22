use gd_rehearse::itest::*;
use godot::classes::Object;
use godot::obj::Gd;

#[gditest]
fn simple_test() {
    let test = 1 + 1;
    assert_eq!(test, 2);
}

#[gditest]
fn second_test() {
    let test = 1 + 1;
    assert_eq!(test, 2);
}
