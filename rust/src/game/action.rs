use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Direction {
    Left,
    Right,
}

impl Direction {
    pub const fn sign(self) -> f32 {
        match self {
            Self::Left => -1.0,
            Self::Right => 1.0,
        }
    }

    pub const fn opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AttackId {
    Basic,
}

impl fmt::Display for AttackId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Basic => write!(f, "basic_attack"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CharacterIntent {
    None,
    Move(Direction),
    Jump,
    Attack(AttackId),
    Dodge(Direction),
}

impl CharacterIntent {
    pub const fn label(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Move(Direction::Left) => "Move Left",
            Self::Move(Direction::Right) => "Move Right",
            Self::Jump => "Jump",
            Self::Attack(AttackId::Basic) => "Attack",
            Self::Dodge(Direction::Left) => "Dodge Left",
            Self::Dodge(Direction::Right) => "Dodge Right",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ActionState {
    Idle,
    Running,
    Jumping,
    Falling,
    Attacking { attack: AttackId, elapsed: f32 },
    Hurt { remaining: f32 },
    Stunned,
    Dead,
}

impl Default for ActionState {
    fn default() -> Self {
        Self::Idle
    }
}

impl ActionState {
    pub const ATTACK_DURATION: f32 = 0.34;
    pub const ATTACK_ACTIVE_AT: f32 = 0.10;

    pub const fn label(&self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::Running => "Running",
            Self::Jumping => "Jumping",
            Self::Falling => "Falling",
            Self::Attacking { .. } => "Attacking",
            Self::Hurt { .. } => "Hurt",
            Self::Stunned => "Stunned",
            Self::Dead => "Dead",
        }
    }

    /// Priority used by passive reconciliation. Higher priority wins within a
    /// tick, so `Dead` cannot be overwritten by a later attack transition.
    pub const fn priority(&self) -> u16 {
        match self {
            Self::Dead => 1000,
            Self::Stunned => 400,
            Self::Hurt { .. } => 300,
            Self::Attacking { .. } => 100,
            Self::Idle | Self::Running | Self::Jumping | Self::Falling => 0,
        }
    }

    pub const fn is_busy(&self) -> bool {
        matches!(
            self,
            Self::Attacking { .. } | Self::Hurt { .. } | Self::Stunned | Self::Dead
        )
    }
}
