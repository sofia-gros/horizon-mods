// Shared base-stat aggregation for tool definitions using head/handle parts.
// Traits, modifiers, tier and final stat clamps are applied separately.
use serde_json::{Map, Value};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BaseStats {
    pub durability: f32,
    pub durability_multiplier: f32,
    pub durability_boost_unit: f32,
    pub durability_multiplier_unit: f32,
    pub durability_base_raw: f32,
    pub durability_percent_raw: f32,
    pub mining_speed: f32,
    pub mining_speed_base_raw: f32,
    pub mining_speed_add_raw: f32,
    pub mining_speed_percent_raw: f32,
    pub mining_speed_add_multiplier: f32,
    pub mining_speed_all_multiplier: f32,
    pub attack_damage: f32,
    pub attack_damage_base_raw: f32,
    pub attack_damage_add_raw: f32,
    pub attack_damage_percent_raw: f32,
    pub attack_damage_boost_unit: f32,
    pub attack_damage_multiplier: f32,
    pub attack_damage_multiplier_unit: f32,
    pub attack_speed: f32,
    pub attack_speed_base_raw: f32,
    pub attack_speed_add_raw: f32,
    pub attack_speed_percent_raw: f32,
    pub attack_speed_multiplier_unit: f32,
    pub attack_speed_boost_unit: f32,
    pub draw_speed: f32,
    pub draw_speed_raw: [f32; 4],
    pub draw_speed_add_multiplier: f32,
    pub velocity: f32,
    pub velocity_raw: [f32; 4],
    pub velocity_add_multiplier: f32,
    pub accuracy: f32,
    pub accuracy_raw: [f32; 4],
    pub accuracy_add_multiplier: f32,
    pub projectile_damage: f32,
    pub projectile_damage_raw: [f32; 4],
    pub projectile_damage_add_multiplier: f32,
    pub overslime_material: f32,
    pub armor: f32,
    pub armor_add_multiplier: f32,
    pub armor_toughness: f32,
    pub armor_toughness_add_multiplier: f32,
    pub knockback_resistance: f32,
    pub knockback_resistance_add_multiplier: f32,
}

#[derive(Clone, Copy)]
struct Accumulator {
    base: f32,
    add: f32,
    percent: f32,
    multiply: f32,
}
/// The registered chestplate item and its original data definition have
/// different IDs; other namespaces and equipment names are not rewritten.
pub fn definition_name<'a>(namespace: &str, item_name: &'a str) -> &'a str {
    if namespace == "tconstruct" && item_name == "slimy_chestplate" { "slime_chestplate" } else { item_name }
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
    stats: &mut [Accumulator; 12],
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
        "skull" => {
            stats[7].update(number("armor", 0.0)? * scale);
        }
        "slime" => {
            let durability = number("durability", 1.0)? * scale;
            stats[0].update(durability);
            // SlimeStats.apply in the pinned source adds scaled durability
            // to the Overslime stat; the separate tooltip field is not used.
            stats[11].add += durability;
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

/// Source mining tier is the maximum of material stats actually applied as heads.
pub(super) fn mining_tier(definition: &[u8], materials: &[Map<String, Value>]) -> Option<Option<&'static str>> {
    let definition: Value = serde_json::from_slice(definition).ok()?;
    let mut best: Option<(u8, &'static str)> = None;
    for module in definition.get("modules")?.as_array()? {
        let (field, material_stats) = match module.get("type")?.as_str()? {
            "tconstruct:part_stats" => ("parts", false),
            "tconstruct:material_stats" => ("stat_types", true),
            _ => continue,
        };
        let parts = module.get(field)?.as_array()?;
        if parts.len() != materials.len() { return None; }
        for (part, material) in parts.iter().zip(materials) {
            let id = part.as_str().or_else(|| part.get(if material_stats { "stat" } else { "item" }).and_then(Value::as_str))?;
            let head = if material_stats { id == "tconstruct:head" } else { part_kind(id)? == "head" };
            if !head { continue; }
            let tier = material.get("tconstruct:head")?.get("mining_tier")?.as_str()?;
            let candidate = match tier {
                "minecraft:wood" => (0, "wooden"), "minecraft:gold" => (0, "gold"),
                "minecraft:stone" => (1, "stone"), "minecraft:iron" => (2, "iron"),
                "minecraft:diamond" => (3, "diamond"), "minecraft:netherite" => (4, "netherite"),
                _ => return None,
            };
            if best.is_none_or(|(level, _)| candidate.0 > level) { best = Some(candidate); }
        }
    }
    Some(best.map(|(_, tier)| tier))
}

/// Tier upgrades set a minimum without lowering stronger materials.
pub(super) fn gem_tier(tier: Option<&'static str>, diamond: bool, emerald: bool) -> Option<&'static str> {
    let rank = |t: &str| match t { "wooden" | "gold" => 0, "stone" => 1, "iron" => 2, "diamond" => 3, "netherite" => 4, _ => -1 };
    let Some(current) = tier else { return if diamond { Some("diamond") } else if emerald { Some("iron") } else { None }; };
    if diamond && rank(current) < 3 { Some("diamond") }
    else if emerald && rank(current) < 2 { Some("iron") }
    else { Some(current) }
}

/// Compute the supported shared material-derived numeric base. `materials` contains the
/// already merged source stats object for each ordered part material.
pub fn base_stats(definition: &[u8], materials: &[Map<String, Value>]) -> Option<BaseStats> {
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
        Accumulator::new(2.0),
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
        "tconstruct:projectile_damage",
        "tconstruct:overslime",
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
        durability_base_raw: stats[0].base + stats[0].add,
        durability_percent_raw: stats[0].percent,
        mining_speed: stats[1].result(),
        mining_speed_base_raw: stats[1].base,
        mining_speed_add_raw: stats[1].add,
        mining_speed_percent_raw: stats[1].percent,
        mining_speed_add_multiplier: (1.0 + stats[1].percent) * stats[1].multiply,
        mining_speed_all_multiplier: stats[1].multiply,
        attack_damage: stats[2].result(),
        attack_damage_base_raw: stats[2].base,
        attack_damage_add_raw: stats[2].add,
        attack_damage_percent_raw: stats[2].percent,
        attack_damage_boost_unit: (stats[2].base + stats[2].add) * stats[2].multiply,
        attack_damage_multiplier: (1.0 + stats[2].percent) * stats[2].multiply,
        attack_damage_multiplier_unit: stats[2].multiply,
        attack_speed: stats[3].result(),
        attack_speed_base_raw: stats[3].base,
        attack_speed_add_raw: stats[3].add,
        attack_speed_percent_raw: stats[3].percent,
        attack_speed_multiplier_unit: stats[3].multiply,
        attack_speed_boost_unit: (stats[3].base + stats[3].add) * stats[3].multiply,
        draw_speed: stats[4].result(),
        draw_speed_raw: [stats[4].base, stats[4].add, stats[4].percent, stats[4].multiply],
        draw_speed_add_multiplier: (1.0 + stats[4].percent) * stats[4].multiply,
        velocity: stats[5].result(),
        velocity_raw: [stats[5].base, stats[5].add, stats[5].percent, stats[5].multiply],
        velocity_add_multiplier: (1.0 + stats[5].percent) * stats[5].multiply,
        accuracy: stats[6].result(),
        accuracy_raw: [stats[6].base, stats[6].add, stats[6].percent, stats[6].multiply],
        accuracy_add_multiplier: (1.0 + stats[6].percent) * stats[6].multiply,
        projectile_damage: stats[10].result(),
        projectile_damage_raw: [stats[10].base, stats[10].add, stats[10].percent, stats[10].multiply],
        projectile_damage_add_multiplier: (1.0 + stats[10].percent) * stats[10].multiply,
        overslime_material: stats[11].result(),
        armor: stats[7].result(),
        armor_add_multiplier: (1.0 + stats[7].percent) * stats[7].multiply,
        armor_toughness: stats[8].result(),
        armor_toughness_add_multiplier: (1.0 + stats[8].percent) * stats[8].multiply,
        knockback_resistance: stats[9].result(),
        knockback_resistance_add_multiplier: (1.0 + stats[9].percent) * stats[9].multiply,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn skull_slime_and_chest_definition_keep_source_values() {
        let mut stats: [Accumulator; 12] = std::array::from_fn(|_| Accumulator::new(0.0));
        let skull = serde_json::json!({"tconstruct:skull":{"armor": 3}}).as_object().unwrap().clone();
        let slime = serde_json::json!({"tconstruct:slime":{"durability": 40}}).as_object().unwrap().clone();
        apply_material(&mut stats, "skull", &skull, 1.0).unwrap();
        apply_material(&mut stats, "slime", &slime, 0.5).unwrap();
        assert_eq!(stats[7].result(), 3.0);
        assert_eq!(stats[11].result(), 20.0);
        assert_eq!(definition_name("tconstruct", "slimy_chestplate"), "slime_chestplate");
        assert_eq!(definition_name("example", "slimy_chestplate"), "slimy_chestplate");
    }

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
                durability_base_raw: 85.0,
                durability_percent_raw: 0.0,
                mining_speed: 3.5,
                mining_speed_base_raw: 0.0,
                mining_speed_add_raw: 3.5,
                mining_speed_percent_raw: 0.0,
                mining_speed_add_multiplier: 1.0,
                mining_speed_all_multiplier: 1.0,
                attack_damage: 1.75,
                attack_damage_base_raw: 0.0,
                attack_damage_add_raw: 1.75,
                attack_damage_percent_raw: 0.0,
                attack_damage_boost_unit: 1.75,
                attack_damage_multiplier: 1.0,
                attack_damage_multiplier_unit: 1.0,
                attack_speed: 1.2,
                attack_speed_base_raw: 0.0,
                attack_speed_add_raw: 1.2,
                attack_speed_percent_raw: 0.0,
                attack_speed_multiplier_unit: 1.0,
                attack_speed_boost_unit: 1.2,
                draw_speed: 1.0,
                draw_speed_raw: [1.0, 0.0, 0.0, 1.0],
                draw_speed_add_multiplier: 1.0,
                velocity: 1.0,
                velocity_raw: [1.0, 0.0, 0.0, 1.0],
                velocity_add_multiplier: 1.0,
                accuracy: 0.75,
                accuracy_raw: [0.75, 0.0, 0.0, 1.0],
                accuracy_add_multiplier: 1.0,
                projectile_damage: 2.0,
                projectile_damage_raw: [2.0, 0.0, 0.0, 1.0],
                projectile_damage_add_multiplier: 1.0,
                overslime_material: 0.0,
                armor: 0.0,
                armor_add_multiplier: 1.0,
                armor_toughness: 0.0,
                armor_toughness_add_multiplier: 1.0,
                knockback_resistance: 0.0,
                knockback_resistance_add_multiplier: 1.0
            }
        );
    }
}
