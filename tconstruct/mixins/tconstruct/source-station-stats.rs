// Shared base-stat aggregation for tool definitions using head/handle parts.
// Traits, modifiers, tier and final stat clamps are applied separately.
use serde_json::{Map, Value};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct BaseStats {
    pub durability: f32,
    pub durability_multiplier: f32,
    pub durability_boost_unit: f32,
    pub durability_multiplier_unit: f32,
    pub mining_speed: f32,
    pub attack_damage: f32,
    pub attack_damage_boost_unit: f32,
    pub attack_damage_multiplier: f32,
    pub attack_damage_multiplier_unit: f32,
    pub attack_speed: f32,
    pub attack_speed_boost_unit: f32,
    pub draw_speed: f32,
    pub velocity: f32,
    pub accuracy: f32,
    pub armor: f32,
    pub armor_toughness: f32,
    pub knockback_resistance: f32,
}

#[derive(Clone, Copy)]
struct Accumulator {
    base: f32,
    add: f32,
    percent: f32,
    multiply: f32,
}
pub(super) fn part_kind(id: &str) -> Option<&'static str> {
    Some(match id.strip_prefix("tconstruct:")? {
        "pick_head" | "hammer_head" | "small_axe_head" | "broad_axe_head" | "small_blade"
        | "broad_blade" | "adze_head" | "large_plate" => "head",
        "tool_handle" | "tough_handle" => "handle",
        "tool_binding" | "tough_binding" => "binding",
        "bow_limb" => "limb",
        "bow_grip" => "grip",
        "bowstring" => "bowstring",
        "arrow_head" => "arrow_head",
        "arrow_shaft" => "arrow_shaft",
        "fletching" => "fletching",
        "helmet_plating" => "plating_helmet",
        "chestplate_plating" => "plating_chestplate",
        "leggings_plating" => "plating_leggings",
        "boots_plating" => "plating_boots",
        "maille" => "maille",
        _ => return None,
    })
}
impl Accumulator {
    fn new(base: f32) -> Self {
        Self {
            base,
            add: 0.0,
            percent: 0.0,
            multiply: 1.0,
        }
    }
    fn update(&mut self, value: f32) {
        self.base = 0.0;
        self.add += value;
    }
    fn result(self) -> f32 {
        (self.base + self.add) * (1.0 + self.percent) * self.multiply
    }
}

fn apply_material(
    stats: &mut [Accumulator; 10],
    kind: &str,
    material: &Map<String, Value>,
    scale: f32,
) -> Option<()> {
    if matches!(
        kind,
        "binding"
            | "bowstring"
            | "arrow_head"
            | "arrow_shaft"
            | "fletching"
            | "maille"
            | "shield_core"
            | "cuirass"
            | "ribcage"
            | "shell"
            | "laces"
            | "skull"
    ) {
        return Some(());
    }
    let fields = material.get(&format!("tconstruct:{kind}"))?.as_object()?;
    let number = |key: &str, default: f32| -> Option<f32> {
        match fields.get(key) {
            Some(value) => Some(value.as_f64()? as f32),
            None => Some(default),
        }
    };
    match kind {
        "head" => {
            stats[0].update(number("durability", 0.0)? * scale);
            stats[1].update(number("mining_speed", 0.0)? * scale);
            stats[2].update(number("melee_attack", 0.0)? * scale);
        }
        "handle" => {
            for (index, key) in [
                (0, "durability"),
                (1, "mining_speed"),
                (2, "melee_damage"),
                (3, "melee_speed"),
            ] {
                stats[index].percent += number(key, 0.0)? * scale;
            }
        }
        "slime" => {
            stats[0].update(number("durability", 1.0)? * scale);
        }
        "limb" => {
            stats[0].update(number("durability", 1.0)? * scale);
            for (index, key) in [(4, "draw_speed"), (5, "velocity"), (6, "accuracy")] {
                stats[index].add += number(key, 0.0)? * scale;
            }
        }
        "grip" => {
            stats[0].percent += number("durability", 0.0)? * scale;
            stats[6].add += number("accuracy", 0.0)? * scale;
            stats[2].update(number("melee_damage", 0.0)? * scale);
        }
        kind if kind.starts_with("plating_") => {
            for (index, key) in [
                (0, "durability"),
                (7, "armor"),
                (8, "toughness"),
                (9, "knockback_resistance"),
            ] {
                stats[index].update(number(key, 0.0)? * scale);
            }
        }
        _ => return None,
    }
    Some(())
}

/// Compute the supported shared material-derived numeric base. `materials` contains the
/// already merged source stats object for each ordered part material.
pub(super) fn base_stats(definition: &[u8], materials: &[Map<String, Value>]) -> Option<BaseStats> {
    let definition: Value = serde_json::from_slice(definition).ok()?;
    let mut stats = [
        Accumulator::new(1.0),
        Accumulator::new(1.0),
        Accumulator::new(0.0),
        Accumulator::new(1.0),
        Accumulator::new(1.0),
        Accumulator::new(1.0),
        Accumulator::new(0.75),
        Accumulator::new(0.0),
        Accumulator::new(0.0),
        Accumulator::new(0.0),
    ];
    let keys = [
        "tconstruct:durability",
        "tconstruct:mining_speed",
        "tconstruct:attack_damage",
        "tconstruct:attack_speed",
        "tconstruct:draw_speed",
        "tconstruct:velocity",
        "tconstruct:accuracy",
        "tconstruct:armor",
        "tconstruct:armor_toughness",
        "tconstruct:knockback_resistance",
    ];
    for module in definition.get("modules")?.as_array()? {
        match module.get("type")?.as_str()? {
            "tconstruct:part_stats" => {
                let parts = module.get("parts")?.as_array()?;
                if parts.len() != materials.len() {
                    return None;
                }
                for (part, material) in parts.iter().zip(materials) {
                    let id = part
                        .as_str()
                        .or_else(|| part.get("item").and_then(Value::as_str))?;
                    let scale = match part.get("scale") {
                        Some(value) => value.as_f64()? as f32,
                        None => 1.0,
                    };
                    let kind = part_kind(id)?;
                    apply_material(&mut stats, kind, material, scale)?;
                }
            }
            "tconstruct:material_stats" => {
                let kinds = module.get("stat_types")?.as_array()?;
                if kinds.len() != materials.len() {
                    return None;
                }
                for (kind, material) in kinds.iter().zip(materials) {
                    let id = kind
                        .as_str()
                        .or_else(|| kind.get("stat").and_then(Value::as_str))?;
                    let scale = match kind.get("scale") {
                        Some(value) => value.as_f64()? as f32,
                        None => 1.0,
                    };
                    apply_material(&mut stats, id.strip_prefix("tconstruct:")?, material, scale)?;
                }
            }
            "tconstruct:base_stats" | "tconstruct:multiply_stats" => {
                let multiply = module.get("type")?.as_str()? == "tconstruct:multiply_stats";
                let fields = module
                    .get(if multiply { "multipliers" } else { "stats" })?
                    .as_object()?;
                for (key, value) in fields {
                    let Some(index) = keys.iter().position(|known| known == key) else {
                        continue;
                    };
                    let value = value.as_f64()? as f32;
                    if multiply {
                        stats[index].multiply *= value;
                    } else {
                        stats[index].update(value);
                    }
                }
            }
            _ => {}
        }
    }
    Some(BaseStats {
        durability: stats[0].result(),
        durability_multiplier: (1.0 + stats[0].percent) * stats[0].multiply,
        durability_boost_unit: (stats[0].base + stats[0].add) * stats[0].multiply,
        durability_multiplier_unit: stats[0].multiply,
        mining_speed: stats[1].result(),
        attack_damage: stats[2].result(),
        attack_damage_boost_unit: (stats[2].base + stats[2].add) * stats[2].multiply,
        attack_damage_multiplier: (1.0 + stats[2].percent) * stats[2].multiply,
        attack_damage_multiplier_unit: stats[2].multiply,
        attack_speed: stats[3].result(),
        attack_speed_boost_unit: (stats[3].base + stats[3].add) * stats[3].multiply,
        draw_speed: stats[4].result(),
        velocity: stats[5].result(),
        accuracy: stats[6].result(),
        armor: stats[7].result(),
        armor_toughness: stats[8].result(),
        knockback_resistance: stats[9].result(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn applies_limb_and_grip_source_stats() {
        let definition = br#"{"modules":[{"type":"tconstruct:part_stats","parts":["tconstruct:bow_limb","tconstruct:bow_grip"]}]}"#;
        let limb = serde_json::json!({"tconstruct:limb":{"durability":100,"draw_speed":0.2,"velocity":0.3,"accuracy":0.1}}).as_object().unwrap().clone();
        let grip = serde_json::json!({"tconstruct:grip":{"durability":0.5,"accuracy":0.05,"melee_damage":2}}).as_object().unwrap().clone();
        let stats = base_stats(definition, &[limb, grip]).unwrap();
        assert_eq!(stats.durability, 150.0);
        assert_eq!(stats.durability_multiplier, 1.5);
        assert!((stats.draw_speed - 1.2).abs() < 0.0001);
        assert!((stats.velocity - 1.3).abs() < 0.0001);
        assert!((stats.accuracy - 0.9).abs() < 0.0001);
        assert_eq!(stats.attack_damage, 2.0);
    }
    #[test]
    fn applies_plating_source_stats() {
        let definition = br#"{"modules":[{"type":"tconstruct:part_stats","parts":["tconstruct:helmet_plating","tconstruct:maille"]}]}"#;
        let plate = serde_json::json!({"tconstruct:plating_helmet":{"durability":160,"armor":3,"toughness":2,"knockback_resistance":0.1}}).as_object().unwrap().clone();
        let stats = base_stats(definition, &[plate, Map::new()]).unwrap();
        assert_eq!(stats.durability, 160.0);
        assert_eq!(stats.armor, 3.0);
        assert_eq!(stats.armor_toughness, 2.0);
        assert!((stats.knockback_resistance - 0.1).abs() < 0.0001);
    }
    #[test]
    fn applies_material_stats_with_source_scale() {
        let definition = br#"{"modules":[{"type":"tconstruct:material_stats","stat_types":["tconstruct:laces",{"stat":"tconstruct:slime","scale":1.5}]}]}"#;
        let material = serde_json::json!({"tconstruct:slime":{"durability":100}})
            .as_object()
            .unwrap()
            .clone();
        let stats = base_stats(definition, &[Map::new(), material]).unwrap();
        assert_eq!(stats.durability, 150.0);
    }
    #[test]
    fn combines_material_and_definition_values() {
        let definition = br#"{"modules":[{"type":"tconstruct:part_stats","parts":["tconstruct:pick_head","tconstruct:tool_handle","tconstruct:tool_binding"]},{"type":"tconstruct:base_stats","stats":{"tconstruct:attack_damage":0.5,"tconstruct:attack_speed":1.2}}]}"#;
        let head = serde_json::json!({"tconstruct:head":{"durability":85,"mining_speed":3.5,"melee_attack":1.25}}).as_object().unwrap().clone();
        let handle = serde_json::json!({"tconstruct:handle":{"durability":0,"mining_speed":0,"melee_damage":0,"melee_speed":0}}).as_object().unwrap().clone();
        let binding = serde_json::json!({"tconstruct:binding":{}})
            .as_object()
            .unwrap()
            .clone();
        let stats = base_stats(definition, &[head, handle, binding]).unwrap();
        assert_eq!(
            stats,
            BaseStats {
                durability: 85.0,
                durability_multiplier: 1.0,
                durability_boost_unit: 85.0,
                durability_multiplier_unit: 1.0,
                mining_speed: 3.5,
                attack_damage: 1.75,
                attack_damage_boost_unit: 1.75,
                attack_damage_multiplier: 1.0,
                attack_damage_multiplier_unit: 1.0,
                attack_speed: 1.2,
                attack_speed_boost_unit: 1.2,
                draw_speed: 1.0,
                velocity: 1.0,
                accuracy: 0.75,
                armor: 0.0,
                armor_toughness: 0.0,
                knockback_resistance: 0.0
            }
        );
    }
}
