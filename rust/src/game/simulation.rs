use std::collections::BTreeMap;

use super::action::CharacterIntent;
use super::ai::{AiDecision, AiObservation, DecisionTrace, EnemyAi, TargetObservation};
use super::character::{Character, CharacterKind, CharacterSnapshot, EntityId};
use super::combat::{CombatEvent, build_basic_hit};
use super::controller::{PlayerController, PlayerInput};
use super::{AttackId, Vec2};

#[derive(Clone, Debug, PartialEq)]
pub enum SimulationEvent {
    IntentAccepted {
        entity: EntityId,
        intent: CharacterIntent,
    },
    IntentRejected {
        entity: EntityId,
        intent: CharacterIntent,
    },
    AttackStarted {
        entity: EntityId,
    },
    Combat(CombatEvent),
    StateChanged {
        entity: EntityId,
        from: String,
        to: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct WorldSnapshot {
    pub tick: u64,
    pub characters: Vec<CharacterSnapshot>,
}

pub struct Simulation {
    pub tick: u64,
    pub characters: BTreeMap<EntityId, Character>,
    pub enemy_ai: EnemyAi,
    pub last_events: Vec<SimulationEvent>,
    pub last_player_intent: CharacterIntent,
    pub last_ai_decision: Option<AiDecision>,
    pub last_ai_trace: Option<DecisionTrace>,
}

impl Default for Simulation {
    fn default() -> Self {
        Self::new()
    }
}

impl Simulation {
    pub fn new() -> Self {
        let mut characters = BTreeMap::new();
        characters.insert(
            1,
            Character::new(1, CharacterKind::Player, Vec2::new(300.0, 330.0)),
        );
        characters.insert(
            2,
            Character::new(2, CharacterKind::Enemy, Vec2::new(850.0, 330.0)),
        );
        Self {
            tick: 0,
            characters,
            enemy_ai: EnemyAi::default(),
            last_events: Vec::new(),
            last_player_intent: CharacterIntent::None,
            last_ai_decision: None,
            last_ai_trace: None,
        }
    }

    pub fn snapshot(&self) -> WorldSnapshot {
        WorldSnapshot {
            tick: self.tick,
            characters: self.characters.values().map(Character::snapshot).collect(),
        }
    }

    pub fn character(&self, entity: EntityId) -> Option<&Character> {
        self.characters.get(&entity)
    }

    pub fn character_mut(&mut self, entity: EntityId) -> Option<&mut Character> {
        self.characters.get_mut(&entity)
    }

    pub fn sync_position(&mut self, entity: EntityId, position: Vec2, grounded: bool) {
        if let Some(character) = self.characters.get_mut(&entity) {
            character.sync_position(position, grounded);
        }
    }

    /// One deterministic simulation boundary:
    /// Gather -> Snapshot -> Decide -> Resolve interactions/characters.
    /// Godot movement is intentionally performed by the adapter after this
    /// method returns, using the resolved velocity.
    pub fn tick(&mut self, delta: f32, input: PlayerInput) {
        self.tick = self.tick.saturating_add(1);
        self.last_events.clear();

        // Gather: read only the previous world state for all controllers.
        let snapshot = self.snapshot();
        let player_intent = if snapshot.characters.iter().any(|c| c.id == 1) {
            PlayerController::decide(input)
        } else {
            CharacterIntent::None
        };
        self.last_player_intent = player_intent.clone();

        let ai_decision = self.decide_enemy(&snapshot);
        self.last_ai_trace = Some(ai_decision.trace.clone());
        self.last_ai_decision = Some(ai_decision.clone());

        // Decision + character resolve. Controllers only submitted intents;
        // these calls are the single place where action state can change.
        self.apply_intent(1, player_intent);
        self.apply_intent(2, ai_decision.intent);

        // Advance timers before interaction resolve. Attack frames become
        // facts; they do not mutate another character directly.
        let mut attack_events = Vec::new();
        for character in self.characters.values_mut() {
            let was = character.action.label().to_owned();
            if let Some(attack) = character.advance_timers(delta) {
                attack_events.push((character.id, attack));
            }
            character.reconcile_state();
            if was != character.action.label() {
                self.last_events.push(SimulationEvent::StateChanged {
                    entity: character.id,
                    from: was,
                    to: character.action.label().to_owned(),
                });
            }
        }

        // Interaction resolve uses the same pre-tick positions for all hits.
        let hit_facts = self.resolve_interactions(attack_events);
        for hit in hit_facts {
            if let Some(target) = self.characters.get_mut(&hit.target) {
                target.inbox.hits.push(hit);
            }
        }

        // Character resolve consumes inboxes only after every interaction has
        // been gathered, making hit order deterministic and inspectable.
        let ids: Vec<EntityId> = self.characters.keys().copied().collect();
        for id in ids {
            let Some(character) = self.characters.get_mut(&id) else {
                continue;
            };
            let hits = std::mem::take(&mut character.inbox.hits);
            for hit in hits {
                let Some(damage) = character.apply_hit(&hit) else {
                    continue;
                };
                self.last_events.push(SimulationEvent::Combat(CombatEvent {
                    attacker: hit.attacker,
                    target: hit.target,
                    damage,
                    target_health: character.health.current,
                    hit_stun: hit.hit_stun,
                }));
            }
            character.reconcile_state();
        }
    }

    fn apply_intent(&mut self, entity: EntityId, intent: CharacterIntent) {
        let Some(character) = self.characters.get_mut(&entity) else {
            return;
        };
        let accepted = character.try_perform(&intent);
        self.last_events.push(if accepted {
            if matches!(intent, CharacterIntent::Attack(_)) {
                SimulationEvent::AttackStarted { entity }
            } else {
                SimulationEvent::IntentAccepted { entity, intent }
            }
        } else {
            SimulationEvent::IntentRejected { entity, intent }
        });
    }

    fn decide_enemy(&mut self, snapshot: &WorldSnapshot) -> AiDecision {
        let enemy = snapshot
            .characters
            .iter()
            .find(|character| character.id == 2);
        let target = snapshot
            .characters
            .iter()
            .find(|character| character.id == 1);
        let observation = match (enemy, target) {
            (Some(enemy), Some(target)) => AiObservation {
                self_position: enemy.position,
                target: Some(TargetObservation {
                    entity: target.id,
                    position: target.position,
                    distance: enemy.position.distance_to(target.position),
                    visible: true,
                }),
                health_ratio: enemy.health_ratio,
                capabilities: enemy.capabilities.clone(),
                attack_range: self
                    .characters
                    .get(&2)
                    .map(|c| c.stats.attack_range)
                    .unwrap_or(82.0),
            },
            (Some(enemy), None) => AiObservation {
                self_position: enemy.position,
                target: None,
                health_ratio: enemy.health_ratio,
                capabilities: enemy.capabilities.clone(),
                attack_range: 82.0,
            },
            _ => AiObservation {
                self_position: Vec2::ZERO,
                target: None,
                health_ratio: 0.0,
                capabilities: super::CharacterCapabilities {
                    can_move: false,
                    can_jump: false,
                    can_attack: false,
                    move_speed_multiplier: 1.0,
                },
                attack_range: 82.0,
            },
        };
        self.enemy_ai.decide(&observation)
    }

    fn resolve_interactions(&self, attack_events: Vec<(EntityId, AttackId)>) -> Vec<super::Hit> {
        let mut hits = Vec::new();
        for (attacker_id, attack) in attack_events {
            let Some(attacker) = self.characters.get(&attacker_id) else {
                continue;
            };
            for target in self.characters.values() {
                if let Some(hit) = build_basic_hit(attacker, target, attack) {
                    hits.push(hit);
                }
            }
        }
        hits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::status::StatusEffect;
    use crate::game::{ActionState, Direction};

    #[test]
    fn stun_is_a_status_and_reconciles_to_stunned() {
        let mut simulation = Simulation::new();
        let player = simulation.character_mut(1).unwrap();
        player.statuses.apply_stun(1.0);
        player.reconcile_state();
        assert_eq!(player.action, ActionState::Stunned);
        assert!(!player.capabilities.can_move);
        assert!(!player.capabilities.can_attack);

        player.statuses.tick(1.0);
        player.reconcile_state();
        assert_eq!(player.action, ActionState::Idle);
        assert!(player.capabilities.can_move);
        assert!(player.statuses.labels().is_empty());
        assert!(!player.statuses.labels().contains(&"Stun"));
        let _ = StatusEffect::Stun { remaining: 0.1 };
    }

    #[test]
    fn dead_has_higher_priority_than_stun_or_attack() {
        let mut simulation = Simulation::new();
        let player = simulation.character_mut(1).unwrap();
        player.action = ActionState::Attacking {
            attack: AttackId::Basic,
            elapsed: 0.2,
        };
        player.health.current = 0;
        player.statuses.apply_stun(1.0);
        player.reconcile_state();
        assert_eq!(player.action, ActionState::Dead);
    }

    #[test]
    fn player_attack_creates_a_hit_without_direct_controller_mutation() {
        let mut simulation = Simulation::new();
        simulation
            .character_mut(2)
            .unwrap()
            .sync_position(Vec2::new(385.0, 330.0), true);
        simulation.tick(
            0.09,
            PlayerInput {
                attack_pressed: true,
                ..Default::default()
            },
        );
        assert_eq!(simulation.character(1).unwrap().action.label(), "Attacking");
        assert!(
            simulation
                .last_events
                .iter()
                .any(|event| matches!(event, SimulationEvent::AttackStarted { entity: 1 }))
        );

        simulation.tick(0.02, PlayerInput::default());
        let damage = simulation.last_events.iter().find_map(|event| match event {
            SimulationEvent::Combat(event) => Some(event.damage),
            _ => None,
        });
        assert_eq!(damage, Some(25));
        assert_eq!(
            simulation.character(2).unwrap().action,
            ActionState::Stunned
        );
    }

    #[test]
    fn enemy_uses_snapshot_to_chase_then_attack() {
        let mut simulation = Simulation::new();
        simulation
            .character_mut(1)
            .unwrap()
            .sync_position(Vec2::new(700.0, 330.0), true);
        simulation
            .character_mut(2)
            .unwrap()
            .sync_position(Vec2::new(300.0, 330.0), true);
        simulation.tick(1.0 / 60.0, PlayerInput::default());
        assert_eq!(
            simulation.last_ai_decision.as_ref().unwrap().intent,
            CharacterIntent::Move(Direction::Right)
        );
        assert_eq!(simulation.enemy_ai.mode, super::super::ai::AiMode::Chase);

        simulation
            .character_mut(2)
            .unwrap()
            .sync_position(Vec2::new(650.0, 330.0), true);
        simulation.tick(1.0 / 60.0, PlayerInput::default());
        assert_eq!(
            simulation.last_ai_decision.as_ref().unwrap().intent,
            CharacterIntent::Attack(AttackId::Basic)
        );
    }

    #[test]
    fn jump_enters_airborne_state_and_gravity_is_resolved_in_core() {
        let mut simulation = Simulation::new();
        simulation.tick(
            1.0 / 60.0,
            PlayerInput {
                jump_pressed: true,
                ..Default::default()
            },
        );
        let player = simulation.character(1).unwrap();
        assert_eq!(player.action, ActionState::Jumping);
        assert!(!player.movement.grounded);
        assert!(player.movement.velocity.y > -520.0);
    }
}
