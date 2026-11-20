//! Original FloatToolStat/ModifierStatsBuilder and built-in material application (MIT; see LICENSE).
use crate::{material_stats::Stats, materials::Definitions, tool_materials::MaterialList};
use pluto_lib::Identifier;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Stat {
    Durability, UseItemSpeed, AttackDamage, AttackSpeed, MiningSpeed, Armor, ArmorToughness,
    KnockbackResistance, BlockAmount, BlockAngle, DrawSpeed, Velocity, Accuracy,
    ProjectileDamage, WaterInertia, SeaLuck, Lure, Overslime,
    /// EdibleModule.HUNGER, SATURATION, EAT_DURATION and COUNTER_CHANCE.
    Hunger, Saturation, EatDuration, EdibleCounterChance,
}
impl Stat {
    pub fn from_id(id: &Identifier) -> Option<Self> {
        if id.namespace() != "tconstruct" { return None; }
        Some(match id.path() {
            "durability" => Self::Durability, "use_item_speed" => Self::UseItemSpeed,
            "attack_damage" => Self::AttackDamage, "attack_speed" => Self::AttackSpeed,
            "mining_speed" => Self::MiningSpeed, "armor" => Self::Armor,
            "armor_toughness" => Self::ArmorToughness, "knockback_resistance" => Self::KnockbackResistance,
            "block_amount" => Self::BlockAmount, "block_angle" => Self::BlockAngle,
            "draw_speed" => Self::DrawSpeed, "velocity" => Self::Velocity, "accuracy" => Self::Accuracy,
            "projectile_damage" => Self::ProjectileDamage, "water_inertia" => Self::WaterInertia,
            "sea_luck" => Self::SeaLuck, "lure" => Self::Lure, "overslime" => Self::Overslime,
            "hunger" => Self::Hunger, "saturation" => Self::Saturation, "eat_duration" => Self::EatDuration,
            "edible_counter_chance" => Self::EdibleCounterChance,
            _ => return None,
        })
    }
    /// Path of the tconstruct stat identifier, the NBT key suffix.
    pub fn path(self) -> &'static str {
        match self {
            Self::Durability => "durability", Self::UseItemSpeed => "use_item_speed",
            Self::AttackDamage => "attack_damage", Self::AttackSpeed => "attack_speed",
            Self::MiningSpeed => "mining_speed", Self::Armor => "armor",
            Self::ArmorToughness => "armor_toughness", Self::KnockbackResistance => "knockback_resistance",
            Self::BlockAmount => "block_amount", Self::BlockAngle => "block_angle",
            Self::DrawSpeed => "draw_speed", Self::Velocity => "velocity", Self::Accuracy => "accuracy",
            Self::ProjectileDamage => "projectile_damage", Self::WaterInertia => "water_inertia",
            Self::SeaLuck => "sea_luck", Self::Lure => "lure", Self::Overslime => "overslime",
            Self::Hunger => "hunger", Self::Saturation => "saturation", Self::EatDuration => "eat_duration",
            Self::EdibleCounterChance => "edible_counter_chance",
        }
    }
    /// The registered default value of the stat (its limits' first entry).
    pub fn default_value(self) -> f32 { self.limits().0 }
    /// FloatToolStat.clamp: the value held inside the stat's limits.
    pub fn clamp(self, value: f32) -> f32 { let (_, min, max) = self.limits(); value.clamp(min, max) }
    fn limits(self) -> (f32, f32, f32) {
        match self {
            Self::Durability => (1.0, 1.0, i32::MAX as f32),
            Self::UseItemSpeed => (0.2, 0.0, 1.0),
            Self::AttackDamage => (0.0, 0.0, 2048.0),
            Self::AttackSpeed | Self::DrawSpeed | Self::Velocity => (1.0, 0.0, 1024.0),
            Self::MiningSpeed => (1.0, 0.1, 2048.0),
            Self::Armor => (0.0, 0.0, 30.0),
            Self::ArmorToughness => (0.0, 0.0, 20.0),
            Self::KnockbackResistance => (0.0, 0.0, 1.0),
            Self::BlockAmount => (5.0, 0.0, 2048.0),
            Self::BlockAngle => (120.0, 0.0, 180.0),
            Self::Accuracy => (0.75, 0.1, 1.0),
            Self::ProjectileDamage => (2.0, 0.0, 1024.0),
            Self::WaterInertia => (0.6, 0.01, 0.99),
            Self::SeaLuck => (0.0, 0.0, 1024.0),
            Self::Lure => (0.0, 0.0, 5.0),
            Self::Overslime => (0.0, 0.0, i16::MAX as f32),
            Self::Hunger | Self::Saturation => (0.0, 0.0, 200.0),
            Self::EatDuration => (16.0, 0.0, 100.0),
            Self::EdibleCounterChance => (0.0, 0.0, 1.0),
        }
    }
}
#[derive(Clone, Debug)]
struct FloatBuilder { base: f32, add: f32, percent: f32, multiply: f32 }
#[derive(Clone, Debug, Default)]
pub(crate) struct Builder {
    values: BTreeMap<Stat, FloatBuilder>,
    multipliers: BTreeMap<Stat, f32>,
    tier: Option<Identifier>,
}
impl Builder {
    fn entry(&mut self, stat: Stat) -> &mut FloatBuilder {
        self.values.entry(stat).or_insert_with(|| FloatBuilder { base: stat.limits().0, add: 0.0, percent: 0.0, multiply: 1.0 })
    }
    /// update sums explicit values and removes the implicit base; it is not assignment.
    pub fn update(&mut self, stat: Stat, value: f32) { let b = self.entry(stat); b.add += value; b.base = 0.0; }
    pub fn add(&mut self, stat: Stat, value: f64) { let b = self.entry(stat); b.add = (f64::from(b.add) + value) as f32; }
    pub fn percent(&mut self, stat: Stat, value: f64) { let b = self.entry(stat); b.percent = (f64::from(b.percent) + value) as f32; }
    pub fn multiply(&mut self, stat: Stat, value: f64) { let b = self.entry(stat); b.multiply = (f64::from(b.multiply) * value) as f32; }
    pub fn multiply_all(&mut self, stat: Stat, value: f64) {
        self.multiply(stat, value);
        let multiplier = self.multipliers.entry(stat).or_insert(1.0);
        *multiplier = (f64::from(*multiplier) * value) as f32;
    }
    /// ModifierStatsBuilder.multiplier: records only the global multiplier, not the stat's own.
    pub fn multiplier_only(&mut self, stat: Stat, value: f64) {
        let multiplier = self.multipliers.entry(stat).or_insert(1.0);
        *multiplier = (f64::from(*multiplier) * value) as f32;
    }
    pub fn multiplier(&self, stat: Stat) -> f32 { self.multipliers.get(&stat).copied().unwrap_or(1.0) }
    /// Intermediate queries are intentionally unclamped, as in ModifierStatsBuilder.getStat.
    pub fn get(&self, stat: Stat) -> f32 {
        let Some(b) = self.values.get(&stat) else { return stat.limits().0; };
        let value = (b.base + b.add) * (1.0 + b.percent) * b.multiply;
        if stat == Stat::Overslime { value * self.multiplier(Stat::Durability) } else { value }
    }
    pub fn build(&self) -> BTreeMap<Stat, f32> {
        self.values.keys().filter_map(|&stat| {
            let value = self.get(stat);
            let (default, min, max) = stat.limits();
            // Java Float.equals distinguishes signed zeros and treats all NaNs as equal.
            if value.to_bits() == default.to_bits() { return None; }
            Some((stat, value.clamp(min, max)))
        }).collect()
    }
    /// MultiplierNBT.Builder.set: exactly 1.0 is omitted and negatives clamp to zero.
    pub fn build_multipliers(&self) -> BTreeMap<Stat, f32> {
        self.multipliers.iter().filter(|(_, value)| **value != 1.0)
            .map(|(stat, value)| (*stat, value.max(0.0))).collect()
    }
    pub fn tier(&self) -> Identifier { self.tier.clone().unwrap_or_else(|| Identifier::parse("minecraft:wood").unwrap()) }
    pub(crate) fn update_tier(&mut self, tier: &Identifier) {
        let rank = |tier: &Identifier| ["wood", "gold", "stone", "iron", "diamond", "netherite"].iter().position(|name| *name == tier.path()).unwrap_or(0);
        // HarvestTiers.max stores the larger tier even when it equals the default, so the
        // stat key exists (and is serialized) once any contribution touched it.
        if self.tier.is_none() || rank(tier) > rank(&self.tier()) { self.tier = Some(tier.clone()); }
    }
    /// The tier only if some module contributed one; absent means no stat key at all.
    pub fn touched_tier(&self) -> Option<Identifier> { self.tier.clone() }
    pub fn apply(&mut self, stats: &Stats, scale: f32) {
        use Stat::*;
        match *stats {
            Stats::Head { durability, mining_speed, ref tier, attack } => {
                self.update(Durability, durability as f32 * scale);
                self.update(AttackDamage, attack * scale);
                self.update(MiningSpeed, mining_speed * scale);
                self.update_tier(tier); // tier is never scaled
            }
            Stats::Handle { durability, mining_speed, melee_speed, attack } => {
                self.percent(Durability, f64::from(durability * scale));
                self.percent(AttackDamage, f64::from(attack * scale));
                self.percent(AttackSpeed, f64::from(melee_speed * scale));
                self.percent(MiningSpeed, f64::from(mining_speed * scale));
            }
            Stats::Limb { durability, draw_speed, velocity, accuracy } => {
                self.update(Durability, durability as f32 * scale);
                self.add(DrawSpeed, f64::from(draw_speed * scale));
                self.add(Velocity, f64::from(velocity * scale));
                self.add(Accuracy, f64::from(accuracy * scale));
            }
            Stats::Grip { durability, accuracy, attack } => {
                self.percent(Durability, f64::from(durability * scale));
                self.add(Accuracy, f64::from(accuracy * scale));
                self.update(AttackDamage, attack * scale);
            }
            Stats::Plating { durability, armor, toughness, knockback_resistance } => {
                self.update(Durability, durability as f32 * scale);
                self.update(Armor, armor * scale);
                self.update(ArmorToughness, toughness * scale);
                self.update(KnockbackResistance, knockback_resistance * scale);
            }
            Stats::Skull { armor, .. } => self.update(Armor, armor as f32 * scale),
            Stats::Slime { durability, .. } => {
                self.update(Durability, durability as f32 * scale);
                // This exact Java revision uses durability, NOT overslime_capacity, here.
                self.add(Overslime, f64::from(durability as f32 * scale));
            }
            Stats::Repair { .. } | Stats::Statless => {}
        }
    }
    /// MaterialStatsModule: an empty material list contributes nothing. A short
    /// nonempty list still applies registered defaults for each missing position.
    pub fn apply_materials(&mut self, materials: &MaterialList, stat_types: &[(Identifier, f32)], registry: &Definitions) {
        if materials.as_slice().is_empty() { return; }
        for (index, (stat, scale)) in stat_types.iter().enumerate() {
            let material = materials.as_slice().get(index).cloned().unwrap_or_else(crate::tool_materials::MaterialVariant::unknown);
            let stats = material.material_id().and_then(|id| registry.get_stats(&id, stat));
            if let Some(stats) = stats { self.apply(stats, *scale); }
            else if let Some(default) = crate::material_stats::default_stats(stat) { self.apply(&default, *scale); }
        }
    }
}

/// Typed statistical contribution of a tool-definition module. The outer module
/// dispatcher must still install its other hooks (traits, repair, volatile data).
#[derive(Clone, Debug)]
pub(crate) enum StatModule {
    Base { numbers: Vec<(Stat, f32)>, tier: Option<Identifier> },
    Multiply(Vec<(Stat, f32)>),
    Materials { types: Vec<(Identifier, f32)>, parts: Option<Vec<Identifier>>, primary: i32 },
}
impl StatModule {
    pub fn decode(value: &serde_json::Value, parts: &BTreeMap<Identifier, crate::tool_materials::ToolPart>) -> Result<Self, String> {
        use serde_json::Value;
        let object = value.as_object().ok_or("tool module must be an object")?;
        let id = |value: &Value| -> Result<Identifier, String> {
            Identifier::parse(value.as_str().ok_or("expected ID string")?).map_err(|e| e.to_string())
        };
        let float = |value: &Value| value.as_f64().map(|v| v as f32).ok_or_else(|| "expected numeric stat value".to_string());
        let kind = id(object.get("type").ok_or("module type is required")?)?;
        if kind.namespace() != "tconstruct" { return Err(format!("unsupported tool stat module {kind}")); }
        match kind.path() {
            "base_stats" | "multiply_stats" => {
                let multiplying = kind.path() == "multiply_stats";
                let key = if multiplying { "multipliers" } else { "stats" };
                let map = object.get(key).and_then(Value::as_object).ok_or_else(|| format!("{key} must be an object"))?;
                let mut numbers = Vec::new();
                let mut tier = None;
                for (name, value) in map {
                    let name = Identifier::parse(name).map_err(|e| e.to_string())?;
                    if !multiplying && name.to_string() == "tconstruct:harvest_tier" {
                        let parsed = id(value)?;
                        if parsed.namespace() != "minecraft" || !["wood", "gold", "stone", "iron", "diamond", "netherite"].contains(&parsed.path()) {
                            return Err(format!("unknown tool tier {parsed}"));
                        }
                        tier = Some(parsed);
                    } else {
                        let stat = Stat::from_id(&name).ok_or_else(|| format!("unsupported numeric tool stat {name}"))?;
                        numbers.push((stat, float(value)?));
                    }
                }
                if multiplying { Ok(Self::Multiply(numbers)) } else { Ok(Self::Base { numbers, tier }) }
            }
            "material_stats" | "part_stats" => {
                let is_parts = kind.path() == "part_stats";
                let key = if is_parts { "parts" } else { "stat_types" };
                let nested = if is_parts { "item" } else { "stat" };
                let values = object.get(key).and_then(Value::as_array).filter(|values| !values.is_empty())
                    .ok_or_else(|| format!("{key} requires a nonempty array"))?;
                let mut types = Vec::new();
                let mut part_ids = Vec::new();
                for value in values {
                    let (name, scale) = if let Some(fields) = value.as_object() {
                        // StatScaleField requires scale for object form; it does not default to one.
                        (id(fields.get(nested).ok_or_else(|| format!("missing {nested}"))?)?,
                         float(fields.get("scale").ok_or("nested material input requires scale")?)?)
                    } else { (id(value)?, 1.0) };
                    let stat = if is_parts {
                        part_ids.push(name.clone());
                        parts.get(&name).ok_or_else(|| format!("unknown tool part {name}"))?.stat.clone()
                    } else { name };
                    types.push((stat, scale));
                }
                let primary = match object.get("primary_part").filter(|v| !v.is_null()) {
                    None => 0,
                    Some(Value::Number(n)) => crate::materials::number_int(&n.to_string()),
                    _ => return Err("primary_part must be numeric".into()),
                };
                // Pinned Mantle FROM_MINUS_ONE actually caps at Short.MAX_VALUE,
                // despite its comment saying integer max. The Java convenience
                // builder's separate part-count bound is not imposed by JSON.
                if !(-1..=i16::MAX as i32).contains(&primary) { return Err("primary_part must be between -1 and 32767".into()); }
                Ok(Self::Materials { types, parts: is_parts.then_some(part_ids), primary })
            }
            _ => Err(format!("unsupported tool stat module {kind}")),
        }
    }
    pub fn apply(&self, builder: &mut Builder, materials: &MaterialList, registry: &Definitions) {
        match self {
            Self::Base { numbers, tier } => {
                for &(stat, value) in numbers { builder.update(stat, value); }
                if let Some(tier) = tier { builder.update_tier(tier); }
            }
            Self::Multiply(numbers) => for &(stat, value) in numbers { builder.multiply_all(stat, f64::from(value)); },
            Self::Materials { types, .. } => builder.apply_materials(materials, types, registry),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::Nbt;
    #[test]
    fn stat_module_decoding_preserves_required_scales_and_does_not_fake_unknown_types() {
        use serde_json::json;
        let parts = BTreeMap::new();
        let valid = json!({"type":"tconstruct:material_stats", "stat_types":["tconstruct:head", {"stat":"tconstruct:handle", "scale":-0.5}], "primary_part":99});
        let StatModule::Materials { types, primary, parts: ids } = StatModule::decode(&valid, &parts).unwrap() else { panic!("material module"); };
        assert_eq!(types[1].1, -0.5);
        assert_eq!(primary, 99, "JSON constructor accepts out-of-range primary; it simply adds no primary trait hook");
        assert!(ids.is_none());
        for invalid in [
            json!({"type":"tconstruct:material_stats", "stat_types":[], "primary_part":0}),
            json!({"type":"tconstruct:material_stats", "stat_types":["tconstruct:head"], "primary_part":32768}),
            json!({"type":"tconstruct:material_stats", "stat_types":["tconstruct:head"], "primary_part":-2}),
            json!({"type":"tconstruct:material_stats", "stat_types":[{"stat":"tconstruct:head"}]}),
            json!({"type":"tconstruct:part_stats", "parts":["test:unregistered"]}),
            json!({"type":"tconstruct:base_stats", "stats":{"test:missing":2}}),
            json!({"type":"tconstruct:multiply_stats", "multipliers":{"tconstruct:harvest_tier":2}}),
            json!({"type":"tconstruct:tool_actions", "tool_actions":[]}),
        ] { assert!(StatModule::decode(&invalid, &parts).is_err(), "{invalid}"); }
    }
    #[test]
    fn updates_additions_percentages_and_global_multipliers_keep_java_order() {
        let mut b = Builder::default();
        b.add(Stat::AttackSpeed, 0.5);
        assert_eq!(b.get(Stat::AttackSpeed), 1.5);
        b.update(Stat::AttackSpeed, 1.2);
        b.update(Stat::AttackSpeed, 0.3);
        b.percent(Stat::AttackSpeed, 0.5);
        b.multiply_all(Stat::AttackSpeed, 2.0);
        assert_eq!(b.get(Stat::AttackSpeed), 6.0);
        assert_eq!(b.multiplier(Stat::AttackSpeed), 2.0);
        b.update(Stat::Armor, 50.0);
        assert_eq!(b.get(Stat::Armor), 50.0);
        assert_eq!(b.build()[&Stat::Armor], 30.0);
        assert!(!b.build().contains_key(&Stat::Durability));
    }
    #[test]
    fn material_application_distinguishes_empty_missing_and_repair_only_parts() {
        let types = vec![(Identifier::parse("tconstruct:head").unwrap(), 2.0)];
        let mut b = Builder::default();
        b.apply_materials(&MaterialList::default(), &types, &Definitions::default());
        assert_eq!(b.get(Stat::Durability), 1.0);
        let materials = MaterialList::from_nbt(Some(&Nbt::List(vec![Nbt::String("test:missing".into())])));
        b.apply_materials(&materials, &types, &Definitions::default());
        assert_eq!(b.get(Stat::Durability), 2.0);
        b.apply(&Stats::Repair { durability: 500 }, 1.0);
        b.apply(&Stats::Skull { durability: 300, armor: 4 }, 0.5);
        assert_eq!(b.get(Stat::Durability), 2.0);
        assert_eq!(b.get(Stat::Armor), 2.0);
        b.apply(&Stats::Slime { durability: 100, overslime: 9 }, 0.5);
        b.multiply_all(Stat::Durability, 2.0);
        assert_eq!(b.get(Stat::Overslime), 100.0);
    }
    #[test]
    fn heads_sum_scaled_values_but_tier_uses_maximum_even_with_zero_scale() {
        let mut b = Builder::default();
        b.apply(&Stats::Head { durability:100, mining_speed:4.0, attack:2.0, tier:Identifier::parse("minecraft:iron").unwrap() }, 0.5);
        b.apply(&Stats::Head { durability:200, mining_speed:6.0, attack:4.0, tier:Identifier::parse("minecraft:diamond").unwrap() }, 0.0);
        b.apply(&Stats::Handle { durability:0.2, mining_speed:0.5, melee_speed:0.1, attack:0.25 }, 1.0);
        assert!((b.get(Stat::Durability) - 60.0).abs() < 0.00001);
        assert_eq!(b.get(Stat::MiningSpeed), 3.0);
        assert_eq!(b.get(Stat::AttackDamage), 1.25);
        assert_eq!(b.tier().to_string(), "minecraft:diamond");
    }
}
