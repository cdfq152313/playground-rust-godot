use super::{CharacterCapabilities, CharacterIntent, Direction, EntityId, Vec2};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AiMode {
    Patrol,
    Chase,
    Combat,
    Retreat,
}

impl AiMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Patrol => "Patrol",
            Self::Chase => "Chase",
            Self::Combat => "Combat",
            Self::Retreat => "Retreat",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TargetObservation {
    pub entity: EntityId,
    pub position: Vec2,
    pub distance: f32,
    pub visible: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AiObservation {
    pub self_position: Vec2,
    pub target: Option<TargetObservation>,
    pub health_ratio: f32,
    pub capabilities: CharacterCapabilities,
    pub attack_range: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DecisionTrace {
    pub mode: AiMode,
    pub lines: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AiDecision {
    pub intent: CharacterIntent,
    pub trace: DecisionTrace,
}

#[derive(Clone, Debug)]
pub struct EnemyAi {
    pub mode: AiMode,
    pub last_decision: Option<AiDecision>,
}

impl Default for EnemyAi {
    fn default() -> Self {
        Self {
            mode: AiMode::Patrol,
            last_decision: None,
        }
    }
}

impl EnemyAi {
    pub fn decide(&mut self, observation: &AiObservation) -> AiDecision {
        let mut lines = Vec::new();
        let (intent, mode) = if !observation.capabilities.can_move
            && !observation.capabilities.can_attack
        {
            lines.push("Cannot act: capabilities blocked".to_owned());
            (CharacterIntent::None, self.mode)
        } else if observation.health_ratio <= 0.2 {
            self.mode = AiMode::Retreat;
            lines.push("Retreat: health <= 20%".to_owned());
            let direction = observation
                .target
                .as_ref()
                .map(|target| {
                    if target.position.x < observation.self_position.x {
                        Direction::Right
                    } else {
                        Direction::Left
                    }
                })
                .unwrap_or(Direction::Left);
            (CharacterIntent::Move(direction), self.mode)
        } else if let Some(target) = observation.target.as_ref().filter(|target| target.visible) {
            if target.distance <= observation.attack_range && observation.capabilities.can_attack {
                self.mode = AiMode::Combat;
                lines.push(format!(
                    "Attack: distance {:.0} <= range {:.0}",
                    target.distance, observation.attack_range
                ));
                (CharacterIntent::Attack(super::AttackId::Basic), self.mode)
            } else {
                self.mode = AiMode::Chase;
                lines.push(format!(
                    "Chase: distance {:.0} > range {:.0}",
                    target.distance, observation.attack_range
                ));
                let direction = if target.position.x < observation.self_position.x {
                    Direction::Left
                } else {
                    Direction::Right
                };
                (CharacterIntent::Move(direction), self.mode)
            }
        } else {
            self.mode = AiMode::Patrol;
            lines.push("Patrol: no visible target".to_owned());
            (CharacterIntent::None, self.mode)
        };

        let decision = AiDecision {
            intent,
            trace: DecisionTrace { mode, lines },
        };
        self.last_decision = Some(decision.clone());
        decision
    }
}
