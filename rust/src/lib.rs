use godot::prelude::*;
mod itest;
mod player;

struct MyExtension;

#[gdextension]
unsafe impl ExtensionLibrary for MyExtension {}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}
