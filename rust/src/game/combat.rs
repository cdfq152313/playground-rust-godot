use super::{AttackId, Character, EntityId, Vec2};

#[derive(Clone, Debug, PartialEq)]
pub struct Hit {
    pub attacker: EntityId,
    pub target: EntityId,
    pub damage: i32,
    pub knockback: Vec2,
    pub hit_stun: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CombatEvent {
    pub attacker: EntityId,
    pub target: EntityId,
    pub damage: i32,
    pub target_health: i32,
    pub hit_stun: f32,
}

pub fn build_basic_hit(attacker: &Character, target: &Character, attack: AttackId) -> Option<Hit> {
    let distance = attacker
        .movement
        .position
        .distance_to(target.movement.position);
    if distance > attacker.stats.attack_range || attacker.kind == target.kind {
        return None;
    }

    let direction = if target.movement.position.x >= attacker.movement.position.x {
        1.0
    } else {
        -1.0
    };
    let (damage, hit_stun) = match attack {
        AttackId::Basic => (attacker.stats.attack_damage, 0.55),
    };
    Some(Hit {
        attacker: attacker.id,
        target: target.id,
        damage,
        knockback: Vec2::new(direction * 170.0, -90.0),
        hit_stun,
    })
}
