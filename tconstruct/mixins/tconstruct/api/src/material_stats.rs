//! MaterialStatsManager and original built-in stat records (SlimeKnights, MIT; see LICENSE).
use pluto_lib::{Identifier, JsonResourceStack};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

pub(crate) const FOLDER: &str = "tinkering/materials/stats";
pub(crate) type MaterialStats = BTreeMap<Identifier, BTreeMap<Identifier, Stats>>;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Stats {
    Head { durability: i32, mining_speed: f32, tier: Identifier, attack: f32 },
    Handle { durability: f32, mining_speed: f32, melee_speed: f32, attack: f32 },
    Limb { durability: i32, draw_speed: f32, velocity: f32, accuracy: f32 },
    Grip { durability: f32, accuracy: f32, attack: f32 },
    Plating { durability: i32, armor: f32, toughness: f32, knockback_resistance: f32 },
    Skull { durability: i32, armor: i32 },
    Slime { durability: i32, overslime: i32 },
    Repair { durability: i32 },
    Statless,
}

fn int(json: &Map<String, Value>, key: &str, minimum: i32, default: Option<i32>) -> Result<i32, String> {
    let value = match json.get(key).filter(|v| !v.is_null()) {
        Some(Value::Number(n)) => super::materials::number_int(&n.to_string()),
        None => default.ok_or_else(|| format!("missing {key}"))?,
        _ => return Err(format!("{key} must be a number")),
    };
    if value < minimum { return Err(format!("{key} must not be less than {minimum}")); }
    Ok(value)
}
fn float(json: &Map<String, Value>, key: &str, default: f32, minimum: f32) -> Result<f32, String> {
    let value = match json.get(key).filter(|v| !v.is_null()) {
        None => default,
        Some(Value::Number(n)) => n.as_f64().ok_or_else(|| format!("invalid {key}"))? as f32,
        _ => return Err(format!("{key} must be a number")),
    };
    if value < minimum || value.is_nan() { return Err(format!("{key} must not be less than {minimum}")); }
    Ok(value)
}
fn decode(id: &Identifier, json: &Map<String, Value>) -> Result<Option<Stats>, String> {
    if id.namespace() != "tconstruct" { return Ok(None); }
    let durability = || int(json, "durability", 1, None);
    let positive = |key, default| float(json, key, default, 0.0);
    let any = |key| float(json, key, 0.0, f32::NEG_INFINITY);
    Ok(Some(match id.path() {
        "head" => {
            let name = match json.get("mining_tier").filter(|v| !v.is_null()) {
                None => "minecraft:wood".to_owned(), Some(Value::String(s)) => s.clone(),
                Some(Value::Number(n)) => n.to_string(), Some(Value::Bool(b)) => b.to_string(),
                _ => return Err("mining_tier must be a primitive".into()),
            };
            let tier = Identifier::parse(&name).map_err(|e| e.to_string())?;
            // No other-mod integration: the original mod adds no harvest tiers.
            if tier.namespace() != "minecraft" || !matches!(tier.path(), "wood" | "gold" | "stone" | "iron" | "diamond" | "netherite") {
                return Err(format!("Unknown harvest tier {tier}"));
            }
            Stats::Head { durability: durability()?, mining_speed: positive("mining_speed", 1.0)?, tier, attack: positive("melee_attack", 1.0)? }
        }
        "handle" => Stats::Handle { durability: any("durability")?, mining_speed: any("mining_speed")?, melee_speed: any("melee_speed")?, attack: any("melee_damage")? },
        "limb" => Stats::Limb { durability: durability()?, draw_speed: any("draw_speed")?, velocity: any("velocity")?, accuracy: any("accuracy")? },
        "grip" => Stats::Grip { durability: any("durability")?, accuracy: any("accuracy")?, attack: positive("melee_damage", 0.0)? },
        "plating_helmet" | "plating_chestplate" | "plating_leggings" | "plating_boots" | "plating_shield" => Stats::Plating {
            durability: durability()?, armor: if id.path() == "plating_shield" { 0.0 } else { positive("armor", 0.0)? },
            toughness: positive("toughness", 0.0)?, knockback_resistance: positive("knockback_resistance", 0.0)?,
        },
        "skull" => Stats::Skull { durability: durability()?, armor: int(json, "armor", 0, Some(0))? },
        "slime" => Stats::Slime { durability: durability()?, overslime: int(json, "overslime_capacity", 0, None)? },
        "ribcage" | "shell" | "laces" => Stats::Repair { durability: int(json, "repair_amount", 1, None)? },
        "binding" | "bowstring" | "maille" | "shield_core" | "repair_kit" | "cuirass" | "arrow_head" | "arrow_shaft" | "fletching" => Stats::Statless,
        _ => return Ok(None),
    }))
}

/// Whether `id` is a registered built-in stat type (an unregistered tconstruct path decodes to None).
pub(crate) fn is_registered(id: &Identifier) -> bool {
    let json = serde_json::json!({"durability":1,"overslime_capacity":0,"repair_amount":1});
    matches!(decode(id, json.as_object().expect("constant object")), Ok(Some(_)))
}

/// Defaults of registered MaterialStatType records, not defaults for absent JSON files.
pub(crate) fn default_stats(id: &Identifier) -> Option<Stats> {
    let defaults = serde_json::json!({"durability":1,"overslime_capacity":0,"repair_amount":1});
    // Handle's durability is a multiplier and its registered default is zero.
    let defaults = if id.namespace() == "tconstruct" && matches!(id.path(), "handle" | "grip") {
        serde_json::json!({})
    } else { defaults };
    decode(id, defaults.as_object().expect("constant object")).expect("valid built-in defaults")
}

fn merge(builder: &mut BTreeMap<Identifier, Map<String, Value>>, value: &Value) -> Result<(), String> {
    let object = value.as_object().ok_or("stats document must be an object")?;
    let Some(stats) = object.get("stats").filter(|v| !v.is_null()) else { return Ok(()); };
    // Gson first deserializes all resource-location keys, before parse mutates the builder.
    let entries = stats.as_object().ok_or("stats must be an object")?.iter()
        .map(|(id, value)| Identifier::parse(id).map(|id| (id, value)).map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    for (id, value) in entries {
        if value.is_null() { builder.remove(&id); }
        else {
            // Deliberately shallow; MaterialStatsManager replaces each field's whole value.
            let fields = value.as_object().ok_or_else(|| format!("stats {id} must be an object"))?;
            builder.entry(id).or_default().extend(fields.clone());
        }
    }
    Ok(())
}

pub(crate) fn load(stacks: &[JsonResourceStack], problems: &mut Vec<String>) -> MaterialStats {
    let mut builders = BTreeMap::<Identifier, BTreeMap<Identifier, Map<String, Value>>>::new();
    for stack in stacks {
        for (layer, value) in stack.layers.iter().enumerate() {
            let result = match value {
                Ok(value) if !value.is_null() => merge(builders.entry(stack.id.clone()).or_default(), value),
                Ok(_) => Err("null or empty stats document".into()), Err(error) => Err(error.clone()),
            };
            if let Err(error) = result { problems.push(format!("Could not read material stats {} layer {layer}: {error}", stack.id)); }
        }
    }
    builders.into_iter().map(|(material, builder)| {
        let mut stats = BTreeMap::new();
        for (id, json) in builder {
            match decode(&id, &json) {
                Ok(Some(value)) => { stats.insert(id, value); }
                Ok(None) => {
                    let optional = match json.get("optional") {
                        None => Ok(false), Some(Value::Bool(b)) => Ok(*b),
                        Some(Value::String(s)) => Ok(s.eq_ignore_ascii_case("true")), Some(Value::Number(_)) => Ok(false),
                        _ => Err("optional must be a primitive"),
                    };
                    if optional != Ok(true) { problems.push(format!("Skipping unregistered material stat {id} for {material}: {optional:?}")); }
                }
                Err(error) => problems.push(format!("Failed to parse material stat {id} for {material}: {error}")),
            }
        }
        (material, stats)
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn id(s: &str) -> Identifier { Identifier::parse(s).unwrap() }
    #[test]
    fn fields_merge_before_validation_and_null_removes_a_type() {
        let mut errors = Vec::new();
        let loaded = load(&[JsonResourceStack { id: id("test:wood"), layers: vec![
            Ok(json!({"stats":{"tconstruct:head":{"durability":20,"mining_speed":2},"tconstruct:binding":{}}})),
            Ok(json!({"stats":{"tconstruct:head":{"melee_attack":4},"tconstruct:binding":null}})),
            Ok(json!({"stats":{"tconstruct:head":{"mining_speed":null},"tconstruct:limb":{"durability":0},
                "test:optional":{"optional":true},"test:required":{}}})),
        ]}], &mut errors);
        let stats = &loaded[&id("test:wood")];
        assert_eq!(stats[&id("tconstruct:head")], Stats::Head { durability:20, mining_speed:1.0, tier:id("minecraft:wood"), attack:4.0 });
        assert!(!stats.contains_key(&id("tconstruct:binding")));
        assert!(!stats.contains_key(&id("tconstruct:limb")));
        assert_eq!(errors.len(), 2);
    }
    #[test]
    fn distinct_stat_contracts_and_numeric_bounds() {
        let decode_value = |name, value: Value| decode(&id(name), value.as_object().unwrap()).unwrap().unwrap();
        assert_eq!(decode_value("tconstruct:handle", json!({"durability":-0.3})), Stats::Handle { durability:-0.3, mining_speed:0.0, melee_speed:0.0, attack:0.0 });
        assert_eq!(decode_value("tconstruct:plating_shield", json!({"durability":10,"armor":"ignored"})), Stats::Plating { durability:10, armor:0.0, toughness:0.0, knockback_resistance:0.0 });
        assert_eq!(decode_value("tconstruct:slime", json!({"durability":10.9,"overslime_capacity":4})), Stats::Slime { durability:10, overslime:4 });
        assert_eq!(decode_value("tconstruct:shell", json!({"repair_amount":8})), Stats::Repair { durability:8 });
        assert!(decode(&id("tconstruct:grip"), json!({"melee_damage":-1}).as_object().unwrap()).is_err());
        assert!(decode(&id("tconstruct:head"), json!({"durability":10,"mining_tier":"test:missing"}).as_object().unwrap()).is_err());
        assert!(decode(&id("tconstruct:head"), json!({"durability":"10"}).as_object().unwrap()).is_err());
    }
}
