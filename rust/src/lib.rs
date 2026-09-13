use godot::prelude::*;

pub mod game;
mod godot_adapter;

struct MyExtension;

#[gdextension]
unsafe impl ExtensionLibrary for MyExtension {}
