use super::combat::Hit;
use super::{Vec2, status::CharacterCapabilities, status::StatusEffects};
use super::{action::ActionState, action::AttackId, action::CharacterIntent};

pub type EntityId = u32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CharacterKind {
    Player,
    Enemy,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Health {
    pub current: i32,
    pub max: i32,
}

impl Health {
    pub fn ratio(self) -> f32 {
        if self.max <= 0 {
            0.0
        } else {
            self.current.max(0) as f32 / self.max as f32
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CharacterStats {
    pub move_speed: f32,
    pub jump_speed: f32,
    pub attack_damage: i32,
    pub attack_range: f32,
    pub attack_cooldown: f32,
}

impl CharacterStats {
    pub const fn player() -> Self {
        Self {
            move_speed: 260.0,
            jump_speed: 520.0,
            attack_damage: 25,
            attack_range: 100.0,
            attack_cooldown: 0.34,
        }
    }

    pub const fn enemy() -> Self {
        Self {
            move_speed: 155.0,
            jump_speed: 450.0,
            attack_damage: 12,
            attack_range: 82.0,
            attack_cooldown: 0.52,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MovementState {
    pub position: Vec2,
    pub velocity: Vec2,
    pub grounded: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CharacterInbox {
    pub hits: Vec<Hit>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Character {
    pub id: EntityId,
    pub kind: CharacterKind,
    pub health: Health,
    pub stats: CharacterStats,
    pub statuses: StatusEffects,
    pub capabilities: CharacterCapabilities,
    pub action: ActionState,
    pub movement: MovementState,
    pub inbox: CharacterInbox,
    pub attack_cooldown_remaining: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CharacterSnapshot {
    pub id: EntityId,
    pub kind: CharacterKind,
    pub position: Vec2,
    pub health_ratio: f32,
    pub capabilities: CharacterCapabilities,
    pub action: ActionState,
}

impl Character {
    pub fn new(id: EntityId, kind: CharacterKind, position: Vec2) -> Self {
        let stats = match kind {
            CharacterKind::Player => CharacterStats::player(),
            CharacterKind::Enemy => CharacterStats::enemy(),
        };
        let statuses = StatusEffects::default();
        let capabilities = CharacterCapabilities::for_character(kind, &statuses, false);
        Self {
            id,
            kind,
            health: Health {
                current: 100,
                max: 100,
            },
            stats,
            statuses,
            capabilities,
            action: ActionState::Idle,
            movement: MovementState {
                position,
                velocity: Vec2::ZERO,
                grounded: true,
            },
            inbox: CharacterInbox::default(),
            attack_cooldown_remaining: 0.0,
        }
    }

    pub fn snapshot(&self) -> CharacterSnapshot {
        CharacterSnapshot {
            id: self.id,
            kind: self.kind,
            position: self.movement.position,
            health_ratio: self.health.ratio(),
            capabilities: self.capabilities.clone(),
            action: self.action.clone(),
        }
    }

    pub fn sync_position(&mut self, position: Vec2, grounded: bool) {
        self.movement.position = position;
        self.movement.grounded = grounded;
        if grounded && self.movement.velocity.y > 0.0 {
            self.movement.velocity.y = 0.0;
        }
    }

    pub fn refresh_capabilities(&mut self) {
        self.capabilities = CharacterCapabilities::for_character(
            self.kind,
            &self.statuses,
            self.health.current <= 0,
        );
    }

    /// Active transition: a controller asks to do something. This method does
    /// not know whether the source was a player or AI.
    pub fn try_perform(&mut self, intent: &CharacterIntent) -> bool {
        if self.health.current <= 0 {
            return false;
        }

        match intent {
            CharacterIntent::None => {
                if !self.action.is_busy() {
                    self.movement.velocity.x = 0.0;
                    self.action = if self.movement.grounded {
                        ActionState::Idle
                    } else {
                        ActionState::Falling
                    };
                }
                true
            }
            CharacterIntent::Move(direction) => {
                if !self.capabilities.can_move || self.action.is_busy() {
                    return false;
                }
                self.movement.velocity.x = direction.sign()
                    * self.stats.move_speed
                    * self.capabilities.move_speed_multiplier;
                self.action = ActionState::Running;
                true
            }
            CharacterIntent::Jump => {
                if !self.capabilities.can_jump || !self.movement.grounded || self.action.is_busy() {
                    return false;
                }
                self.movement.velocity.y = -self.stats.jump_speed;
                self.movement.grounded = false;
                self.action = ActionState::Jumping;
                true
            }
            CharacterIntent::Attack(AttackId::Basic) => {
                if !self.capabilities.can_attack
                    || self.attack_cooldown_remaining > 0.0
                    || self.action.is_busy()
                {
                    return false;
                }
                self.movement.velocity.x = 0.0;
                self.attack_cooldown_remaining = self.stats.attack_cooldown;
                self.action = ActionState::Attacking {
                    attack: AttackId::Basic,
                    elapsed: 0.0,
                };
                true
            }
            CharacterIntent::Dodge(direction) => {
                if !self.capabilities.can_move || self.action.is_busy() {
                    return false;
                }
                self.movement.velocity.x = direction.sign() * self.stats.move_speed * 1.8;
                self.action = ActionState::Running;
                true
            }
        }
    }

    /// Advance local timers and report attack active frames. It does not apply
    /// hits; cross-entity effects are resolved by the simulation layer.
    pub fn advance_timers(&mut self, delta: f32) -> Option<AttackId> {
        let delta = delta.max(0.0);
        self.attack_cooldown_remaining = (self.attack_cooldown_remaining - delta).max(0.0);
        self.statuses.tick(delta);

        // Gravity is a gameplay movement rule; the adapter still delegates
        // the actual collision integration to CharacterBody2D::move_and_slide.
        if !self.movement.grounded && self.health.current > 0 {
            self.movement.velocity.y += 1_500.0 * delta;
        }

        let mut attack_event = None;
        if let ActionState::Attacking { attack, elapsed } = &mut self.action {
            let old_elapsed = *elapsed;
            *elapsed += delta;
            if old_elapsed < ActionState::ATTACK_ACTIVE_AT
                && *elapsed >= ActionState::ATTACK_ACTIVE_AT
            {
                attack_event = Some(*attack);
            }
            if *elapsed >= ActionState::ATTACK_DURATION {
                self.action = if self.movement.grounded {
                    ActionState::Idle
                } else {
                    ActionState::Falling
                };
            }
        }

        if let ActionState::Hurt { remaining } = &mut self.action {
            *remaining = (*remaining - delta).max(0.0);
            if *remaining <= 0.0 {
                self.action = if self.movement.grounded {
                    ActionState::Idle
                } else {
                    ActionState::Falling
                };
            }
        }
        attack_event
    }

    /// Passive reconciliation: statuses and health influence the final action
    /// through capabilities and priority, never by calling a state transition
    /// directly from a status effect.
    pub fn reconcile_state(&mut self) {
        self.refresh_capabilities();
        let desired = if self.health.current <= 0 {
            ActionState::Dead
        } else if self.statuses.has_stun() {
            ActionState::Stunned
        } else if matches!(self.action, ActionState::Stunned) {
            if self.movement.grounded {
                ActionState::Idle
            } else {
                ActionState::Falling
            }
        } else {
            self.action.clone()
        };

        if desired.priority() >= self.action.priority()
            || matches!(self.action, ActionState::Stunned)
        {
            self.action = desired;
        }
        if !self.capabilities.can_move && !matches!(self.action, ActionState::Dead) {
            self.movement.velocity.x = 0.0;
        }
        if matches!(self.action, ActionState::Dead) {
            self.movement.velocity = Vec2::ZERO;
        }
    }

    pub fn apply_hit(&mut self, hit: &Hit) -> Option<i32> {
        if self.health.current <= 0 || self.statuses.is_invulnerable() {
            return None;
        }
        let damage = hit.damage.max(0);
        self.health.current = (self.health.current - damage).max(0);
        self.movement.velocity = self.movement.velocity + hit.knockback;
        if hit.hit_stun > 0.0 && self.health.current > 0 {
            self.statuses.apply_stun(hit.hit_stun);
        }
        Some(damage)
    }
}
