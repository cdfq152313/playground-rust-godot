use godot::classes::{CharacterBody2D, ICharacterBody2D};
use godot::prelude::*;

use crate::game::{ActionState, CharacterKind};

/// Thin Godot representation of a gameplay character.
///
/// It owns only the engine-facing base node. Gameplay state lives in
/// `game::Character` and is pushed here during the Apply/Presentation phases.
#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
pub struct ActorNode {
    pub(crate) base: Base<CharacterBody2D>,
}

#[godot_api]
impl ICharacterBody2D for ActorNode {
    fn init(base: Base<CharacterBody2D>) -> Self {
        Self { base }
    }
}

impl ActorNode {
    pub(crate) fn apply_presentation(
        &mut self,
        kind: CharacterKind,
        state: &ActionState,
        health_ratio: f32,
    ) {
        let color = match state {
            ActionState::Idle => match kind {
                CharacterKind::Player => Color::from_rgba(0.28, 0.72, 1.0, 1.0),
                CharacterKind::Enemy => Color::from_rgba(1.0, 0.28, 0.42, 1.0),
            },
            ActionState::Running => match kind {
                CharacterKind::Player => Color::from_rgba(0.35, 0.9, 1.0, 1.0),
                CharacterKind::Enemy => Color::from_rgba(1.0, 0.4, 0.5, 1.0),
            },
            ActionState::Jumping | ActionState::Falling => Color::from_rgba(0.55, 0.78, 1.0, 1.0),
            ActionState::Attacking { .. } => Color::from_rgba(1.0, 0.78, 0.25, 1.0),
            ActionState::Hurt { .. } => Color::from_rgba(1.0, 0.35, 0.35, 1.0),
            ActionState::Stunned => Color::from_rgba(0.7, 0.45, 1.0, 1.0),
            ActionState::Dead => Color::from_rgba(0.25, 0.25, 0.3, 0.65),
        };
        let health_alpha = (0.45 + health_ratio.clamp(0.0, 1.0) * 0.55).clamp(0.0, 1.0);
        self.base_mut()
            .set_modulate(Color::from_rgba(color.r, color.g, color.b, health_alpha));
        let scale = if matches!(state, ActionState::Attacking { .. }) {
            Vector2::new(1.22, 1.0)
        } else if matches!(state, ActionState::Stunned) {
            Vector2::new(0.92, 0.92)
        } else {
            Vector2::ONE
        };
        self.base_mut().set_scale(scale);
    }
}
