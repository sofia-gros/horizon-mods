//! `library.tools.stat`: the numbers of a tool (`IToolStat`, `FloatToolStat`, `IntegerToolStat`, `PercentToolStat`, `CapacityStat`,
//! `MaxToolStat`, `ToolTierStat`), how modifiers change them (`ModifierStatsBuilder`) and the stats the mod registers (`ToolStats`).
//!
//! What the original does with `Component`s (colours, translation keys, number formats) is the game's drawing and is left to it; the
//! stat keeps its colour and name so the game can draw them.
use std::collections::{BTreeMap, HashMap};

use crate::resource::ResourceId;

/// The id of a tool stat (`tconstruct:durability`).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ToolStatId(pub ResourceId);

impl ToolStatId {
    pub fn tconstruct(name: &str) -> Self {
        Self(ResourceId::try_new("tconstruct", name).expect("valid stat name"))
    }

    pub fn try_parse(text: &str) -> Option<Self> {
        ResourceId::try_parse(text).map(Self)
    }
}

impl std::fmt::Display for ToolStatId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// What a stat holds.
#[derive(Clone, Debug, PartialEq)]
pub enum StatValue {
    Float(f32),
    /// A harvest tier by id (`ToolTierStat`).
    Tier(ResourceId),
}

impl StatValue {
    pub fn float(&self) -> Option<f32> {
        match self {
            Self::Float(value) => Some(*value),
            Self::Tier(_) => None,
        }
    }
}

/// The kinds of stat. All the numeric ones share `FloatToolStat`'s arithmetic.
#[derive(Clone, Debug, PartialEq)]
pub enum StatKind {
    /// `FloatToolStat`.
    Float,
    /// `IntegerToolStat`: drawn as a whole number.
    Integer,
    /// `PercentToolStat`: drawn as a percentage.
    Percent,
    /// `CapacityStat`: a whole amount with a unit; `format_key` names the unit's translation.
    Capacity { format_key: String },
    /// `MaxToolStat`: the largest value wins; never below 0.
    Max,
    /// `ToolTierStat`: the highest harvest tier wins.
    Tier,
}

/// One stat: its id, colour, default, bounds and the item tags it applies to (empty means every item).
#[derive(Clone, Debug, PartialEq)]
pub struct ToolStat {
    pub id: ToolStatId,
    pub kind: StatKind,
    pub color: u32,
    pub default: f32,
    pub min: f32,
    pub max: f32,
    /// `supports(item)`: the item is in any of these tags.
    pub item_tags: Vec<String>,
    /// Marked by `registerConditional`: usable in a conditional stat.
    pub conditional: bool,
}

impl ToolStat {
    fn float(name: &str, kind: StatKind, color: u32, default: f32, min: f32, max: f32, tags: &[&str]) -> Self {
        Self { id: ToolStatId::tconstruct(name), kind, color, default, min, max, item_tags: tags.iter().map(|t| (*t).to_owned()).collect(), conditional: false }
    }

    pub fn is_numeric(&self) -> bool {
        !matches!(self.kind, StatKind::Tier | StatKind::Max)
    }

    /// `supports`: whether the stat applies to an item, given a way to ask whether it is in a tag.
    pub fn supports(&self, in_tag: &dyn Fn(&str) -> bool) -> bool {
        self.item_tags.is_empty() || self.item_tags.iter().any(|tag| in_tag(tag))
    }

    /// `clamp`.
    pub fn clamp(&self, value: f32) -> f32 {
        match self.kind {
            StatKind::Tier => value,
            StatKind::Max => value.max(0.0),
            _ => value.clamp(self.min, self.max),
        }
    }

    /// The value an item with no modifiers has.
    pub fn default_value(&self, minimum_tier: &ResourceId) -> StatValue {
        match self.kind {
            StatKind::Tier => StatValue::Tier(minimum_tier.clone()),
            _ => StatValue::Float(self.default),
        }
    }
}

/// The harvest tiers in order from the lowest (`HarvestTiers`): the vanilla ones, then those other mods add.
#[derive(Clone, Debug)]
pub struct HarvestTiers {
    order: Vec<ResourceId>,
}

impl HarvestTiers {
    pub fn new(order: Vec<ResourceId>) -> Self {
        assert!(!order.is_empty(), "there is at least one tier");
        Self { order }
    }

    /// The tiers of the game: wood, stone, iron, diamond, netherite (gold digs like wood and is not above it).
    pub fn vanilla() -> Self {
        Self::new(["wood", "stone", "iron", "diamond", "netherite"].iter().map(|n| ResourceId::try_new("minecraft", n).unwrap()).collect())
    }

    /// `minTier`.
    pub fn min_tier(&self) -> &ResourceId {
        &self.order[0]
    }

    fn rank(&self, tier: &ResourceId) -> Option<usize> {
        self.order.iter().position(|t| t == tier)
    }

    /// `max`: the higher of two tiers; a tier that is not known loses to one that is.
    pub fn max<'a>(&self, a: &'a ResourceId, b: &'a ResourceId) -> &'a ResourceId {
        match (self.rank(a), self.rank(b)) {
            (Some(x), Some(y)) => if y > x { b } else { a },
            (None, Some(_)) => b,
            _ => a,
        }
    }
}

/// How one stat is being worked out: the original's `FloatBuilder` (base, add, percent, multiply) or the best tier so far.
#[derive(Clone, Debug)]
enum StatBuilder {
    Float { base: f32, add: f32, percent: f32, multiply: f32 },
    Max(i32),
    Tier(ResourceId),
}

/// `ModifierStatsBuilder`: what the modifiers of a tool do to its stats, then the result.
pub struct ModifierStatsBuilder<'a> {
    stats: &'a ToolStats,
    builders: BTreeMap<ToolStatId, StatBuilder>,
    multipliers: BTreeMap<ToolStatId, f32>,
}

impl<'a> ModifierStatsBuilder<'a> {
    pub fn new(stats: &'a ToolStats) -> Self {
        Self { stats, builders: BTreeMap::new(), multipliers: BTreeMap::new() }
    }

    fn stat(&self, id: &ToolStatId) -> &'a ToolStat {
        self.stats.get(id).unwrap_or_else(|| panic!("tool stat {id} is not registered"))
    }

    fn builder(&mut self, id: &ToolStatId) -> &mut StatBuilder {
        let stat = self.stat(id);
        let tiers_floor = self.stats.tiers.min_tier().clone();
        self.builders.entry(id.clone()).or_insert_with(|| match stat.kind {
            StatKind::Tier => StatBuilder::Tier(tiers_floor),
            StatKind::Max => StatBuilder::Max(0),
            _ => StatBuilder::Float { base: stat.default, add: 0.0, percent: 0.0, multiply: 1.0 },
        })
    }

    /// `update`: a stat set outright (for a tier or a max stat, the higher wins; for a number, it replaces the base).
    pub fn update(&mut self, id: &ToolStatId, value: &StatValue) {
        let tiers = self.stats.tiers.clone();
        match (self.builder(id), value) {
            (StatBuilder::Float { base, add, .. }, StatValue::Float(v)) => {
                *add += v;
                *base = 0.0;
            }
            (StatBuilder::Max(best), StatValue::Float(v)) => *best = (*best).max(*v as i32),
            (StatBuilder::Tier(best), StatValue::Tier(v)) => *best = tiers.max(best, v).clone(),
            _ => panic!("the value does not fit the stat {id}"),
        }
    }

    /// `add`.
    pub fn add(&mut self, id: &ToolStatId, value: f64) {
        if let StatBuilder::Float { add, .. } = self.builder(id) {
            *add += value as f32;
        }
    }

    /// `percent`: adds to the percent boost (0.1 is +10 %).
    pub fn percent(&mut self, id: &ToolStatId, factor: f64) {
        if let StatBuilder::Float { percent, .. } = self.builder(id) {
            *percent += factor as f32;
        }
    }

    /// `multiply`.
    pub fn multiply(&mut self, id: &ToolStatId, factor: f64) {
        if let StatBuilder::Float { multiply, .. } = self.builder(id) {
            *multiply *= factor as f32;
        }
    }

    /// `multiplyAll`: multiplies the stat and keeps the factor to apply to values added after.
    pub fn multiply_all(&mut self, id: &ToolStatId, factor: f64) {
        self.multiply(id, factor);
        self.add_multiplier(id, factor);
    }

    /// `multiplier(stat, value)`: a factor kept to apply to values added after, without changing the stat now.
    pub fn add_multiplier(&mut self, id: &ToolStatId, factor: f64) {
        *self.multipliers.entry(id.clone()).or_insert(1.0) *= factor as f32;
    }

    /// `getMultiplier`.
    pub fn multiplier(&self, id: &ToolStatId) -> f32 {
        self.multipliers.get(id).copied().unwrap_or(1.0)
    }

    fn built(&self, id: &ToolStatId, builder: &StatBuilder) -> StatValue {
        match builder {
            StatBuilder::Float { base, add, percent, multiply } => StatValue::Float((base + add) * (1.0 + percent) * multiply),
            StatBuilder::Max(best) => StatValue::Float(*best as f32),
            StatBuilder::Tier(tier) => {
                let _ = id;
                StatValue::Tier(tier.clone())
            }
        }
    }

    /// `getStat`: the value so far, or the default when nothing touched the stat.
    pub fn get(&self, id: &ToolStatId) -> StatValue {
        match self.builders.get(id) {
            Some(builder) => self.built(id, builder),
            None => self.stat(id).default_value(self.stats.tiers.min_tier()),
        }
    }

    /// `build`: the stats that differ from their default (`StatsNBT`), each within its bounds.
    pub fn build(&self) -> BTreeMap<ToolStatId, StatValue> {
        self.builders
            .iter()
            .map(|(id, builder)| (id.clone(), self.clamped(id, self.built(id, builder))))
            .filter(|(id, value)| *value != self.stat(id).default_value(self.stats.tiers.min_tier()))
            .collect()
    }

    fn clamped(&self, id: &ToolStatId, value: StatValue) -> StatValue {
        match value {
            StatValue::Float(v) => StatValue::Float(self.stat(id).clamp(v)),
            tier => tier,
        }
    }

    /// `buildMultipliers`.
    pub fn build_multipliers(&self) -> BTreeMap<ToolStatId, f32> {
        self.multipliers.clone()
    }
}

/// `ToolStats`: the stats and the harvest tiers.
#[derive(Clone, Debug)]
pub struct ToolStats {
    all: HashMap<ToolStatId, ToolStat>,
    pub tiers: HarvestTiers,
}

impl ToolStats {
    /// An empty registry over the given harvest tiers.
    pub fn empty(tiers: HarvestTiers) -> Self {
        Self { all: HashMap::new(), tiers }
    }

    /// `register`; a second registration of an id is refused.
    pub fn register(&mut self, stat: ToolStat) -> Result<(), String> {
        if self.all.contains_key(&stat.id) {
            return Err(format!("Attempt to register duplicate tool stat {}", stat.id));
        }
        self.all.insert(stat.id.clone(), stat);
        Ok(())
    }

    pub fn get(&self, id: &ToolStatId) -> Option<&ToolStat> {
        self.all.get(id)
    }

    pub fn all(&self) -> impl Iterator<Item = &ToolStat> {
        self.all.values()
    }

    /// `supportsConditional`.
    pub fn supports_conditional(&self, id: &ToolStatId) -> bool {
        self.get(id).is_some_and(|s| s.conditional)
    }

    /// The stats the mod registers, as `ToolStats` lists them.
    pub fn tinkers(tiers: HarvestTiers) -> Self {
        use StatKind::{Float, Integer, Percent, Tier};
        let mut stats = Self::empty(tiers);
        let float = |name, color, default, min, max, tags: &[&str]| ToolStat::float(name, Float, color, default, min, max, tags);
        let integer = |name, color, default, min, max, tags: &[&str]| ToolStat::float(name, Integer, color, default, min, max, tags);
        let conditional = |mut stat: ToolStat| {
            stat.conditional = true;
            stat
        };
        let mut all = vec![
            integer("durability", 0xFF47CC47, 1.0, 1.0, i32::MAX as f32, &["tconstruct:modifiable/durability"]),
            float("use_item_speed", 0xFF78A0CD, 0.2, 0.0, 1.0, &["tconstruct:modifiable/held"]),
            float("attack_damage", 0xFFD76464, 0.0, 0.0, 2048.0, &["tconstruct:modifiable/melee"]),
            float("attack_speed", 0xFF8547CC, 1.0, 0.0, 1024.0, &["tconstruct:modifiable/melee/weapon"]),
            float("mining_speed", 0xFF78A0CD, 1.0, 0.1, 2048.0, &["tconstruct:modifiable/harvest"]),
            ToolStat::float("harvest_tier", Tier, 0, 0.0, 0.0, 0.0, &["tconstruct:modifiable/harvest"]),
            float("armor", 0xFF8547CC, 0.0, 0.0, 30.0, &["tconstruct:modifiable/armor"]),
            float("armor_toughness", 0xFF8547CC, 0.0, 0.0, 20.0, &["tconstruct:modifiable/armor"]),
            float("knockback_resistance", 0xFF8547CC, 0.0, 0.0, 1.0, &["tconstruct:modifiable/armor"]),
            float("block_amount", 0xFF78A0CD, 5.0, 0.0, 2048.0, &["tconstruct:modifiable/held"]),
            conditional(float("block_angle", 0xFF78A0CD, 120.0, 0.0, 180.0, &["tconstruct:modifiable/held"])),
            conditional(float("draw_speed", 0xFF8547CC, 1.0, 0.0, 1024.0, &["tconstruct:modifiable/ranged"])),
            conditional(float("velocity", 0xFF78A0CD, 1.0, 0.0, 1024.0, &["tconstruct:modifiable/ranged", "tconstruct:modifiable/ammo"])),
            conditional(float("accuracy", 0xFF8547CC, 0.75, 0.1, 1.0, &["tconstruct:modifiable/ranged", "tconstruct:modifiable/ammo"])),
            conditional(float("projectile_damage", 0xFFD76464, 2.0, 0.0, 1024.0, &["tconstruct:modifiable/ranged/launcher", "tconstruct:modifiable/ammo"])),
            conditional(ToolStat::float("water_inertia", Percent, 0xFF5A82F3, 0.6, 0.01, 0.99, &[])),
            conditional(integer("sea_luck", 0xFF345EC3, 0.0, 0.0, 1024.0, &["tconstruct:modifiable/fishing_rods"])),
            conditional(integer("lure", 0xFFCBCC18, 0.0, 0.0, 5.0, &["tconstruct:modifiable/fishing_rods"])),
            // registered by `OverslimeModule` in the original; the slime stats need it
            integer("overslime", 0xFF71DC85, 0.0, 0.0, i16::MAX as f32, &["tconstruct:modifiable/durability"]),
            // registered by their modules in the original: the tool tank and energy, and the edible modifiers
            ToolStat::float("tank_capacity", StatKind::Capacity { format_key: "mantle.gui.fluid.millibucket".to_owned() }, 0xA0A0A0, 0.0, 0.0, i32::MAX as f32, &[]),
            ToolStat::float("max_energy", StatKind::Capacity { format_key: "tconstruct.tool_stat.energy".to_owned() }, 0xA00000, 0.0, 0.0, i32::MAX as f32, &[]),
            integer("hunger", 0xFFF0A8A4, 0.0, 0.0, 200.0, &["tconstruct:modifiable/interactable/charge", "tconstruct:modifiable/armor"]),
            float("saturation", 0xFFF0A8A4, 0.0, 0.0, 200.0, &["tconstruct:modifiable/interactable/charge", "tconstruct:modifiable/armor"]),
            float("eat_duration", 0xFFF0A8A4, 16.0, 0.0, 100.0, &["tconstruct:modifiable/interactable/charge", "tconstruct:modifiable/armor"]),
            ToolStat::float("edible_counter_chance", Percent, 0xFFF0A8A4, 0.0, 0.0, 1.0, &["tconstruct:modifiable/interactable/charge", "tconstruct:modifiable/armor"]),
        ];
        for stat in all.drain(..) {
            stats.register(stat).expect("the original's stats have distinct names");
        }
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats() -> ToolStats {
        ToolStats::tinkers(HarvestTiers::vanilla())
    }

    fn id(name: &str) -> ToolStatId {
        ToolStatId::tconstruct(name)
    }

    #[test]
    fn the_original_registers_eighteen_stats_and_those_its_modules_add() {
        let stats = stats();
        assert_eq!(stats.all().count(), 25);
        assert!(stats.supports_conditional(&id("draw_speed")));
        assert!(!stats.supports_conditional(&id("durability")));
        let mut again = stats.clone();
        assert!(again.register(stats.get(&id("armor")).unwrap().clone()).is_err());
    }

    #[test]
    fn numbers_work_out_as_base_plus_add_times_percent_times_multiply() {
        let stats = stats();
        let mut builder = ModifierStatsBuilder::new(&stats);
        let damage = id("attack_damage");
        builder.update(&damage, &StatValue::Float(3.0));
        builder.add(&damage, 1.0);
        builder.percent(&damage, 0.5);
        builder.multiply(&damage, 2.0);
        // (0 base after the update + 3 + 1) * 1.5 * 2
        assert_eq!(builder.get(&damage), StatValue::Float(12.0));
        builder.multiply_all(&damage, 0.5);
        assert_eq!(builder.get(&damage), StatValue::Float(6.0));
        assert_eq!(builder.multiplier(&damage), 0.5);
        assert_eq!(builder.multiplier(&id("armor")), 1.0);
    }

    #[test]
    fn an_untouched_stat_is_its_default_and_is_not_built() {
        let stats = stats();
        let mut builder = ModifierStatsBuilder::new(&stats);
        assert_eq!(builder.get(&id("attack_speed")), StatValue::Float(1.0));
        builder.add(&id("attack_speed"), 0.0);
        builder.add(&id("armor"), 2.0);
        let built = builder.build();
        assert_eq!(built.len(), 1, "an added zero leaves the default, so it is not stored");
        assert_eq!(built[&id("armor")], StatValue::Float(2.0));
    }

    #[test]
    fn the_harvest_tier_keeps_the_highest() {
        let stats = stats();
        let mut builder = ModifierStatsBuilder::new(&stats);
        let tier = |n: &str| StatValue::Tier(ResourceId::try_new("minecraft", n).unwrap());
        builder.update(&id("harvest_tier"), &tier("iron"));
        builder.update(&id("harvest_tier"), &tier("stone"));
        assert_eq!(builder.get(&id("harvest_tier")), tier("iron"));
        assert_eq!(stats.get(&id("harvest_tier")).unwrap().default_value(stats.tiers.min_tier()), tier("wood"));
    }

    #[test]
    fn clamping_and_support_follow_the_stat() {
        let stats = stats();
        assert_eq!(stats.get(&id("accuracy")).unwrap().clamp(5.0), 1.0);
        assert_eq!(stats.get(&id("accuracy")).unwrap().clamp(0.0), 0.1);
        let in_tag = |tag: &str| tag == "tconstruct:modifiable/ammo";
        assert!(stats.get(&id("velocity")).unwrap().supports(&in_tag));
        assert!(!stats.get(&id("armor")).unwrap().supports(&in_tag));
        assert!(stats.get(&id("water_inertia")).unwrap().supports(&in_tag), "no tags means every item");
    }
}

/// `ToolType` (`tools.stats.ToolType`): the groups of tools by the item tag they are in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ToolType {
    Melee,
    Harvest,
    Ranged,
    Launcher,
    Armor,
    MeleeUnarmed,
}

impl ToolType {
    /// `getTag`.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Melee => "tconstruct:modifiable/melee/weapon",
            Self::Harvest => "tconstruct:modifiable/harvest",
            Self::Ranged => "tconstruct:modifiable/ranged",
            Self::Launcher => "tconstruct:modifiable/ranged/launcher",
            Self::Armor => "tconstruct:modifiable/armor",
            Self::MeleeUnarmed => "tconstruct:modifiable/melee",
        }
    }

    /// `getSerializedName`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Melee => "melee",
            Self::Harvest => "harvest",
            Self::Ranged => "ranged",
            Self::Launcher => "launcher",
            Self::Armor => "armor",
            Self::MeleeUnarmed => "melee_unarmed",
        }
    }

    /// `NO_MELEE`.
    pub const NO_MELEE: [ToolType; 3] = [Self::Harvest, Self::Ranged, Self::Armor];

    /// `from(item, types...)`: the first of `types` whose tag the item is in.
    pub fn from(in_tag: &dyn Fn(&str) -> bool, types: &[ToolType]) -> Option<ToolType> {
        types.iter().copied().find(|t| in_tag(t.tag()))
    }
}

#[cfg(test)]
mod tool_type_tests {
    use super::*;

    #[test]
    fn the_first_matching_type_wins() {
        let in_tag = |tag: &str| tag == "tconstruct:modifiable/ranged" || tag == "tconstruct:modifiable/armor";
        assert_eq!(ToolType::from(&in_tag, &ToolType::NO_MELEE), Some(ToolType::Ranged));
        assert_eq!(ToolType::from(&in_tag, &[ToolType::Melee]), None);
        assert_eq!(ToolType::MeleeUnarmed.name(), "melee_unarmed");
    }
}

/// The original's `ToolStatsModifierBuilderTest`, ported case by case.
#[cfg(test)]
mod original_builder_tests {
    use super::*;

    fn stats() -> ToolStats {
        ToolStats::tinkers(HarvestTiers::vanilla())
    }

    fn id(name: &str) -> ToolStatId {
        ToolStatId::tconstruct(name)
    }

    /// `StatsNBT.get`: the built value, or the default when the stat is not in it.
    fn built(stats: &ToolStats, builder: &ModifierStatsBuilder<'_>, name: &str) -> StatValue {
        builder.build().get(&id(name)).cloned().unwrap_or_else(|| stats.get(&id(name)).unwrap().default_value(stats.tiers.min_tier()))
    }

    fn tier(name: &str) -> StatValue {
        StatValue::Tier(ResourceId::try_new("minecraft", name).unwrap())
    }

    #[test]
    fn empty() {
        let stats = stats();
        assert!(ModifierStatsBuilder::new(&stats).build().is_empty());
    }

    #[test]
    fn tier_stat_default_set_and_largest() {
        let stats = stats();
        let mut builder = ModifierStatsBuilder::new(&stats);
        builder.add(&id("attack_damage"), 1.0);
        assert_eq!(built(&stats, &builder, "harvest_tier"), tier("wood"));
        let mut builder = ModifierStatsBuilder::new(&stats);
        builder.update(&id("harvest_tier"), &tier("diamond"));
        assert_eq!(built(&stats, &builder, "harvest_tier"), tier("diamond"));
        let mut builder = ModifierStatsBuilder::new(&stats);
        builder.update(&id("harvest_tier"), &tier("iron"));
        builder.update(&id("harvest_tier"), &tier("diamond"));
        assert_eq!(built(&stats, &builder, "harvest_tier"), tier("diamond"));
        let mut builder = ModifierStatsBuilder::new(&stats);
        builder.update(&id("harvest_tier"), &tier("netherite"));
        builder.update(&id("harvest_tier"), &tier("diamond"));
        assert_eq!(built(&stats, &builder, "harvest_tier"), tier("netherite"));
    }

    #[test]
    fn float_stat_default_add_and_multiply() {
        let stats = stats();
        let int = |builder: &ModifierStatsBuilder<'_>, name| built(&stats, builder, name).float().unwrap() as i32;
        let mut builder = ModifierStatsBuilder::new(&stats);
        builder.add(&id("attack_damage"), 1.0);
        assert_eq!(int(&builder, "durability"), 1, "an untouched stat is its default");
        let mut builder = ModifierStatsBuilder::new(&stats);
        builder.add(&id("durability"), 10.0);
        assert_eq!(int(&builder, "durability"), 11);
        builder.add(&id("durability"), 15.0);
        assert_eq!(int(&builder, "durability"), 26);
        let mut builder = ModifierStatsBuilder::new(&stats);
        builder.multiply(&id("durability"), 2.0);
        assert_eq!(int(&builder, "durability"), 2);
        builder.multiply(&id("durability"), 1.5);
        assert_eq!(int(&builder, "durability"), 3);
    }

    #[test]
    fn durability_does_not_depend_on_the_order_of_add_and_multiply() {
        let stats = stats();
        let mut a = ModifierStatsBuilder::new(&stats);
        a.add(&id("durability"), 10.0);
        a.multiply(&id("durability"), 2.0);
        let mut b = ModifierStatsBuilder::new(&stats);
        b.multiply(&id("durability"), 2.0);
        b.add(&id("durability"), 10.0);
        assert_eq!(built(&stats, &a, "durability"), StatValue::Float(22.0));
        assert_eq!(built(&stats, &b, "durability"), StatValue::Float(22.0));
    }

    #[test]
    fn values_stay_within_their_bounds() {
        let stats = stats();
        let mut builder = ModifierStatsBuilder::new(&stats);
        builder.add(&id("durability"), f64::from(i16::MIN));
        assert_eq!(built(&stats, &builder, "durability"), StatValue::Float(stats.get(&id("durability")).unwrap().min));
        let mut builder = ModifierStatsBuilder::new(&stats);
        builder.add(&id("attack_damage"), 4096.0);
        assert_eq!(built(&stats, &builder, "attack_damage"), StatValue::Float(stats.get(&id("attack_damage")).unwrap().max));
    }
}
