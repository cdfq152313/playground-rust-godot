/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
*/

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
