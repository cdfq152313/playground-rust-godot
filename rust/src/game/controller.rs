use super::action::{AttackId, CharacterIntent, Direction};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerInput {
    pub left: bool,
    pub right: bool,
    pub jump_pressed: bool,
    pub attack_pressed: bool,
}

/// Player controller is intentionally symmetrical with `EnemyAi`: it turns
/// an input sample into an intent and never touches a Character directly.
pub struct PlayerController;

impl PlayerController {
    pub fn decide(input: PlayerInput) -> CharacterIntent {
        if input.attack_pressed {
            CharacterIntent::Attack(AttackId::Basic)
        } else if input.left {
            CharacterIntent::Move(Direction::Left)
        } else if input.right {
            CharacterIntent::Move(Direction::Right)
        } else if input.jump_pressed {
            CharacterIntent::Jump
        } else {
            CharacterIntent::None
        }
    }
}
