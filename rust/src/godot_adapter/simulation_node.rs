use godot::classes::{INode2D, Input, Label, Node2D};
use godot::prelude::*;

use super::actor_node::ActorNode;
use crate::game::{ActionState, PlayerInput, Simulation, Vec2};

/// Godot-side composition root and simulation boundary.
///
/// `_physics_process` is the only place that advances the gameplay model. The
/// node gathers input and engine positions, calls the Rust simulation, then
/// applies resolved velocity through `CharacterBody2D::move_and_slide`.
#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct SimulationNode {
    base: Base<Node2D>,
    simulation: Simulation,
    player: Option<Gd<ActorNode>>,
    enemy: Option<Gd<ActorNode>>,
    debug_label: Option<Gd<Label>>,
    help_label: Option<Gd<Label>>,
}

#[godot_api]
impl INode2D for SimulationNode {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            base,
            simulation: Simulation::new(),
            player: None,
            enemy: None,
            debug_label: None,
            help_label: None,
        }
    }

    fn ready(&mut self) {
        self.player = Some(self.base().get_node_as::<ActorNode>("Player"));
        self.enemy = Some(self.base().get_node_as::<ActorNode>("Enemy"));
        self.debug_label = Some(self.base().get_node_as::<Label>("UI/DebugPanel"));
        self.help_label = Some(self.base().get_node_as::<Label>("UI/Help"));
        if let Some(help_label) = &mut self.help_label {
            help_label.set_text(
                "A / D 移動    Space 跳躍    J 攻擊\nEnemy AI 會依同一份 World Snapshot 追擊與攻擊",
            );
        }
        godot_print!("Simulation ready: Gather -> Decide -> Resolve -> Apply");
    }

    fn physics_process(&mut self, delta: f64) {
        self.sync_engine_positions();
        let input = self.gather_input();
        self.simulation.tick(delta as f32, input);
        self.apply_physics();
        self.update_presentation();
        self.update_debug_ui();
    }
}

impl SimulationNode {
    fn gather_input(&self) -> PlayerInput {
        let input = Input::singleton();
        PlayerInput {
            left: input.is_action_pressed("move_left"),
            right: input.is_action_pressed("move_right"),
            jump_pressed: input.is_action_just_pressed("jump"),
            attack_pressed: input.is_action_just_pressed("attack"),
        }
    }

    fn sync_engine_positions(&mut self) {
        let player_position = self.player.as_ref().map(|node| {
            let node = node.bind();
            from_godot(node.base().get_position())
        });
        let enemy_position = self.enemy.as_ref().map(|node| {
            let node = node.bind();
            from_godot(node.base().get_position())
        });
        if let Some(position) = player_position {
            self.simulation
                .sync_position(1, position, position.y >= 348.0);
        }
        if let Some(position) = enemy_position {
            self.simulation
                .sync_position(2, position, position.y >= 348.0);
        }
    }

    fn apply_physics(&mut self) {
        Self::apply_character_physics(&mut self.simulation, 1, &mut self.player);
        Self::apply_character_physics(&mut self.simulation, 2, &mut self.enemy);
    }

    fn apply_character_physics(
        simulation: &mut Simulation,
        entity: u32,
        node: &mut Option<Gd<ActorNode>>,
    ) {
        let Some(character) = simulation.character(entity) else {
            return;
        };
        let velocity = character.movement.velocity;
        let dead = matches!(character.action, ActionState::Dead);
        if let Some(node) = node {
            let mut node = node.bind_mut();
            node.base_mut().set_velocity(to_godot(velocity));
            if !dead {
                node.base_mut().move_and_slide();
            }
            let position = from_godot(node.base().get_position());
            // The actor's 72px collision shape rests on the floor at y=349.
            // Keep this fallback for the first frame, before Godot reports a
            // floor collision to CharacterBody2D.
            let grounded = node.base().is_on_floor() || position.y >= 348.0;
            simulation.sync_position(entity, position, grounded);
        }
    }

    fn update_presentation(&mut self) {
        for (character, node) in [
            (self.simulation.character(1), &mut self.player),
            (self.simulation.character(2), &mut self.enemy),
        ] {
            let (Some(character), Some(node)) = (character, node) else {
                continue;
            };
            node.bind_mut().apply_presentation(
                character.kind,
                &character.action,
                character.health.ratio(),
            );
        }
    }

    fn update_debug_ui(&mut self) {
        let Some(debug_label) = &mut self.debug_label else {
            return;
        };
        let Some(enemy) = self.simulation.character(2) else {
            return;
        };
        let ai_trace = self.simulation.last_ai_trace.as_ref();
        let trace = ai_trace
            .map(|trace| trace.lines.join("\n  "))
            .unwrap_or_else(|| "No decision yet".to_owned());
        let target_distance = self
            .simulation
            .character(1)
            .map(|player| {
                enemy
                    .movement
                    .position
                    .distance_to(player.movement.position)
            })
            .unwrap_or_default();
        let text = format!(
            "TICK {:04}\n\nPLAYER\n  Action: {}\n  HP: {}/{}\n  Intent: {}\n\nENEMY #2\n  AI Mode: {}\n  Action: {}\n  HP: {}/{}\n  Distance: {:.0}\n  Decision: {}\n  {}",
            self.simulation.tick,
            self.simulation
                .character(1)
                .map(|c| c.action.label())
                .unwrap_or("Missing"),
            self.simulation
                .character(1)
                .map(|c| c.health.current)
                .unwrap_or(0),
            self.simulation
                .character(1)
                .map(|c| c.health.max)
                .unwrap_or(0),
            self.simulation.last_player_intent.label(),
            self.simulation.enemy_ai.mode.label(),
            enemy.action.label(),
            enemy.health.current,
            enemy.health.max,
            target_distance,
            self.simulation
                .last_ai_decision
                .as_ref()
                .map(|d| d.intent.label())
                .unwrap_or("None"),
            trace,
        );
        debug_label.set_text(text.as_str());

        for event in &self.simulation.last_events {
            if let crate::game::SimulationEvent::Combat(combat) = event {
                godot_print!(
                    "Combat: {} -> {} damage={} stun={:.2}",
                    combat.attacker,
                    combat.target,
                    combat.damage,
                    combat.hit_stun
                );
            }
        }
    }
}

fn to_godot(value: Vec2) -> Vector2 {
    Vector2::new(value.x, value.y)
}

fn from_godot(value: Vector2) -> Vec2 {
    Vec2::new(value.x, value.y)
}
