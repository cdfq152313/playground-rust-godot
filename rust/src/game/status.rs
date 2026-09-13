use super::character::CharacterKind;

#[derive(Clone, Debug, PartialEq)]
pub enum StatusEffect {
    Stun { remaining: f32 },
    Invulnerable { remaining: f32 },
    Slow { remaining: f32, multiplier: f32 },
}

impl StatusEffect {
    fn tick(&mut self, delta: f32) {
        match self {
            Self::Stun { remaining }
            | Self::Invulnerable { remaining }
            | Self::Slow { remaining, .. } => *remaining = (*remaining - delta).max(0.0),
        }
    }

    fn active(&self) -> bool {
        match self {
            Self::Stun { remaining }
            | Self::Invulnerable { remaining }
            | Self::Slow { remaining, .. } => *remaining > 0.0,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StatusEffects {
    effects: Vec<StatusEffect>,
}

impl StatusEffects {
    pub fn apply_stun(&mut self, duration: f32) {
        if duration <= 0.0 {
            return;
        }

        if let Some(StatusEffect::Stun { remaining }) = self
            .effects
            .iter_mut()
            .find(|effect| matches!(effect, StatusEffect::Stun { .. }))
        {
            *remaining = remaining.max(duration);
        } else {
            self.effects.push(StatusEffect::Stun {
                remaining: duration,
            });
        }
    }

    pub fn apply_invulnerability(&mut self, duration: f32) {
        if duration > 0.0 {
            self.effects.push(StatusEffect::Invulnerable {
                remaining: duration,
            });
        }
    }

    pub fn apply_slow(&mut self, duration: f32, multiplier: f32) {
        if duration > 0.0 {
            self.effects.push(StatusEffect::Slow {
                remaining: duration,
                multiplier: multiplier.clamp(0.0, 1.0),
            });
        }
    }

    pub fn tick(&mut self, delta: f32) {
        for effect in &mut self.effects {
            effect.tick(delta.max(0.0));
        }
        self.effects.retain(StatusEffect::active);
    }

    pub fn has_stun(&self) -> bool {
        self.effects
            .iter()
            .any(|effect| matches!(effect, StatusEffect::Stun { remaining } if *remaining > 0.0))
    }

    pub fn is_invulnerable(&self) -> bool {
        self.effects.iter().any(
            |effect| matches!(effect, StatusEffect::Invulnerable { remaining } if *remaining > 0.0),
        )
    }

    pub fn slow_multiplier(&self) -> f32 {
        self.effects
            .iter()
            .filter_map(|effect| match effect {
                StatusEffect::Slow {
                    remaining,
                    multiplier,
                } if *remaining > 0.0 => Some(*multiplier),
                _ => None,
            })
            .fold(1.0, f32::min)
    }

    pub fn labels(&self) -> Vec<&'static str> {
        self.effects
            .iter()
            .filter_map(|effect| match effect {
                StatusEffect::Stun { .. } => Some("Stun"),
                StatusEffect::Invulnerable { .. } => Some("Invulnerable"),
                StatusEffect::Slow { .. } => Some("Slow"),
            })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CharacterCapabilities {
    pub can_move: bool,
    pub can_jump: bool,
    pub can_attack: bool,
    pub move_speed_multiplier: f32,
}

impl CharacterCapabilities {
    pub fn for_character(kind: CharacterKind, statuses: &StatusEffects, is_dead: bool) -> Self {
        let stunned = statuses.has_stun();
        Self {
            can_move: !is_dead && !stunned,
            can_jump: !is_dead && !stunned,
            can_attack: !is_dead
                && !stunned
                && matches!(kind, CharacterKind::Player | CharacterKind::Enemy),
            move_speed_multiplier: statuses.slow_multiplier(),
        }
    }
}
