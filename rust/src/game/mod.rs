//! Engine-light gameplay model.
//!
//! This module deliberately contains no Godot nodes, signals, scene lookups or
//! animation calls.  The simulation owns the order of operations for one tick;
//! controllers submit intents and the resolvers decide what actually happens.

pub mod action;
pub mod ai;
pub mod character;
pub mod combat;
pub mod controller;
pub mod simulation;
pub mod status;

pub use action::{ActionState, AttackId, CharacterIntent, Direction};
pub use ai::{AiDecision, AiMode, AiObservation, EnemyAi, TargetObservation};
pub use character::{Character, CharacterKind, CharacterSnapshot, EntityId};
pub use combat::{CombatEvent, Hit};
pub use controller::{PlayerController, PlayerInput};
pub use simulation::{Simulation, SimulationEvent, WorldSnapshot};
pub use status::{CharacterCapabilities, StatusEffect, StatusEffects};

/// A small engine-light replacement for a 2D position/value type.
///
/// Keeping this type local means unit tests do not need a running Godot
/// runtime. The Godot adapter converts it to/from `godot::builtin::Vector2`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn distance_to(self, other: Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

impl std::ops::Add for Vec2 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl std::ops::Mul<f32> for Vec2 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}
