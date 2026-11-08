//! `tools.stats`: the stats a material gives a part (`HeadMaterialStats`, `HandleMaterialStats`, `LimbMaterialStats`, `GripMaterialStats`,
//! `PlatingMaterialStats`, `SkullStats`, `SlimeStats`, `RepairStats`, `StatlessMaterialStats`), and the registration the registry does for
//! them in `MaterialRegistry`'s constructor.
use std::any::Any;
use std::sync::Arc;

use serde_json::{Map, Value};

use super::stat::{ModifierStatsBuilder, StatValue, ToolStatId};
use crate::materials::registry::MaterialRegistry;
use crate::materials::stats::{singleton, MaterialStatType, MaterialStats};
use crate::materials::MaterialStatsId;
use crate::resource::ResourceId;

fn id(name: &str) -> MaterialStatsId {
    MaterialStatsId::from_resource(ResourceId::try_new("tconstruct", name).expect("valid name"))
}

fn stat(name: &str) -> ToolStatId {
    ToolStatId::tconstruct(name)
}

/// `FloatLoadable.ANY / FROM_ZERO .defaultField`.
fn float(object: &Map<String, Value>, key: &str, default: f32, from_zero: bool) -> Result<f32, String> {
    let value = match object.get(key) {
        None => return Ok(default),
        Some(value) => value.as_f64().ok_or_else(|| format!("'{key}' must be a number"))? as f32,
    };
    if from_zero && value < 0.0 {
        return Err(format!("'{key}' must be at least 0, was {value}"));
    }
    Ok(value)
}

/// `IntLoadable.FROM_ONE.requiredField` and its relatives.
fn integer(object: &Map<String, Value>, key: &str, default: Option<i32>, min: i32) -> Result<i32, String> {
    let value = match (object.get(key), default) {
        (None, Some(default)) => return Ok(default),
        (None, None) => return Err(format!("missing '{key}'")),
        (Some(value), _) => value.as_i64().ok_or_else(|| format!("'{key}' must be a whole number"))? as i32,
    };
    if value < min {
        return Err(format!("'{key}' must be at least {min}, was {value}"));
    }
    Ok(value)
}

/// The durability field every repairable stat starts with: required, at least 1.
fn durability(object: &Map<String, Value>) -> Result<i32, String> {
    integer(object, "durability", None, 1)
}

fn tier(object: &Map<String, Value>) -> Result<ResourceId, String> {
    match object.get("mining_tier") {
        None => Ok(ResourceId::try_new("minecraft", "wood").unwrap()),
        Some(value) => value.as_str().and_then(ResourceId::try_parse).ok_or_else(|| "'mining_tier' must be a tier id".to_owned()),
    }
}

macro_rules! lazy_id {
    ($name:ident, $text:expr) => {
        fn $name() -> &'static MaterialStatsId {
            static ID: std::sync::OnceLock<MaterialStatsId> = std::sync::OnceLock::new();
            ID.get_or_init(|| id($text))
        }
    };
}

/// `HeadMaterialStats`.
#[derive(Clone, Debug, PartialEq)]
pub struct HeadMaterialStats {
    pub durability: i32,
    pub mining_speed: f32,
    pub tier: ResourceId,
    pub attack: f32,
}

lazy_id!(head_id, "head");

impl HeadMaterialStats {
    pub fn kind() -> MaterialStatType {
        MaterialStatType {
            id: head_id().clone(),
            default_stats: Arc::new(Self { durability: 1, mining_speed: 1.0, tier: ResourceId::try_new("minecraft", "wood").unwrap(), attack: 1.0 }),
            loader: Arc::new(|o| {
                Ok(Arc::new(Self { durability: durability(o)?, mining_speed: float(o, "mining_speed", 1.0, true)?, tier: tier(o)?, attack: float(o, "melee_attack", 1.0, true)? }) as Arc<dyn MaterialStats>)
            }),
            can_repair: true,
        }
    }
}

impl MaterialStats for HeadMaterialStats {
    fn id(&self) -> &MaterialStatsId {
        head_id()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn apply(&self, b: &mut ModifierStatsBuilder<'_>, scale: f32) {
        b.update(&stat("durability"), &StatValue::Float(self.durability as f32 * scale));
        b.update(&stat("attack_damage"), &StatValue::Float(self.attack * scale));
        b.update(&stat("mining_speed"), &StatValue::Float(self.mining_speed * scale));
        b.update(&stat("harvest_tier"), &StatValue::Tier(self.tier.clone()));
    }
    fn repair_durability(&self) -> Option<i32> {
        Some(self.durability)
    }
}

/// `HandleMaterialStats`: percentages that change the head's numbers.
#[derive(Clone, Debug, PartialEq)]
pub struct HandleMaterialStats {
    pub durability: f32,
    pub mining_speed: f32,
    pub melee_speed: f32,
    pub attack_damage: f32,
}

lazy_id!(handle_id, "handle");

impl HandleMaterialStats {
    pub fn kind() -> MaterialStatType {
        MaterialStatType {
            id: handle_id().clone(),
            default_stats: Arc::new(Self { durability: 0.0, mining_speed: 0.0, melee_speed: 0.0, attack_damage: 0.0 }),
            loader: Arc::new(|o| {
                Ok(Arc::new(Self {
                    durability: float(o, "durability", 0.0, false)?,
                    mining_speed: float(o, "mining_speed", 0.0, false)?,
                    melee_speed: float(o, "melee_speed", 0.0, false)?,
                    attack_damage: float(o, "melee_damage", 0.0, false)?,
                }) as Arc<dyn MaterialStats>)
            }),
            can_repair: false,
        }
    }

    /// `Builder.percents().build()` with multipliers: a factor of 1.2 is +0.2, rounded to hundredths.
    pub fn from_multipliers(durability: f32, mining_speed: f32, attack_speed: f32, attack_damage: f32) -> Self {
        let percent = |m: f32| ((m * 100.0).round() - 100.0) / 100.0;
        Self { durability: percent(durability), mining_speed: percent(mining_speed), melee_speed: percent(attack_speed), attack_damage: percent(attack_damage) }
    }
}

impl MaterialStats for HandleMaterialStats {
    fn id(&self) -> &MaterialStatsId {
        handle_id()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn apply(&self, b: &mut ModifierStatsBuilder<'_>, scale: f32) {
        b.percent(&stat("durability"), f64::from(self.durability * scale));
        b.percent(&stat("attack_damage"), f64::from(self.attack_damage * scale));
        b.percent(&stat("attack_speed"), f64::from(self.melee_speed * scale));
        b.percent(&stat("mining_speed"), f64::from(self.mining_speed * scale));
    }
}

/// `LimbMaterialStats`.
#[derive(Clone, Debug, PartialEq)]
pub struct LimbMaterialStats {
    pub durability: i32,
    pub draw_speed: f32,
    pub velocity: f32,
    pub accuracy: f32,
}

lazy_id!(limb_id, "limb");

impl LimbMaterialStats {
    pub fn kind() -> MaterialStatType {
        MaterialStatType {
            id: limb_id().clone(),
            default_stats: Arc::new(Self { durability: 1, draw_speed: 0.0, velocity: 0.0, accuracy: 0.0 }),
            loader: Arc::new(|o| {
                Ok(Arc::new(Self { durability: durability(o)?, draw_speed: float(o, "draw_speed", 0.0, false)?, velocity: float(o, "velocity", 0.0, false)?, accuracy: float(o, "accuracy", 0.0, false)? }) as Arc<dyn MaterialStats>)
            }),
            can_repair: true,
        }
    }
}

impl MaterialStats for LimbMaterialStats {
    fn id(&self) -> &MaterialStatsId {
        limb_id()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn apply(&self, b: &mut ModifierStatsBuilder<'_>, scale: f32) {
        b.update(&stat("durability"), &StatValue::Float(self.durability as f32 * scale));
        b.add(&stat("draw_speed"), f64::from(self.draw_speed * scale));
        b.add(&stat("velocity"), f64::from(self.velocity * scale));
        b.add(&stat("accuracy"), f64::from(self.accuracy * scale));
    }
    fn repair_durability(&self) -> Option<i32> {
        Some(self.durability)
    }
}

/// `GripMaterialStats`.
#[derive(Clone, Debug, PartialEq)]
pub struct GripMaterialStats {
    pub durability: f32,
    pub accuracy: f32,
    pub melee_damage: f32,
}

lazy_id!(grip_id, "grip");

impl GripMaterialStats {
    pub fn kind() -> MaterialStatType {
        MaterialStatType {
            id: grip_id().clone(),
            default_stats: Arc::new(Self { durability: 0.0, accuracy: 0.0, melee_damage: 0.0 }),
            loader: Arc::new(|o| {
                Ok(Arc::new(Self { durability: float(o, "durability", 0.0, false)?, accuracy: float(o, "accuracy", 0.0, false)?, melee_damage: float(o, "melee_damage", 0.0, true)? }) as Arc<dyn MaterialStats>)
            }),
            can_repair: false,
        }
    }
}

impl MaterialStats for GripMaterialStats {
    fn id(&self) -> &MaterialStatsId {
        grip_id()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn apply(&self, b: &mut ModifierStatsBuilder<'_>, scale: f32) {
        b.percent(&stat("durability"), f64::from(self.durability * scale));
        b.add(&stat("accuracy"), f64::from(self.accuracy * scale));
        b.update(&stat("attack_damage"), &StatValue::Float(self.melee_damage * scale));
    }
}

/// `PlatingMaterialStats` for the four armor slots and the shield (`armor` is 0 for the shield).
#[derive(Clone, Debug, PartialEq)]
pub struct PlatingMaterialStats {
    pub kind: MaterialStatsId,
    pub durability: i32,
    pub armor: f32,
    pub toughness: f32,
    pub knockback_resistance: f32,
}

/// The names of the plating stat types, in the order of `PlatingMaterialStats.TYPES`.
pub const PLATING_TYPES: [&str; 5] = ["plating_helmet", "plating_chestplate", "plating_leggings", "plating_boots", "plating_shield"];

impl PlatingMaterialStats {
    pub fn kinds() -> Vec<MaterialStatType> {
        PLATING_TYPES
            .iter()
            .map(|name| {
                let kind = id(name);
                let shield = *name == "plating_shield";
                let loader_kind = kind.clone();
                MaterialStatType {
                    id: kind.clone(),
                    default_stats: Arc::new(Self { kind, durability: 1, armor: 0.0, toughness: 0.0, knockback_resistance: 0.0 }),
                    loader: Arc::new(move |o| {
                        Ok(Arc::new(Self {
                            kind: loader_kind.clone(),
                            durability: durability(o)?,
                            armor: if shield { 0.0 } else { float(o, "armor", 0.0, true)? },
                            toughness: float(o, "toughness", 0.0, true)?,
                            knockback_resistance: float(o, "knockback_resistance", 0.0, true)?,
                        }) as Arc<dyn MaterialStats>)
                    }),
                    can_repair: true,
                }
            })
            .collect()
    }
}

impl MaterialStats for PlatingMaterialStats {
    fn id(&self) -> &MaterialStatsId {
        &self.kind
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn apply(&self, b: &mut ModifierStatsBuilder<'_>, scale: f32) {
        b.update(&stat("durability"), &StatValue::Float(self.durability as f32 * scale));
        b.update(&stat("armor"), &StatValue::Float(self.armor * scale));
        b.update(&stat("armor_toughness"), &StatValue::Float(self.toughness * scale));
        b.update(&stat("knockback_resistance"), &StatValue::Float(self.knockback_resistance * scale));
    }
    fn repair_durability(&self) -> Option<i32> {
        Some(self.durability)
    }
}

/// `SkullStats`.
#[derive(Clone, Debug, PartialEq)]
pub struct SkullStats {
    pub durability: i32,
    pub armor: i32,
}

lazy_id!(skull_id, "skull");

impl SkullStats {
    pub fn kind() -> MaterialStatType {
        MaterialStatType {
            id: skull_id().clone(),
            default_stats: Arc::new(Self { durability: 1, armor: 0 }),
            loader: Arc::new(|o| Ok(Arc::new(Self { durability: durability(o)?, armor: integer(o, "armor", Some(0), 0)? }) as Arc<dyn MaterialStats>)),
            can_repair: true,
        }
    }
}

impl MaterialStats for SkullStats {
    fn id(&self) -> &MaterialStatsId {
        skull_id()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn apply(&self, b: &mut ModifierStatsBuilder<'_>, scale: f32) {
        b.update(&stat("armor"), &StatValue::Float(self.armor as f32 * scale));
    }
    fn repair_durability(&self) -> Option<i32> {
        Some(self.durability)
    }
}

/// `SlimeStats`: durability and the overslime capacity.
#[derive(Clone, Debug, PartialEq)]
pub struct SlimeStats {
    pub durability: i32,
    pub overslime: i32,
}

lazy_id!(slime_id, "slime");

impl SlimeStats {
    pub fn kind() -> MaterialStatType {
        MaterialStatType {
            id: slime_id().clone(),
            default_stats: Arc::new(Self { durability: 1, overslime: 0 }),
            loader: Arc::new(|o| Ok(Arc::new(Self { durability: durability(o)?, overslime: integer(o, "overslime_capacity", None, 0)? }) as Arc<dyn MaterialStats>)),
            can_repair: true,
        }
    }
}

impl MaterialStats for SlimeStats {
    fn id(&self) -> &MaterialStatsId {
        slime_id()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn apply(&self, b: &mut ModifierStatsBuilder<'_>, scale: f32) {
        // the original adds `durability * scale` to the overslime capacity as well (its own TODO notes the oddity)
        b.update(&stat("durability"), &StatValue::Float(self.durability as f32 * scale));
        b.add(&stat("overslime"), f64::from(self.durability as f32 * scale));
    }
    fn repair_durability(&self) -> Option<i32> {
        Some(self.durability)
    }
}

/// `RepairStats`: ribcage, shell and laces, each only a repair amount.
#[derive(Clone, Debug, PartialEq)]
pub struct RepairStats {
    pub kind: MaterialStatsId,
    pub durability: i32,
}

/// The three repair stat types.
pub const REPAIR_TYPES: [&str; 3] = ["ribcage", "shell", "laces"];

impl RepairStats {
    pub fn kinds() -> Vec<MaterialStatType> {
        REPAIR_TYPES
            .iter()
            .map(|name| {
                let kind = id(name);
                let loader_kind = kind.clone();
                MaterialStatType {
                    id: kind.clone(),
                    default_stats: Arc::new(Self { kind, durability: 1 }),
                    loader: Arc::new(move |o| Ok(Arc::new(Self { kind: loader_kind.clone(), durability: integer(o, "repair_amount", None, 1)? }) as Arc<dyn MaterialStats>)),
                    can_repair: true,
                }
            })
            .collect()
    }
}

impl MaterialStats for RepairStats {
    fn id(&self) -> &MaterialStatsId {
        &self.kind
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn repair_durability(&self) -> Option<i32> {
        Some(self.durability)
    }
}

/// `StatlessMaterialStats`: parts with no numbers of their own.
#[derive(Clone, Debug, PartialEq)]
pub struct StatlessMaterialStats {
    pub kind: MaterialStatsId,
}

/// The names of the stat-less types.
pub const STATLESS_TYPES: [&str; 9] = ["binding", "bowstring", "maille", "shield_core", "repair_kit", "cuirass", "arrow_head", "arrow_shaft", "fletching"];

impl StatlessMaterialStats {
    pub fn named(name: &str) -> Self {
        Self { kind: id(name) }
    }

    pub fn kind_of(name: &str) -> MaterialStatType {
        singleton(Arc::new(Self::named(name)))
    }
}

impl MaterialStats for StatlessMaterialStats {
    fn id(&self) -> &MaterialStatsId {
        &self.kind
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The stat group the traits of a stat type fall back to (`MaterialRegistry.MELEE_HARVEST` and the others).
pub fn trait_group(name: &str) -> MaterialStatsId {
    id(name)
}

impl MaterialRegistry {
    /// `MaterialRegistry()`: the stat types of the mod with the traits each falls back to.
    pub fn with_tinkers_stats() -> Self {
        let mut registry = Self::new();
        let melee_harvest = Some(trait_group("melee_harvest"));
        let ranged = Some(trait_group("ranged"));
        let armor = Some(trait_group("armor"));
        let ammo = Some(trait_group("ammo"));
        let mut add = |kind: MaterialStatType, fallback: &Option<MaterialStatsId>| {
            registry.register_stat_type_with_fallback(kind, fallback.clone()).expect("the original's stat types have distinct ids");
        };
        // melee harvest
        add(HeadMaterialStats::kind(), &melee_harvest);
        add(HandleMaterialStats::kind(), &melee_harvest);
        add(StatlessMaterialStats::kind_of("binding"), &melee_harvest);
        // ranged
        add(LimbMaterialStats::kind(), &ranged);
        add(GripMaterialStats::kind(), &ranged);
        add(StatlessMaterialStats::kind_of("bowstring"), &ranged);
        // armor
        for kind in PlatingMaterialStats::kinds() {
            add(kind, &armor);
        }
        for name in ["cuirass", "maille", "shield_core"] {
            add(StatlessMaterialStats::kind_of(name), &armor);
        }
        // slimesuit
        add(SlimeStats::kind(), &armor);
        add(SkullStats::kind(), &armor);
        let mut repairs = RepairStats::kinds().into_iter();
        add(repairs.next().expect("ribcage"), &None); // not armor, so the melee and harvest traits win out
        for kind in repairs {
            add(kind, &armor);
        }
        // ammo
        for name in ["arrow_head", "arrow_shaft", "fletching"] {
            add(StatlessMaterialStats::kind_of(name), &ammo);
        }
        // misc
        add(StatlessMaterialStats::kind_of("repair_kit"), &None);
        registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::stat::{HarvestTiers, ToolStats};
    use serde_json::json;

    fn registry() -> MaterialRegistry {
        MaterialRegistry::with_tinkers_stats()
    }

    fn rid(text: &str) -> ResourceId {
        ResourceId::try_parse(text).unwrap()
    }

    fn mat(text: &str) -> crate::materials::MaterialId {
        crate::materials::MaterialId::try_parse(text).unwrap()
    }

    #[test]
    fn the_original_registers_all_its_stat_types() {
        let registry = registry();
        assert_eq!(registry.stats.stat_type_ids().count(), 23);
        assert!(registry.stats.stat_type(&id("plating_shield")).is_some());
        assert!(registry.stats.stat_type(&id("ribcage")).unwrap().can_repair);
        assert!(!registry.stats.stat_type(&id("binding")).unwrap().can_repair);
        assert!(registry.stats.stat_type(&id("head")).unwrap().can_repair);
        assert!(!registry.stats.stat_type(&id("handle")).unwrap().can_repair);
    }

    #[test]
    fn head_and_handle_work_out_a_tools_stats() {
        let mut registry = registry();
        let problems = registry.stats.load(&[
            (rid("tconstruct:wood"), json!({"stats": {"tconstruct:head": {"durability": 60, "melee_attack": 0.0, "mining_speed": 2.0, "mining_tier": "minecraft:wood"}, "tconstruct:handle": {"durability": 0.1, "melee_damage": 0.25, "melee_speed": -0.5, "mining_speed": 0.0}}})),
        ]);
        assert!(problems.is_empty(), "{problems:?}");
        let tool_stats = ToolStats::tinkers(HarvestTiers::vanilla());
        let mut builder = ModifierStatsBuilder::new(&tool_stats);
        let wood = mat("tconstruct:wood");
        registry.material_stats(&wood, &id("head")).unwrap().apply(&mut builder, 1.0);
        registry.material_stats(&wood, &id("handle")).unwrap().apply(&mut builder, 1.0);
        // the head sets 60 durability, the handle adds +10 %; the speed is 50 % slower
        assert_eq!(builder.get(&stat("durability")), StatValue::Float(66.0));
        assert_eq!(builder.get(&stat("attack_speed")), StatValue::Float(0.5));
        assert_eq!(builder.get(&stat("mining_speed")), StatValue::Float(2.0));
    }

    #[test]
    fn required_fields_and_bounds_are_enforced() {
        let mut registry = registry();
        let problems = registry.stats.load(&[(rid("tconstruct:odd"), json!({"stats": {"tconstruct:head": {"durability": 0}, "tconstruct:limb": {}, "tconstruct:grip": {"melee_damage": -1.0}, "tconstruct:slime": {"durability": 5}}}))]);
        assert_eq!(problems.len(), 4, "{problems:?}");
    }

    #[test]
    fn handles_made_from_multipliers_round_to_hundredths() {
        let handle = HandleMaterialStats::from_multipliers(1.2, 1.0, 0.5, 1.333);
        assert_eq!((handle.durability, handle.mining_speed, handle.melee_speed, handle.attack_damage), (0.2, 0.0, -0.5, 0.33));
    }

    #[test]
    fn traits_fall_back_to_the_group_of_their_stat_type() {
        let mut registry = registry();
        registry.traits.load(&[(rid("tconstruct:wood"), json!({"perStat": {"tconstruct:melee_harvest": [{"name": "tconstruct:cultivated"}], "tconstruct:ammo": [{"name": "tconstruct:economical"}]}}))]);
        let wood = mat("tconstruct:wood");
        assert_eq!(registry.traits(&wood, &id("head"))[0].modifier.to_string(), "tconstruct:cultivated", "head -> melee_harvest");
        assert_eq!(registry.traits(&wood, &id("arrow_head"))[0].modifier.to_string(), "tconstruct:economical", "arrow head -> ammo");
        assert!(registry.traits(&wood, &id("ribcage")).is_empty(), "the ribcage has no fallback");
    }
}
