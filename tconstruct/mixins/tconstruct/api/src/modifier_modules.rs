//! Data-driven modifier modules with a ModifierCondition (SlimeKnights, MIT; see LICENSE):
//! volatile_flag, volatile_int, volatile_float, modifier_slot, rarity and stat_boost. Every other module the
//! original ships is recorded as unbound by the caller; a module of these kinds with a hook
//! binding or stat Tinkers cannot honor is also unbound, never half-installed.
use crate::{
    formula::Formula,
    mining_speed::StatFormula,
    modifier_entries::{nbt_int, Entry},
    tool_predicate::{Facts, Predicate},
    tool_stats::{Builder, Stat},
};
use pluto_lib::{Identifier, Nbt, NbtCompound};
use serde_json::{Map, Value};

/// ModifierCondition: the modifier level range and the tool predicate, both of which must hold.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Condition { tool: Predicate, min: i32, max: i32 }

impl Condition {
    fn decode(fields: &Map<String, Value>) -> Result<Self, String> {
        let tool = match fields.get("tool").filter(|value| !value.is_null()) {
            None => Predicate::Any,
            Some(value) => Predicate::decode(value)?,
        };
        let (min, max) = crate::modifiers::range(fields.get("modifier_level"))?;
        Ok(Self { tool, min, max })
    }
    fn matches(&self, entry: &Entry, facts: &Facts) -> bool {
        self.min <= entry.level && entry.level <= self.max && self.tool.matches(facts)
    }
    fn tool_matches(&self, facts: &Facts) -> bool { self.tool.matches(facts) }
}

/// StatBoostModule.StatOperation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Operation { Add, Percent, MultiplyBase, MultiplyConditional, MultiplyAll }

/// MaterialRepairModule's repair amount: the constant `durability` field, or the durability of the repair
/// material's stats of one type (MaterialRepairModule.getDurability).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum RepairAmount { Constant(i32), StatType(Identifier) }

/// The shipped presentation-only modules. Tinkers has no client text/colour rendering to feed, so these are
/// recorded with their data and have no effect on game state (in the original they only style names, bars and tooltips).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Display {
    /// ShowInteractionSourceModule (display name).
    InteractionSource,
    /// MaterialVariantColorModule (display name): colours the name by the tool's material.
    MaterialVariantColor { material: Identifier },
    /// DurabilityBarColorModule (durability display): a fixed RGB bar colour.
    DurabilityColor { color: i32 },
    /// ModifierVariantColorModule (display name).
    VariantColor,
    /// ModifierVariantNameModule (display name): the formatter that appends the variant.
    VariantName { formatter: Identifier },
    /// StatTooltipModule (tooltip): the stat line it adds.
    StatTooltip { stat: Identifier, condition: Condition },
    /// BannerModule (display name, tooltip): colours the name by the first stored pattern and lists them.
    BannerPatterns,
    /// DyeModule (display name): names the modifier by the dye colour stored on the tool.
    Dyed,
    /// TrimModule (display name): formats the name with the trim material and pattern stored on the tool.
    /// The trim itself is the game's own layer on worn armor; a held tool's trim has no layer to draw on
    /// here (an armor layer is one texture and a tint, and no stack carries a trim component).
    Trim,
    /// EmbellishmentModule (display name, remove, raw data): names the modifier by the material variant
    /// stored for it in the tool's persistent data, coloured by that material. The stored variant is
    /// migrated through material redirects on build (its raw data half, run by the rebuild), and the
    /// stored key goes when the modifier does.
    Embellished,
}

#[derive(Clone, Debug, PartialEq)]
/// What a boot walk module does at each cell it walks over (ArmorWalkRadiusModule.walkOn),
/// at the entry's effective level.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum WalkModule {
    /// FireWalkerModule: place fire where fire can be placed.
    Fire { radius: f32 },
    /// GlowWalkerModule: place a glow block in a dark empty cell, once per walk.
    Glow { radius: f32, min_light: i32, damage: i32 },
    /// CoverGroundWalkerModule: cover an empty cell with the named block where it can survive.
    Cover { block: Identifier, radius: f32 },
}

pub(crate) enum Module {
    Flag { id: Identifier, condition: Condition },
    Rarity(i32),
    /// SweepingEdgeModule: a leveling value (no condition) added onto the volatile float `tconstruct:sweep_percent`.
    Sweep { flat: f32, each: f32 },
    Int { id: Identifier, flat: i32, each: i32, condition: Condition },
    /// VolatileFloatModule: a float added onto the volatile data under its flag (and onto a launched
    /// projectile's persistent data, the same second half the int and flag modules have).
    Float { id: Identifier, flat: f32, each: f32, condition: Condition },
    /// CraftCountModule: multiplies how many of the tool a craft produces (station building and part swapping).
    CraftCount { flat: f32, each: f32, condition: Condition },
    /// MaterialRepairModule: a material that repairs the tool, for a constant durability or the durability of
    /// one of the material's stat types.
    MaterialRepair { material: Identifier, amount: RepairAmount, condition: Condition },
    Slot { name: String, flat: i32, each: i32, condition: Condition },
    Boost { stat: Stat, operation: Operation, flat: f32, each: f32, condition: Condition },
    /// SetStatModule for the harvest tier: the tier becomes the larger of the current one and this.
    SetTier { tier: Identifier, condition: Condition },
    /// ModifierTraitModule: a nested modifier this one brings along.
    Trait { modifier: Identifier, level: i32, fixed_level: bool, condition: Condition },
    /// ReduceToolDamageModule: probabilistically cancels a fraction of tool damage.
    ReduceDamage { cause: CausePredicate, formula: LevelFormula, condition: Condition },
    /// ToolDamageRangeModule: clamps the damage of one use into [min, max] (min == max sets it) for the causes it names.
    DamageRange { min: i32, max: i32, cause: CausePredicate, condition: Condition },
    /// StatCopyModule: adds `percentage` of the source stat (without its multiplier) to the target stat, after the other stat modules.
    StatCopy { target: Stat, source: Stat, flat: f32, each: f32, condition: Condition },
    /// RepairModule: scales the durability a repair restores.
    Repair { formula: LevelFormula, condition: Condition },
    /// ModifierRequirementsModule: a modifier added through a recipe needs this of the tool.
    Requirements { requirement: Predicate, min: i32, max: i32, translation_key: String },
    /// SwappableSlotModule: slots of the type a swappable recipe stored under `key` (default: the modifier's ID).
    SwappableSlot { key: Option<Identifier>, count: i32, condition: Condition },
    /// SwappableSlotModule.BonusSlot: extra slots of `bonus` while the stored slot type is `matching`.
    BonusSlot { key: Option<Identifier>, matching: String, bonus: String, count: i32, condition: Condition },
    /// SwappableToolTraitsModule: the definition's traits for `hook`, while the stored value equals `matching`
    /// (any value when `matching` is empty).
    SwappableTraits { key: Option<Identifier>, matching: String, hook: Identifier },
    /// CreativeSlotModifier: the slots stored in the tool's `tconstruct:creative` persistent compound.
    CreativeSlots,
    /// StatOverrideModifier: the stats stored under `tconstruct:override_bonus` (added, dropping the base) and
    /// `tconstruct:override_multiplier` (multiplied). Only the float stats are supported.
    StatOverride,
    /// ConditionalMiningSpeedModule: changes the dig speed of the blocks it names.
    MiningSpeed { module: Box<crate::mining_speed::MiningSpeed>, condition: Condition },
    /// ConditionalMeleeDamageModule without target or attacker conditions: changes the damage of a melee hit.
    MeleeDamage { module: Box<crate::melee_damage::MeleeDamage>, condition: Condition },
    /// KnockbackModule: adds knockback to a melee hit, filtered on the target's entity type tags.
    Knockback { module: Box<crate::knockback::Knockback>, condition: Condition },
    /// MobEffectModule.Weapon: a melee hit that may put a mob effect on the target.
    WeaponEffect { module: Box<crate::weapon_effect::WeaponEffect>, condition: Condition },
    /// MobEffectModule.ToolUsage: an effect the tool puts on its holder as it is used (a swing, a shot, shearing a mob).
    UsageEffect { module: Box<crate::weapon_effect::ToolUsage>, condition: Condition },
    /// BlockDamageSourceModule: armor that blocks damage of the sources it names.
    BlockDamage { source: crate::protection::SourceCondition, condition: Condition },
    /// ConditionalStatModule: changes a ranged stat while the holder is in the named state.
    ConditionalStat { module: Box<crate::conditional_stat::ConditionalStat>, condition: Condition },
    /// ConditionalPowerModule: changes the power of a projectile by what the holder and the target are doing.
    ConditionalPower { module: Box<crate::conditional_stat::ConditionalPower> },
    /// EffectImmunityModule: armor that makes its wearer immune to a mob effect.
    EffectImmunity { module: Box<crate::effect_immunity::EffectImmunity>, condition: Condition },
    /// CapacityBarModule: a bar in persistent data under the modifier's ID.
    CapacityBar { bar: crate::capacity::Bar },
    /// DurabilityShieldModule: soaks tool wear from the modifier's capacity bar.
    DurabilityShield { shield: crate::capacity::Shield },
    /// MeleeCapacityModule: a melee hit that adds to (or drains) a capacity bar.
    CapacityMelee { module: Box<crate::capacity::MeleeFill>, condition: Condition },
    /// DamageToCapacityModule: armor that turns the damage its wearer takes into capacity.
    CapacityDamage { module: Box<crate::capacity::DamageFill>, condition: Condition },
    /// MiningCapacityModule: a block the tool breaks that adds to (or drains) a capacity bar.
    CapacityMining { module: Box<crate::capacity::MiningFill>, condition: Condition },
    /// LootToCapacityModule: the loot of a block the tool breaks, eaten into a capacity bar.
    CapacityLoot { module: Box<crate::capacity::LootFill>, condition: Condition },
    /// LaunchCapacityModule: a projectile the tool launches that adds to (or drains) a capacity bar.
    CapacityLaunch { module: Box<crate::capacity::LaunchFill>, condition: Condition },
    /// LifestealModule: the melee half (a critical hit of a fully charged swing heals its wielder by a share of the damage dealt).
    Lifesteal { module: Box<crate::lifesteal::Lifesteal>, condition: Condition },
    /// InventoryModule: item slots the modifier adds to the tool, kept in its persistent data.
    Inventory { module: Box<crate::tool_inventory::Inventory>, condition: Condition },
    /// QuiverInventoryModule: an inventory whose slots take the quiver's ammunition (arrows, and for the right
    /// tool also fireworks and ballista ammo) instead of a filter's items.
    QuiverInventory { module: Box<crate::tool_inventory::Inventory>, condition: Condition },
    /// UpdateHealthModule: armor that refreshes its wearer's health as it goes on or comes off.
    UpdateHealth { module: crate::wearer_modules::UpdateHealth, condition: Condition },
    /// InventoryMenuModule: the key that opens the tool's inventory screen.
    InventoryMenu(crate::tool_inventory::MenuKey),
    /// InventorySlotMenuModule: clicking the tool in the player's inventory opens its inventory screen.
    InventorySlotMenu,
    /// BulkQuiverModule (BowAmmoModifierHook): the tool fires arrows from its own inventory; when it checks the
    /// standard arrows first, the quiver is only a backup for when the game found none itself.
    BulkQuiver { check_standard_arrows: bool },
    /// TrickQuiverModule (BowAmmoModifierHook): the tool fires the stack in the selected slot of its own inventory.
    TrickQuiver,
    /// ShieldStrapModule (ArmorInteractModifierHook): the armor keybind on one of these keys rotates the piece's
    /// inventory through the off hand. The swap itself is armor_interact_hook's; see there for the server surface.
    ShieldStrap { keys: Vec<crate::tool_inventory::MenuKey> },
    /// ToolBeltModule (ArmorInteractModifierHook): the armor keybind on one of these keys swaps the piece's
    /// inventory with the wearer's main inventory, slot for slot.
    ToolBelt { keys: Vec<crate::tool_inventory::MenuKey> },
    /// ShareDurabilityModule (ToolDamageModifierHook): wear on this tool also damages it directly by the consume
    /// amount, and charges the shield bar of a random other carried tool by the grant amount per point.
    ShareDurability { shield: Identifier, grant: (i32, i32), consume: (i32, i32) },
    /// ShearsModule (ToolActionModifierHook): the tool performs the four shear actions while its condition holds.
    /// The shearing it starts rides the game's shear gates (see tool_behavior_hook); the entity-interact shearing
    /// of sheep and kin has no gate to ride (the port's item behaviour notes name the match guards), and the
    /// area shearing around a target needs a nearby-entity enumeration no event offers.
    Shears { flat: f32, per_level: f32, expanded: f32, condition: Condition },
    /// FishingModule (ToolActionModifierHook): the tool performs the fishing rod cast action. The cast itself
    /// spawns the original's own hook entity, which no spawn reach in this tree can create (the projectile and
    /// book services of the foundation lane are not in this base), so only the action answer is bound.
    Fishing,
    /// MaxArmorAttributeModule: an attribute worked from the largest level among the worn pieces.
    MaxArmorAttribute { module: Box<crate::max_armor::MaxArmorAttribute>, condition: Condition },
    /// GoldenAttributeModule: an attribute worked from how many of the wearer's armor pieces have gold.
    GoldenAttribute { module: Box<crate::max_armor::GoldenAttribute>, condition: Condition },
    /// LightspeedAttributeModule: boots that grant an attribute while their wearer walks in bright light.
    LightspeedAttribute { module: crate::wearer_modules::LightspeedAttribute, condition: Condition },
    /// ArmorLevelModule: the modifier's level added up over the worn pieces under a key.
    ArmorLevel { module: Box<crate::max_armor::ArmorLevel> },
    /// ShowOffhandModule: a chest piece counts 1 toward the `tconstruct:show_empty_offhand` armor level while worn.
    ShowOffhand { allow_broken: bool },
    /// MobDisguiseModule: armor that makes mobs of one type see its wearer from half as far for each level.
    MobDisguise { module: crate::mob_disguise::MobDisguise },
    /// EdibleModule: a tool with a hunger stat can be eaten with a right click.
    Edible,
    /// SeveringModule: when the mob it kills drops no head, the severing recipes for the mob may add their item (ProcessLootModifierHook).
    Severing,
    /// EdibleConsumeDurabilityModule: eating the tool wears it.
    EdibleWear { usage: crate::edible::DurabilityUsage, condition: Condition },
    /// EdibleCureRandomEffectModule: eating the tool cures one of the eater's effects.
    EdibleCure { condition: Condition },
    /// EdibleRemoveEffectModule: eating the tool removes one effect from the eater.
    EdibleRemove { effect: Identifier, condition: Condition },
    /// EdibleRepresentativeItemModule: the item an edible tool presents as eaten — its eating
    /// sounds while the use runs, and what an integration would be told was eaten.
    EdibleRepresentative { item: Identifier, condition: Condition },
    /// ThornsModule: armor that hurts the mob that hit its wearer.
    Thorns { module: Box<crate::thorns::Thorns>, condition: Condition },
    /// FieryAttackModule and FreezingAttackModule: a melee hit that sets the target on fire or freezes it.
    AttackAction { module: Box<crate::hit_actions::Attack>, condition: Condition },
    /// FieryCounterModule, FreezingCounterModule and KnockbackCounterModule: armor that sets the mob that hit its wearer on fire, freezes or knocks it back.
    CounterAction { module: Box<crate::hit_actions::Counter>, condition: Condition },
    /// MobEffectModule.ArmorCounter: armor that may put a mob effect on the mob that hit its wearer.
    CounterEffect { module: Box<crate::weapon_effect::WeaponEffect>, condition: Condition },
    /// MobEffectModule.ArmorAttack: armor that may put a mob effect on the mob its wearer hits.
    ArmorAttackEffect { module: Box<crate::weapon_effect::WeaponEffect>, condition: Condition },
    /// The deprecated MobEffectModule.Legacy: a weapon effect and an armor counter in one module.
    LegacyEffect { weapon: Box<crate::weapon_effect::WeaponEffect>, counter: Box<crate::weapon_effect::WeaponEffect>, condition: Condition },
    /// AttributeModule for a vanilla attribute: published as an attribute modifier on the stack for each listed slot.
    /// A presentation-only module (display name, durability bar, tooltip): it changes no tool state, so the port records it and its data.
    Display(Display),
    /// TheOneProbeModule (raw data): writes the probe keys into the tool's top-level data.
    TheOneProbe,
    /// ToolActionsModule (ToolActionModifierHook): answers whether the tool can perform each named tool action.
    ToolActions { actions: Vec<String>, condition: Condition },
    /// ToolActionTransformModule: a right click that turns the clicked block into its transformed state.
    ToolAction { action: crate::tool_actions::ToolAction, sound: String, require_ground: bool, condition: Condition },
    /// ToolActionWalkerTransformModule: walking turns the ground under a replaceable cell into its transformed state.
    WalkTransform { action: crate::tool_actions::ToolAction, sound: String, radius: crate::wearer_modules::Leveling, condition: Condition },
    /// FireWalkerModule: walking places fire in the cells fire can be placed at.
    FireWalker { radius: crate::wearer_modules::Leveling },
    /// GlowWalkerModule: walking places a glow block in a dark empty cell, wearing the boots.
    GlowWalker { radius: crate::wearer_modules::Leveling, min_light: i32, damage: i32 },
    /// CoverGroundWalkerModule: walking covers an empty cell with a block where it can survive.
    CoverGround { block: Identifier, radius: crate::wearer_modules::Leveling, condition: Condition },
    /// ConstantEnchantmentModule: the tool counts as carrying this enchantment at this level (published as the stack's enchantments).
    Enchantment { name: String, flat: f32, each: f32, ignoring: bool, condition: Condition },
    /// LootingModule.Armor: a worn piece adds this looting to its wearer's kills while it sits in one of the listed slots.
    ArmorLooting { flat: f32, each: f32, slots: Vec<String>, condition: Condition },
    /// EnchantmentModule.ArmorHarvest: a worn piece lends this enchantment to the stack its wearer
    /// breaks a block with, while it sits in one of the listed slots (the merge runs in the
    /// block-break flow, block_break_hook).
    ArmorHarvestEnchantment { name: String, flat: f32, each: f32, slots: Vec<String>, condition: Condition },
    /// AutosmeltModule: a drop a cooking recipe of one of the listed types takes becomes its
    /// cooked result (the conversion runs in the block-break loot flow, block_break_hook).
    Autosmelt { recipe_types: Vec<String>, extra_drop_chance: f32 },
    /// SmeltingModule: the tool cooks what its input inventory holds into its output inventory,
    /// powered by the tool's use (the cooking runs in tool_stack::cook_smelting; the output's key
    /// is the JSON's `output_key` or, at use, the modifier's ID with an `_output` suffix).
    Smelting { recipe_type: String, multiplier: f32, input: Box<crate::tool_inventory::Inventory>, output: Box<crate::tool_inventory::Inventory>, condition: Condition },
    /// ClearEffectOnUnequipModule: taking the armor off takes the effect off its wearer.
    ClearEffectOnUnequip { effect: Identifier, condition: Condition },
    Protection { module: crate::protection::Protection, condition: Condition },
    /// FlameBarrierModule (protection): protection against everything but fire, scaled by how
    /// long the wearer has been burning (ConductingModifier.bonusScale).
    FlameBarrier { flat: f32, each: f32, condition: Condition },
    /// AdjustDamageModule on the modify damage hook: changes the damage the wearer is about to take.
    AdjustDamage { module: crate::protection::AdjustDamage, condition: Condition },
    /// DepthProtectionModule: protection by the wearer's height.
    DepthProtection { module: crate::wearer_modules::DepthProtection, condition: Condition },
    /// OvershieldModule: protection paid for with the overslime on the piece.
    Overshield { module: crate::wearer_modules::Overshield },
    /// RecurrentProtectionModule (on the modify hurt hook): armor that builds momentum as its wearer takes damage.
    RecurrentProtection { module: crate::wearer_modules::RecurrentProtection },
    /// RestoreLostHealthModule: armor that turns part of the damage into slow healing. The module runs on the hook it is
    /// bound to: `hurt` as the damage arrives, before armor (the shipped `restore`), `damage` on what armor left (the
    /// shipped `rebuff`, the module type's default binding); a binding to both runs it at both.
    RestoreLostHealth { module: Box<crate::wearer_modules::RestoreLostHealth>, condition: Condition, hurt: bool, damage: bool },
    /// DamageOnUnequipModule: taking the armor off hurts its wearer.
    DamageOnUnequip { module: crate::wearer_modules::DamageOnUnequip, condition: Condition },
    /// ReduceEffectOnUnequipModule: taking the armor off shortens the wearer's effects of one category.
    ReduceEffectOnUnequip { module: crate::wearer_modules::ReduceEffectOnUnequip, condition: Condition },
    /// An attribute module for `tconstruct:generic.protection_cap`, which the host has no attribute for: it raises the cap on summed protection.
    ProtectionCap { formula: StatFormula, condition: Condition },
    Attribute { attribute: String, operation: &'static str, formula: StatFormula, slots: Vec<String>, condition: Condition },
    Punch { module: Box<crate::punch::Punch>, condition: Condition },
    ReversePunch { module: Box<crate::punch::ReversePunch> },
    /// DamageOnShootModule: firing the tool hurts its holder.
    DamageOnShoot { module: crate::projectile_modules::DamageOnShoot, condition: Condition },
    /// ExtinguishCampfireModule: a right click that puts out lit campfires.
    CampfireExtinguish { condition: Condition },
    /// BlockingModule: the tool is raised like a shield on a right click, and performs the shield block action.
    Blocking,
    /// InfinityModule: the launcher fires its named ammo without the shooter carrying any, paid for with
    /// durability. The creative-only pickup the original sets on the fired arrow has no surface here (the
    /// launch and shot events cannot change the arrow), so a supplied arrow can be picked up again.
    Infinity { module: crate::projectile_modules::Infinity },
    /// ArrowPierceModule: a shot's pierce level, set when the projectile spawns.
    ArrowPierce { module: crate::projectile_modules::ArrowPierce },
    /// ProjectileBounceModule: a block hit bounces the projectile while bounces remain.
    ProjectileBounce { module: crate::projectile_modules::ProjectileBounce },
    /// ProjectileGravityModule: the shot flies without gravity until its delay passes.
    ProjectileGravity { module: crate::projectile_modules::ProjectileGravity },
    /// ProjectileFuseModule: the shot's fuse ends after its time in ticks.
    ProjectileFuse { module: crate::projectile_modules::ProjectileFuse },
    /// ProjectileExplosionModule: a hit or a fuse's end explodes the projectile.
    ProjectileExplosion { module: crate::projectile_modules::ProjectileExplosion },
    /// ProjectileAttractMobsModule: a hit or a fuse's end pulls nearby living entities in.
    ProjectileAttractMobs { module: crate::projectile_modules::ProjectileAttractMobs },
    /// ProjectilePlaceGlowModule: a hit places a glow block where the projectile landed.
    ProjectilePlaceGlow { module: crate::projectile_modules::ProjectilePlaceGlow },
    /// RestrictAngleModule: a shot's direction is clamped to a fan of fixed angles.
    RestrictAngle { module: crate::projectile_modules::RestrictAngle },
    /// ThrowingModule: the tool is thrown on release of a charged use, as a ThrownTool.
    Throwing,
    MeleeInstrument { module: Box<crate::melee_instrument::MeleeInstrument>, condition: Condition },
    Overgrowth { module: Box<crate::overgrowth::Overgrowth>, condition: Condition },
    Kinetic,
    /// ConductingModifier (a code modifier, no data module): melee and projectile damage grow with
    /// the attacker's fire (crate::conducting).
    Conducting,
    /// InsatiableModifier (a code modifier): melee and projectile damage grow with the insatiable
    /// effects its hits feed (the grants ride the weapon hit, in weapon_effect_hook).
    Insatiable,
    /// LaceratingModifier (a code modifier): melee hits and armor counters put the bleeding effect
    /// on the other side of the hit (the grants ride the weapon and wearer hits, in weapon_effect_hook).
    Lacerating,
    /// MomentumModifier (a code modifier): dig speed and draw speed grow with the momentum effects.
    /// Its block-break grant (whose length follows the block's hardness) and its launch grant (the
    /// shooter's current effect) wait for facts neither transport reports; see the module header of
    /// mining_speed for the dig facts that do arrive.
    Momentum,
    /// DwarvenModifier (a code modifier): dig speed and projectile velocity grow the deeper the
    /// holder is under the level's top and fade near it (mining_speed::dwarven_boost over the dig
    /// report's block facts: the dug block's y and the level's build height).
    Dwarven,
    /// EnderportingModifier (a code modifier): the holder trades places with what the tool touches —
    /// a melee hit swaps the attacker and the mob, a harvested block pulls the holder onto it, and a
    /// projectile carries the swap to where it lands. Its behaviours run in the hit, break and impact
    /// callbacks (enderporting_hook and flight_hook), which read this module off the tool.
    Enderporting,
    /// HarvestModule: right-clicking a harvestable block harvests it in place (and the tool's area
    /// with it). Its behaviour runs in the use-on-block callback (harvest_hook), which reads this
    /// module off the tool.
    Harvest,
    /// SelfDestructiveModifier (a code modifier): holding shift with the helmet key primes the
    /// wearer with the self-destructing effect, whose last tick explodes. Its behaviour runs in
    /// the armor interact receiver and the player tick (armor_interact_hook), which read this
    /// module off the tool. The effect's own movement-speed payload is an effect attribute,
    /// which the effect-attribute reach owns; it is not applied here.
    SelfDestructive,
    /// StrongBonesModifier (a code modifier): drinking milk with the helmet grants damage
    /// resistance (and the calcified effect for calcifiable armor), cured when the helmet
    /// comes off. Its behaviour runs in strong_bones_hook, which reads this module off the tool.
    StrongBones,
    /// EnderclearanceModule: a hit may teleport the mob it landed on (and a hit on the wearer may
    /// teleport the attacker) to a random place within a diameter. The original tries up to its
    /// teleport chances for a spot the mob fits; the port's teleport names one destination, so a
    /// single random spot in the diameter is named (see weapon_effect_hook).
    Enderclearance { chance: (f32, f32), diameter: (f32, f32) },
    /// ChannelingModule: a hit may call lightning down on the mob it landed on, at a chance by the
    /// weather over the attacker. The shipped data forbids melee (allow_melee false), so its live
    /// half there is the launcher's, which is the projectile side's; the melee half below runs for
    /// data that allows it.
    Channeling { allow_melee: bool, chance_clear: f32, chance_rain: f32, chance_thunder: f32 },
    /// TankModule: the tool's fluid tank itself, carried by the code-defined tank handler
    /// (see tool_tank). Its volatile hook feeds the fluid capability, not volatile data keys.
    Tank,
    /// BucketModule: the tool places and picks up a bucket of its tank's fluid.
    Bucket { module: crate::tool_tank::Bucket },
    /// TankInteractionModule: a use on a block with a fluid handler trades with the tank.
    TankInteraction { module: crate::tool_tank::TankInteraction },
    /// MeltingModule: melts dropped items and hit entities into the tank.
    Melting { module: Box<crate::tool_tank::Melting>, condition: Condition },
    /// SlurpingModule: drinking the tank's fluid applies its effects to the holder.
    Slurping { module: Box<crate::tool_tank::Slurping> },
    /// SpillingModule: a hit applies the tank fluid's effects to the target.
    Spilling { module: crate::tool_tank::Spilling, condition: Condition },
    /// SpittingModule: charging and releasing fires the tank's fluid as projectiles.
    Spitting { module: Box<crate::tool_tank::Spitting> },
    /// SplashingModule: a use applies the tank fluid's effects over an area.
    Splashing { module: Box<crate::tool_tank::Splashing> },
    /// OverburnModule: burns the tank's fluid as melting fuel to regrow overslime.
    Overburn,
    /// SmashingModule: an ammo tank whose amount is the fluid effect ingredient's amount.
    Smashing,
}
const CREATIVE_KEY: &str = "tconstruct:creative";
const OVERRIDE_BONUS: &str = "tconstruct:override_bonus";
const OVERRIDE_MULTIPLIER: &str = "tconstruct:override_multiplier";

/// How a quiver module draws ammunition from its modifier's inventory (BowAmmoModifierHook).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum QuiverDraw {
    /// BulkQuiverModule: the first ammunition stack; a backup only when it checks the standard arrows.
    Bulk { check_standard_arrows: bool },
    /// TrickQuiverModule: the stack in the selected slot, whatever it is.
    Trick,
}

/// SlotType.getIfPresent: a swappable recipe only stores a created slot type, so any stored name is one.
fn is_known_slot(name: &str) -> bool { !name.is_empty() }

/// ToolDataNBT.addSlots: ignores 0 and removes the key when the total returns to 0.
fn add_slots(data: &mut NbtCompound, name: &str, count: i32) {
    if count == 0 { return; }
    let total = nbt_int(data.get(name)).wrapping_add(count);
    if total == 0 { data.remove(name); } else { data.insert(name.to_owned(), Nbt::Int(total)); }
}

/// A ModifierFormula of either form. `Simple` is flat + each_level * level, which is the scaled
/// level handed to the module's fallback; `PostFix` runs a formula over named arguments.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum LevelFormula { Simple { flat: f32, each: f32 }, PostFix(Formula), Stat(StatFormula) }

impl LevelFormula {
    /// `names` are the argument names after the level: ["value","multiplier"] or ["factor"].
    fn decode(fields: &Map<String, Value>, names: &[&str]) -> Result<Option<Self>, String> {
        if fields.contains_key("variables") { return Ok(None); } // data-defined variables are not implemented
        if let Some(formula) = fields.get("formula") {
            let mut all = vec!["level"];
            all.extend_from_slice(names);
            return Ok(Some(Self::PostFix(Formula::decode(formula, &all)?)));
        }
        Ok(Some(Self::Simple { flat: float(fields, "flat")?, each: float(fields, "each_level")? }))
    }
    /// processLevel: the scaled level for simple formulas, the plain effective level for post fix.
    fn level(&self, entry: &Entry) -> f32 {
        match self { Self::Simple { flat, each } => flat + each * entry.effective_level(), Self::PostFix(_) | Self::Stat(_) => entry.effective_level() }
    }
}

/// ModifierPredicate over the modifier that caused tool damage (the subset the original ships).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum CausePredicate { Any, Tag(Identifier), Single(Identifier), Not(Box<CausePredicate>) }

impl CausePredicate {
    pub(crate) fn decode(value: Option<&Value>) -> Result<Option<Self>, String> {
        let Some(value) = value.filter(|value| !value.is_null()) else { return Ok(Some(Self::Any)); };
        let object = value.as_object().ok_or("cause must be an object")?;
        Self::typed(object, object.get("type").and_then(Value::as_str).ok_or("cause type missing")?)
    }
    fn typed(object: &Map<String, Value>, kind: &str) -> Result<Option<Self>, String> {
        Ok(Some(match kind {
            "tconstruct:tag" => Self::Tag(identifier(object, "tag")?),
            "tconstruct:single" => Self::Single(identifier(object, "modifier")?),
            "mantle:inverted" => {
                let inner = object.get("inverted_type").and_then(Value::as_str).ok_or("inverted_type missing")?;
                let Some(inner) = Self::typed(object, inner)? else { return Ok(None); };
                Self::Not(Box::new(inner))
            }
            _ => return Ok(None),
        }))
    }
    /// `cause` is None for ordinary tool use (ModifierId.EMPTY), which no tag or single match.
    pub(crate) fn matches(&self, cause: Option<&Identifier>, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> bool {
        match self {
            Self::Any => true,
            Self::Tag(tag) => cause.is_some_and(|cause| in_tag(cause, tag)),
            Self::Single(modifier) => cause == Some(modifier),
            Self::Not(inner) => !inner.matches(cause, in_tag),
        }
    }
}

fn short(fields: &Map<String, Value>, key: &str) -> Result<i32, String> {
    match fields.get(key).filter(|value| !value.is_null()) {
        None => Ok(0),
        Some(Value::Number(number)) => {
            let value = crate::materials::number_int(&number.to_string());
            if (-32768..=32767).contains(&value) { Ok(value) } else { Err(format!("{key} must fit in a short")) }
        }
        Some(_) => Err(format!("{key} must be a number")),
    }
}
fn float(fields: &Map<String, Value>, key: &str) -> Result<f32, String> {
    match fields.get(key).filter(|value| !value.is_null()) {
        None => Ok(0.0),
        Some(Value::Number(number)) => number.as_f64().map(|value| value as f32).ok_or_else(|| format!("invalid {key}")),
        Some(_) => Err(format!("{key} must be a number")),
    }
}
/// LevelingIntModule.FIELD: the `level` field is a leveling int whose plain number means the amount for each level (LevelingInt.EACH_LEVEL),
/// or an object with `flat` and `each_level` parts; absent, it is the level itself. Returns (flat, each_level).
fn leveling_int(fields: &Map<String, Value>) -> Result<(f32, f32), String> {
    match fields.get("level") {
        None | Some(Value::Null) => Ok((0.0, 1.0)),
        Some(Value::Number(number)) => Ok((0.0, number.as_f64().ok_or("level must be a number")? as f32)),
        Some(Value::Object(level)) => Ok((float(level, "flat")?, float(level, "each_level")?)),
        Some(_) => Err("level must be a number or a leveling value".into()),
    }
}
/// A leveling value under its own key: a plain number for the flat part, or an object with `flat`
/// and `each_level` parts; absent, both parts are zero. Returns (flat, each_level).
fn leveling_keyed(fields: &Map<String, Value>, key: &str) -> Result<(f32, f32), String> {
    match fields.get(key) {
        None | Some(Value::Null) => Ok((0.0, 0.0)),
        Some(Value::Number(number)) => Ok((number.as_f64().ok_or_else(|| format!("{key} must be a number"))? as f32, 0.0)),
        Some(Value::Object(level)) => Ok((float(level, "flat")?, float(level, "each_level")?)),
        Some(_) => Err(format!("{key} must be a number or a leveling value")),
    }
}
/// A predicate field that is absent or the always-true `mantle:any`; a module with any other predicate depends on state its event lacks.
fn is_any_predicate(fields: &Map<String, Value>, key: &str) -> bool {
    match fields.get(key) {
        None | Some(Value::Null) => true,
        Some(Value::Object(predicate)) => predicate.get("type").and_then(Value::as_str) == Some("mantle:any"),
        Some(_) => false,
    }
}

/// ModuleWithKey.FIELD: an optional persistent-data key, the modifier's ID when absent.
fn optional_key(fields: &Map<String, Value>) -> Result<Option<Identifier>, String> {
    match fields.get("key").filter(|value| !value.is_null()) { None => Ok(None), Some(_) => identifier(fields, "key").map(Some) }
}

/// A SlotType name: lowercase letters, digits and underscores (the original regex also allows empty).
fn slot_name(fields: &Map<String, Value>, key: &str) -> Result<String, String> {
    let name = fields.get(key).and_then(Value::as_str).ok_or_else(|| format!("{key} must be a slot name"))?;
    if !name.bytes().all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'_')) { return Err(format!("invalid modifier slot name {name}")); }
    Ok(name.to_owned())
}

fn identifier(fields: &Map<String, Value>, key: &str) -> Result<Identifier, String> {
    Identifier::parse(fields.get(key).and_then(Value::as_str).ok_or_else(|| format!("{key} must be an ID string"))?).map_err(|e| e.to_string())
}

/// LevelingInt.LOADABLE for a required field: a plain number is the per-level amount (the EACH_LEVEL form), an
/// object names `flat` and `each_level`; both parts are shorts. Returns (flat, each_level).
fn leveling_int_keyed(fields: &Map<String, Value>, key: &str) -> Result<(i32, i32), String> {
    let part = |value: &Value| -> Result<i32, String> {
        let Value::Number(number) = value else { return Err(format!("{key} must be a leveling int")); };
        let value = crate::materials::number_int(&number.to_string());
        if (-32768..=32767).contains(&value) { Ok(value) } else { Err(format!("{key} must fit in a short")) }
    };
    match fields.get(key).filter(|value| !value.is_null()) {
        Some(Value::Object(object)) => {
            let named = |name: &str| match object.get(name).filter(|value| !value.is_null()) { None => Ok(0), Some(value) => part(value) };
            Ok((named("flat")?, named("each_level")?))
        }
        Some(value) => Ok((0, part(value)?)),
        None => Err(format!("{key} must be a leveling int")),
    }
}

/// LevelingInt.compute: `flat + each_level * level`, truncated.
pub(crate) fn leveling_compute(leveling: (i32, i32), level: f32) -> i32 { (leveling.0 as f32 + leveling.1 as f32 * level) as i32 }

/// The `on_key` list of an armor interact module (ShieldStrapModule/ToolBeltModule read a set of tooltip keys).
fn key_list(fields: &Map<String, Value>) -> Result<Vec<crate::tool_inventory::MenuKey>, String> {
    let Some(Value::Array(keys)) = fields.get("on_key").filter(|value| !value.is_null()) else { return Err("on_key must be a list of key names".into()); };
    keys.iter().map(|key| match key.as_str() { Some(name) => crate::tool_inventory::MenuKey::from_name(name), None => Err("on_key must be a list of key names".into()) }).collect()
}

impl Module {
    /// `Ok(None)` means a known kind this port cannot honor in this form (unbound);
    /// `Err` means a module it does implement is malformed.
    pub fn decode(kind: &str, fields: &Map<String, Value>, hooks: &[Identifier]) -> Result<Option<Self>, String> {
        let only = |hook: &str| hooks.iter().all(|bound| bound.to_string() == hook);
        // A tool predicate type this port lacks leaves the module unbound; a malformed one still rejects.
        for key in ["tool", "requirement"] {
            let Some(predicate) = fields.get(key).filter(|value| !value.is_null()) else { continue; };
            if let Err(error) = Predicate::decode(predicate) {
                if error.starts_with("unimplemented tool predicate") || error.starts_with("only single-modifier") { return Ok(None); }
            }
        }
        Ok(Some(match kind {
            "tconstruct:rarity" if only("tconstruct:volatile_data") => {
                // RarityModule has no condition: its loader reads only the rarity.
                Self::Rarity(match fields.get("rarity").and_then(Value::as_str) {
                    Some("common") => 0, Some("uncommon") => 1, Some("rare") => 2, Some("epic") => 3,
                    _ => return Err("rarity must be common, uncommon, rare or epic".into()),
                })
            }
            "tconstruct:volatile_flag" if only("tconstruct:volatile_data") => Self::Flag { id: identifier(fields, "flag")?, condition: Condition::decode(fields)? },
            "tconstruct:volatile_int" if only("tconstruct:volatile_data") => Self::Int {
                id: identifier(fields, "flag")?, flat: short(fields, "flat")?, each: short(fields, "each_level")?, condition: Condition::decode(fields)?,
            },
            // The float binds on its volatile data hook, like the flag and the int; a binding of nothing but the
            // projectile launch hook stays unbound (a flag bound that way is unbound too), as the launched
            // projectile's persistent data has no consumer yet.
            "tconstruct:volatile_float" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:volatile_data" | "tconstruct:projectile_launch"))
                && (hooks.is_empty() || hooks.iter().any(|bound| bound.to_string() == "tconstruct:volatile_data")) => Self::Float {
                id: identifier(fields, "flag")?, flat: float(fields, "flat")?, each: float(fields, "each_level")?, condition: Condition::decode(fields)?,
            },
            "tconstruct:craft_count" if only("tconstruct:craft_count") => {
                // CraftCountModule: the multiplier is a required leveling value object.
                let multiplier = fields.get("multiplier").filter(|value| !value.is_null()).ok_or("multiplier missing")?.as_object().ok_or("multiplier must be a leveling value")?;
                Self::CraftCount { flat: float(multiplier, "flat")?, each: float(multiplier, "each_level")?, condition: Condition::decode(fields)? }
            }
            "tconstruct:material_repair" if only("tconstruct:material_repair") => {
                // MaterialRepairModule.LOADER: the either form keyed on durability, then stat_type.
                let amount = if let Some(value) = fields.get("durability").filter(|value| !value.is_null()) {
                    let amount = value.as_i64().ok_or("durability must be an integer")?;
                    if amount < 1 || amount > i64::from(i32::MAX) { return Err("durability must be at least 1".into()); }
                    RepairAmount::Constant(amount as i32)
                } else if fields.get("stat_type").is_some_and(|value| !value.is_null()) {
                    RepairAmount::StatType(identifier(fields, "stat_type")?)
                } else { return Err("material repair needs a durability or a stat_type".into()); };
                Self::MaterialRepair { material: identifier(fields, "material")?, amount, condition: Condition::decode(fields)? }
            }
            "tconstruct:modifier_slot" if only("tconstruct:volatile_data") => {
                let name = fields.get("name").and_then(Value::as_str).ok_or("name must be a slot name")?;
                if !name.bytes().all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'_')) { return Err(format!("invalid modifier slot name {name}")); }
                // The older count field is the per-level amount when each_level is absent.
                let each = if fields.contains_key("each_level") { short(fields, "each_level")? } else { short(fields, "count")? };
                Self::Slot { name: name.to_owned(), flat: short(fields, "flat")?, each, condition: Condition::decode(fields)? }
            }
            "tconstruct:sweeping_edge" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:volatile_data" | "tconstruct:tooltip")) => {
                // SweepingEdgeModule has no condition: its loader reads only the leveling value.
                Self::Sweep { flat: float(fields, "flat")?, each: float(fields, "each_level")? }
            }
            "tconstruct:trait" if only("tconstruct:modifier_traits") => {
                let level = match fields.get("level").filter(|value| !value.is_null()) {
                    None => 1,
                    Some(Value::Number(number)) => crate::materials::number_int(&number.to_string()),
                    Some(_) => return Err("level must be a number".into()),
                };
                if level < 1 { return Err("trait level must be at least 1".into()); }
                let Some(Value::Bool(fixed_level)) = fields.get("fixed_level") else { return Err("fixed_level must be a boolean".into()); };
                Self::Trait { modifier: identifier(fields, "name")?, level, fixed_level: *fixed_level, condition: Condition::decode(fields)? }
            }
            // The tooltip binding is presentation; the original binds both by default.
            "tconstruct:reduce_tool_damage" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:tool_damage" | "tconstruct:tooltip"))
                && !fields.get("entity").is_some_and(|value| !value.is_null()) => {
                let Some(cause) = CausePredicate::decode(fields.get("cause"))? else { return Ok(None); };
                // Variables read the holder of the tool (`which` absent), as ConditionalStatFormula's do.
                let formula = if fields.contains_key("variables") {
                    let Some(formula) = StatFormula::decode_in(fields, &["level", "value", "multiplier"], crate::mining_speed::EntityScope::Holder)? else { return Ok(None); };
                    LevelFormula::Stat(formula)
                } else {
                    let Some(formula) = LevelFormula::decode(fields, &["value", "multiplier"])? else { return Ok(None); };
                    formula
                };
                Self::ReduceDamage { cause, formula, condition: Condition::decode(fields)? }
            }
            "tconstruct:tool_damage_range" if hooks.iter().all(|bound| bound.to_string() == "tconstruct:tool_damage") => {
                // A holder condition needs the holder's state, which the tool damage event does not carry.
                if fields.get("holder").is_some_and(|value| !value.is_null()) { return Ok(None); }
                let bound = |key: &str, default: i32| -> Result<i32, String> {
                    match fields.get(key).filter(|value| !value.is_null()) {
                        None => Ok(default),
                        Some(Value::Number(number)) => { let value = crate::materials::number_int(&number.to_string()); if value < 0 { Err(format!("{key} must not be negative")) } else { Ok(value) } }
                        Some(_) => Err(format!("{key} must be a number")),
                    }
                };
                let Some(cause) = CausePredicate::decode(fields.get("cause"))? else { return Ok(None); };
                Self::DamageRange { min: bound("min", 0)?, max: bound("max", i32::MAX)?, cause, condition: Condition::decode(fields)? }
            }
            "tconstruct:stat_copy" if only("tconstruct:tool_stats") => {
                let (Some(target), Some(source)) = (Stat::from_id(&identifier(fields, "target")?), Stat::from_id(&identifier(fields, "source")?)) else { return Ok(None); };
                Self::StatCopy { target, source, flat: float(fields, "flat")?, each: float(fields, "each_level")?, condition: Condition::decode(fields)? }
            }
            "tconstruct:soulspeed" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:enchantments" | "tconstruct:tooltip")) => {
                // SoulSpeedModule: a constant minecraft:soul_speed enchantment from a leveling int.
                Self::Enchantment { name: "minecraft:soul_speed".to_owned(), flat: short(fields, "flat")? as f32, each: short(fields, "each_level")? as f32, ignoring: false, condition: Condition::decode(fields)? }
            }
            "tconstruct:requirements" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:validate_upgrade" | "tconstruct:requirements")) => {
                // A requirement with a predicate type this port lacks is unbound (the early check above).
                let requirement = Predicate::decode(fields.get("requirement").ok_or("requirement missing")?)?;
                let (min, max) = crate::modifiers::range(fields.get("modifier_level"))?;
                let translation_key = fields.get("translation_key").and_then(Value::as_str).ok_or("translation_key must be a string")?.to_owned();
                Self::Requirements { requirement, min, max, translation_key }
            }
            "tconstruct:repair" if only("tconstruct:repair_factor") => {
                let Some(formula) = LevelFormula::decode(fields, &["factor"])? else { return Ok(None); };
                Self::Repair { formula, condition: Condition::decode(fields)? }
            }
            "tconstruct:stat_boost" if only("tconstruct:tool_stats") => {
                let stat_id = identifier(fields, "stat")?;
                // A stat of another domain is not representable here and stays unbound. The tank
                // capacity stat is this domain's own since the tool tank was ported (tool_tank):
                // Stat::from_id knows tconstruct:tank_capacity, so the tank family's boosts bind.
                let Some(stat) = Stat::from_id(&stat_id) else { return Ok(None); };
                let operation = match fields.get("operation").and_then(Value::as_str) {
                    Some("add") => Operation::Add, Some("percent") => Operation::Percent,
                    Some("multiply_base") => Operation::MultiplyBase,
                    Some("multiply_conditional") => Operation::MultiplyConditional,
                    Some("multiply_all") => Operation::MultiplyAll,
                    _ => return Err("operation must be add, percent, multiply_base, multiply_conditional or multiply_all".into()),
                };
                Self::Boost { stat, operation, flat: float(fields, "flat")?, each: float(fields, "each_level")?, condition: Condition::decode(fields)? }
            }
            "tconstruct:set_stat" if only("tconstruct:tool_stats") => {
                // Only the harvest tier is a settable stat the shipped data uses; others stay unbound.
                if identifier(fields, "stat")?.to_string() != "tconstruct:harvest_tier" { return Ok(None); }
                let tier = identifier(fields, "value")?;
                if !matches!(tier.path(), "wood" | "gold" | "stone" | "iron" | "diamond" | "netherite") { return Err(format!("unknown tool tier {tier}")); }
                Self::SetTier { tier, condition: Condition::decode(fields)? }
            }
            // Display name is presentation and removal is run by the remove paths; neither changes this binding.
            "tconstruct:swappable_slot" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:volatile_data" | "tconstruct:display_name" | "tconstruct:remove")) => {
                Self::SwappableSlot { key: optional_key(fields)?, count: short(fields, "slots")?, condition: Condition::decode(fields)? }
            }
            "tconstruct:swappable_bonus_slot" if only("tconstruct:volatile_data") => Self::BonusSlot {
                key: optional_key(fields)?, matching: slot_name(fields, "match")?, bonus: slot_name(fields, "bonus")?,
                count: short(fields, "slots")?, condition: Condition::decode(fields)?,
            },
            "tconstruct:swappable_tool_traits" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:modifier_traits" | "tconstruct:display_name" | "tconstruct:remove")) => {
                let hook = identifier(fields, "hook")?;
                // ToolTraitHook.supportsHook: the main traits hook and the two alternates the definitions bind.
                if !matches!(hook.to_string().as_str(), "tconstruct:tool_traits" | "tconstruct:rebalanced_trait" | "tconstruct:trim_trait") { return Ok(None); }
                let matching = match fields.get("match") { None | Some(Value::Null) => String::new(), Some(Value::String(text)) => text.clone(), Some(_) => return Err("match must be a string".into()) };
                Self::SwappableTraits { key: optional_key(fields)?, matching, hook }
            }
            // The code-defined creative_slot modifier's module; it binds only volatile data and removal.
            "tconstruct:conditional_mining_speed" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:break_speed" | "tconstruct:tooltip")) => {
                match crate::mining_speed::MiningSpeed::decode(fields)? {
                    Some(module) => Self::MiningSpeed { module: Box::new(module), condition: Condition::decode(fields)? },
                    None => return Ok(None),
                }
            }
            "tconstruct:conditional_stat" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:conditional_stat" | "tconstruct:tooltip")) => {
                match crate::conditional_stat::ConditionalStat::decode(fields)? {
                    Some(module) => Self::ConditionalStat { module: Box::new(module), condition: Condition::decode(fields)? },
                    None => return Ok(None),
                }
            }
            "tconstruct:conditional_power" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:projectile_launch" | "tconstruct:projectile_shot" | "tconstruct:projectile_hit" | "tconstruct:sling_force" | "tconstruct:tooltip")) => {
                match crate::conditional_stat::ConditionalPower::decode(fields)? {
                    Some(module) => Self::ConditionalPower { module: Box::new(module) },
                    None => return Ok(None),
                }
            }
            "tconstruct:conditional_melee_damage" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:melee_damage" | "tconstruct:monster_melee_damage" | "tconstruct:tooltip")) => {
                match crate::melee_damage::MeleeDamage::decode(fields)? {
                    Some(module) => Self::MeleeDamage { module: Box::new(module), condition: Condition::decode(fields)? },
                    None => return Ok(None),
                }
            }
            "tconstruct:weapon_mob_effect" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:melee_hit" | "tconstruct:monster_melee_hit" | "tconstruct:projectile_hit")) => {
                match crate::weapon_effect::WeaponEffect::decode(fields)? {
                    Some(module) => Self::WeaponEffect { module: Box::new(module), condition: Condition::decode(fields)? },
                    None => return Ok(None),
                }
            }
            "tconstruct:tool_usage_mob_effect" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:block_break" | "tconstruct:melee_hit" | "tconstruct:monster_melee_hit" | "tconstruct:projectile_launch" | "tconstruct:projectile_shot" | "tconstruct:projectile_thrown" | "tconstruct:projectile_hit" | "tconstruct:plant_harvest" | "tconstruct:shear_entity" | "tconstruct:sling_launch")) => {
                match crate::weapon_effect::ToolUsage::decode(fields, hooks)? {
                    Some(module) => Self::UsageEffect { module: Box::new(module), condition: Condition::decode(fields)? },
                    None => return Ok(None),
                }
            }
            "tconstruct:block_damage" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:damage_block" | "tconstruct:tooltip")) => {
                let source = match fields.get("damage_source") { None | Some(Value::Null) => crate::protection::SourceCondition::Any, Some(value) => match crate::protection::SourceCondition::decode(value)? { Some(source) => source, None => return Ok(None) } };
                Self::BlockDamage { source, condition: Condition::decode(fields)? }
            }
            "tconstruct:capacity_bar" if hooks.iter().all(|bound| bound.to_string() == "tconstruct:capacity_bar") => {
                match crate::capacity::Bar::decode(fields)? { Some(bar) => Self::CapacityBar { bar }, None => return Ok(None) }
            }
            "tconstruct:melee_capacity" if only("tconstruct:melee_hit") => {
                match crate::capacity::MeleeFill::decode(fields)? { Some(module) => Self::CapacityMelee { module: Box::new(module), condition: Condition::decode(fields)? }, None => return Ok(None) }
            }
            "tconstruct:damage_to_capacity" if only("tconstruct:modify_damage") => {
                match crate::capacity::DamageFill::decode(fields)? { Some(module) => Self::CapacityDamage { module: Box::new(module), condition: Condition::decode(fields)? }, None => return Ok(None) }
            }
            "tconstruct:mining_capacity" if only("tconstruct:block_harvest") => {
                Self::CapacityMining { module: Box::new(crate::capacity::MiningFill::decode(fields)?), condition: Condition::decode(fields)? }
            }
            "tconstruct:launch_capacity" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:projectile_launch" | "tconstruct:projectile_shot" | "tconstruct:projectile_thrown")) => {
                Self::CapacityLaunch { module: Box::new(crate::capacity::LaunchFill::decode(fields)?), condition: Condition::decode(fields)? }
            }
            "tconstruct:lifesteal" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:melee_hit" | "tconstruct:monster_melee_hit" | "tconstruct:projectile_launch" | "tconstruct:projectile_hit" | "tconstruct:tooltip")) => {
                match crate::lifesteal::Lifesteal::decode(fields)? { Some(module) => Self::Lifesteal { module: Box::new(module), condition: Condition::decode(fields)? }, None => return Ok(None) }
            }
            "tconstruct:loot_to_capacity" if only("tconstruct:process_loot") => {
                Self::CapacityLoot { module: Box::new(crate::capacity::LootFill::decode(fields)?), condition: Condition::decode(fields)? }
            }
            "tconstruct:durability_shield" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:tool_damage" | "tconstruct:durability_display")) => {
                match crate::capacity::Shield::decode(fields)? { Some(shield) => Self::DurabilityShield { shield }, None => return Ok(None) }
            }
            "tconstruct:effect_immunity" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:equipment_change")) => {
                Self::EffectImmunity { module: Box::new(crate::effect_immunity::EffectImmunity::decode(fields)?), condition: Condition::decode(fields)? }
            }
            "tconstruct:inventory" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:inventory" | "tconstruct:volatile_data" | "tconstruct:validate" | "tconstruct:remove" | "tconstruct:slot_stack")) => {
                match crate::tool_inventory::Inventory::decode(fields)? {
                    Some(module) => Self::Inventory { module: Box::new(module), condition: Condition::decode(fields)? },
                    None => return Ok(None),
                }
            }
            "tconstruct:quiver_inventory" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:inventory" | "tconstruct:volatile_data" | "tconstruct:validate" | "tconstruct:remove" | "tconstruct:slot_stack")) => {
                match crate::tool_inventory::Inventory::decode_quiver(fields)? {
                    Some(module) => Self::QuiverInventory { module: Box::new(module), condition: Condition::decode(fields)? },
                    None => return Ok(None),
                }
            }
            "tconstruct:update_health" if only("tconstruct:equipment_change") => {
                match crate::wearer_modules::UpdateHealth::decode(fields)? {
                    Some(module) => Self::UpdateHealth { module, condition: Condition::decode(fields)? },
                    None => return Ok(None),
                }
            }
            "tconstruct:inventory_menu" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:armor_interact" | "tconstruct:general_interact")) => {
                Self::InventoryMenu(crate::tool_inventory::MenuKey::decode(fields)?)
            }
            "tconstruct:inventory_slot_menu" if only("tconstruct:slot_stack") => Self::InventorySlotMenu,
            "tconstruct:bulk_quiver" if only("tconstruct:bow_ammo") => {
                // BulkQuiverModule.LOADER: the one field defaults to checking the standard arrows.
                let check = match fields.get("check_standard_arrows").filter(|value| !value.is_null()) {
                    None => true,
                    Some(Value::Bool(value)) => *value,
                    Some(_) => return Err("check_standard_arrows must be a boolean".into()),
                };
                Self::BulkQuiver { check_standard_arrows: check }
            }
            "tconstruct:trick_quiver" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:bow_ammo" | "tconstruct:general_interact" | "tconstruct:remove")) => Self::TrickQuiver,
            "tconstruct:shield_strap" if only("tconstruct:armor_interact") => Self::ShieldStrap { keys: key_list(fields)? },
            "tconstruct:tool_belt" if only("tconstruct:armor_interact") => Self::ToolBelt { keys: key_list(fields)? },
            "tconstruct:share_durability" if only("tconstruct:tool_damage") => Self::ShareDurability {
                shield: identifier(fields, "shield")?,
                grant: leveling_int_keyed(fields, "grant")?,
                consume: leveling_int_keyed(fields, "consume")?,
            },
            "tconstruct:shears" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:entity_interact" | "tconstruct:tool_action")) => {
                // ShearsModule.LOADER: three FROM_ZERO floats, all optional, over a modifier condition.
                Self::Shears { flat: float(fields, "flat")?, per_level: float(fields, "per_level")?, expanded: float(fields, "expanded")?, condition: Condition::decode(fields)? }
            }
            "tconstruct:fishing" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:general_interact" | "tconstruct:tool_action" | "tconstruct:equipment_change")) => Self::Fishing,
            "tconstruct:max_armor_attribute" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:equipment_change" | "tconstruct:tooltip")) => {
                match crate::max_armor::MaxArmorAttribute::decode(fields)? {
                    Some(module) => Self::MaxArmorAttribute { module: Box::new(module), condition: Condition::decode(fields)? },
                    None => return Ok(None),
                }
            }
            "tconstruct:golden_attribute" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:equipment_change" | "tconstruct:tooltip")) => {
                match crate::max_armor::GoldenAttribute::decode(fields)? {
                    Some(module) => Self::GoldenAttribute { module: Box::new(module), condition: Condition::decode(fields)? },
                    None => return Ok(None),
                }
            }
            "tconstruct:lightspeed_attribute" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:boot_walk" | "tconstruct:equipment_change" | "tconstruct:tooltip")) => {
                match crate::wearer_modules::LightspeedAttribute::decode(fields)? {
                    Some(module) => Self::LightspeedAttribute { module, condition: Condition::decode(fields)? },
                    None => return Ok(None),
                }
            }
            "tconstruct:armor_level" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:equipment_change")) => {
                Self::ArmorLevel { module: Box::new(crate::max_armor::ArmorLevel::decode(fields)?) }
            }
            "tconstruct:show_offhand" if only("tconstruct:equipment_change") => {
                // ShowOffhandModule is an enum module: the required mode says whether a broken piece still counts.
                Self::ShowOffhand { allow_broken: match fields.get("mode").and_then(Value::as_str) {
                    Some("allow_broken") => true, Some("disallow_broken") => false,
                    _ => return Err("mode must be allow_broken or disallow_broken".into()),
                } }
            }
            "tconstruct:mob_disguise" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:equipment_change")) => {
                Self::MobDisguise { module: crate::mob_disguise::MobDisguise::decode(fields)? }
            }
            "tconstruct:edible" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:general_interact" | "tconstruct:tool_using" | "tconstruct:on_attacked")) => Self::Edible,
            // A holder condition needs the eater's state, which the finish event does not carry.
            "tconstruct:edible_consume_durability" if hooks.iter().all(|bound| bound.to_string() == "tconstruct:edible_effect") => {
                if crate::edible::has_holder(fields) { return Ok(None); }
                Self::EdibleWear { usage: crate::edible::DurabilityUsage::decode(fields)?, condition: Condition::decode(fields)? }
            }
            "tconstruct:edible_cure_random_effect" if hooks.iter().all(|bound| bound.to_string() == "tconstruct:edible_effect") => {
                if crate::edible::has_holder(fields) { return Ok(None); }
                Self::EdibleCure { condition: Condition::decode(fields)? }
            }
            "tconstruct:edible_remove_effect" if hooks.iter().all(|bound| bound.to_string() == "tconstruct:edible_effect") => {
                if crate::edible::has_holder(fields) { return Ok(None); }
                Self::EdibleRemove { effect: identifier(fields, "effect")?, condition: Condition::decode(fields)? }
            }
            // EdibleRepresentativeItemModule: its hooks are the using tick (the eating sounds) and
            // the edible effect (the list an integration is told); a holder condition needs the
            // eater's state, which neither carries.
            "tconstruct:edible_representative_item" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:tool_using" | "tconstruct:edible_effect")) => {
                if crate::edible::has_holder(fields) { return Ok(None); }
                // ItemStackLoadable.REQUIRED_ITEM_NBT: a bare item id, or an object naming one in `id`.
                let item = match fields.get("representative_item") {
                    Some(Value::String(id)) => Identifier::parse(id).map_err(|e| e.to_string())?,
                    Some(Value::Object(object)) => identifier(object, "id")?,
                    _ => return Err("representative_item must name an item".into()),
                };
                Self::EdibleRepresentative { item, condition: Condition::decode(fields)? }
            }
            "tconstruct:thorns" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:on_attacked")) => {
                match crate::thorns::Thorns::decode(fields)? {
                    Some(module) => Self::Thorns { module: Box::new(module), condition: Condition::decode(fields)? },
                    None => return Ok(None),
                }
            }
            "tconstruct:fiery_attack" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:melee_hit" | "tconstruct:monster_melee_hit" | "tconstruct:projectile_launch" | "tconstruct:projectile_shot" | "tconstruct:projectile_thrown" | "tconstruct:projectile_hit")) => {
                Self::AttackAction { module: Box::new(crate::hit_actions::Attack::decode(crate::hit_actions::Effect::Fire, fields)?), condition: Condition::decode(fields)? }
            }
            "tconstruct:freezing_attack" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:melee_hit" | "tconstruct:monster_melee_hit" | "tconstruct:projectile_hit")) => {
                Self::AttackAction { module: Box::new(crate::hit_actions::Attack::decode(crate::hit_actions::Effect::Freeze, fields)?), condition: Condition::decode(fields)? }
            }
            "tconstruct:fiery_counter" | "tconstruct:freezing_counter" | "tconstruct:knockback_counter" if only("tconstruct:on_attacked") => {
                let effect = match kind { "tconstruct:fiery_counter" => crate::hit_actions::Effect::Fire, "tconstruct:freezing_counter" => crate::hit_actions::Effect::Freeze, _ => crate::hit_actions::Effect::Knockback };
                match crate::hit_actions::Counter::decode(effect, fields)? {
                    Some(module) => Self::CounterAction { module: Box::new(module), condition: Condition::decode(fields)? },
                    None => return Ok(None),
                }
            }
            "tconstruct:counter_mob_effect" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:on_attacked")) => {
                match crate::weapon_effect::WeaponEffect::decode_counter(fields)? {
                    Some(module) => Self::CounterEffect { module: Box::new(module), condition: Condition::decode(fields)? },
                    None => return Ok(None),
                }
            }
            "tconstruct:armor_attack_mob_effect" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:damage_dealt")) => {
                match crate::weapon_effect::WeaponEffect::decode_armor_attack(fields)? {
                    Some(module) => Self::ArmorAttackEffect { module: Box::new(module), condition: Condition::decode(fields)? },
                    None => return Ok(None),
                }
            }
            "tconstruct:mob_effect" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:on_attacked" | "tconstruct:melee_hit" | "tconstruct:monster_melee_hit" | "tconstruct:projectile_hit")) => {
                match crate::weapon_effect::WeaponEffect::decode_legacy(fields)? {
                    Some((weapon, counter)) => Self::LegacyEffect { weapon: Box::new(weapon), counter: Box::new(counter), condition: Condition::decode(fields)? },
                    None => return Ok(None),
                }
            }
            "tconstruct:knockback" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:melee_hit" | "tconstruct:tooltip")) => {
                match crate::knockback::Knockback::decode(fields)? {
                    Some(module) => Self::Knockback { module: Box::new(module), condition: Condition::decode(fields)? },
                    None => return Ok(None),
                }
            }
            // ExtinguishCampfireModule is a BlockTransformModule like the tool action transform: a right click that puts
            // out a lit campfire. Like the transform sharing its consumer, only the clicked block is transformed, not the
            // tool's AOE neighbours (BlockTransformModule.afterBlockUse runs both in the original).
            "tconstruct:campfire_extinguish" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:block_interact" | "tconstruct:aoe_highlight")) => {
                Self::CampfireExtinguish { condition: Condition::decode(fields)? }
            }
            // BlockingModule: a right click raises the tool like a shield (the per-stack use the blocking hook registers).
            "tconstruct:blocking" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:general_interact" | "tconstruct:tool_action")) => Self::Blocking,
            "tconstruct:tool_action_transform" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:block_interact" | "tconstruct:tool_action" | "tconstruct:aoe_highlight")) => {
                let Some(action) = crate::tool_actions::ToolAction::parse(fields.get("tool_action").and_then(Value::as_str).ok_or("tool_action missing")?) else { return Ok(None); };
                let sound = fields.get("sound").and_then(Value::as_str).ok_or("sound missing")?.to_owned();
                let require_ground = match fields.get("require_ground") { Some(Value::Bool(value)) => *value, None | Some(Value::Null) => false, Some(_) => return Err("require_ground must be boolean".into()) };
                Self::ToolAction { action, sound, require_ground, condition: Condition::decode(fields)? }
            }
            // ToolActionWalkerTransformModule (BOOT_WALK + TOOL_ACTION): the boot form of the transform above.
            "tconstruct:tool_action_walk_transform" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:boot_walk" | "tconstruct:tool_action")) => {
                let Some(action) = crate::tool_actions::ToolAction::parse(fields.get("tool_action").and_then(Value::as_str).ok_or("tool_action missing")?) else { return Ok(None); };
                let sound = fields.get("sound").and_then(Value::as_str).ok_or("sound missing")?.to_owned();
                Self::WalkTransform { action, sound, radius: crate::wearer_modules::Leveling::decode(fields.get("radius"), "radius")?, condition: Condition::decode(fields)? }
            }
            // FireWalkerModule, GlowWalkerModule, CoverGroundWalkerModule (BOOT_WALK): the walk
            // forms in walk_hook, over the sturdy-face fact PlutoLib publishes from the game's
            // block table and the world light read.
            "tconstruct:fire_walker" if hooks.iter().all(|bound| bound.to_string() == "tconstruct:boot_walk") => {
                Self::FireWalker { radius: crate::wearer_modules::Leveling::decode(fields.get("radius"), "radius")? }
            }
            "tconstruct:glow_walker" if hooks.iter().all(|bound| bound.to_string() == "tconstruct:boot_walk") => {
                let min_light = fields.get("min_light").and_then(Value::as_i64).ok_or("min_light must be an integer")?;
                let damage = fields.get("tool_damage").and_then(Value::as_i64).ok_or("tool_damage must be an integer")?;
                Self::GlowWalker { radius: crate::wearer_modules::Leveling::decode(fields.get("radius"), "radius")?, min_light: min_light as i32, damage: damage as i32 }
            }
            "tconstruct:cover_ground" if hooks.iter().all(|bound| bound.to_string() == "tconstruct:boot_walk") => {
                Self::CoverGround { block: identifier(fields, "block")?, radius: crate::wearer_modules::Leveling::decode(fields.get("radius"), "radius")?, condition: Condition::decode(fields)? }
            }
            // Presentation-only modules: one block, bound as recorded no-ops (Java: display name, durability display and tooltip hooks only).
            "tconstruct:show_interaction_source" if hooks.iter().all(|bound| bound.to_string() == "tconstruct:display_name") => Self::Display(Display::InteractionSource),
            "tconstruct:variant_color" if hooks.iter().all(|bound| bound.to_string() == "tconstruct:display_name") => Self::Display(Display::VariantColor),
            "tconstruct:material_variant_color" if hooks.iter().all(|bound| bound.to_string() == "tconstruct:display_name") => Self::Display(Display::MaterialVariantColor { material: identifier(fields, "material")? }),
            "tconstruct:variant_name" if hooks.iter().all(|bound| bound.to_string() == "tconstruct:display_name") => Self::Display(Display::VariantName { formatter: identifier(fields, "formatter")? }),
            "tconstruct:durability_color" if hooks.iter().all(|bound| bound.to_string() == "tconstruct:durability_display") => {
                let text = fields.get("color").and_then(Value::as_str).ok_or("color must be a hex string")?;
                let color = i32::from_str_radix(text.trim_start_matches('#'), 16).ok().filter(|color| (0..=0xFF_FFFF).contains(color)).ok_or("color must be an RGB hex value")?;
                Self::Display(Display::DurabilityColor { color })
            }
            "tconstruct:stat_tooltip" if hooks.iter().all(|bound| bound.to_string() == "tconstruct:tooltip") => Self::Display(Display::StatTooltip { stat: identifier(fields, "stat")?, condition: Condition::decode(fields)? }),
            // The cosmetic modules: presentation in the original too (a coloured or formatted display name, a tooltip of
            // patterns); their removal halves forget what they stored, which `on_removed` below runs.
            "tconstruct:banner" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:display_name" | "tconstruct:tooltip")) => Self::Display(Display::BannerPatterns),
            "tconstruct:dye" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:display_name" | "tconstruct:remove")) => Self::Display(Display::Dyed),
            "tconstruct:trim" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:display_name" | "tconstruct:remove")) => Self::Display(Display::Trim),
            // EmbellishmentModule: display presentation as the other cosmetics, plus a raw data half the
            // rebuild runs (the stored variant follows material redirects) and a removal half below.
            "tconstruct:embellishment" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:display_name" | "tconstruct:remove" | "tconstruct:raw_data")) => Self::Display(Display::Embellished),
            // TheOneProbeModule: a singleton writing two keys into the tool's top-level data on build.
            "tconstruct:the_one_probe" if hooks.iter().all(|bound| bound.to_string() == "tconstruct:raw_data") => Self::TheOneProbe,
            "tconstruct:tool_actions" if hooks.iter().all(|bound| bound.to_string() == "tconstruct:tool_action") => {
                let names = fields.get("tool_actions").and_then(Value::as_array).ok_or("tool_actions must be an array")?;
                let mut actions = Vec::new();
                for name in names { let name = name.as_str().filter(|name| !name.is_empty()).ok_or("tool action must be a name")?; if !actions.iter().any(|known| known == name) { actions.push(name.to_owned()); } }
                Self::ToolActions { actions, condition: Condition::decode(fields)? }
            }
            "tconstruct:constant_enchantment" | "tconstruct:enchantment_ignoring_protection" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:enchantments" | "tconstruct:tooltip" | "tconstruct:protection")) && (kind == "tconstruct:enchantment_ignoring_protection" || !hooks.iter().any(|bound| bound.to_string() == "tconstruct:protection")) => {
                let name = identifier(fields, "name")?.to_string();
                let (flat, each) = leveling_int(fields)?;
                Self::Enchantment { name, flat, each, ignoring: kind == "tconstruct:enchantment_ignoring_protection", condition: Condition::decode(fields)? }
            }
            // LootingModule.Weapon: the looting level the tool adds when it kills. The game takes the looting of the killer's weapon from the weapon's own
            // enchantments, so a module whose holder, target and damage source predicates accept everything is exactly a constant looting enchantment.
            "tconstruct:weapon_looting" if only("tconstruct:weapon_looting") => {
                if !["holder", "target", "damage_source"].iter().all(|key| is_any_predicate(fields, key)) { return Ok(None); }
                let (flat, each) = leveling_int(fields)?;
                Self::Enchantment { name: "minecraft:looting".to_owned(), flat, each, ignoring: false, condition: Condition::decode(fields)? }
            }
            // LootingModule.Armor: a worn piece adds its level to the looting of its wearer's kills in the listed slots. The loot
            // flows of this port carry no holder, target or damage source, so only the ANY forms bind (as for weapon looting).
            "tconstruct:armor_looting" if only("tconstruct:armor_looting") => {
                if !["holder", "target", "damage_source"].iter().all(|key| is_any_predicate(fields, key)) { return Ok(None); }
                let (flat, each) = leveling_int(fields)?;
                let slots = fields.get("slots").and_then(Value::as_array).ok_or("slots must be an array")?.iter()
                    .map(|slot| match slot.as_str() { Some(name @ ("mainhand" | "offhand" | "feet" | "legs" | "chest" | "head")) => Ok(name.to_owned()), _ => Err("unknown equipment slot".to_owned()) })
                    .collect::<Result<Vec<_>, _>>()?;
                Self::ArmorLooting { flat, each, slots, condition: Condition::decode(fields)? }
            }
            // EnchantmentModule.ArmorHarvest: a worn piece lends its enchantment to the breaking stack for the loot roll.
            // Its block and holder predicates have no facts in the break flow, so only the ANY forms bind (as for looting).
            "tconstruct:armor_harvest_enchantment" if only("tconstruct:harvest_enchantments") => {
                if !["block", "holder"].iter().all(|key| is_any_predicate(fields, key)) { return Ok(None); }
                let name = identifier(fields, "name")?.to_string();
                let (flat, each) = leveling_int(fields)?;
                let slots = fields.get("slots").and_then(Value::as_array).ok_or("slots must be an array")?.iter()
                    .map(|slot| match slot.as_str() { Some(name @ ("mainhand" | "offhand" | "feet" | "legs" | "chest" | "head")) => Ok(name.to_owned()), _ => Err("unknown equipment slot".to_owned()) })
                    .collect::<Result<Vec<_>, _>>()?;
                Self::ArmorHarvestEnchantment { name, flat, each, slots, condition: Condition::decode(fields)? }
            }
            // AutosmeltModule: drops a cooking recipe takes become their cooked result.
            "tconstruct:autosmelt" if only("tconstruct:process_loot") => {
                let recipe_types = match fields.get("recipe_types") {
                    Some(Value::String(one)) => vec![one.clone()],
                    Some(Value::Array(many)) => many.iter().map(|kind| kind.as_str().map(str::to_owned).ok_or("recipe_types must name recipe types")).collect::<Result<Vec<_>, _>>()?,
                    _ => return Err("recipe_types must be a recipe type or a list of them".into()),
                };
                Self::Autosmelt { recipe_types, extra_drop_chance: float(fields, "extra_drop_chance")? }
            }
            // SmeltingModule: the module's own fields are its input inventory's (InventoryModule.LOADER
            // is a direct field); the output inventory is built on it, keyed by `output_key` or the
            // modifier's ID with an `_output` suffix, patterned by `output_pattern`.
            "tconstruct:smelting" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:melee_hit" | "tconstruct:monster_melee_hit" | "tconstruct:launcher_hit" | "tconstruct:block_harvest" | "tconstruct:projectile_launch" | "tconstruct:on_attacked" | "tconstruct:plant_harvest" | "tconstruct:shear_entity" | "tconstruct:sling_launch")) => {
                let Some(input) = crate::tool_inventory::Inventory::decode(fields)? else { return Ok(None) };
                let optional_id = |key: &str| match fields.get(key).filter(|value| !value.is_null()) {
                    None => Ok(None),
                    Some(value) => Identifier::parse(value.as_str().ok_or_else(|| format!("{key} must be an ID string"))?).map(Some).map_err(|e| e.to_string()),
                };
                let output_pattern = match optional_id("output_pattern")? {
                    Some(pattern) => pattern,
                    None => Identifier::parse("tconstruct:result").expect("valid id"),
                };
                Self::Smelting {
                    recipe_type: identifier(fields, "recipe_type")?.to_string(),
                    multiplier: float(fields, "multiplier")?,
                    output: Box::new(input.as_output(optional_id("output_key")?, Some(output_pattern))),
                    input: Box::new(input),
                    condition: Condition::decode(fields)?,
                }
            }
            "tconstruct:severing" if only("tconstruct:process_loot") => Self::Severing,
            "tconstruct:adjust_damage" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:modify_damage" | "tconstruct:tooltip")) => {
                let Some(module) = crate::protection::AdjustDamage::decode(fields)? else { return Ok(None); };
                Self::AdjustDamage { module, condition: Condition::decode(fields)? }
            }
            "tconstruct:protection" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:protection" | "tconstruct:tooltip")) => {
                let Some(module) = crate::protection::Protection::decode(fields)? else { return Ok(None); };
                Self::Protection { module, condition: Condition::decode(fields)? }
            }
            // FlameBarrierModule: a leveling amount (its fields are the value itself, LevelingValue.directField).
            "tconstruct:flame_barrier" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:protection" | "tconstruct:tooltip")) => {
                Self::FlameBarrier { flat: float(fields, "flat")?, each: float(fields, "each_level")?, condition: Condition::decode(fields)? }
            }
            "tconstruct:depth_protection" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:protection" | "tconstruct:tooltip")) => {
                let Some(module) = crate::wearer_modules::DepthProtection::decode(fields)? else { return Ok(None); };
                Self::DepthProtection { module, condition: Condition::decode(fields)? }
            }
            "tconstruct:overshield" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:protection" | "tconstruct:tooltip")) => {
                Self::Overshield { module: crate::wearer_modules::Overshield::decode(fields)? }
            }
            // Only on the modify hurt hook: this port hears that hook on its modify damage event, whose amount is the
            // damage after vanilla's armor and enchantment protection, as the original's LivingHurtEvent hands it over.
            "tconstruct:recurrent_protection" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:modify_hurt" | "tconstruct:tooltip")) => {
                Self::RecurrentProtection { module: crate::wearer_modules::RecurrentProtection::decode(fields)? }
            }
            // On the modify hurt hook the module works on the damage as it arrives, before armor; on the modify damage hook
            // (the default binding, and the shipped rebuff) on what armor left. A binding to no stage runs nowhere.
            "tconstruct:restore_lost_health" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:modify_hurt" | "tconstruct:modify_damage" | "tconstruct:tooltip")) => {
                let hurt = hooks.iter().any(|bound| bound.to_string() == "tconstruct:modify_hurt");
                let damage = hooks.is_empty() || hooks.iter().any(|bound| bound.to_string() == "tconstruct:modify_damage");
                if !hurt && !damage { return Ok(None); }
                let Some(module) = crate::wearer_modules::RestoreLostHealth::decode(fields)? else { return Ok(None); };
                Self::RestoreLostHealth { module: Box::new(module), condition: Condition::decode(fields)?, hurt, damage }
            }
            "tconstruct:damage_on_unequip" if hooks.iter().all(|bound| bound.to_string() == "tconstruct:equipment_change") => {
                Self::DamageOnUnequip { module: crate::wearer_modules::DamageOnUnequip::decode(fields)?, condition: Condition::decode(fields)? }
            }
            "tconstruct:reduce_effect_on_unequip" if hooks.iter().all(|bound| bound.to_string() == "tconstruct:equipment_change") => {
                Self::ReduceEffectOnUnequip { module: crate::wearer_modules::ReduceEffectOnUnequip::decode(fields)?, condition: Condition::decode(fields)? }
            }
            "tconstruct:clear_effect_on_unequip" if hooks.iter().all(|bound| bound.to_string() == "tconstruct:equipment_change") => {
                Self::ClearEffectOnUnequip { effect: identifier(fields, "effect")?, condition: Condition::decode(fields)? }
            }
            "tconstruct:attribute" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:attributes" | "tconstruct:equipment_change" | "tconstruct:tooltip")) => {
                // Only vanilla attributes exist in the host; a Tinkers or Forge attribute stays unbound. The Tinkers ones
                // (TinkerAttributes: generic.bouncy, generic.jump_boost, generic.knockback_multiplier, the good and bad effect
                // duration multipliers, generic.crouch_damage_multiplier, generic.chrysophilite, player.use_item_speed,
                // player.mining_speed_multiplier, player.jump_count, player.critical_damage, player.experience_multiplier)
                // cannot bind yet for two reasons: PlutoLib has no way to register an attribute the game does not have, so a
                // stack entry naming one would be data nothing applies, and every reader of those attributes in the original
                // is a Tinkers event handler over a game system PlutoLib does not report (experience gains, jumps and falls,
                // knockback taken, critical hits, effect applications, using an item, swimming). The two with a reader in
                // this port are the exceptions above and below: the protection cap is the port's own protection math, and
                // chrysophilite is worked out in the loot flow itself. Binding the rest waits on attribute registration.
                let name = fields.get("attribute").and_then(Value::as_str).ok_or("attribute missing")?;
                if name == "tconstruct:generic.protection_cap" && fields.get("operation").and_then(Value::as_str) == Some("addition") {
                    let Some(formula) = StatFormula::decode(fields, &["level", "value", "multiplier", "base"])? else { return Ok(None); };
                    return Ok(Some(Self::ProtectionCap { formula, condition: Condition::decode(fields)? }));
                }
                // Forge's gravity, reach and step height attributes are the vanilla ones under the game's own names, with the same
                // base and operations; the Tinkers safe fall distance is a distance added to the vanilla safe fall distance.
                let name = match name {
                    "forge:entity_gravity" => "minecraft:gravity", "forge:block_reach" => "minecraft:block_interaction_range",
                    "forge:entity_reach" => "minecraft:entity_interaction_range", "forge:step_height_addition" => "minecraft:step_height",
                    "tconstruct:generic.safe_fall_distance" => "minecraft:safe_fall_distance", other => other,
                };
                let Some(path) = name.strip_prefix("minecraft:") else { return Ok(None); };
                let path = path.strip_prefix("generic.").or_else(|| path.strip_prefix("player.")).unwrap_or(path);
                let operation = match fields.get("operation").and_then(Value::as_str) {
                    Some("addition") => "add_value", Some("multiply_base") => "add_multiplied_base", Some("multiply_total") => "add_multiplied_total",
                    _ => return Ok(None),
                };
                // The original's formula has only `level` by default; a variable naming an input this port lacks leaves it unbound.
                let Some(formula) = StatFormula::decode(fields, &["level", "value", "multiplier", "base"])? else { return Ok(None); };
                let slots = fields.get("slots").and_then(Value::as_array).ok_or("slots must be an array")?.iter()
                    .map(|slot| match slot.as_str() { Some(name @ ("mainhand" | "offhand" | "feet" | "legs" | "chest" | "head")) => Ok(name.to_owned()), _ => Err("unknown equipment slot".to_owned()) })
                    .collect::<Result<Vec<_>, _>>()?;
                Self::Attribute { attribute: format!("minecraft:{path}"), operation, formula, slots, condition: Condition::decode(fields)? }
            }
            // PunchModule: knockback for the tool's shots, set on plain arrows and added on knockback projectiles.
            "tconstruct:punch" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:projectile_launch" | "tconstruct:projectile_shot")) => Self::Punch { module: Box::new(crate::punch::Punch::decode(fields)?), condition: Condition::decode(fields)? },
            // ReversePunchModule: a knockback projectile pushes back on the entity it hits.
            "tconstruct:reverse_punch" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:projectile_hit" | "tconstruct:projectile_hit_client")) => Self::ReversePunch { module: Box::new(crate::punch::ReversePunch::decode(fields)?) },
            // MeleeInstrumentModule: a melee hit plays the instrument the tool is made of. A predicate on a fact this port
            // cannot report leaves the module unbound, as the other predicate modules do.
            "tconstruct:melee_instrument" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:melee_hit" | "tconstruct:monster_melee_hit" | "tconstruct:damage_dealt")) => {
                let Some(module) = crate::melee_instrument::MeleeInstrument::decode(fields)? else { return Ok(None); };
                Self::MeleeInstrument { module: Box::new(module), condition: Condition::decode(fields)? }
            }
            // OvergrowthModule: overslime regrows while the tool sits in an inventory.
            "tconstruct:overgrowth" if hooks.iter().all(|bound| bound.to_string() == "tconstruct:inventory_tick") => Self::Overgrowth { module: Box::new(crate::overgrowth::Overgrowth::decode(fields)?), condition: Condition::decode(fields)? },
            // The tool tank family (tool_tank). The tank module is code-only in the original —
            // TankModule has no data serializer — so this arm binds the code modifier's JSON.
            "tconstruct:tank" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:volatile_data" | "tconstruct:validate" | "tconstruct:remove" | "tconstruct:slot_stack" | "tconstruct:display_name")) => Self::Tank,
            "tconstruct:bucket" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:block_interact" | "tconstruct:general_interact")) => {
                let Some(module) = crate::tool_tank::Bucket::decode(fields)? else { return Ok(None); };
                Self::Bucket { module }
            }
            "tconstruct:tank_interaction" if only("tconstruct:block_interact") => {
                let Some(module) = crate::tool_tank::TankInteraction::decode(fields)? else { return Ok(None); };
                Self::TankInteraction { module }
            }
            "tconstruct:melting" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:melee_hit" | "tconstruct:monster_melee_hit" | "tconstruct:launcher_hit" | "tconstruct:process_loot")) => Self::Melting {
                module: Box::new(crate::tool_tank::Melting::decode(fields)?),
                condition: Condition::decode(fields)?,
            },
            "tconstruct:slurping" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:general_interact" | "tconstruct:tool_using" | "tconstruct:armor_interact" | "tconstruct:inventory_tick" | "tconstruct:equipment_change")) => Self::Slurping { module: Box::new(crate::tool_tank::Slurping::decode(fields)?) },
            "tconstruct:spilling" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:melee_hit" | "tconstruct:monster_melee_hit" | "tconstruct:launcher_hit")) => Self::Spilling {
                module: crate::tool_tank::Spilling::decode(fields)?,
                condition: Condition::decode(fields)?,
            },
            "tconstruct:spitting" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:general_interact" | "tconstruct:armor_interact" | "tconstruct:inventory_tick")) => Self::Spitting { module: Box::new(crate::tool_tank::Spitting::decode(fields)?) },
            "tconstruct:splashing" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:entity_interact" | "tconstruct:block_interact" | "tconstruct:aoe_highlight")) => Self::Splashing { module: Box::new(crate::tool_tank::Splashing::decode(fields)?) },
            "tconstruct:overburn" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:inventory_tick" | "tconstruct:remove")) => Self::Overburn,
            "tconstruct:smashing" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:projectile_launch" | "tconstruct:projectile_shot" | "tconstruct:projectile_hit" | "tconstruct:projectile_fuse" | "tconstruct:volatile_data" | "tconstruct:validate" | "tconstruct:remove" | "tconstruct:display_name" | "tconstruct:tooltip")) => Self::Smashing,
            // KineticModule: being hit feeds the wearer's insatiable effect.
            "tconstruct:kinetic" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:on_attacked" | "tconstruct:tooltip")) => Self::Kinetic,
            // The code modifiers' module kinds: they occur only in the definitions modifiers.rs
            // builds in code for the original's class-based modifiers, never in shipped data.
            "tconstruct:conducting" => Self::Conducting,
            "tconstruct:insatiable" => Self::Insatiable,
            "tconstruct:lacerating" => Self::Lacerating,
            "tconstruct:momentum" => Self::Momentum,
            "tconstruct:self_destructive" => Self::SelfDestructive,
            "tconstruct:strong_bones" => Self::StrongBones,
            "tconstruct:enderporting" => Self::Enderporting,
            "tconstruct:harvest" => Self::Harvest,
            "tconstruct:dwarven" => Self::Dwarven,
            "tconstruct:enderclearance" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:melee_hit" | "tconstruct:on_attacked" | "tconstruct:tooltip")) => {
                Self::Enderclearance { chance: leveling_keyed(fields, "chance")?, diameter: leveling_keyed(fields, "diameter")? }
            }
            "tconstruct:channeling" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:melee_hit" | "tconstruct:tooltip")) => {
                let allow_melee = match fields.get("allow_melee") { None | Some(Value::Null) => true, Some(Value::Bool(value)) => *value, Some(_) => return Err("allow_melee must be boolean".into()) };
                Self::Channeling { allow_melee, chance_clear: float(fields, "chance_clear")?, chance_rain: float(fields, "chance_rain")?, chance_thunder: float(fields, "chance_thunder")? }
            }
            // DamageOnShootModule: firing the tool hurts its holder (the sling half has no event here; a mob
            // shooting cannot be hurt back either, as the launch event takes no actions).
            "tconstruct:damage_on_shoot" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:projectile_launch" | "tconstruct:projectile_shot" | "tconstruct:projectile_thrown" | "tconstruct:sling_launch")) => {
                Self::DamageOnShoot { module: crate::projectile_modules::DamageOnShoot::decode(fields)?, condition: Condition::decode(fields)? }
            }
            // InfinityModule: the bow ammo half binds on the ammo event; the removal half is run by the remove paths.
            "tconstruct:infinity" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:bow_ammo" | "tconstruct:projectile_launch" | "tconstruct:projectile_shot" | "tconstruct:remove")) => {
                let Some(module) = crate::projectile_modules::Infinity::decode(fields)? else { return Ok(None); };
                Self::Infinity { module }
            }
            // The in-flight modules: bound in flight_hook on the spawn, flight and impact tellings.
            "tconstruct:arrow_pierce" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:projectile_shot")) => {
                Self::ArrowPierce { module: crate::projectile_modules::ArrowPierce::decode(fields)? }
            }
            "tconstruct:projectile_bounce" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:projectile_hit")) => {
                Self::ProjectileBounce { module: crate::projectile_modules::ProjectileBounce::decode(fields)? }
            }
            "tconstruct:projectile_gravity" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:projectile_shot" | "tconstruct:schedule_projectile_task")) => {
                Self::ProjectileGravity { module: crate::projectile_modules::ProjectileGravity::decode(fields)? }
            }
            "tconstruct:projectile_fuse" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:projectile_shot" | "tconstruct:schedule_projectile_task")) => {
                Self::ProjectileFuse { module: crate::projectile_modules::ProjectileFuse::decode(fields)? }
            }
            "tconstruct:projectile_explosion" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:projectile_hit" | "tconstruct:projectile_fuse")) => {
                Self::ProjectileExplosion { module: crate::projectile_modules::ProjectileExplosion::decode(fields)? }
            }
            "tconstruct:projectile_attract_mobs" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:projectile_hit" | "tconstruct:projectile_fuse")) => {
                Self::ProjectileAttractMobs { module: crate::projectile_modules::ProjectileAttractMobs::decode(fields)? }
            }
            "tconstruct:projectile_place_glow" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:projectile_hit" | "tconstruct:projectile_launch")) => {
                Self::ProjectilePlaceGlow { module: crate::projectile_modules::ProjectilePlaceGlow::decode(fields)? }
            }
            "tconstruct:restrict_projectile_angle" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:projectile_shot" | "tconstruct:sling_launch")) => {
                Self::RestrictAngle { module: crate::projectile_modules::RestrictAngle::decode(fields)? }
            }
            // ThrowingModule: the shipped record declares no hooks (the interaction hooks are the class's own).
            "tconstruct:throwing" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:general_interaction")) => Self::Throwing,
            "tconstruct:stat_override" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:tool_stats" | "tconstruct:remove")) => Self::StatOverride,
            "tconstruct:creative_slots" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:volatile_data" | "tconstruct:remove")) => Self::CreativeSlots,
            _ => return Ok(None),
        }))
    }

    /// The volatile-data hook. Modules of other hooks do nothing here.
    pub fn apply_volatile(&self, entry: &Entry, facts: &Facts, data: &mut NbtCompound) {
        match self {
            Self::Flag { id, condition } => { if condition.matches(entry, facts) { data.insert(id.to_string(), Nbt::Byte(1)); } }
            Self::Rarity(ordinal) => {
                let current = nbt_int(data.get("tconstruct:rarity"));
                if *ordinal > current { data.insert("tconstruct:rarity".into(), Nbt::Int(*ordinal)); }
            }
            Self::Sweep { flat, each } => {
                // addVolatileData: the value for the effective level joins the sweep percent already there.
                let previous = data.get("tconstruct:sweep_percent").and_then(|value| value.as_f64()).unwrap_or(0.0) as f32;
                data.insert("tconstruct:sweep_percent".into(), Nbt::Float(previous + flat + each * entry.effective_level()));
            }
            Self::Int { id, flat, each, condition } => {
                if condition.matches(entry, facts) {
                    // LevelingInt.compute(float): (int)(flat + eachLevel * level), added with Java wrapping.
                    let amount = (*flat as f32 + *each as f32 * entry.effective_level()) as i32;
                    let previous = nbt_int(data.get(&id.to_string()));
                    data.insert(id.to_string(), Nbt::Int(previous.wrapping_add(amount)));
                }
            }
            Self::Slot { name, flat, each, condition } => {
                if condition.matches(entry, facts) {
                    // computeForLevel: nothing for a non-positive level; addSlots ignores 0 and removes a total of 0.
                    let level = entry.effective_level();
                    let count = if level > 0.0 { (*flat as f32 + *each as f32 * level) as i32 } else { 0 };
                    if count != 0 {
                        let total = nbt_int(data.get(name)).wrapping_add(count);
                        if total == 0 { data.remove(name); } else { data.insert(name.clone(), Nbt::Int(total)); }
                    }
                }
            }
            Self::SwappableSlot { key, count, condition } => {
                if condition.matches(entry, facts) {
                    let name = (facts.persistent_string)(key.as_ref().unwrap_or(&entry.id));
                    if is_known_slot(&name) { add_slots(data, &name, *count); }
                }
            }
            Self::BonusSlot { key, matching, bonus, count, condition } => {
                if condition.matches(entry, facts) {
                    let name = (facts.persistent_string)(key.as_ref().unwrap_or(&entry.id));
                    if !name.is_empty() && *matching == name { add_slots(data, bonus, *count); }
                }
            }
            Self::Float { id, flat, each, condition } => {
                if condition.matches(entry, facts) {
                    // VolatileFloatModule.updateVolatileData: the computed value is added onto the flag's.
                    let previous = data.get(&id.to_string()).and_then(Nbt::as_f64).unwrap_or(0.0) as f32;
                    data.insert(id.to_string(), Nbt::Float(previous + *flat + *each * entry.effective_level()));
                }
            }
            Self::Inventory { module, condition } | Self::QuiverInventory { module, condition } => {
                // ToolInventoryCapability.addSlots, when the module's condition holds.
                if condition.matches(entry, facts) { crate::tool_inventory::add_slots(data, module.volatile_slots(entry)); }
            }
            Self::Smelting { input, output, condition, .. } => {
                // The smelting module adds both of its inventories (addModules registers each).
                if condition.matches(entry, facts) {
                    crate::tool_inventory::add_slots(data, input.volatile_slots(entry));
                    crate::tool_inventory::add_slots(data, output.volatile_slots(entry));
                }
            }
            Self::InventoryMenu(_) | Self::InventorySlotMenu | Self::BulkQuiver { .. } | Self::TrickQuiver | Self::ShieldStrap { .. } | Self::ToolBelt { .. } | Self::ShareDurability { .. } | Self::Shears { .. } | Self::Fishing => {}
            Self::StatOverride | Self::ConditionalStat { .. } | Self::ConditionalPower { .. } | Self::MiningSpeed { .. } | Self::MeleeDamage { .. } | Self::Knockback { .. } | Self::WeaponEffect { .. } | Self::UsageEffect { .. } | Self::CounterEffect { .. } | Self::ArmorAttackEffect { .. } | Self::AttackAction { .. } | Self::CounterAction { .. } | Self::LegacyEffect { .. } | Self::Thorns { .. } | Self::EffectImmunity { .. } | Self::MobDisguise { .. } | Self::MaxArmorAttribute { .. } | Self::GoldenAttribute { .. } | Self::LightspeedAttribute { .. } | Self::ArmorLevel { .. } | Self::ShowOffhand { .. } | Self::CapacityBar { .. } | Self::CapacityMelee { .. } | Self::CapacityLaunch { .. } | Self::Lifesteal { .. } | Self::CapacityDamage { .. } | Self::DurabilityShield { .. } | Self::BlockDamage { .. } | Self::Attribute { .. } | Self::ToolAction { .. } | Self::Display(_) | Self::ToolActions { .. } | Self::Enchantment { .. } | Self::Protection { .. } | Self::ProtectionCap { .. } | Self::FlameBarrier { .. } | Self::ClearEffectOnUnequip { .. } | Self::Edible | Self::EdibleWear { .. } | Self::EdibleCure { .. } | Self::EdibleRemove { .. } | Self::EdibleRepresentative { .. } | Self::AdjustDamage { .. } | Self::DepthProtection { .. } | Self::Overshield { .. } | Self::RecurrentProtection { .. } | Self::RestoreLostHealth { .. } | Self::DamageOnUnequip { .. } | Self::ReduceEffectOnUnequip { .. } | Self::CapacityMining { .. } | Self::CapacityLoot { .. } | Self::Severing | Self::ArmorLooting { .. } | Self::ArmorHarvestEnchantment { .. } | Self::Autosmelt { .. } | Self::CraftCount { .. } | Self::MaterialRepair { .. } | Self::UpdateHealth { .. } | Self::Punch { .. } | Self::ReversePunch { .. } | Self::DamageOnShoot { .. } | Self::Infinity { .. } | Self::CampfireExtinguish { .. } | Self::Blocking | Self::WalkTransform { .. } | Self::FireWalker { .. } | Self::GlowWalker { .. } | Self::CoverGround { .. } | Self::MeleeInstrument { .. } | Self::Overgrowth { .. } | Self::Kinetic | Self::TheOneProbe | Self::Conducting | Self::Insatiable | Self::Lacerating | Self::Momentum | Self::Dwarven | Self::Enderporting | Self::Harvest | Self::SelfDestructive | Self::StrongBones | Self::Enderclearance { .. } | Self::Channeling { .. } | Self::ArrowPierce { .. } | Self::ProjectileBounce { .. } | Self::ProjectileGravity { .. } | Self::ProjectileFuse { .. } | Self::ProjectileExplosion { .. } | Self::ProjectileAttractMobs { .. } | Self::ProjectilePlaceGlow { .. } | Self::RestrictAngle { .. } | Self::Throwing => {}
            Self::CreativeSlots => {
                // addSlots for each stored slot type that exists (a valid slot name here).
                for (name, count) in (facts.persistent_numbers)(&Identifier::parse(CREATIVE_KEY).expect("valid key")) {
                    if is_known_slot(&name) { add_slots(data, &name, count as i32); }
                }
            }
            Self::SwappableTraits { .. } | Self::Boost { .. } | Self::SetTier { .. } | Self::Trait { .. } | Self::ReduceDamage { .. } | Self::DamageRange { .. } | Self::StatCopy { .. } | Self::Repair { .. } | Self::Requirements { .. } | Self::Tank | Self::Bucket { .. } | Self::TankInteraction { .. } | Self::Melting { .. } | Self::Slurping { .. } | Self::Spilling { .. } | Self::Spitting { .. } | Self::Splashing { .. } | Self::Overburn | Self::Smashing => {}
        }
    }

    /// RawDataModifierHook.addRawData over the tool's top-level data (the root of its custom data,
    /// where another mod reads it). Run by the rebuild into a fresh compound, like the volatile pass.
    pub fn apply_raw_data(&self, _entry: &Entry, facts: &Facts, data: &mut NbtCompound) {
        match self {
            // TheOneProbeModule: the hand key for a tool in the held tag, the helmet key for a helmet.
            Self::TheOneProbe => {
                if let Some(item) = facts.item {
                    if (facts.in_tag)(item, &Identifier::new("tconstruct", "modifiable/held").expect("valid tag")) { data.insert("theoneprobe_hand".into(), Nbt::Byte(1)); }
                    if (facts.in_tag)(item, &Identifier::new("tconstruct", "modifiable/armor/helmets").expect("valid tag")) { data.insert("theoneprobe".into(), Nbt::Byte(1)); }
                }
            }
            _ => {}
        }
    }

    /// EmbellishmentModule.addRawData: the variant stored under the modifier's own ID in the tool's
    /// persistent data follows material redirects on build. The persistent edit (key, new value) when
    /// the stored variant's material moved; nothing for any other module or an unmoved variant.
    pub fn embellish_migration(&self, modifier: &Identifier, persistent: &NbtCompound, materials: &crate::materials::Definitions) -> Option<(String, Nbt)> {
        if !matches!(self, Self::Display(Display::Embellished)) { return None; }
        let key = modifier.to_string();
        let Some(Nbt::String(stored)) = persistent.get(&key) else { return None; };
        let variant = crate::tool_materials::MaterialVariant::parse(stored)?;
        let resolved = variant.resolved(materials);
        (resolved.to_string() != *stored).then(|| (key, Nbt::String(resolved.to_string())))
    }

    /// ModifierRemovalHook.onRemoved for a modifier that left the tool: a swappable slot forgets its stored type.
    pub fn on_removed(&self, modifier: &Identifier, persistent: &mut NbtCompound) {
        match self {
            Self::SwappableSlot { key, .. } => { persistent.remove(&key.as_ref().unwrap_or(modifier).to_string()); }
            // TankModule.onRemoved: the tank's content leaves with the handler.
            Self::Tank => { persistent.remove(crate::tool_tank::FLUID_KEY); }
            // OverburnModule.onRemoved: forget the burn in progress.
            Self::Overburn => { persistent.remove(&modifier.to_string()); }
            // SmashingModule.onRemoved: clear the stored fluid and its validation level.
            Self::Smashing => { crate::tool_tank::smashing_clear(persistent); }
            // Only a module that matches a stored value owns it.
            Self::SwappableTraits { key, matching, .. } if !matching.is_empty() => { persistent.remove(&key.as_ref().unwrap_or(modifier).to_string()); }
            Self::CreativeSlots => { persistent.remove(CREATIVE_KEY); }
            // CapacityBarValidator.onRemoved empties the bar.
            Self::CapacityBar { .. } => { persistent.remove(&modifier.to_string()); }
            // TrickQuiverModule.onRemoved forgets the selected slot.
            Self::TrickQuiver => { persistent.remove("tconstruct:trick_quiver_selected"); }
            // InfinityModule.onRemoved forgets the stored variant.
            Self::Infinity { module } => { if let Some(key) = module.variant_key(modifier) { persistent.remove(&key); } }
            // DyeModule.onRemoved forgets the dye colour, stored under the modifier's own ID.
            Self::Display(Display::Dyed) => { persistent.remove(&modifier.to_string()); }
            // EmbellishmentModule.onRemoved forgets the stored variant, kept under the modifier's own ID.
            Self::Display(Display::Embellished) => { persistent.remove(&modifier.to_string()); }
            // TrimModule.onRemoved forgets the trim pattern and material.
            Self::Display(Display::Trim) => { persistent.remove(&format!("{modifier}_pattern")); persistent.remove(&format!("{modifier}_material")); }
            // A tool that still holds items cannot lose the modifier (checked by the caller); an empty list's key goes.
            Self::Inventory { module, .. } | Self::QuiverInventory { module, .. } => { let _ = crate::tool_inventory::on_removed(module, modifier, persistent); }
            // SmeltingModule: both of its inventories go the same way (the output under its resolved key).
            Self::Smelting { input, output, .. } => {
                let _ = crate::tool_inventory::on_removed(input, modifier, persistent);
                let output = match output.named_key() { Some(_) => (**output).clone(), None => output.with_key(Identifier::parse(&format!("{modifier}_output")).expect("valid id")) };
                let _ = crate::tool_inventory::on_removed(&output, modifier, persistent);
            }
            Self::StatOverride => { persistent.remove(OVERRIDE_BONUS); persistent.remove(OVERRIDE_MULTIPLIER); }
            _ => {}
        }
    }

    /// BreakSpeedModifierHook for a conditional mining speed module: the speed after it, with its priority
    /// (percent modules run after the additive ones, at 75).
    pub fn mining_speed(&self, entry: &Entry, facts: &Facts, tool: &crate::mining_speed::ToolView, context: &crate::mining_speed::Context, speed: f32) -> f32 {
        // DwarvenModifier.modifyBreakSpeed: on an effective dig, the speed grows with depth under
        // the level's top and fades near it; the bonus is the dig's own multiplier (haste, fatigue,
        // water, air) times the tool's mining speed multiplier times the mining bonus of 6. A dig
        // that reported no build height leaves the speed alone.
        if let Self::Dwarven = self {
            if !context.effective { return speed; }
            let Some(build) = context.facts.build_height else { return speed; };
            let bonus = context.multiplier * tool.multiplier(Stat::MiningSpeed) * 6.0;
            return crate::mining_speed::dwarven_boost(build[1], context.facts.pos_y as f32, entry.effective_level(), speed, bonus);
        }
        // MomentumModifier.modifyBreakSpeed: on an effective dig, the speed grows by the harvest
        // momentum the holder carries (a quarter more a modifier level at the effect's tenth level).
        if let Self::Momentum = self {
            if !context.effective { return speed; }
            let carried = crate::entity_facts::effect_level_of(context.holder.as_ref(), "tconstruct:momentum_harvest");
            return speed * (1.0 + entry.effective_level() * carried / 40.0);
        }
        let Self::MiningSpeed { module, condition } = self else { return speed; };
        module.apply(entry, condition.matches(entry, facts), tool, context, speed)
    }
    /// ConditionalStatModifierHook for a conditional stat module: the stat value after it.
    pub fn conditional_stat(&self, entry: &Entry, facts: &Facts, tool: &crate::mining_speed::ToolView, holder_in_tag: &dyn Fn(&Identifier) -> bool, stat: Stat, value: f32, multiplier: f32) -> f32 {
        match self {
            // ConductingModifier.modifyStat: projectile damage grows with the holder's fire.
            Self::Conducting if stat == Stat::ProjectileDamage => return value * crate::conducting::damage_multiplier(tool.entities.attacker, entry.effective_level()),
            // InsatiableModifier.modifyStat: a quarter of the ranged insatiable effect a level.
            Self::Insatiable if stat == Stat::ProjectileDamage => return value + crate::entity_facts::effect_level_of(tool.entities.attacker, "tconstruct:insatiable_ranged") * entry.effective_level() / 4.0 * multiplier,
            // MomentumModifier.modifyStat: draw speed grows with the ranged momentum effect.
            Self::Momentum if stat == Stat::DrawSpeed => return value * (1.0 + entry.effective_level() * crate::entity_facts::effect_level_of(tool.entities.attacker, "tconstruct:momentum_ranged") / 40.0),
            // DwarvenModifier.modifyStat: projectile velocity follows the same depth curve, its
            // per-level bonus the velocity bonus of 0.05 times the stat's own multiplier. The
            // holder's height and the level's build height must both be reported (the dig report
            // carries them; a stat read that has neither leaves the stat standing).
            Self::Dwarven if stat == Stat::Velocity => {
                let (Some(y), Some(build)) = (tool.entities.attacker.and_then(|holder| holder.y), tool.facts.build_height) else { return value; };
                return crate::mining_speed::dwarven_boost(build[1], y as f32, entry.effective_level(), value, multiplier * 0.05);
            }
            _ => {}
        }
        let Self::ConditionalStat { module, condition } = self else { return value; };
        module.apply(entry, condition.matches(entry, facts), tool, holder_in_tag, stat, value, multiplier)
    }
    /// Whether a conditional stat module is a percent module (None for other modules).
    pub fn conditional_stat_percent(&self) -> Option<bool> {
        match self {
            Self::ConditionalStat { module, .. } => Some(module.percent()),
            // The code modifiers below are not percent modules; they run at their modifier's priority.
            Self::Conducting | Self::Insatiable | Self::Momentum | Self::Dwarven => Some(false),
            _ => None,
        }
    }
    /// The conditional power module, if this is one.
    pub fn conditional_power(&self) -> Option<&crate::conditional_stat::ConditionalPower> {
        match self { Self::ConditionalPower { module } => Some(module), _ => None }
    }
    /// MeleeDamageModifierHook for a conditional melee damage module: the damage after it.
    pub fn melee_damage(&self, entry: &Entry, facts: &Facts, tool: &crate::mining_speed::ToolView, base: f32, damage: f32, target_in_tag: &dyn Fn(&Identifier) -> bool) -> f32 {
        match self {
            // ConductingModifier.getMeleeDamage: a percent boost scaled by the attacker's fire.
            Self::Conducting => return damage * crate::conducting::damage_multiplier(tool.entities.attacker, entry.effective_level()),
            // InsatiableModifier.getMeleeDamage: half the melee insatiable effect a level, scaled
            // by the tool's attack damage multiplier.
            Self::Insatiable => return damage + crate::entity_facts::effect_level_of(tool.entities.attacker, "tconstruct:insatiable_melee") * entry.effective_level() / 2.0 * tool.multiplier(Stat::AttackDamage),
            _ => {}
        }
        let Self::MeleeDamage { module, condition } = self else { return damage; };
        module.apply(entry, condition.matches(entry, facts), tool, base, damage, target_in_tag)
    }
    /// MeleeHitModifierHook.beforeMeleeHit for a knockback module: the strength after it (a melee hit, never a projectile's).
    pub fn knockback(&self, entry: &Entry, facts: &Facts, tool: &crate::mining_speed::ToolView, strength: f32, target_in_tag: &dyn Fn(&Identifier) -> bool) -> f32 {
        let Self::Knockback { module, condition } = self else { return strength; };
        module.apply(entry, condition.matches(entry, facts), tool, strength, false, target_in_tag)
    }
    /// The effect a weapon effect module puts on the target of a melee hit, if it lands (`rolls` are uniform values in [0, 1)).
    pub fn weapon_effect(&self, entry: &Entry, facts: &Facts, in_tag: &dyn Fn(&Identifier) -> bool, rolls: &mut dyn FnMut() -> f32) -> Option<(bool, crate::weapon_effect::Grant)> {
        match self {
            Self::WeaponEffect { module, condition } => module.grant(entry, condition.matches(entry, facts), in_tag, rolls).map(|grant| (module.before_melee(), grant)),
            Self::LegacyEffect { weapon, condition, .. } => weapon.grant(entry, condition.matches(entry, facts), in_tag, rolls).map(|grant| (weapon.before_melee(), grant)),
            _ => None,
        }
    }
    /// The effect a tool usage module puts on the tool's holder for `usage`, if it lands (`rolls` are uniform values in [0, 1)).
    pub fn usage_effect(&self, entry: &Entry, facts: &Facts, usage: crate::weapon_effect::Usage, rolls: &mut dyn FnMut() -> f32) -> Option<crate::weapon_effect::Grant> {
        let Self::UsageEffect { module, condition } = self else { return None; };
        module.grant(entry, condition.matches(entry, facts), usage, rolls)
    }
    /// The effect an armor counter module puts on the mob that hit the wearer, if it lands (`rolls` are uniform values in [0, 1)).
    pub fn counter_effect(&self, entry: &Entry, facts: &Facts, in_tag: &dyn Fn(&Identifier) -> bool, rolls: &mut dyn FnMut() -> f32) -> Option<(crate::weapon_effect::Whom, crate::weapon_effect::Grant)> {
        match self {
            Self::CounterEffect { module, condition } => module.grant(entry, condition.matches(entry, facts), in_tag, rolls).map(|grant| (module.whom(), grant)),
            Self::LegacyEffect { counter, condition, .. } => counter.grant(entry, condition.matches(entry, facts), in_tag, rolls).map(|grant| (counter.whom(), grant)),
            _ => None,
        }
    }
    /// The effect an armor attack module puts on the mob its wearer hit in melee, if it lands (`rolls` are uniform values in [0, 1)).
    pub fn armor_attack_effect(&self, entry: &Entry, facts: &Facts, source: &crate::protection::DamageSource, in_tag: &dyn Fn(&Identifier) -> bool, rolls: &mut dyn FnMut() -> f32) -> Option<crate::weapon_effect::Grant> {
        let Self::ArmorAttackEffect { module, condition } = self else { return None; };
        module.grant_armor_attack(entry, condition.matches(entry, facts), source, in_tag, rolls)
    }
    /// The damage a thorns module deals to the mob that hit the wearer, if its chance lands (`rolls` are uniform values in [0, 1)).
    pub fn thorns(&self, entry: &Entry, facts: &Facts, in_tag: &dyn Fn(&Identifier) -> bool, rolls: &mut dyn FnMut() -> f32) -> Option<crate::thorns::Hurt> {
        let Self::Thorns { module, condition } = self else { return None; };
        module.hurt(entry, condition.matches(entry, facts), in_tag, rolls)
    }
    /// The action a fiery or freezing attack module does to the mob a melee hit landed on, when it lands.
    pub fn attack_action(&self, entry: &Entry, facts: &Facts) -> Option<pluto_lib::EntityAction> {
        let Self::AttackAction { module, condition } = self else { return None; };
        module.action(entry, condition.matches(entry, facts))
    }
    /// The action a counter module does to the mob that hit the wearer, if its chance lands (`rolls` are uniform values in [0, 1)).
    pub fn counter_action(&self, entry: &Entry, facts: &Facts, in_tag: &dyn Fn(&Identifier) -> bool, rolls: &mut dyn FnMut() -> f32) -> Option<pluto_lib::EntityAction> {
        let Self::CounterAction { module, condition } = self else { return None; };
        module.value(entry, condition.matches(entry, facts), in_tag, rolls).and_then(|value| module.action(value))
    }
    /// Whether a block damage module blocks damage from `source`, when its conditions hold.
    pub fn blocks_damage(&self, entry: &Entry, facts: &Facts, source: &crate::protection::DamageSource) -> bool {
        let Self::BlockDamage { source: blocked, condition } = self else { return false; };
        condition.matches(entry, facts) && blocked.matches(source)
    }
    /// Whether this is a block damage module.
    pub fn is_block_damage(&self) -> bool { matches!(self, Self::BlockDamage { .. }) }
    /// The effect an immunity module makes the wearer immune to, with the count it adds, when its conditions hold.
    pub fn effect_immunity(&self, entry: &Entry, facts: &Facts) -> Option<(Identifier, i32)> {
        let Self::EffectImmunity { module, condition } = self else { return None; };
        module.immunity(entry, condition.matches(entry, facts))
    }
    /// The effect an armor piece's clear-on-unequip module takes off its wearer, when its conditions hold.
    pub fn clear_effect_on_unequip(&self, entry: &Entry, facts: &Facts) -> Option<Identifier> {
        let Self::ClearEffectOnUnequip { effect, condition } = self else { return None; };
        condition.matches(entry, facts).then(|| effect.clone())
    }
    /// What an unequipped armor piece's damage-on-unequip module hurts the wearer by: the damage type and the amount.
    pub fn damage_on_unequip(&self, entry: &Entry, facts: &Facts) -> Option<(String, f32)> {
        let Self::DamageOnUnequip { module, condition } = self else { return None; };
        if condition.matches(entry, facts) { module.hurt(entry) } else { None }
    }
    /// What an unequipped armor piece's reduce-effect module does: the category it shortens and the share it takes off.
    pub fn reduce_effect_on_unequip(&self, entry: &Entry, facts: &Facts) -> Option<(pluto_lib::EffectCategory, f32)> {
        let Self::ReduceEffectOnUnequip { module, condition } = self else { return None; };
        if !condition.matches(entry, facts) { return None; }
        let percent = module.percent(entry);
        (percent != 0.0).then(|| (module.category(), percent))
    }
    /// The restore-lost-health module's result for `amount` of damage taken at the stage `hurt` names (the damage as it
    /// arrives, before armor, or what armor left), when the module is bound to that stage and its conditions hold.
    pub fn restore_lost_health(&self, entry: &Entry, facts: &Facts, source: &crate::protection::DamageSource, amount: f32, hurt: bool, roll: &mut dyn FnMut() -> f32) -> Option<crate::wearer_modules::Restore> {
        let Self::RestoreLostHealth { module, condition, hurt: on_hurt, damage: on_damage } = self else { return None; };
        if (hurt && !on_hurt) || (!hurt && !on_damage) { return None; }
        if !condition.matches(entry, facts) { return None; }
        module.restore(entry, source, amount, roll)
    }
    /// The capacity bar this module declares.
    pub fn capacity_bar(&self) -> Option<&crate::capacity::Bar> { match self { Self::CapacityBar { bar } => Some(bar), _ => None } }
    /// The durability shield this module declares.
    pub fn shield(&self) -> Option<&crate::capacity::Shield> { match self { Self::DurabilityShield { shield } => Some(shield), _ => None } }
    /// The inventory this module adds to a tool, with its conditions evaluated against the tool.
    pub fn inventory_part(&self, entry: &Entry, facts: &Facts) -> Option<crate::tool_inventory::Part> {
        match self {
            Self::Inventory { module, condition } => Some(crate::tool_inventory::Part::new((**module).clone(), entry.clone(), condition.matches(entry, facts), condition.tool_matches(facts))),
            Self::QuiverInventory { module, condition } => {
                let mut part = crate::tool_inventory::Part::new((**module).clone(), entry.clone(), condition.matches(entry, facts), condition.tool_matches(facts));
                // Whether the tool is a crossbow is a fact of the tool; whether it is a ballista also reads its
                // volatile data, which the caller notes (tool_stack::inventory).
                let crossbow = facts.item.is_some_and(|item| (facts.in_tag)(item, &Identifier::parse("tconstruct:modifiable/ranged/crossbows").expect("valid tag")));
                part.note_tool(crossbow, false);
                Some(part)
            }
            _ => None,
        }
    }
    /// Every inventory this module adds to a tool: a smelting module adds its input and its output
    /// (addModules registers both), in that order; any other module adds its one part or none.
    pub fn inventory_parts(&self, entry: &Entry, facts: &Facts) -> Vec<crate::tool_inventory::Part> {
        match self {
            Self::Smelting { .. } => match self.smelting_parts(entry, facts) { Some((input, output, _, _)) => vec![input, output], None => Vec::new() },
            _ => self.inventory_part(entry, facts).into_iter().collect(),
        }
    }
    /// AutosmeltModule's terms: the recipe types to try, in order, and the chance of an extra item
    /// per level past the first; None for another module (the module carries no condition).
    pub fn autosmelt(&self) -> Option<(&[String], f32)> {
        let Self::Autosmelt { recipe_types, extra_drop_chance } = self else { return None; };
        Some((recipe_types, *extra_drop_chance))
    }
    /// SmeltingModule's inventories as parts of this tool: the input part, the output part (its key
    /// resolved: the JSON's `output_key`, else the modifier's ID with an `_output` suffix), the
    /// recipe type and the multiplier; None for another module.
    pub fn smelting_parts(&self, entry: &Entry, facts: &Facts) -> Option<(crate::tool_inventory::Part, crate::tool_inventory::Part, &str, f32)> {
        let Self::Smelting { recipe_type, multiplier, input, output, condition } = self else { return None; };
        let active = condition.matches(entry, facts);
        let tool_matches = condition.tool_matches(facts);
        let input_part = crate::tool_inventory::Part::new((**input).clone(), entry.clone(), active, tool_matches);
        let output_inventory = match output.named_key() {
            Some(_) => (**output).clone(),
            None => output.with_key(Identifier::parse(&format!("{}_output", entry.id)).ok()?),
        };
        let output_part = crate::tool_inventory::Part::new(output_inventory, entry.clone(), active, tool_matches);
        Some((input_part, output_part, recipe_type, *multiplier))
    }
    /// CraftCountModule.modifyCraftCount: the running count multiplied by the module's value, when its
    /// condition holds (the caller stops the chain at zero, as the composed hook does).
    pub fn craft_count(&self, entry: &Entry, facts: &Facts, count: f32) -> f32 {
        let Self::CraftCount { flat, each, condition } = self else { return count; };
        if condition.matches(entry, facts) { count * (*flat + *each * entry.effective_level()) } else { count }
    }
    /// MaterialRepairModule.getRepairAmount for one material stack: the module's amount times the modifier's
    /// level, for its own material when the condition holds. `durability_of` answers a stat type's durability
    /// for a material (the definition-level lookup).
    pub fn material_repair_amount(&self, entry: &Entry, facts: &Facts, material: &Identifier, durability_of: &dyn Fn(&Identifier, &Identifier) -> i32) -> i32 {
        let Self::MaterialRepair { material: own, amount, condition } = self else { return 0; };
        if own != material || !condition.matches(entry, facts) { return 0; }
        let base = match amount { RepairAmount::Constant(amount) => *amount, RepairAmount::StatType(stat) => durability_of(material, stat) };
        base * entry.level
    }
    /// MaterialRepairModule.isRepairMaterial: the material is the module's and the condition holds.
    pub fn is_repair_material(&self, entry: &Entry, facts: &Facts, material: &Identifier) -> bool {
        let Self::MaterialRepair { material: own, condition, .. } = self else { return false; };
        own == material && condition.matches(entry, facts)
    }
    /// MaterialRepairModule.addRepairMaterials: the module's material, with no condition check.
    pub fn repair_material(&self) -> Option<&Identifier> {
        let Self::MaterialRepair { material, .. } = self else { return None; };
        Some(material)
    }
    /// VolatileFloatModule on a launched projectile: the value it adds to the projectile's persistent data
    /// under its flag, when its condition holds (the launch event reports no stack to write it to, so this
    /// half has no live caller yet, like the projectile power helpers).
    pub fn projectile_float(&self, entry: &Entry, facts: &Facts) -> Option<(Identifier, f32)> {
        let Self::Float { id, flat, each, condition } = self else { return None; };
        if condition.matches(entry, facts) { Some((id.clone(), *flat + *each * entry.effective_level())) } else { None }
    }
    /// UpdateHealthModule for a change in `slot`: the health delta going on (equipped) or coming off.
    pub fn update_health(&self, entry: &Entry, facts: &Facts, slot: &str, equipped: bool) -> f32 {
        let Self::UpdateHealth { module, condition } = self else { return 0.0; };
        if condition.matches(entry, facts) { module.delta(entry, slot, equipped) } else { 0.0 }
    }
    /// The key that opens the inventory screen, for an inventory menu module.
    pub fn inventory_menu_key(&self) -> Option<crate::tool_inventory::MenuKey> {
        if let Self::InventoryMenu(key) = self { Some(*key) } else { None }
    }
    /// The quiver draw this module performs over its own modifier's inventory, if it is a quiver module.
    pub fn quiver_draw(&self) -> Option<QuiverDraw> {
        match self {
            Self::BulkQuiver { check_standard_arrows } => Some(QuiverDraw::Bulk { check_standard_arrows: *check_standard_arrows }),
            Self::TrickQuiver => Some(QuiverDraw::Trick),
            _ => None,
        }
    }
    /// The claim a worn piece makes on an attribute through a max armor attribute module, when its conditions hold.
    pub fn max_armor_candidate(&self, entry: &Entry, facts: &Facts, broken: bool) -> Option<crate::max_armor::Candidate> {
        let Self::MaxArmorAttribute { module, condition } = self else { return None; };
        module.candidate(entry, condition.matches(entry, facts), broken)
    }
    /// The key and levels an armor level module adds for a piece worn in `slot`.
    pub fn armor_level(&self, entry: &Entry, slot: &str, broken: bool) -> Option<(Identifier, i32)> {
        match self {
            Self::ArmorLevel { module } => module.level(entry, broken),
            // ShowOffhandModule adds a flat 1 (never scaled by level) when a chest piece is equipped and takes it back
            // when it comes off; the per-tick totals over the worn pieces are the port's form of that, so a worn chest
            // piece counts 1 while the mode's broken rule holds.
            Self::ShowOffhand { allow_broken } => (slot == "chest" && (!broken || *allow_broken))
                .then(|| (Identifier::new("tconstruct", "show_empty_offhand").expect("valid key"), 1)),
            _ => None,
        }
    }
    /// The (attribute, amount) a golden attribute module grants a wearer with `total_gold` golden pieces.
    pub fn golden_amount(&self, entry: &Entry, facts: &Facts, total_gold: i32) -> Option<(String, f64)> {
        let Self::GoldenAttribute { module, condition } = self else { return None; };
        module.amount(condition.matches(entry, facts), total_gold)
    }
    /// Whether this is a golden attribute module.
    pub fn is_golden_attribute(&self) -> bool { matches!(self, Self::GoldenAttribute { .. }) }
    /// The lightspeed module and the entry's effective level, when its condition holds.
    pub fn lightspeed(&self, entry: &Entry, facts: &Facts) -> Option<(&crate::wearer_modules::LightspeedAttribute, f32)> {
        let Self::LightspeedAttribute { module, condition } = self else { return None; };
        condition.matches(entry, facts).then(|| (module, entry.effective_level()))
    }
    /// Whether this is a lightspeed attribute module.
    pub fn is_lightspeed_attribute(&self) -> bool { matches!(self, Self::LightspeedAttribute { .. }) }
    /// Whether this is a max armor attribute module.
    pub fn is_max_armor_attribute(&self) -> bool { matches!(self, Self::MaxArmorAttribute { .. }) }
    /// Whether this is an armor level module.
    pub fn is_armor_level(&self) -> bool { matches!(self, Self::ArmorLevel { .. } | Self::ShowOffhand { .. }) }
    /// Whether this is an effect immunity module.
    pub fn is_effect_immunity(&self) -> bool { matches!(self, Self::EffectImmunity { .. }) }
    /// The mob type a disguise module makes the wearer pass for, with the count it adds.
    pub fn mob_disguise(&self, entry: &Entry) -> Option<(Identifier, i32)> {
        let Self::MobDisguise { module } = self else { return None; };
        Some(module.disguise(entry))
    }
    /// Whether this is the edible module itself.
    pub fn is_edible(&self) -> bool { matches!(self, Self::Edible) }
    /// EdibleRepresentativeItemModule: the item this module presents as eaten, when its condition holds.
    pub fn representative_item(&self, entry: &Entry, facts: &Facts) -> Option<&Identifier> {
        let Self::EdibleRepresentative { item, condition } = self else { return None; };
        condition.matches(entry, facts).then_some(item)
    }
    /// What eating the tool does for this edible effect module, when its conditions hold.
    pub fn edible_effect(&self, entry: &Entry, facts: &Facts) -> Option<crate::edible::Effect> {
        match self {
            Self::EdibleWear { usage, condition } if condition.matches(entry, facts) => crate::edible::wear(*usage, entry),
            Self::EdibleCure { condition } if condition.matches(entry, facts) => Some(crate::edible::Effect::CureRandomEffect),
            Self::EdibleRemove { effect, condition } if condition.matches(entry, facts) => Some(crate::edible::Effect::RemoveEffect(effect.clone())),
            _ => None,
        }
    }
    /// Whether this is a mob disguise module.
    pub fn is_mob_disguise(&self) -> bool { matches!(self, Self::MobDisguise { .. }) }
    /// Whether this is a fiery or freezing attack module.
    pub fn is_attack_action(&self) -> bool { matches!(self, Self::AttackAction { .. }) }
    /// Whether this is a fiery, freezing or knockback counter module.
    pub fn is_counter_action(&self) -> bool { matches!(self, Self::CounterAction { .. }) }
    /// The bar a melee capacity module fills and by how much, when its conditions hold for the hit.
    pub fn melee_capacity(&self, entry: &Entry, facts: &Facts, target_in_tag: &dyn Fn(&Identifier) -> bool, attacker_in_tag: &dyn Fn(&Identifier) -> bool) -> Option<(Option<&Identifier>, i32)> {
        let Self::CapacityMelee { module, condition } = self else { return None; };
        if !condition.matches(entry, facts) { return None; }
        module.grant_for(entry, target_in_tag, attacker_in_tag)
    }
    /// The damage to capacity module, when its conditions hold and the damage is of the source it names.
    pub fn damage_capacity(&self, entry: &Entry, facts: &Facts, source: &crate::protection::DamageSource) -> Option<&crate::capacity::DamageFill> {
        let Self::CapacityDamage { module, condition } = self else { return None; };
        (condition.matches(entry, facts) && module.matches(source)).then_some(&**module)
    }
    /// The bar a mining capacity module fills and by how much for one block broken, when its conditions hold and it runs before the break
    /// (`before`) or after it.
    pub fn mining_capacity(&self, entry: &Entry, facts: &Facts, before: bool) -> Option<(Option<&Identifier>, i32)> {
        let Self::CapacityMining { module, condition } = self else { return None; };
        if !condition.matches(entry, facts) { return None; }
        module.grant_for(entry, before)
    }
    /// The loot to capacity module, when its conditions hold.
    pub fn loot_capacity(&self, entry: &Entry, facts: &Facts) -> Option<&crate::capacity::LootFill> {
        let Self::CapacityLoot { module, condition } = self else { return None; };
        condition.matches(entry, facts).then_some(&**module)
    }
    /// The bar a launch capacity module fills and by how much for one projectile launched, when its conditions hold.
    pub fn launch_capacity(&self, entry: &Entry, facts: &Facts) -> Option<(Option<&Identifier>, i32)> {
        let Self::CapacityLaunch { module, condition } = self else { return None; };
        condition.matches(entry, facts).then(|| module.grant_for(entry))
    }
    /// The health a lifesteal module gives the wielder of a critical melee hit that dealt `damage`, when its conditions hold.
    pub fn lifesteal(&self, entry: &Entry, facts: &Facts, damage: f32, blacklisted: bool, attacker_in_tag: &dyn Fn(&Identifier) -> bool, target_in_tag: &dyn Fn(&Identifier) -> bool) -> Option<f32> {
        let Self::Lifesteal { module, condition } = self else { return None; };
        module.heal(entry, condition.matches(entry, facts), damage, blacklisted, attacker_in_tag, target_in_tag)
    }
    /// Whether this is a launch capacity module.
    pub fn is_launch_capacity(&self) -> bool { matches!(self, Self::CapacityLaunch { .. }) }
    /// Whether this is a lifesteal module.
    pub fn is_lifesteal(&self) -> bool { matches!(self, Self::Lifesteal { .. }) }
    /// Whether this is a mining capacity module.
    pub fn is_mining_capacity(&self) -> bool { matches!(self, Self::CapacityMining { .. }) }
    /// Whether this is a loot to capacity module.
    pub fn is_loot_capacity(&self) -> bool { matches!(self, Self::CapacityLoot { .. }) }
    /// The severing module's effective level of its modifier, None for any other module.
    pub fn severing_level(&self, entry: &Entry) -> Option<f32> { matches!(self, Self::Severing).then(|| entry.effective_level()) }
    /// The knockback a punch module sets or adds on a shot of the tool, when its conditions hold.
    pub fn punch(&self, entry: &Entry, facts: &Facts, arrow: bool) -> Option<crate::punch::Shot> {
        let Self::Punch { module, condition } = self else { return None; };
        module.shoot(entry, condition.matches(entry, facts), arrow)
    }
    /// The reverse punch module, None for any other module.
    pub fn reverse_punch(&self) -> Option<&crate::punch::ReversePunch> {
        let Self::ReversePunch { module } = self else { return None; };
        Some(module)
    }
    /// The sound a melee instrument module plays for a hit, when its conditions and predicates hold.
    pub fn melee_instrument(&self, entry: &Entry, facts: &Facts, materials: &[crate::tool_materials::MaterialVariant], attacker_in_tag: &dyn Fn(&Identifier) -> bool, target_in_tag: &dyn Fn(&Identifier) -> bool, roll: f32) -> Option<crate::melee_instrument::Sound> {
        let Self::MeleeInstrument { module, condition } = self else { return None; };
        module.sound(condition.matches(entry, facts), materials, attacker_in_tag, target_in_tag, roll)
    }
    /// Whether this is an overgrowth module.
    pub fn is_overgrowth(&self) -> bool { matches!(self, Self::Overgrowth { .. }) }
    /// Whether this is the tank handler's tank module.
    pub fn is_tank(&self) -> bool { matches!(self, Self::Tank) }
    /// Whether this is an overburn module (its tick is tool_stack::overburn_tick).
    pub fn is_overburn(&self) -> bool { matches!(self, Self::Overburn) }
    /// Whether this is a smashing module.
    pub fn is_smashing(&self) -> bool { matches!(self, Self::Smashing) }
    /// The melting module, when its condition holds for the entry.
    pub fn melting(&self, entry: &Entry, facts: &Facts) -> Option<&crate::tool_tank::Melting> {
        match self { Self::Melting { module, condition } if condition.matches(entry, facts) => Some(module), _ => None }
    }
    /// The slurping module.
    pub fn slurping(&self) -> Option<&crate::tool_tank::Slurping> { match self { Self::Slurping { module } => Some(module), _ => None } }
    /// The spilling effect level for the entry, when its condition holds for it.
    pub fn spilling_level(&self, entry: &Entry, facts: &Facts) -> Option<f32> {
        match self { Self::Spilling { module, condition } if condition.matches(entry, facts) => Some(module.level(entry)), _ => None }
    }
    /// The spitting module.
    pub fn spitting(&self) -> Option<&crate::tool_tank::Spitting> { match self { Self::Spitting { module } => Some(module), _ => None } }
    /// The splashing module.
    pub fn splashing(&self) -> Option<&crate::tool_tank::Splashing> { match self { Self::Splashing { module } => Some(module), _ => None } }
    /// The bucket module.
    pub fn bucket(&self) -> Option<&crate::tool_tank::Bucket> { match self { Self::Bucket { module } => Some(module), _ => None } }
    /// The tank interaction module.
    pub fn tank_interaction(&self) -> Option<&crate::tool_tank::TankInteraction> { match self { Self::TankInteraction { module } => Some(module), _ => None } }
    /// Whether an overgrowth module grows the tool's overslime on this tick, when its conditions hold.
    pub fn overgrowth(&self, entry: &Entry, facts: &Facts, nbt: &mut NbtCompound, env: &crate::tool_stack::Environment, roll: f32) -> bool {
        let Self::Overgrowth { module, condition } = self else { return false; };
        module.grow(entry, condition.matches(entry, facts), nbt, env, roll)
    }
    /// The level a kinetic module contributes to its modifier's total, None for any other module. The original counts the
    /// plain level here (SlotInChargeModule reads the entry level), not the effective one.
    pub fn kinetic_level(&self, entry: &Entry) -> Option<i32> { matches!(self, Self::Kinetic).then(|| entry.level) }
    /// Whether this is the insatiable code module.
    pub fn is_insatiable(&self) -> bool { matches!(self, Self::Insatiable) }
    /// The enderclearance module's (chance, diameter) at an entry's effective level, None for any
    /// other module.
    pub fn enderclearance(&self, entry: &Entry) -> Option<(f32, f32)> {
        let Self::Enderclearance { chance, diameter } = self else { return None; };
        let level = entry.effective_level();
        Some((chance.0 + chance.1 * level, diameter.0 + diameter.1 * level))
    }
    /// The channeling module's (clear, rain, thunder) chances when it allows melee, None for any
    /// other module and for a channeling module whose data forbids melee.
    pub fn channeling_melee(&self) -> Option<(f32, f32, f32)> {
        let Self::Channeling { allow_melee, chance_clear, chance_rain, chance_thunder } = self else { return None; };
        allow_melee.then_some((*chance_clear, *chance_rain, *chance_thunder))
    }
    /// The plain level a lacerating module contributes, None for any other module (the original's
    /// applyEffect reads the entry's level, as the kinetic total does).
    pub fn lacerating_level(&self, entry: &Entry) -> Option<i32> { matches!(self, Self::Lacerating).then(|| entry.level) }
    /// A recurrent protection module and the level its entry contributes to the total over the worn pieces,
    /// None for any other module. The original counts the plain level here too (SlotInChargeModule).
    pub fn recurrent_protection(&self, entry: &Entry) -> Option<(i32, crate::wearer_modules::RecurrentProtection)> {
        match self { Self::RecurrentProtection { module } => Some((entry.level, module.clone())), _ => None }
    }
    /// What a damage on shoot module hurts the tool's holder by for firing it: the damage type and the amount.
    pub fn damage_on_shoot(&self, entry: &Entry, facts: &Facts) -> Option<(String, f32)> {
        let Self::DamageOnShoot { module, condition } = self else { return None; };
        if condition.matches(entry, facts) { module.hurt(entry) } else { None }
    }
    /// The infinity module, if this is one.
    pub fn infinity(&self) -> Option<&crate::projectile_modules::Infinity> {
        match self { Self::Infinity { module } => Some(module), _ => None }
    }
    /// The arrow pierce module, if this is one.
    pub fn arrow_pierce(&self) -> Option<&crate::projectile_modules::ArrowPierce> {
        match self { Self::ArrowPierce { module } => Some(module), _ => None }
    }
    /// The projectile bounce module, if this is one.
    pub fn projectile_bounce(&self) -> Option<&crate::projectile_modules::ProjectileBounce> {
        match self { Self::ProjectileBounce { module } => Some(module), _ => None }
    }
    /// The projectile gravity module, if this is one.
    pub fn projectile_gravity(&self) -> Option<&crate::projectile_modules::ProjectileGravity> {
        match self { Self::ProjectileGravity { module } => Some(module), _ => None }
    }
    /// The projectile fuse module, if this is one.
    pub fn projectile_fuse(&self) -> Option<&crate::projectile_modules::ProjectileFuse> {
        match self { Self::ProjectileFuse { module } => Some(module), _ => None }
    }
    /// The projectile explosion module, if this is one.
    pub fn projectile_explosion(&self) -> Option<&crate::projectile_modules::ProjectileExplosion> {
        match self { Self::ProjectileExplosion { module } => Some(module), _ => None }
    }
    /// The projectile attract mobs module, if this is one.
    pub fn projectile_attract(&self) -> Option<&crate::projectile_modules::ProjectileAttractMobs> {
        match self { Self::ProjectileAttractMobs { module } => Some(module), _ => None }
    }
    /// The projectile place glow module, if this is one.
    pub fn projectile_glow(&self) -> Option<&crate::projectile_modules::ProjectilePlaceGlow> {
        match self { Self::ProjectilePlaceGlow { module } => Some(module), _ => None }
    }
    /// The restrict angle module, if this is one.
    pub fn restrict_angle(&self) -> Option<&crate::projectile_modules::RestrictAngle> {
        match self { Self::RestrictAngle { module } => Some(module), _ => None }
    }
    /// Whether this is the throwing module.
    pub fn is_throwing(&self) -> bool { matches!(self, Self::Throwing) }
    /// Whether this is the enderporting module (EnderportingModifier's swap behaviours).
    pub fn is_enderporting(&self) -> bool { matches!(self, Self::Enderporting) }
    /// Whether this is the harvest module (HarvestModule's right-click harvest).
    pub fn is_harvest(&self) -> bool { matches!(self, Self::Harvest) }
    /// Whether this is a melee capacity module.
    pub fn is_melee_capacity(&self) -> bool { matches!(self, Self::CapacityMelee { .. }) }
    /// Whether this is a damage to capacity module.
    pub fn is_damage_capacity(&self) -> bool { matches!(self, Self::CapacityDamage { .. }) }
    /// Whether this is a thorns module.
    pub fn is_thorns(&self) -> bool { matches!(self, Self::Thorns { .. }) }
    /// Whether this is an armor counter effect module.
    pub fn is_counter_effect(&self) -> bool { matches!(self, Self::CounterEffect { .. } | Self::LegacyEffect { .. }) }
    /// Whether this is an armor attack effect module.
    pub fn is_armor_attack_effect(&self) -> bool { matches!(self, Self::ArmorAttackEffect { .. }) }
    /// Whether this is a tool usage effect module.
    pub fn is_usage_effect(&self) -> bool { matches!(self, Self::UsageEffect { .. }) }
    /// Whether this is a weapon effect module.
    pub fn is_weapon_effect(&self) -> bool { matches!(self, Self::WeaponEffect { .. } | Self::LegacyEffect { .. }) }
    /// Whether this is a knockback module.
    pub fn is_knockback(&self) -> bool { matches!(self, Self::Knockback { .. }) }
    /// What this module adds to the protection cap attribute (0.05 is one point and a quarter of protection).
    pub fn protection_cap(&self, entry: &Entry, facts: &Facts, nbt: &NbtCompound) -> f32 {
        let Self::ProtectionCap { formula, condition } = self else { return 0.0; };
        if !condition.matches(entry, facts) { return 0.0; }
        formula.eval(entry, &crate::mining_speed::ToolView::of(nbt), 0.0, 1.0, 0.0)
    }
    /// ProtectionModifierHook: the running protection after this module, over a damage source.
    pub fn protection(&self, entry: &Entry, facts: &Facts, nbt: &NbtCompound, source: &crate::protection::DamageSource, value: f32) -> f32 {
        // EnchantmentModule.Protection: the game's own enchantment protection counted the enchantment this module gave, so its points come off again.
        if let Self::Enchantment { name, flat, each, ignoring: true, condition } = self {
            let level = (flat + each * entry.effective_level()) as i32;
            if level > 0 && condition.matches(entry, facts) && !source.tags.contains_vanilla("bypasses_enchantments", source.damage_type) {
                return value - crate::protection::enchantment_points(name, level, source);
            }
            return value;
        }
        if let Self::FlameBarrier { flat, each, condition } = self {
            if !condition.matches(entry, facts) { return value; }
            // FlameBarrierModule.getProtectionModifier: nothing against fire itself, and only for
            // damage protection can stop (DamageSourcePredicate.CAN_PROTECT); the bonus is the
            // amount at the entry's level, scaled by how long the wearer has been burning.
            if source.tags.contains_vanilla("is_fire", source.damage_type) || source.tags.contains_vanilla("bypasses_invulnerability", source.damage_type) { return value; }
            let amount = flat + each * entry.effective_level();
            if amount == 0.0 { return value; }
            return value + crate::conducting::bonus_scale(source.entities.target) * amount;
        }
        if let Self::DepthProtection { module, condition } = self {
            return if condition.matches(entry, facts) { module.protect(entry, source, value) } else { value };
        }
        let Self::Protection { module, condition } = self else { return value; };
        if !condition.matches(entry, facts) || !module.source().matches(source) || !module.wearer_matches(source) || !module.attacker_matches(source) { return value; }
        module.formula().eval(entry, &crate::mining_speed::ToolView::with(nbt, source.entities), value, 1.0, 0.0)
    }
    /// OvershieldModule as a ProtectionModifierHook: the running protection after this module, with the overslime it spends taken off `work`.
    pub fn overshield(&self, entry: &Entry, work: &mut NbtCompound, source: &crate::protection::DamageSource, value: f32) -> f32 {
        let Self::Overshield { module } = self else { return value; };
        module.protect(entry, work, source, value)
    }
    /// ModifyDamageModifierHook: the damage the wearer takes after this module; `entities` are the hit's attacker and the wearer (as the target).
    pub fn adjust_damage(&self, entry: &Entry, facts: &Facts, nbt: &NbtCompound, source: &crate::protection::DamageSource, entities: &crate::entity_facts::Entities, amount: f32) -> f32 {
        let Self::AdjustDamage { module, condition } = self else { return amount; };
        if !condition.matches(entry, facts) { return amount; }
        let holder_in_tag = |tag: &Identifier| crate::melee_damage::state_in_tag(tag, entities.attacker, entities.target);
        module.apply(entry, &crate::mining_speed::ToolView::with(nbt, *entities), source, &holder_in_tag, amount)
    }
    /// The enchantment this module gives the tool and its level, when its conditions hold and the level is positive.
    pub fn enchantment(&self, entry: &Entry, facts: &Facts) -> Option<(String, i32)> {
        let Self::Enchantment { name, flat, each, condition, .. } = self else { return None; };
        let level = (flat + each * entry.effective_level()) as i32;
        (level > 0 && condition.matches(entry, facts)).then(|| (name.clone(), level))
    }
    /// ArmorLootingModifierHook for one worn piece: the looting this module adds while the piece sits in `slot`
    /// (LootingModule.Armor.updateArmorLooting: the slot must be listed and the condition must match).
    pub fn armor_looting(&self, entry: &Entry, facts: &Facts, slot: &str) -> i32 {
        let Self::ArmorLooting { flat, each, slots, condition } = self else { return 0; };
        if !slots.iter().any(|listed| listed.as_str() == slot) || !condition.matches(entry, facts) { return 0; }
        let level = (flat + each * entry.effective_level()) as i32;
        if level > 0 { level } else { 0 }
    }
    /// HarvestEnchantmentsModifierHook for one worn piece: the enchantment this module lends the
    /// breaking stack while the piece sits in `slot`, and its level (EnchantmentModule.ArmorHarvest:
    /// the slot must be listed and the condition must match; the level may be 0 or negative, which
    /// the merge reads as taking levels away).
    pub fn armor_harvest_enchantment(&self, entry: &Entry, facts: &Facts, slot: &str) -> Option<(String, i32)> {
        let Self::ArmorHarvestEnchantment { name, flat, each, slots, condition } = self else { return None; };
        if !slots.iter().any(|listed| listed.as_str() == slot) || !condition.matches(entry, facts) { return None; }
        Some((name.clone(), (flat + each * entry.effective_level()) as i32))
    }
    /// The tool action this module performs on a right click, with its sound and whether the block above must be clear.
    pub fn tool_action(&self, entry: &Entry, facts: &Facts) -> Option<(crate::tool_actions::ToolAction, String, bool)> {
        let Self::ToolAction { action, sound, require_ground, condition } = self else { return None; };
        condition.matches(entry, facts).then(|| (*action, sound.clone(), *require_ground))
    }
    /// The walk transform this module performs, with its sound and its radius at the entry's effective level
    /// (ArmorWalkRadiusModule.getRadius; the EXPANDED volatile the original adds is written by no module this port
    /// binds, so it contributes 0 here as everywhere else the port reads volatile data).
    pub fn walk_transform(&self, entry: &Entry, facts: &Facts) -> Option<(crate::tool_actions::ToolAction, String, f32)> {
        let Self::WalkTransform { action, sound, radius, condition } = self else { return None; };
        condition.matches(entry, facts).then(|| (*action, sound.clone(), radius.compute(entry.effective_level())))
    }
    /// The boot walk this module performs, at the entry's effective level (ArmorWalkRadiusModule,
    /// the radius as in walk_transform above).
    pub fn walk_module(&self, entry: &Entry, facts: &Facts) -> Option<WalkModule> {
        match self {
            Self::FireWalker { radius } => Some(WalkModule::Fire { radius: radius.compute(entry.effective_level()) }),
            Self::GlowWalker { radius, min_light, damage } => Some(WalkModule::Glow { radius: radius.compute(entry.effective_level()), min_light: *min_light, damage: *damage }),
            Self::CoverGround { block, radius, condition } if condition.matches(entry, facts) => Some(WalkModule::Cover { block: block.clone(), radius: radius.compute(entry.effective_level()) }),
            _ => None,
        }
    }
    /// ToolActionModifierHook.canPerformAction: a tool actions module answers for its named actions, a shears
    /// module for the four shear actions and a fishing module for the cast, each while its condition holds
    /// (ShearsModule.canPerformAction, FishingModule.canPerformAction); a blocking module performs the shield
    /// block action, and a walk transform module its own action, while its condition holds.
    pub fn can_perform_action(&self, entry: &Entry, facts: &Facts, name: &str) -> bool {
        match self {
            Self::ToolActions { actions, condition } => condition.matches(entry, facts) && actions.iter().any(|action| action == name),
            Self::Shears { condition, .. } => condition.matches(entry, facts) && matches!(name, "shears_dig" | "shears_harvest" | "shears_carve" | "shears_disarm"),
            Self::Fishing => name == "fishing_rod_cast",
            // BlockingModule.canPerformAction: a blocking tool performs the shield block action.
            Self::Blocking => name == "shield_block",
            // ToolActionWalkerTransformModule.canPerformAction: its own action, while its condition holds.
            Self::WalkTransform { action, condition, .. } => condition.matches(entry, facts) && action.name() == name,
            _ => false,
        }
    }
    /// Whether this is a blocking module (BlockingModule: the tool is raised like a shield on a right click).
    pub fn blocking(&self) -> bool { matches!(self, Self::Blocking) }
    /// Whether a campfire extinguish module puts out a lit campfire the tool is used on, when its conditions hold.
    pub fn extinguishes_campfires(&self, entry: &Entry, facts: &Facts) -> bool {
        let Self::CampfireExtinguish { condition } = self else { return false; };
        condition.matches(entry, facts)
    }
    /// AttributesModifierHook for an attribute module: one attribute modifier entry per listed slot among `wanted`.
    pub fn attribute_entries(&self, entry: &Entry, facts: &Facts, nbt: &NbtCompound, wanted: &[&str]) -> Vec<NbtCompound> {
        let Self::Attribute { attribute, operation, formula, slots, condition } = self else { return Vec::new(); };
        if !condition.matches(entry, facts) { return Vec::new(); }
        let amount = f64::from(formula.eval(entry, &crate::mining_speed::ToolView::of(nbt), 0.0, 1.0, 0.0));
        let short = attribute.rsplit(':').next().unwrap_or(attribute);
        slots.iter().filter(|slot| wanted.contains(&slot.as_str())).map(|slot| {
            let mut modifier = NbtCompound::new();
            modifier.insert("type".into(), Nbt::String(attribute.clone()));
            modifier.insert("id".into(), Nbt::String(format!("{}/{short}/{slot}", entry.id)));
            modifier.insert("amount".into(), Nbt::Double(amount));
            modifier.insert("operation".into(), Nbt::String((*operation).to_owned()));
            modifier.insert("slot".into(), Nbt::String(slot.clone()));
            modifier
        }).collect()
    }
    /// Whether this is a percent melee damage module (None for other modules).
    pub fn melee_percent(&self) -> Option<bool> {
        match self {
            Self::MeleeDamage { module, .. } => Some(module.percent()),
            // The code modifiers below are not percent modules; they run at their modifier's priority.
            Self::Conducting | Self::Insatiable => Some(false),
            _ => None,
        }
    }
    /// The module's priority override, if it is a mining speed module.
    pub fn mining_percent(&self) -> Option<bool> {
        match self {
            Self::MiningSpeed { module, .. } => Some(module.percent()),
            // MomentumModifier is not a percent module; it runs at its modifier's priority (75).
            // Neither is DwarvenModifier; it runs at its own priority (85).
            Self::Momentum | Self::Dwarven => Some(false),
            _ => None,
        }
    }

    /// ModifierTraitModule.addTraits: the entry this module adds for `entry`, if any. A fixed-level
    /// trait joins once (the first time its owner is met); otherwise its level scales with the owner's.
    pub fn trait_entry(&self, entry: &Entry, facts: &Facts, first_encounter: bool) -> Option<Entry> {
        let Self::Trait { modifier, level, fixed_level, condition } = self else { return None; };
        if !condition.matches(entry, facts) { return None; }
        if *fixed_level { return first_encounter.then(|| Entry::new(modifier.clone(), *level)); }
        Some(Entry::new(modifier.clone(), if entry.level == 1 { *level } else { level.wrapping_mul(entry.level) }))
    }
    /// SwappableToolTraitsModule.addTraits: the alternate hook's entries, added at their own levels.
    pub fn swappable_traits(&self, entry: &Entry, facts: &Facts) -> Vec<Entry> {
        let Self::SwappableTraits { key, matching, hook } = self else { return Vec::new(); };
        if !matching.is_empty() && *matching != (facts.persistent_string)(key.as_ref().unwrap_or(&entry.id)) { return Vec::new(); }
        (facts.hook_traits)(hook).into_iter().map(|trait_entry| Entry::new(trait_entry.id, trait_entry.level)).collect()
    }
    pub fn is_trait(&self) -> bool { matches!(self, Self::Trait { .. } | Self::SwappableTraits { .. }) }

    /// ValidateModifierHook (VALIDATE_UPGRADE): the translation key of the error when this
    /// modifier's level is in range and the tool does not meet the requirement.
    pub fn validate_upgrade(&self, entry: &Entry, facts: &Facts) -> Option<&str> {
        let Self::Requirements { requirement, min, max, translation_key } = self else { return None; };
        (*min <= entry.level && entry.level <= *max && !requirement.matches(facts)).then_some(translation_key.as_str())
    }

    /// ToolDamageModifierHook.beforeDamageTool: the amount left after this module's probabilistic
    /// reduction. `random` yields uniform values in [0, 1).
    pub fn reduce_damage(&self, entry: &Entry, facts: &Facts, nbt: &NbtCompound, cause: Option<&Identifier>,
        modifier_in_tag: &dyn Fn(&Identifier, &Identifier) -> bool, amount: i32, random: &mut dyn FnMut() -> f32) -> i32 {
        if let Self::DamageRange { min, max, cause: wanted, condition } = self {
            if !condition.matches(entry, facts) || !wanted.matches(cause, modifier_in_tag) { return amount; }
            // ToolDamageRangeModule: a closed range deals exactly that, otherwise the amount is clamped into it.
            return if min == max { *min } else { if amount < *min { *min } else if amount > *max { *max } else { amount } };
        }
        let Self::ReduceDamage { cause: wanted, formula, condition } = self else { return amount; };
        if !condition.matches(entry, facts) || !wanted.matches(cause, modifier_in_tag) { return amount; }
        // ConditionalStatFormula.apply(.., value 0, multiplier 1); a simple formula's IDENTITY is the scaled level.
        let percentage = match formula {
            LevelFormula::Simple { .. } => formula.level(entry),
            LevelFormula::PostFix(post_fix) => match post_fix.apply(&[formula.level(entry), 0.0, 1.0]) { Ok(value) => value, Err(_) => return amount },
            // The variables read the player holding the tool, as PlutoLib reported them as the tool wore (the fallback when it did not).
            LevelFormula::Stat(stat) => {
                let holder = crate::entity_facts::holder();
                stat.eval(entry, &crate::mining_speed::ToolView::with(nbt, crate::entity_facts::Entities { attacker: holder.as_ref(), target: None }), 0.0, 1.0, 0.0)
            }
        };
        if percentage >= 1.0 { return 0; }
        if percentage <= 0.0 { return amount; }
        // Each point of damage is cancelled with the given probability.
        (0..amount.max(0)).filter(|_| random() >= percentage).count() as i32
    }

    /// RepairFactorModifierHook.getRepairFactor for this module.
    pub fn repair_factor(&self, entry: &Entry, facts: &Facts, factor: f32) -> f32 {
        let Self::Repair { formula, condition } = self else { return factor; };
        if !condition.matches(entry, facts) { return factor; }
        match formula {
            // FallbackFormula.PERCENT: value * (1 + level), where the value argument is the factor.
            LevelFormula::Simple { .. } => factor * (1.0 + formula.level(entry)),
            LevelFormula::PostFix(post_fix) => post_fix.apply(&[formula.level(entry), factor]).unwrap_or(factor),
            LevelFormula::Stat(_) => factor,
        }
    }

    /// The module's own priority for the tool-stats hook (StatCopyModule.getPriority), if it overrides the modifier's.
    pub fn stat_priority(&self) -> Option<i32> { matches!(self, Self::StatCopy { .. }).then_some(50) }

    /// The tool-stats hook.
    pub fn apply_stats(&self, entry: &Entry, facts: &Facts, builder: &mut Builder) {
        if let Self::SetTier { tier, condition } = self {
            // ToolTierStat.supports: harvest tools only (no item context means no filter).
            let harvest = facts.item.is_none() || (facts.in_tag)(facts.item.expect("checked"), &Identifier::new("tconstruct", "modifiable/harvest").expect("valid tag"));
            if condition.matches(entry, facts) && harvest { builder.update_tier(tier); }
            return;
        }
        if let Self::StatOverride = self {
            let parse = |name: &str| Identifier::parse(name).ok().and_then(|id| Stat::from_id(&id));
            for (name, value) in (facts.persistent_numbers)(&Identifier::parse(OVERRIDE_BONUS).expect("valid key")) {
                if let Some(stat) = parse(&name) { builder.update(stat, value as f32); }
            }
            for (name, value) in (facts.persistent_numbers)(&Identifier::parse(OVERRIDE_MULTIPLIER).expect("valid key")) {
                if let Some(stat) = parse(&name) { builder.multiply(stat, f64::from(value as f32)); }
            }
            return;
        }
        if let Self::StatCopy { target, source, flat, each, condition } = self {
            if condition.matches(entry, facts) {
                let percentage = flat + each * entry.effective_level();
                let value = builder.get(*source) * percentage / builder.multiplier(*source);
                builder.add(*target, f64::from(value));
            }
            return;
        }
        let Self::Boost { stat, operation, flat, each, condition } = self else { return; };
        if !condition.matches(entry, facts) || !supports(*stat, facts) { return; }
        let value = f64::from(*flat + *each * entry.effective_level());
        match operation {
            Operation::Add => builder.add(*stat, value),
            Operation::Percent => builder.percent(*stat, value),
            Operation::MultiplyBase => builder.multiply(*stat, 1.0 + value),
            Operation::MultiplyConditional => builder.multiplier_only(*stat, 1.0 + value),
            Operation::MultiplyAll => builder.multiply_all(*stat, 1.0 + value),
        }
    }
}

/// INumericToolStat.supports(item): the item tags a stat applies to, any of them. With no item
/// context (previews outside a tool stack) nothing is filtered.
pub(crate) fn supports(stat: Stat, facts: &Facts) -> bool {
    let Some(item) = facts.item else { return true; };
    let tags: &[&str] = match stat {
        Stat::Durability => &["modifiable/durability"],
        Stat::UseItemSpeed | Stat::BlockAmount | Stat::BlockAngle => &["modifiable/held"],
        Stat::AttackDamage => &["modifiable/melee"],
        Stat::AttackSpeed => &["modifiable/melee/weapon"],
        Stat::MiningSpeed => &["modifiable/harvest"],
        Stat::Armor | Stat::ArmorToughness | Stat::KnockbackResistance => &["modifiable/armor"],
        Stat::DrawSpeed => &["modifiable/ranged"],
        Stat::Velocity | Stat::Accuracy => &["modifiable/ranged", "modifiable/ammo"],
        Stat::ProjectileDamage => &["modifiable/ranged/launcher", "modifiable/ammo"],
        Stat::SeaLuck | Stat::Lure => &["modifiable/fishing_rods"],
        // EdibleModule.VALID_TOOLS: tools that charge an interaction, and armor.
        Stat::Hunger | Stat::Saturation | Stat::EatDuration | Stat::EdibleCounterChance => &["modifiable/interactable/charge", "modifiable/armor"],
        // Water inertia and overslime declare no tag; tank capacity neither (CapacityStat ANY).
        Stat::WaterInertia | Stat::Overslime | Stat::TankCapacity => return true,
    };
    tags.iter().any(|tag| (facts.in_tag)(item, &Identifier::new("tconstruct", tag).expect("valid tag id")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
    fn decode(value: Value) -> Option<Module> {
        let fields = value.as_object().unwrap();
        let hooks: Vec<Identifier> = fields.get("hooks").and_then(Value::as_array).into_iter().flatten().map(|h| id(h.as_str().unwrap())).collect();
        Module::decode(fields["type"].as_str().unwrap(), fields, &hooks).unwrap()
    }
    fn facts<'a>(item: Option<&'a Identifier>, in_tag: &'a dyn Fn(&Identifier, &Identifier) -> bool) -> Facts<'a> {
        Facts { item, in_tag, level: &|_, _| 0, tag_level: &|_, _| 0, has_persistent_key: &|_| false, persistent_string: &|_| String::new(), persistent_numbers: &|_| Vec::new(), hook_traits: &|_| Vec::new(), has_hook: &|_| false }
    }
    #[test]
    fn the_one_probe_marks_held_tools_and_helmets_in_the_raw_data() {
        let module = decode(json!({"type":"tconstruct:the_one_probe"})).unwrap();
        let entry = Entry::new(id("tconstruct:the_one_probe"), 1);
        let held = id("tconstruct:pickaxe");
        let in_tag = |_: &Identifier, tag: &Identifier| tag == &id("tconstruct:modifiable/held");
        let mut data = NbtCompound::new();
        module.apply_raw_data(&entry, &facts(Some(&held), &in_tag), &mut data);
        assert_eq!(data["theoneprobe_hand"], Nbt::Byte(1));
        assert!(!data.contains_key("theoneprobe"), "a held tool is not a helmet");
        let helmet = id("tconstruct:slime_helmet");
        let in_tag = |_: &Identifier, tag: &Identifier| tag == &id("tconstruct:modifiable/armor/helmets");
        let mut data = NbtCompound::new();
        module.apply_raw_data(&entry, &facts(Some(&helmet), &in_tag), &mut data);
        assert_eq!(data["theoneprobe"], Nbt::Byte(1));
        assert!(!data.contains_key("theoneprobe_hand"));
        // No item context (a preview outside a stack): nothing is marked.
        let mut data = NbtCompound::new();
        module.apply_raw_data(&entry, &facts(None, &|_, _| true), &mut data);
        assert!(data.is_empty());
    }
    #[test]
    fn embellishment_migrates_its_stored_variant_and_forgets_it_on_removal() {
        let module = decode(json!({"type":"tconstruct:embellishment"})).unwrap();
        let modifier = id("tconstruct:embellishment");
        let mut materials = crate::materials::Definitions::default();
        materials.redirects.insert(id("tconstruct:old_wood"), id("tconstruct:wood"));
        let mut persistent = NbtCompound::new();
        persistent.insert(modifier.to_string(), Nbt::String("tconstruct:old_wood#default".into()));
        let (key, value) = module.embellish_migration(&modifier, &persistent, &materials).expect("the stored variant's material moved");
        assert_eq!(key, "tconstruct:embellishment");
        assert_eq!(value, Nbt::String("tconstruct:wood#default".into()));
        // A variant already at its resolved material does not move, and neither does a missing one.
        persistent.insert(modifier.to_string(), Nbt::String("tconstruct:wood".into()));
        assert!(module.embellish_migration(&modifier, &persistent, &materials).is_none());
        persistent.remove(&modifier.to_string());
        assert!(module.embellish_migration(&modifier, &persistent, &materials).is_none());
        // Removal forgets the stored variant.
        persistent.insert(modifier.to_string(), Nbt::String("tconstruct:wood".into()));
        module.on_removed(&modifier, &mut persistent);
        assert!(!persistent.contains_key("tconstruct:embellishment"));
    }
    #[test]
    fn flame_barrier_protects_a_burning_wearer_against_everything_but_fire() {
        let module = decode(json!({"type":"tconstruct:flame_barrier","each_level":1.875})).unwrap();
        let entry = Entry::new(id("tconstruct:flame_barrier"), 2);
        let nbt = NbtCompound::new();
        let f = facts(None, &|_, _| false);
        let tags = crate::damage_tags::DamageTags::load(&[]).0;
        let protect = |damage_type: &Identifier, wearer: Option<&pluto_lib::EntityState>| {
            let none = |_: &Identifier| false;
            let source = crate::protection::DamageSource { damage_type, indirect: false, tags: &tags, attacker_in_tag: &none, entities: crate::entity_facts::Entities { attacker: None, target: wearer } };
            module.protection(&entry, &f, &nbt, &source, 4.0)
        };
        let burning = pluto_lib::EntityState { fire_ticks: 200, ..Default::default() };
        let mob_attack = id("minecraft:mob_attack");
        // Fully burning at level 2: the whole amount (1.875 a level) lands on the 4 already there.
        assert_eq!(protect(&mob_attack, Some(&burning)), 7.75);
        // Half the fire left: half the bonus.
        let half = pluto_lib::EntityState { fire_ticks: 100, ..Default::default() };
        assert_eq!(protect(&mob_attack, Some(&half)), 4.0 + 1.875);
        // Fire itself is not protected against, and neither is a wearer who is not burning or not reported.
        let on_fire = id("minecraft:on_fire");
        assert_eq!(protect(&on_fire, Some(&burning)), 4.0);
        assert_eq!(protect(&mob_attack, Some(&pluto_lib::EntityState::default())), 4.0);
        assert_eq!(protect(&mob_attack, None), 4.0);
    }
    #[test]
    fn the_code_modifiers_scale_damage_and_speed_with_the_reported_states() {
        let f = facts(None, &|_, _| false);
        let no_tag = |_: &Identifier| false;
        // Conducting at level 1 on a fully burning attacker: melee and projectile damage grow by a fifth.
        let conducting = decode(json!({"type":"tconstruct:conducting"})).unwrap();
        let entry = Entry::new(id("tconstruct:conducting"), 1);
        let burning = pluto_lib::EntityState { fire_ticks: 200, ..Default::default() };
        let nbt = NbtCompound::new();
        let tool = crate::mining_speed::ToolView::with(&nbt, crate::entity_facts::Entities { attacker: Some(&burning), target: None });
        assert_eq!(conducting.melee_damage(&entry, &f, &tool, 10.0, 10.0, &no_tag), 12.0);
        assert_eq!(conducting.conditional_stat(&entry, &f, &tool, &no_tag, Stat::ProjectileDamage, 10.0, 1.0), 12.0);
        // Not burning: no growth. Another stat: untouched.
        let calm = crate::mining_speed::ToolView::of(&nbt);
        assert_eq!(conducting.melee_damage(&entry, &f, &calm, 10.0, 10.0, &no_tag), 10.0);
        assert_eq!(conducting.conditional_stat(&entry, &f, &tool, &no_tag, Stat::DrawSpeed, 10.0, 1.0), 10.0);
        // Insatiable at level 2 with three levels of its melee effect: +3 attack damage (half the
        // effect a level, at the tool's attack damage multiplier of 1 here).
        let insatiable = decode(json!({"type":"tconstruct:insatiable"})).unwrap();
        let entry = Entry::new(id("tconstruct:insatiable"), 2);
        let fed = pluto_lib::EntityState { effects: vec!["tconstruct:insatiable_melee".to_owned()], effect_levels: vec![("tconstruct:insatiable_melee".to_owned(), 2)], ..Default::default() };
        let tool = crate::mining_speed::ToolView::with(&nbt, crate::entity_facts::Entities { attacker: Some(&fed), target: None });
        assert_eq!(insatiable.melee_damage(&entry, &f, &tool, 10.0, 10.0, &no_tag), 13.0);
        // Momentum at level 1 with the harvest effect at its tenth level: a quarter faster digging,
        // and only on an effective dig.
        let momentum = decode(json!({"type":"tconstruct:momentum"})).unwrap();
        let entry = Entry::new(id("tconstruct:momentum"), 1);
        let holder = pluto_lib::EntityState { effects: vec!["tconstruct:momentum_harvest".to_owned()], effect_levels: vec![("tconstruct:momentum_harvest".to_owned(), 9)], ..Default::default() };
        let block = id("minecraft:stone");
        let context = crate::mining_speed::Context { block: &block, block_in_tag: &no_tag, original: 4.0, multiplier: 1.0, effective: true, on_ground: None, eyes_in_water: None, facts: Default::default(), holder: Some(holder) };
        let tool = crate::mining_speed::ToolView::of(&nbt);
        assert_eq!(momentum.mining_speed(&entry, &f, &tool, &context, 4.0), 5.0);
        let context = crate::mining_speed::Context { effective: false, ..context };
        assert_eq!(momentum.mining_speed(&entry, &f, &tool, &context, 4.0), 4.0);
    }
    #[test]
    fn enderclearance_and_channeling_decode_their_hit_numbers() {
        let entry = Entry::new(id("tconstruct:enderclearance"), 2);
        let module = decode(json!({"type":"tconstruct:enderclearance","chance":{"flat":0.25},"diameter":{"each_level":8,"flat":8},"teleport_chances":{"flat":16}})).unwrap();
        assert_eq!(module.enderclearance(&entry), Some((0.25, 24.0)));
        // The shipped channeling forbids melee, so its melee chances stay inert; data allowing
        // melee hands them over.
        let shipped = decode(json!({"type":"tconstruct:channeling","allow_melee":false,"chance_clear":0.15,"chance_rain":0.65,"chance_thunder":1.0})).unwrap();
        assert_eq!(shipped.channeling_melee(), None);
        let allowed = decode(json!({"type":"tconstruct:channeling","allow_melee":true,"chance_clear":0.15,"chance_rain":0.65,"chance_thunder":1.0})).unwrap();
        assert_eq!(allowed.channeling_melee(), Some((0.15, 0.65, 1.0)));
    }
    #[test]
    fn swappable_slots_follow_the_stored_slot_type_and_clear_on_removal() {
        let in_tag = |_: &Identifier, _: &Identifier| true;
        let stored = std::cell::RefCell::new(std::collections::BTreeMap::new());
        stored.borrow_mut().insert("test:swap".to_owned(), "abilities".to_owned());
        let lookup = |key: &Identifier| stored.borrow().get(&key.to_string()).cloned().unwrap_or_default();
        let f = Facts { item: None, in_tag: &in_tag, level: &|_, _| 0, tag_level: &|_, _| 0, has_persistent_key: &|_| false, persistent_string: &lookup, persistent_numbers: &|_| Vec::new(), hook_traits: &|_| Vec::new(), has_hook: &|_| false };
        let slot = decode(json!({"type":"tconstruct:swappable_slot","slots":1})).unwrap();
        let bonus = decode(json!({"type":"tconstruct:swappable_bonus_slot","match":"abilities","bonus":"upgrades","slots":-1})).unwrap();
        let other = decode(json!({"type":"tconstruct:swappable_bonus_slot","match":"defense","bonus":"upgrades","slots":-1})).unwrap();
        let entry = Entry::new(id("test:swap"), 1);
        let mut data = NbtCompound::new();
        for module in [&slot, &bonus, &other] { module.apply_volatile(&entry, &f, &mut data); }
        assert_eq!(data["abilities"], Nbt::Int(1));
        assert_eq!(data["upgrades"], Nbt::Int(-1), "only the module whose match is stored applies");
        // An explicit key reads another persistent entry; an absent value grants nothing.
        let keyed = decode(json!({"type":"tconstruct:swappable_slot","key":"test:other","slots":2})).unwrap();
        let mut none = NbtCompound::new();
        keyed.apply_volatile(&entry, &f, &mut none);
        assert!(none.is_empty());
        // A bonus that cancels the slot's own type removes the key instead of storing zero.
        let cancel = decode(json!({"type":"tconstruct:swappable_bonus_slot","match":"abilities","bonus":"abilities","slots":-1})).unwrap();
        cancel.apply_volatile(&entry, &f, &mut data);
        assert!(!data.contains_key("abilities"));
        // Only the slot module forgets its stored type when the modifier leaves the tool.
        let mut persistent = NbtCompound::new();
        persistent.insert("test:swap".into(), Nbt::String("abilities".into()));
        bonus.on_removed(&id("test:swap"), &mut persistent);
        assert!(persistent.contains_key("test:swap"));
        slot.on_removed(&id("test:swap"), &mut persistent);
        assert!(persistent.is_empty());
        // Hook bindings this port does not run are unbound; display-name and remove are allowed with volatile data.
        assert!(decode(json!({"type":"tconstruct:swappable_slot","slots":1,"hooks":["tconstruct:tooltip"]})).is_none());
        assert!(decode(json!({"type":"tconstruct:swappable_slot","slots":1,"hooks":["tconstruct:volatile_data"]})).is_some());
    }
    #[test]
    fn swappable_tool_traits_add_the_hook_traits_only_for_the_stored_value() {
        let in_tag = |_: &Identifier, _: &Identifier| true;
        let stored = |key: &Identifier| if key.to_string() == "test:rebalanced" { "traits".to_owned() } else { String::new() };
        let hook_traits = |hook: &Identifier| if hook.to_string() == "tconstruct:rebalanced_trait" {
            vec![crate::material_traits::ModifierEntry { id: id("test:extra"), level: 2, optional: false }]
        } else { Vec::new() };
        let f = Facts { item: None, in_tag: &in_tag, level: &|_, _| 0, tag_level: &|_, _| 0, has_persistent_key: &|_| false, persistent_string: &stored, persistent_numbers: &|_| Vec::new(), hook_traits: &hook_traits, has_hook: &|_| false };
        let entry = Entry::new(id("test:rebalanced"), 1);
        let matching = decode(json!({"type":"tconstruct:swappable_tool_traits","hook":"tconstruct:rebalanced_trait","match":"traits"})).unwrap();
        let other_value = decode(json!({"type":"tconstruct:swappable_tool_traits","hook":"tconstruct:rebalanced_trait","match":"other"})).unwrap();
        let always = decode(json!({"type":"tconstruct:swappable_tool_traits","hook":"tconstruct:rebalanced_trait"})).unwrap();
        let wrong_hook = decode(json!({"type":"tconstruct:swappable_tool_traits","hook":"tconstruct:trim_trait"})).unwrap();
        let levels = |module: &Module| module.swappable_traits(&entry, &f).iter().map(|added| (added.id.to_string(), added.level)).collect::<Vec<_>>();
        assert_eq!(levels(&matching), vec![("test:extra".to_owned(), 2)]);
        assert!(levels(&other_value).is_empty());
        assert_eq!(levels(&always), vec![("test:extra".to_owned(), 2)], "an empty match applies whatever is stored");
        assert!(levels(&wrong_hook).is_empty());
        assert!(decode(json!({"type":"tconstruct:swappable_tool_traits","hook":"tconstruct:unknown_hook"})).is_none());
        // Only a module with a match owns the stored value and clears it on removal.
        let mut persistent = NbtCompound::new();
        persistent.insert("test:rebalanced".into(), Nbt::String("traits".into()));
        always.on_removed(&id("test:rebalanced"), &mut persistent);
        assert!(persistent.contains_key("test:rebalanced"));
        matching.on_removed(&id("test:rebalanced"), &mut persistent);
        assert!(persistent.is_empty());
    }

    #[test]
    fn leveling_arithmetic_and_slot_rules_follow_the_original() {
        let in_tag = |_: &Identifier, _: &Identifier| true;
        let f = facts(None, &in_tag);
        let mut data = NbtCompound::new();
        // volatile_int: (int)(flat + each * level), added to what is there.
        let int = decode(json!({"type":"tconstruct:volatile_int","flag":"test:n","flat":1,"each_level":2}));
        int.as_ref().unwrap().apply_volatile(&Entry::new(id("test:m"), 3), &f, &mut data);
        int.as_ref().unwrap().apply_volatile(&Entry::new(id("test:m"), 3), &f, &mut data);
        assert_eq!(data["test:n"], Nbt::Int(14));
        // modifier_slot: the older count field is the per-level amount; a total of zero removes the key.
        let plus = decode(json!({"type":"tconstruct:modifier_slot","name":"upgrades","count":1})).unwrap();
        let minus = decode(json!({"type":"tconstruct:modifier_slot","name":"upgrades","each_level":-1})).unwrap();
        plus.apply_volatile(&Entry::new(id("test:m"), 2), &f, &mut data);
        assert_eq!(data["upgrades"], Nbt::Int(2));
        minus.apply_volatile(&Entry::new(id("test:m"), 2), &f, &mut data);
        assert!(!data.contains_key("upgrades"));
        // A condition on the modifier level gates the module.
        let gated = decode(json!({"type":"tconstruct:volatile_flag","flag":"test:f","modifier_level":{"min":2}})).unwrap();
        gated.apply_volatile(&Entry::new(id("test:m"), 1), &f, &mut data);
        assert!(!data.contains_key("test:f"));
        gated.apply_volatile(&Entry::new(id("test:m"), 2), &f, &mut data);
        assert_eq!(data["test:f"], Nbt::Byte(1));
    }
    #[test]
    fn stat_boosts_apply_each_operation_only_to_supported_conditioned_tools() {
        let pickaxe = id("tconstruct:pickaxe");
        let in_tag = |item: &Identifier, tag: &Identifier| item.path() == "pickaxe" && matches!(tag.path(), "modifiable/harvest" | "modifiable/durability");
        let f = facts(Some(&pickaxe), &in_tag);
        let boost = |stat: &str, operation: &str, flat: f64, each: f64| decode(json!({"type":"tconstruct:stat_boost","stat":stat,"operation":operation,"flat":flat,"each_level":each})).unwrap();
        let mut builder = Builder::default();
        let entry = Entry::new(id("test:m"), 2);
        boost("tconstruct:durability", "add", 10.0, 5.0).apply_stats(&entry, &f, &mut builder);
        boost("tconstruct:durability", "multiply_base", 0.0, 0.5).apply_stats(&entry, &f, &mut builder);
        boost("tconstruct:mining_speed", "multiply_all", 0.0, 0.25).apply_stats(&entry, &f, &mut builder);
        // Not supported by this item's tags: armor stats do nothing on a pickaxe.
        boost("tconstruct:armor", "add", 5.0, 0.0).apply_stats(&entry, &f, &mut builder);
        assert_eq!(builder.get(Stat::Durability), (1.0 + 20.0) * 2.0, "base 1 + add (10 + 5*2), then x(1 + 0.5*2)");
        assert_eq!(builder.get(Stat::Armor), Stat::Armor.default_value());
        assert_eq!(builder.multiplier(Stat::MiningSpeed), 1.5, "multiply_all also records the global multiplier");
        // multiply_conditional only records the multiplier, leaving the stat itself alone.
        let mut conditional = Builder::default();
        boost("tconstruct:mining_speed", "multiply_conditional", 0.0, 0.5).apply_stats(&entry, &f, &mut conditional);
        assert_eq!(conditional.multiplier(Stat::MiningSpeed), 2.0);
        assert_eq!(conditional.get(Stat::MiningSpeed), Stat::MiningSpeed.default_value());
        // A tool predicate condition that fails skips the boost.
        let armor_only = decode(json!({"type":"tconstruct:stat_boost","stat":"tconstruct:durability","operation":"add","flat":99,
            "tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/armor"}})).unwrap();
        let mut skipped = Builder::default();
        armor_only.apply_stats(&entry, &f, &mut skipped);
        assert_eq!(skipped.get(Stat::Durability), Stat::Durability.default_value());
    }
    #[test]
    fn a_reduction_formula_reads_the_light_around_the_player_holding_the_tool() {
        let in_tag = |_: &Identifier, _: &Identifier| false;
        let f = facts(None, &in_tag);
        let entry = Entry::new(id("test:solar"), 1);
        let none = NbtCompound::new();
        // The shipped solar powered: (0.01 * level + 0.04) * the sky light, which is 15 when nobody was named.
        let solar = decode(json!({"type":"tconstruct:reduce_tool_damage","formula":[0.01,"$level","*",0.04,"+","$light","*"],
            "variables":{"light":{"type":"tconstruct:entity","entity_type":"tconstruct:light","fallback":15.0,"light_layer":"sky"}}})).unwrap();
        let dealt = |holder: Option<&pluto_lib::EntityState>, solar: &Module| {
            let mut rolls = [0.1_f32, 0.6, 0.4, 0.9].into_iter();
            crate::entity_facts::with_holder(holder, || solar.reduce_damage(&entry, &f, &none, None, &|_, _| false, 4, &mut || rolls.next().unwrap()))
        };
        // Light 15 less the 5 the weather takes is 10: a half chance each, so the 0.1 and 0.4 rolls cancel.
        let dusk = pluto_lib::EntityState { sky_light: Some(15), sky_darken: 5, ..Default::default() };
        assert_eq!(dealt(Some(&dusk), &solar), 2);
        // Nobody named: the fallback light of 15 is a 75% chance and only the 0.9 roll gets through.
        assert_eq!(dealt(None, &solar), 1);
        assert_eq!(crate::entity_facts::holder(), None, "the holder is only set around the wear");
    }

    #[test]
    fn damage_reduction_is_probabilistic_per_point_and_repair_scales_the_factor() {
        let in_tag = |_: &Identifier, _: &Identifier| false;
        let f = facts(None, &in_tag);
        let entry = Entry::new(id("test:m"), 2);
        let tags = |cause: &Identifier, tag: &Identifier| cause.path() == "bypasser" && tag.path() == "bypass";
        // Reinforced-style: 0.025 * level * (11 - level) = 0.45 at level 2; each damage point is cancelled when the roll is below it.
        let reduce = decode(json!({"type":"tconstruct:reduce_tool_damage","formula":[0.025,"$level","*",11.0,"$level","-","*"]})).unwrap();
        let mut rolls = [0.1_f32, 0.5, 0.44, 0.9].into_iter();
        let dealt = reduce.reduce_damage(&entry, &f, &NbtCompound::new(), None, &tags, 4, &mut || rolls.next().unwrap());
        assert_eq!(dealt, 2, "rolls 0.5 and 0.9 are not below 0.45, so two points land");
        // 100% blocks everything without rolling; 0% blocks nothing without rolling.
        let all = decode(json!({"type":"tconstruct:reduce_tool_damage","flat":1.0})).unwrap();
        assert_eq!(all.reduce_damage(&entry, &f, &NbtCompound::new(), None, &tags, 7, &mut || panic!("no roll expected")), 0);
        let none = decode(json!({"type":"tconstruct:reduce_tool_damage","flat":0.0,"each_level":0.0})).unwrap();
        assert_eq!(none.reduce_damage(&entry, &f, &NbtCompound::new(), None, &tags, 7, &mut || panic!("no roll expected")), 7);
        // A cause predicate: this reduction ignores damage caused by a bypassing modifier.
        let guarded = decode(json!({"type":"tconstruct:reduce_tool_damage","flat":1.0,
            "cause":{"type":"mantle:inverted","inverted_type":"tconstruct:tag","tag":"test:bypass"}})).unwrap();
        assert_eq!(guarded.reduce_damage(&entry, &f, &NbtCompound::new(), None, &tags, 3, &mut || 0.0), 0, "ordinary use is reduced");
        assert_eq!(guarded.reduce_damage(&entry, &f, &NbtCompound::new(), Some(&id("test:bypasser")), &tags, 3, &mut || 0.0), 3, "a bypassing cause is not");
        // Repair: simple form is factor * (1 + scaled level); post fix reads the effective level and the factor.
        let preserved = decode(json!({"type":"tconstruct:repair","each_level":0.15})).unwrap();
        let Some(Module::Repair { .. }) = Some(&preserved) else { panic!() };
        assert!((preserved.repair_factor(&entry, &f, 10.0) - 10.0 * (1.0 + 0.3)).abs() < 1e-5);
        let dense = decode(json!({"type":"tconstruct:repair","formula":["$factor",1.0,0.025,"$level","*",11.0,"$level","-","*","-","*"],"modifier_level":{"max":5}})).unwrap();
        assert!((dense.repair_factor(&entry, &f, 10.0) - 10.0 * (1.0 - 0.025 * 2.0 * 9.0)).abs() < 1e-5);
        assert_eq!(dense.repair_factor(&Entry::new(id("test:m"), 6), &f, 10.0), 10.0, "outside the level range nothing changes");
        // Unsupported shapes are unbound: an entity predicate or data-defined variables.
        assert!(decode(json!({"type":"tconstruct:reduce_tool_damage","flat":1.0,"entity":{"type":"tconstruct:sneaking"}})).is_none());
        assert!(decode(json!({"type":"tconstruct:reduce_tool_damage","formula":["$count"],"variables":{"count":{"type":"tconstruct:entity","entity_type":"tconstruct:equipment_count","slots":["mainhand"],"fallback":0.0}}})).is_some(), "an equipment count over the reported slots binds");
        let bad = json!({"type":"tconstruct:reduce_tool_damage","formula":["$count"],"variables":{"count":{"type":"tconstruct:entity","entity_type":"tconstruct:equipment_count","fallback":0.0}}});
        assert!(Module::decode("tconstruct:reduce_tool_damage", bad.as_object().unwrap(), &[]).is_err(), "the loader requires the slots list, as in the original");
        assert!(decode(json!({"type":"tconstruct:reduce_tool_damage","flat":1.0,"cause":{"type":"mantle:and"}})).is_none());
    }
    #[test]
    fn tool_actions_answer_only_their_named_actions_and_display_modules_change_nothing() {
        let in_tag = |_: &Identifier, _: &Identifier| true;
        let f = facts(None, &in_tag);
        let entry = Entry::new(id("test:m"), 1);
        let actions = decode(json!({"type":"tconstruct:tool_actions","tool_actions":["drill_attack","item_hook"]})).unwrap();
        assert!(actions.can_perform_action(&entry, &f, "item_hook") && !actions.can_perform_action(&entry, &f, "grapple_hook"));
        let late = decode(json!({"type":"tconstruct:tool_actions","tool_actions":["shield_disable"],"modifier_level":{"min":2}})).unwrap();
        assert!(!late.can_perform_action(&entry, &f, "shield_disable"), "the level condition gates the action");
        // Every presentation module keeps its data and leaves the volatile data alone.
        for value in [json!({"type":"tconstruct:show_interaction_source"}), json!({"type":"tconstruct:variant_color"}),
            json!({"type":"tconstruct:material_variant_color","material":"tconstruct:wool"}), json!({"type":"tconstruct:durability_color","color":"FFFFFF"}),
            json!({"type":"tconstruct:variant_name","formatter":"tconstruct:parameter"}), json!({"type":"tconstruct:stat_tooltip","stat":"tconstruct:hunger"})] {
            let module = decode(value.clone()).unwrap_or_else(|| panic!("{value} is unbound"));
            let mut data = NbtCompound::new();
            module.apply_volatile(&entry, &f, &mut data);
            assert!(data.is_empty());
        }
        assert_eq!(decode(json!({"type":"tconstruct:durability_color","color":"00ff80"})), Some(Module::Display(Display::DurabilityColor { color: 0x00FF80 })));
        assert!(Module::decode("tconstruct:durability_color", json!({"color":"zz"}).as_object().unwrap(), &[]).is_err());
        assert!(decode(json!({"type":"tconstruct:show_interaction_source","hooks":["tconstruct:tooltip"]})).is_none(), "a foreign hook binding stays unbound");
    }
    #[test]
    fn sweeping_edge_adds_its_leveling_value_onto_the_sweep_percent() {
        let in_tag = |_: &Identifier, _: &Identifier| true;
        let f = facts(None, &in_tag);
        // The shipped module: each_level 0.25 and no flat, so level 1 adds 0.25 and level 2 adds 0.50.
        let module = decode(json!({"type":"tconstruct:sweeping_edge","each_level":0.25})).unwrap();
        let mut data = NbtCompound::new();
        module.apply_volatile(&Entry::new(id("tconstruct:sweeping_edge"), 1), &f, &mut data);
        assert_eq!(data["tconstruct:sweep_percent"], Nbt::Float(0.25));
        // A second module adds onto what is already there, as the original accumulates the float.
        module.apply_volatile(&Entry::new(id("tconstruct:sweeping_edge"), 2), &f, &mut data);
        assert_eq!(data["tconstruct:sweep_percent"], Nbt::Float(0.75));
        // The flat part does not scale: flat 0.5 with each_level 0.25 at level 2 is 1.0.
        let flat = decode(json!({"type":"tconstruct:sweeping_edge","flat":0.5,"each_level":0.25})).unwrap();
        let mut data = NbtCompound::new();
        flat.apply_volatile(&Entry::new(id("tconstruct:sweeping_edge"), 2), &f, &mut data);
        assert_eq!(data["tconstruct:sweep_percent"], Nbt::Float(1.0));
        // Only the volatile data and tooltip bindings are honored.
        assert!(decode(json!({"type":"tconstruct:sweeping_edge","each_level":0.25,"hooks":["tconstruct:volatile_data"]})).is_some());
        assert!(decode(json!({"type":"tconstruct:sweeping_edge","each_level":0.25,"hooks":["tconstruct:tooltip"]})).is_some());
        assert!(decode(json!({"type":"tconstruct:sweeping_edge","each_level":0.25,"hooks":["tconstruct:melee_hit"]})).is_none());
        assert!(Module::decode("tconstruct:sweeping_edge", json!({"each_level":"high"}).as_object().unwrap(), &[]).is_err());
    }
    #[test]
    fn tank_modules_decode_from_their_shipped_shapes() {
        // The shipped definitions (tinkering/modifiers), fields exactly as the files carry them.
        assert!(decode(json!({"type":"tconstruct:tank","hooks":["tconstruct:volatile_data","tconstruct:validate","tconstruct:remove","tconstruct:slot_stack","tconstruct:display_name"]})).is_some_and(|module| module.is_tank()));
        assert!(decode(json!({"type":"tconstruct:bucket"})).is_some());
        assert!(decode(json!({"type":"tconstruct:tank_interaction","interaction_source":"armor"})).is_some());
        assert!(decode(json!({"type":"tconstruct:melting","temperature":{"flat":500,"each_level":500},"nuggets_per_metal":{"flat":9,"each_level":3},"shards_per_gem":{"flat":6,"each_level":2}})).is_some());
        assert!(decode(json!({"type":"tconstruct:slurping","strength":{"each_level":1.0},"duration":{"flat":21}})).is_some());
        assert!(decode(json!({"type":"tconstruct:spilling","each_level":1.0})).is_some());
        assert!(decode(json!({"type":"tconstruct:spitting","shots":{"each_level":1}})).is_some());
        assert!(decode(json!({"type":"tconstruct:splashing","strength":{"each_level":1.0}})).is_some());
        assert!(decode(json!({"type":"tconstruct:overburn"})).is_some_and(|module| module.is_overburn()));
        assert!(decode(json!({"type":"tconstruct:smashing"})).is_some_and(|module| module.is_smashing()));
        // Malformed tank modules reject; foreign hook bindings stay unbound.
        assert!(Module::decode("tconstruct:melting", json!({"temperature":{"flat":500}}).as_object().unwrap(), &[]).is_err());
        assert!(decode(json!({"type":"tconstruct:bucket","hooks":["tconstruct:tooltip"]})).is_none());
    }
    #[test]
    fn show_offhand_counts_a_flat_one_on_the_chest_only() {
        let key = id("tconstruct:show_empty_offhand");
        let entry = Entry::new(id("tconstruct:stripping"), 3);
        // The shipped module: disallow_broken. The count is a flat 1 at any level, on the chest only.
        let strict = decode(json!({"type":"tconstruct:show_offhand","mode":"disallow_broken"})).unwrap();
        assert_eq!(strict.armor_level(&entry, "chest", false), Some((key.clone(), 1)));
        assert_eq!(strict.armor_level(&entry, "head", false), None);
        assert_eq!(strict.armor_level(&entry, "chest", true), None, "a broken piece counts nothing in this mode");
        let lenient = decode(json!({"type":"tconstruct:show_offhand","mode":"allow_broken"})).unwrap();
        assert_eq!(lenient.armor_level(&entry, "chest", true), Some((key.clone(), 1)));
        // The worn totals keep the key while such a piece is worn.
        assert_eq!(crate::max_armor::totals(vec![strict.armor_level(&entry, "chest", false).unwrap()])[&key], 1);
        // The mode is required and must be one of the two enum names; a foreign hook binding stays unbound.
        assert!(Module::decode("tconstruct:show_offhand", json!({}).as_object().unwrap(), &[]).is_err());
        assert!(Module::decode("tconstruct:show_offhand", json!({"mode":"sometimes"}).as_object().unwrap(), &[]).is_err());
        assert!(decode(json!({"type":"tconstruct:show_offhand","mode":"disallow_broken","hooks":["tconstruct:tooltip"]})).is_none());
    }
    #[test]
    fn unrepresentable_shapes_are_unbound_and_malformed_modules_reject() {
        assert!(decode(json!({"type":"tconstruct:stat_boost","stat":"tconstruct:tank_capacity","operation":"add","flat":1})).is_some(), "the tank capacity stat binds since the tool tank was ported (tool_tank)");
        assert!(decode(json!({"type":"tconstruct:volatile_flag","flag":"a:b","hooks":["tconstruct:projectile_launch"]})).is_none());
        assert!(decode(json!({"type":"tconstruct:stat_boost","stat":"tconstruct:durability","operation":"add","flat":1,"hooks":["tconstruct:volatile_data"]})).is_none());
        assert!(decode(json!({"type":"tconstruct:conditional_melee_damage","attacker":"mantle:sprinting","flat":7.0})).is_some(), "an attacker state condition is bound");
        assert!(decode(json!({"type":"tconstruct:conditional_melee_damage","target":"mantle:on_fire","each_level":2.0})).is_some(), "a target state condition is bound");
        assert!(decode(json!({"type":"tconstruct:conditional_melee_damage","attacker":{"type":"mantle:in_biome","biome":"minecraft:plains"},"flat":7.0})).is_none(), "an attacker condition on the biome stays unbound");
        assert!(decode(json!({"type":"tconstruct:conditional_melee_damage","target":{"type":"mantle:mob_type","mobs":"minecraft:undead"},"each_level":2.0})).is_some(), "a mob type target is bound");
        assert!(decode(json!({"type":"tconstruct:conditional_melee_damage","each_level":2.0})).is_some(), "a condition-free melee module is bound");
        for (kind, fields) in [("tconstruct:stat_boost", json!({"stat":"tconstruct:durability","operation":"explode"})),
            ("tconstruct:modifier_slot", json!({"name":"Bad Name"})), ("tconstruct:volatile_int", json!({"flag":"a:b","flat":99999})),
            ("tconstruct:rarity", json!({"rarity":"mythic"})), ("tconstruct:volatile_flag", json!({"flag":"a:b","modifier_level":0})),
            ("tconstruct:volatile_flag", json!({"flag":"a:b","tool":{}}))] {
            assert!(Module::decode(kind, fields.as_object().unwrap(), &[]).is_err(), "{kind} {fields}");
        }
        // A well-formed predicate of a type this port lacks leaves just that module unbound.
        assert!(decode(json!({"type":"tconstruct:volatile_flag","flag":"a:b","tool":{"type":"tconstruct:has_hook","hook":"x:y"}})).is_none());
    }

    #[test]
    fn craft_count_multiplies_the_running_count_by_its_leveling_value() {
        let in_tag = |_: &Identifier, _: &Identifier| false;
        let f = facts(None, &in_tag);
        // The shipped cheap (half) and economical (double).
        let cheap = decode(json!({"type":"tconstruct:craft_count","multiplier":{"flat":0.5}})).unwrap();
        let economical = decode(json!({"type":"tconstruct:craft_count","multiplier":{"flat":2.0}})).unwrap();
        let entry = Entry::new(id("test:count"), 1);
        assert_eq!(cheap.craft_count(&entry, &f, 16.0), 8.0);
        assert_eq!(economical.craft_count(&entry, &f, 16.0), 32.0);
        // Per level on top of the flat part.
        let leveled = decode(json!({"type":"tconstruct:craft_count","multiplier":{"flat":1.0,"each_level":0.5}})).unwrap();
        assert_eq!(leveled.craft_count(&Entry::new(id("test:count"), 3), &f, 10.0), 25.0);
        // Another hook binding is not honored, and the multiplier is required.
        assert!(decode(json!({"type":"tconstruct:craft_count","multiplier":{"flat":2.0},"hooks":["tconstruct:volatile_data"]})).is_none());
        assert!(Module::decode("tconstruct:craft_count", json!({}).as_object().unwrap(), &[]).is_err());
    }

    #[test]
    fn material_repair_pays_its_amount_times_the_level_for_its_own_material() {
        let in_tag = |_: &Identifier, _: &Identifier| false;
        let f = facts(None, &in_tag);
        let no_durability = |_: &Identifier, _: &Identifier| 0;
        // The shipped stringy: string repairs for a constant 140.
        let stringy = decode(json!({"type":"tconstruct:material_repair","material":"tconstruct:string","durability":140})).unwrap();
        let entry = Entry::new(id("tconstruct:stringy"), 2);
        assert_eq!(stringy.material_repair_amount(&entry, &f, &id("tconstruct:string"), &no_durability), 280);
        assert_eq!(stringy.material_repair_amount(&entry, &f, &id("tconstruct:wood"), &no_durability), 0, "another material");
        assert!(stringy.is_repair_material(&entry, &f, &id("tconstruct:string")));
        assert_eq!(stringy.repair_material(), Some(&id("tconstruct:string")));
        // The stat type form reads the material's durability for that stat type.
        let typed = decode(json!({"type":"tconstruct:material_repair","material":"tconstruct:wood","stat_type":"tconstruct:handle"})).unwrap();
        let durability_of = |_: &Identifier, _: &Identifier| 60;
        assert_eq!(typed.material_repair_amount(&Entry::new(id("test:typed"), 1), &f, &id("tconstruct:wood"), &durability_of), 60);
        // The either form needs one of its keys, and a constant is at least 1.
        assert!(Module::decode("tconstruct:material_repair", json!({"material":"a:b"}).as_object().unwrap(), &[]).is_err());
        assert!(Module::decode("tconstruct:material_repair", json!({"material":"a:b","durability":0}).as_object().unwrap(), &[]).is_err());
    }

    #[test]
    fn volatile_float_adds_onto_the_flag_and_onto_a_projectile() {
        let in_tag = |_: &Identifier, _: &Identifier| false;
        let f = facts(None, &in_tag);
        // The shipped experienced: half a point of experience a level, on both hooks.
        let experienced = decode(json!({"type":"tconstruct:volatile_float","flag":"tconstruct:experience","each_level":0.5,"hooks":["tconstruct:volatile_data","tconstruct:projectile_launch"]})).unwrap();
        let entry = Entry::new(id("tconstruct:experienced"), 3);
        let mut data = NbtCompound::new();
        data.insert("tconstruct:experience".into(), Nbt::Float(1.0));
        experienced.apply_volatile(&entry, &f, &mut data);
        assert_eq!(data["tconstruct:experience"], Nbt::Float(2.5), "added onto what the flag held");
        assert_eq!(experienced.projectile_float(&entry, &f), Some((id("tconstruct:experience"), 1.5)));
        // A binding of nothing but the launch hook stays unbound, like the flag's.
        assert!(decode(json!({"type":"tconstruct:volatile_float","flag":"a:b","hooks":["tconstruct:projectile_launch"]})).is_none());
        assert!(decode(json!({"type":"tconstruct:volatile_float","flag":"a:b"})).is_some(), "no hooks field binds the default");
    }

    #[test]
    fn update_health_binds_for_armor_slots_and_hands_alike() {
        let in_tag = |_: &Identifier, _: &Identifier| false;
        let f = facts(None, &in_tag);
        // The shipped revitalizing armor module carries no bonus; the hand one carries 2 per level.
        let armor = decode(json!({"type":"tconstruct:update_health","slots":["feet","legs","chest","head"],"hooks":["tconstruct:equipment_change"]})).unwrap();
        assert_eq!(armor.update_health(&Entry::new(id("tconstruct:revitalizing"), 2), &f, "chest", true), 0.0);
        let hands = decode(json!({"type":"tconstruct:update_health","bonus":{"each_level":2.0},"slots":["mainhand","offhand"],"hooks":["tconstruct:equipment_change"]})).unwrap();
        assert_eq!(hands.update_health(&Entry::new(id("tconstruct:revitalizing"), 2), &f, "mainhand", true), 4.0, "the tick reports held tools, so the hand module binds");
        assert_eq!(hands.update_health(&Entry::new(id("tconstruct:revitalizing"), 2), &f, "offhand", false), -4.0);
        let bonus = decode(json!({"type":"tconstruct:update_health","bonus":{"each_level":2.0},"slots":["feet"],"hooks":["tconstruct:equipment_change"]})).unwrap();
        assert_eq!(bonus.update_health(&Entry::new(id("tconstruct:revitalizing"), 2), &f, "feet", true), 4.0);
        assert_eq!(bonus.update_health(&Entry::new(id("tconstruct:revitalizing"), 2), &f, "feet", false), -4.0);
    }

    #[test]
    fn golden_attribute_and_lightspeed_bind_in_their_shipped_forms() {
        // The shipped gold guard (a vanilla attribute, addition) binds; chrysophilite's own attribute does not
        // exist in the host, so its golden module stays unbound.
        assert!(decode(json!({"type":"tconstruct:golden_attribute","attribute":"minecraft:generic.max_health","each_level":4.0,"flat":4.0,"operation":"addition"})).is_some());
        assert!(decode(json!({"type":"tconstruct:golden_attribute","attribute":"tconstruct:generic.chrysophilite","each_level":1.0,"flat":1.0,"operation":"addition"})).is_none());
        // The shipped lightspeed attribute binds; a foreign hook binding stays unbound.
        assert!(decode(json!({"type":"tconstruct:lightspeed_attribute","attribute":"minecraft:generic.movement_speed","damage_chance":0.005,"light_layer":"block","min_light":5,"operation":"addition","per_level":9.0e-4})).is_some());
        assert!(decode(json!({"type":"tconstruct:lightspeed_attribute","attribute":"minecraft:generic.movement_speed","damage_chance":0.005,"light_layer":"block","min_light":5,"operation":"addition","per_level":9.0e-4,"hooks":["tconstruct:melee_hit"]})).is_none());
    }
}
