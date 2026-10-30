// Shared base-stat aggregation for tool definitions using head/handle parts.
// Traits, modifiers, tier and final stat clamps are applied separately.
use serde_json::{Map, Value};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct BaseStats {
    pub durability: f32,
    pub mining_speed: f32,
    pub attack_damage: f32,
    pub attack_speed: f32,
}

#[derive(Clone, Copy)]
struct Accumulator { base: f32, add: f32, percent: f32, multiply: f32 }
impl Accumulator {
    fn new(base: f32) -> Self { Self { base, add: 0.0, percent: 0.0, multiply: 1.0 } }
    fn update(&mut self, value: f32) { self.base = 0.0; self.add += value; }
    fn result(self) -> f32 { (self.base + self.add) * (1.0 + self.percent) * self.multiply }
}

/// Compute only the shared head/handle numeric base. `materials` contains the
/// already merged source stats object for each ordered part material.
pub(super) fn base_stats(definition: &[u8], materials: &[Map<String, Value>]) -> Option<BaseStats> {
    let definition: Value = serde_json::from_slice(definition).ok()?;
    let mut stats = [Accumulator::new(1.0), Accumulator::new(1.0), Accumulator::new(0.0), Accumulator::new(1.0)];
    let keys = ["tconstruct:durability", "tconstruct:mining_speed", "tconstruct:attack_damage", "tconstruct:attack_speed"];
    for module in definition.get("modules")?.as_array()? {
        match module.get("type")?.as_str()? {
            "tconstruct:part_stats" => {
                let parts = module.get("parts")?.as_array()?;
                if parts.len() != materials.len() { return None; }
                for (part, material) in parts.iter().zip(materials) {
                    let id = part.as_str().or_else(|| part.get("item").and_then(Value::as_str))?;
                    let scale = match part.get("scale") { Some(value) => value.as_f64()? as f32, None => 1.0 };
                    let kind = match id.strip_prefix("tconstruct:")? {
                        "pick_head" | "hammer_head" | "small_axe_head" | "broad_axe_head" | "small_blade" | "broad_blade" | "adze_head" | "large_plate" => "head",
                        "tool_handle" | "tough_handle" => "handle",
                        "tool_binding" | "tough_binding" => "binding",
                        _ => return None,
                    };
                    if kind == "binding" { continue; }
                    let fields = material.get(&format!("tconstruct:{kind}"))?.as_object()?;
                    let number = |key: &str, default: f32| -> Option<f32> {
                        match fields.get(key) { Some(value) => Some(value.as_f64()? as f32), None => Some(default) }
                    };
                    match kind {
                        "head" => {
                            stats[0].update(number("durability", 0.0)? * scale);
                            stats[1].update(number("mining_speed", 0.0)? * scale);
                            stats[2].update(number("melee_attack", 0.0)? * scale);
                        },
                        "handle" => {
                            for (index, key) in [(0, "durability"), (1, "mining_speed"), (2, "melee_damage"), (3, "melee_speed")] {
                                stats[index].percent += number(key, 0.0)? * scale;
                            }
                        },
                        _ => {},
                    }
                }
            },
            "tconstruct:base_stats" | "tconstruct:multiply_stats" => {
                let multiply = module.get("type")?.as_str()? == "tconstruct:multiply_stats";
                let fields = module.get(if multiply { "multipliers" } else { "stats" })?.as_object()?;
                for (key, value) in fields {
                    let index = keys.iter().position(|known| known == key)?;
                    let value = value.as_f64()? as f32;
                    if multiply { stats[index].multiply *= value; } else { stats[index].update(value); }
                }
            },
            _ => {},
        }
    }
    Some(BaseStats { durability: stats[0].result(), mining_speed: stats[1].result(), attack_damage: stats[2].result(), attack_speed: stats[3].result() })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn combines_material_and_definition_values() {
        let definition = br#"{"modules":[{"type":"tconstruct:part_stats","parts":["tconstruct:pick_head","tconstruct:tool_handle","tconstruct:tool_binding"]},{"type":"tconstruct:base_stats","stats":{"tconstruct:attack_damage":0.5,"tconstruct:attack_speed":1.2}}]}"#;
        let head = serde_json::json!({"tconstruct:head":{"durability":85,"mining_speed":3.5,"melee_attack":1.25}}).as_object().unwrap().clone();
        let handle = serde_json::json!({"tconstruct:handle":{"durability":0,"mining_speed":0,"melee_damage":0,"melee_speed":0}}).as_object().unwrap().clone();
        let binding = serde_json::json!({"tconstruct:binding":{}}).as_object().unwrap().clone();
        let stats = base_stats(definition, &[head, handle, binding]).unwrap();
        assert_eq!(stats, BaseStats { durability: 85.0, mining_speed: 3.5, attack_damage: 1.75, attack_speed: 1.2 });
    }
}
