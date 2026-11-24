//! Counterpart of Fabric's `fabric-entity-events-v1`: entity load and unload, and living-entity damage.
//!
//! Entities are identified by their numeric id (Fabric passes entity objects); PlutoLib grows the data as modules need it.

use std::cell::{Cell, RefCell};
use super::block_entity_api_v1::ItemSlot;

pub use super::entity_spawn_api::{MobSpawnInfo,MobSpawnPreparation,MobSpawnCompletion,MobSpawnAction,MobSpawnCallback};
use super::base::Event;
use super::player_api::StackData;
use super::base::{Nbt, NbtCompound};

/// An entity entering or leaving a world.
#[derive(Clone, Debug, PartialEq)]
pub struct EntityInfo {
    pub id: u32,
    /// The entity type id, such as `minecraft:zombie`.
    pub kind: String,
    pub dimension: String,
}

/// Damage about to be applied to a living entity.
#[derive(Clone, Debug, PartialEq)]
pub struct LivingDamage {
    pub entity: i32,
    pub kind: String,
    /// The damage type id, such as `minecraft:player_attack`.
    pub damage_type: String,
    /// The entity that caused it (the attacker), or a negative id if none.
    pub attacker: i32,
    /// The entity that directly dealt it (a projectile, for example), or a negative id if none.
    pub direct: i32,
    pub amount: f32,
}

static ENTITY_LOAD: Event<EntityInfo, ()> = Event::void();
static ENTITY_UNLOAD: Event<EntityInfo, ()> = Event::void();
static ENTITY_TICK_PRE: Event<EntityInfo, bool> = Event::allow_all();
static ENTITY_TICK_POST: Event<EntityInfo, ()> = Event::void();

/// Fabric: `ServerEntityEvents`.
pub struct ServerEntityEvents;

impl ServerEntityEvents {
    pub const ENTITY_LOAD: &'static Event<EntityInfo, ()> = &ENTITY_LOAD;
    pub const ENTITY_UNLOAD: &'static Event<EntityInfo, ()> = &ENTITY_UNLOAD;
    /// An entity is about to tick (NeoForge: `EntityTickEvent.Pre`): return `false` to cancel this tick of the
    /// entity. Heard for every kind the host ticks, derived types included — this is the tick behaviour surface of
    /// a type declared with the entity type registry, whose declarations carry no code of their own. The entity and
    /// world reaches are open while the callbacks run.
    pub const ENTITY_TICK_PRE: &'static Event<EntityInfo, bool> = &ENTITY_TICK_PRE;
    /// An entity has ticked (NeoForge: `EntityTickEvent.Post`): a notification, heard after the entity's own tick
    /// ran (and not at all for a tick `ENTITY_TICK_PRE` cancelled).
    pub const ENTITY_TICK_POST: &'static Event<EntityInfo, ()> = &ENTITY_TICK_POST;
    /// A player is about to shear a mob (Bukkit: `PlayerShearEntityEvent`; Fabric has no counterpart). Set `cancel` to stop the shearing,
    /// or queue `actions` for the player. Server only; heard before the shears act, so a later callback may still cancel the shearing. The game
    /// reports only a guest's shearing on the owner's LAN world: the world owner's own shearing is heard by no hook.
    pub const SHEAR: &'static Event<ShearAttempt, ()> = &SHEAR;
}

/// A player about to shear a mob, heard on a server before the shearing happens.
#[derive(Clone, Debug)]
pub struct ShearAttempt {
    /// The player's uuid and name.
    pub player: u128,
    pub name: String,
    /// The mob's entity id.
    pub entity: i32,
    /// Whether the shears are in the off hand.
    pub off_hand: bool,
    /// The stack in that hand (its damage, max damage and custom data); None for an empty hand.
    pub tool: Option<StackData>,
    /// Set to true to stop the shearing.
    pub cancel: Cell<bool>,
    /// The actions to do to the player (`Attacker`), which callbacks add to; they land when the shearing is heard, even when a later
    /// callback cancels it.
    pub actions: RefCell<Vec<EntityAction>>,
}

/// What a living entity is doing, as the game reports it when a player's weapon hits a mob.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EntityState {
    /// The entity type id, such as `minecraft:husk`.
    pub kind: String,
    pub sprinting: bool,
    /// Whether it stands on the ground; false in the air.
    pub on_ground: bool,
    /// Whether it is in water.
    pub in_water: bool,
    /// How long it burns for, in ticks; 0 or less when it does not.
    pub fire_ticks: i32,
    pub fire_immune: bool,
    /// The ids of the effects on it, such as `minecraft:poison`.
    pub effects: Vec<String>,
    /// Each effect on it as (id, amplifier): the amplifier is the level less one, 0 for level I. Lists the same effects as `effects`.
    pub effect_levels: Vec<(String, i32)>,
    /// Its health points, and its maximum (the `minecraft:max_health` attribute); None when the game did not report them.
    pub health: Option<f32>,
    pub max_health: Option<f32>,
    /// The biome it stands in, such as `minecraft:plains`; None when the game did not report it.
    pub biome: Option<String>,
    /// That biome's temperature at its height, as `Biome.getTemperature` reports it; None for a biome PlutoLib has no temperature for.
    pub biome_temperature: Option<f32>,
    /// Whether its eyes are in water (`minecraft:water`, a waterlogged block, kelp or seagrass), as opposed to `in_water`, which a touch of water sets.
    pub eyes_in_water: bool,
    /// Whether rain falls on it: it is raining, the sky is open above it and the biome rains rather than snows or stays dry.
    pub rain_exposed: bool,
    /// The share of its armor slots (head, chest, legs, feet) holding armor, 0.0 to 1.0, as `LivingEntity.getArmorCoverPercentage`; None when unknown.
    pub armor_coverage: Option<f32>,
    /// The names of the equipment slots that hold an item (`mainhand`, `offhand`, `head`, `chest`, `legs`, `feet`), as
    /// `LivingEntity.getItemBySlot` finds them filled; None when the game did not report its equipment.
    pub equipment: Option<Vec<String>>,
    /// The armor pieces it wears that carry custom data, as (slot, stack) in `feet`, `legs`, `chest`, `head` order, read from
    /// its saved data with its state; empty when the game did not report them or it wears none. This is how a mob's worn mod
    /// armor reaches the damage events: a player's pieces are remembered from their own tick instead.
    pub armor_stacks: Vec<(String, StackData)>,
    /// The ids of the enchantments on its equipment (the hands and the armor together), as
    /// `LivingEntity.getEnchantmentLevel` would find them present; empty when the game reported
    /// none or did not report its equipment's enchantments. Only presence is carried, not levels.
    pub enchantments: Vec<String>,
    /// Whether it is thundering where it stands: the weather the game reports for its level,
    /// taken with `rain_exposed` (a storm over a sheltered entity still thunders).
    pub thundering: bool,
    /// Whether it crouches: a player holding sneak. A mob never does.
    pub crouching: bool,
    /// Its height, the y of its feet; None when the game did not report it.
    pub y: Option<f64>,
    /// The block light and the sky light at the block it stands in, each 0 to 15 as `Level.getBrightness` reports them for the two layers;
    /// None when the game did not report them. The sky light is the raw one: `sky_darken` is what daylight takes off it.
    pub block_light: Option<u8>,
    pub sky_light: Option<u8>,
    /// How much of the sky light the time of day and the weather take off, 0 to 11 (`Level.getSkyDarken`); 0 where there is no weather or day.
    pub sky_darken: u8,
}

impl EntityState {
    /// The level of the effect `id` on it (the amplifier plus one), or 0 when it does not have the effect or the game did not report levels.
    pub fn effect_level(&self, id: &str) -> i32 {
        self.effect_levels.iter().find(|(effect, _)| effect == id).map_or(0, |(_, amplifier)| amplifier.saturating_add(1))
    }
}

/// `Level.getSkyDarken`: how much of the sky light the weather and the time of day take off, 0 (noon, clear) to 11 (midnight, in a thunderstorm).
/// `rain` and `thunder` are the levels from 0.0 to 1.0 and `day_time` the world's time of day in ticks.
pub fn sky_darken(rain: f32, thunder: f32, day_time: i64) -> u8 {
    // `DimensionType.timeOfDay`: the fraction of the day, eased so the sun lingers near the horizon.
    let fraction = (day_time as f64 / 24000.0 - 0.25).rem_euclid(1.0);
    let eased = 0.5 - (fraction * std::f64::consts::PI).cos() / 2.0;
    let time_of_day = ((fraction * 2.0 + eased) / 3.0) as f32;
    let rain_factor = 1.0 - f64::from(rain.clamp(0.0, 1.0) * 5.0) / 16.0;
    let thunder_factor = 1.0 - f64::from(thunder.clamp(0.0, 1.0) * 5.0) / 16.0;
    let brightness = 0.5 + 2.0 * f64::from((time_of_day * (2.0 * std::f32::consts::PI)).cos()).clamp(-0.25, 0.25);
    ((1.0 - brightness * rain_factor * thunder_factor) * 11.0) as u8
}

/// A status effect's category (`MobEffectCategory`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectCategory { Beneficial, Harmful, Neutral }

/// The category of a vanilla status effect id (`minecraft:speed`); None for an id that is not vanilla's. A vanilla effect can be cured with milk.
pub fn effect_category(id: &str) -> Option<EffectCategory> {
    use EffectCategory::*;
    Some(match id.strip_prefix("minecraft:")? {
        "speed" | "haste" | "strength" | "instant_health" | "jump_boost" | "regeneration" | "resistance" | "fire_resistance" | "water_breathing"
        | "invisibility" | "night_vision" | "health_boost" | "absorption" | "saturation" | "luck" | "slow_falling" | "conduit_power"
        | "dolphins_grace" | "hero_of_the_village" => Beneficial,
        "slowness" | "mining_fatigue" | "instant_damage" | "nausea" | "blindness" | "hunger" | "weakness" | "poison" | "wither" | "levitation"
        | "unluck" | "bad_omen" | "darkness" | "trial_omen" | "raid_omen" | "infested" | "oozing" | "weaving" | "wind_charged" => Harmful,
        "glowing" => Neutral,
        _ => return None,
    })
}

/// A status effect for a callback to put on the mob a weapon hit.
#[derive(Clone, Debug, PartialEq)]
pub struct EffectGrant {
    /// The effect id, such as `minecraft:slowness`; an id the game does not know is skipped.
    pub effect: String,
    /// The amplifier, the level less one: 0 is level I.
    pub amplifier: i32,
    /// How long it lasts, in ticks.
    pub duration: i32,
}

/// A player's weapon hit on a mob, heard on a server before the hit's damage lands.
#[derive(Clone, Debug)]
pub struct WeaponHit {
    /// The mob's entity id and type id, and the attacking player's entity id.
    pub entity: i32,
    pub kind: String,
    pub attacker: i32,
    /// The weapon: its damage, max damage and custom data; None for an empty hand.
    pub weapon: Option<StackData>,
    /// What the player and the mob were doing as the hit landed; None when the game could not tell.
    pub attacker_state: Option<EntityState>,
    pub target_state: Option<EntityState>,
    /// Where the mob stands (its feet position), as the game reported it; None when it could not tell.
    pub target_position: Option<[f64; 3]>,
    /// Where the attacking player stands (their feet position), as the game reported it; None when it could not tell.
    pub attacker_position: Option<[f64; 3]>,
    /// The armor the attacking player wore as of their last tick, as (slot, stack): `feet`, `legs`, `chest`, `head`, only the pieces
    /// carrying custom data. Empty when the player is not known (with several players online only the named player's armor is known).
    pub armor: Vec<(String, StackData)>,
    /// The effects to put on the mob, which callbacks add to.
    pub effects: RefCell<Vec<EffectGrant>>,
    /// The actions to do to the player (`Attacker`) or the mob (`Victim`), which callbacks add to; they land before the hit's damage.
    pub actions: RefCell<Vec<EntityAction>>,
}

/// A server player's tick, heard while the player wears armor or holds a tool that carries custom data.
#[derive(Clone, Debug)]
pub struct PlayerTick {
    /// The player's uuid and entity id.
    pub player: u128,
    pub entity: i32,
    /// The dimension the player is in, such as `minecraft:overworld`.
    pub dimension: String,
    /// The player's feet position.
    pub position: [f64; 3],
    /// Whether the player stands on the ground.
    pub on_ground: bool,
    /// Whether the player holds sneak (`Player.isShiftKeyDown`).
    pub sneaking: bool,
    /// Whether the player glides with an elytra (`isFallFlying`).
    pub gliding: bool,
    /// Whether the player spectates (game mode 3).
    pub spectator: bool,
    /// Whether the player stands in water (the block at their feet holds water).
    pub in_water: bool,
    /// The armor the player wears, as (slot, stack): `feet`, `legs`, `chest`, `head`, only the pieces carrying custom data.
    pub armor: Vec<(String, StackData)>,
    /// The stacks the player holds, as (slot, stack): `mainhand`, `offhand`, only the pieces carrying custom data.
    pub hands: Vec<(String, StackData)>,
    /// The light where the player stands, sampled at the block containing their position raised by half a block (the point the
    /// original's walk modules read): the block light, the sky light, and the sky darken the sky light is read against.
    pub block_light: u8,
    pub sky_light: u8,
    pub sky_darken: u8,
    /// The effects on the player, as (effect id, amplifier, ticks left).
    pub effects: Vec<(String, i32, i32)>,
    /// The effects to take off the player, which callbacks add to.
    pub remove_effects: RefCell<Vec<String>>,
    /// The amounts callbacks add to the base value of the player's attributes, as (attribute id, amount), which callbacks push to every tick
    /// for as long as they want them: an amount no callback names any more is taken off again.
    pub attribute_additions: RefCell<Vec<(String, f64)>>,
    /// The actions to do to the ticking player, as `Attacker`, which callbacks add to; they land at the end of the tick. Only a player the host
    /// can name by entity id is reached (the acting player, or a guest).
    pub actions: RefCell<Vec<EntityAction>>,
}

/// A player's melee hit about to be a critical hit, or not (NeoForge: `CriticalHitEvent`).
/// Heard on the game that runs the attack, for a living target only: `critical` starts as
/// vanilla's answer and `damage_multiplier` at 1.5 for a vanilla critical, 1.0 otherwise;
/// a callback may force or deny the critical and change the multiplier, which scales the
/// damage only while `critical` ends true.
#[derive(Clone, Debug)]
pub struct CriticalHit {
    /// The attacking player's entity id, or -1 when the game names no acting player
    /// (a guest's hit on a LAN world, where the attack runs on another game).
    pub attacker_entity: i32,
    /// Whether the attacker stands on the ground.
    pub attacker_on_ground: bool,
    /// Whether the attacker stands in water (the block at their feet holds water).
    pub attacker_in_water: bool,
    /// The entity type id of the target, such as `minecraft:zombie`.
    pub target_kind: String,
    /// Vanilla's answer: whether the hit would be critical without a mod.
    pub vanilla_critical: bool,
    /// Whether the hit is critical now; change it with `set`.
    pub critical: Cell<bool>,
    /// The damage multiplier now; change it with `set`. Only applies while `critical` is true.
    pub damage_multiplier: Cell<f32>,
}

/// A living entity jumped (NeoForge: `LivingEvent.LivingJumpEvent`). Heard on both
/// games; the event carries no host and no velocity, so a callback that needs to move
/// the entity defers the push to the entity's next tick (a player has no set-velocity:
/// the push lands as the set velocity minus the motion the tick measured).
#[derive(Clone, Debug)]
pub struct LivingJump {
    /// The entity's numeric id.
    pub entity: i32,
    /// The entity type id, such as `minecraft:player`.
    pub kind: String,
}

/// The local player's movement input, after the game read it and before it applies it
/// (NeoForge: `MovementInputUpdateEvent`). Client only. A callback may scale the
/// impulses (an item's use slowdown) or read the jump key state (a double jump);
/// `using` is the stack being used, when one is, for the use-item speed its stats
/// carry; it may not reach the player themselves — the entity reach is not open here, so
/// a motion change is deferred to the player's next tick.
#[derive(Clone, Debug)]
pub struct MovementInput {
    /// The local player's entity id, or -1 when the game names no acting player.
    pub entity: i32,
    /// The forward impulse now; change it with `set`.
    pub forward: Cell<f32>,
    /// The strafe impulse now; change it with `set`.
    pub strafe: Cell<f32>,
    /// Whether the jump key is held.
    pub jump: Cell<bool>,
    /// Whether the sneak key is held.
    pub sneak: Cell<bool>,
    /// Whether the player is using an item (eating, drawing a bow, blocking).
    pub using_item: bool,
    /// The stack being used, when the player is using an item: the held (main-hand) stack,
    /// as the game hands it out. None when the player is not using an item, or the game
    /// names no acting player. A use from the off hand has no surface at this hook (the
    /// native hook names no hand, and the use-item tick that does is server-only), so an
    /// off-hand use reads as the main hand's stack.
    pub using: Option<StackData>,
}

/// Damage for a callback to deal to the mob that hurt the player (an armor's thorns).
#[derive(Clone, Debug, PartialEq)]
pub struct CounterDamage {
    /// The damage type id, such as `minecraft:thorns`.
    pub damage_type: String,
    pub amount: f32,
}

/// Which party of a hit an [`EntityAction`] is done to. In a weapon hit the attacker is the player and the victim the mob; in a hit on a
/// player (`WearerHit`) the attacker is the mob and the victim the wearer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionTarget {
    Attacker,
    Victim,
}

/// How a [`EntityAction::Knockback`] picks the way the target is pushed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KnockbackDirection {
    /// Away from the other party of the hit, as the game's own damage knockback pushes.
    AwayFromOther,
    /// Against the way the target faces, as `Entity.knockback(strength, -sin(yaw), cos(yaw))` pushes.
    AgainstFacing,
    /// Along a caller-given horizontal vector, read as `Entity.knockback(strength, x, z)` reads its own
    /// arguments: the target is pushed along the negated, normalized vector (and up by the strength,
    /// capped as the game's own knockback caps it, when it stands on the ground). A projectile's hit
    /// passes its negated motion, so the target is pushed the way the projectile flew.
    Vector { x: f64, z: f64 },
}

/// Something for a callback to do to one party of a hit, which the game does after the callbacks ran (a weapon hit's before its damage
/// lands, a hit on a player's on the player's next tick). Each action is guarded: one the target cannot take (a fire-immune mob, a player
/// for the actions that only reach mobs, an entity the host cannot reach) is skipped silently.
#[derive(Clone, Debug, PartialEq)]
pub enum EntityAction {
    /// `Entity.setSecondsOnFire`: the target burns for `seconds`, when that is longer than it already burns. Skipped for fire-immune
    /// mobs; players are reached only when the host can reach them (the acting player).
    SetFire { target: ActionTarget, seconds: i32 },
    /// `LivingEntity.heal`: gives the target `amount` health, cut to its maximum. Reaches mobs and players.
    Heal { target: ActionTarget, amount: f32 },
    /// Freezes a mob: the ticks frozen become the larger of 140 (the ticks that freeze it) and what it has, plus `ticks`, and its fire goes
    /// out (`Entity.setTicksFrozen` and `clearFire`). Mobs only, and not the kinds the game makes freeze-immune.
    Freeze { target: ActionTarget, ticks: i32 },
    /// `LivingEntity.knockback`: pushes a mob with `strength` along the horizontal plane, and up when it stands on the ground. Mobs only
    /// (the host cannot push a player); the mob's own knockback resistance is not applied.
    Knockback { target: ActionTarget, strength: f64, direction: KnockbackDirection },
    /// `LivingEntity.addEffect`: puts a status effect on the target, a player or a mob. An effect id the game does not know is skipped;
    /// `amplifier` is the level less one and `duration` is in ticks, and an effect with a negative amplifier or no duration is not given.
    AddEffect { target: ActionTarget, effect: String, amplifier: i32, duration: i32 },
    /// Edits the custom data of the stack a player carries in `slot` (`ItemStack.set(DataComponents.CUSTOM_DATA, ..)`), when that stack is
    /// still of `item`. Players only: a mob target, a slot that holds another item and an edit that changes nothing are skipped. Acts on
    /// the stack as it is when the action lands, not as the callback saw it, so wear that happened in between stays.
    EditStackData { target: ActionTarget, slot: StackSlot, item: String, edits: Vec<StackDataEdit> },
    /// `LivingEntity.hurt`: deals the target `amount` damage of `damage_type` with no attacker, as the host's hurt call does (armor, effects,
    /// invulnerability time and the damage hooks all hear it, so a callback hears its own hurt). A mod's own damage type id works once its
    /// data pack declares it (`data/<namespace>/damage_type/<name>.json`). Players and mobs; one the host cannot reach is skipped.
    Hurt { target: ActionTarget, amount: f32, damage_type: String },
    /// `Entity.teleportTo`: moves the target to `position`. A mob's navigation stops and its fall distance resets, as the game's own
    /// teleport does; another entity keeps the way it faces. A player is moved by the host's own player teleport (the `/tp` path:
    /// they leave their vehicle, lose their motion and fall, and their client is told), keeping the way they face.
    Teleport { target: ActionTarget, position: [f64; 3] },
    /// Sets how many ticks an active effect has left on a player, keeping its level and flags: shortens or lengthens it. An effect the target
    /// does not have, a duration that is not positive and a mob target are skipped. The host takes the effect off and gives it back, so
    /// the game's own rules for giving an effect apply.
    SetEffectDuration { target: ActionTarget, effect: String, duration: i32 },
    /// Folds a multiply-total payload into the base value of the target's `attribute`: the base becomes `factor` times what it is,
    /// applied as a ratio against the factor already folded in, so repeating the action with the same factor changes nothing, and a
    /// factor of 1.0 unfolds it. This is how a multiply-total effect payload on a host attribute is carried (a base move cannot
    /// express it): the self-destructing effect's movement speed, 0.1 while primed. Players only; a mob target and a factor that is
    /// not a positive finite number are skipped. An attribute the host does not know is skipped. The fold is kept apart from the
    /// additions on the same attribute (a lightspeed addition landing on a folded base is unfolded with it), and it is unfolded when
    /// the player leaves, after the additions are taken off (they sat on top of the folded base).
    MultiplyAttribute { target: ActionTarget, attribute: String, factor: f64 },
    /// `LivingEntity.spawnAtLocation` / `ModifierUtil.dropItem`: drops `stack` as an item at the target entity, at the
    /// entity's position as the drop lands (the game's own item spawn, with its pickup delay). Reaches any living
    /// entity the game can name, mobs and the ender dragon included; a target that has left the world drops nothing.
    DropItem { target: ActionTarget, stack: StackData },
    /// Spawns an entity of `kind` at the target's position, as the game's own summon does (the kind is
    /// loaded from its data, projectiles and lightning included). A target the host cannot name, or a
    /// kind the game does not know, spawns nothing.
    Summon { target: ActionTarget, kind: String },
}

/// Which stack of a player an [`EntityAction::EditStackData`] edits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StackSlot { MainHand, OffHand, Feet, Legs, Chest, Head }

/// One change to a stack's custom data: `value` is put at `path` (compound keys from the root, the compounds on the way made as needed), or
/// the entry at `path` is removed when `value` is None.
#[derive(Clone, Debug, PartialEq)]
pub struct StackDataEdit {
    pub path: Vec<String>,
    pub value: Option<Nbt>,
}

/// Applies `edits` in order to `data`; true when any of them changed it. An edit with an empty path is skipped; one whose path runs
/// through a value that is not a compound replaces that value with a compound.
pub(crate) fn edit_custom_data(data: &mut NbtCompound, edits: &[StackDataEdit]) -> bool {
    let before = data.clone();
    for edit in edits {
        let Some((leaf, parents)) = edit.path.split_last() else { continue };
        let mut at = &mut *data;
        let mut present = true;
        for key in parents {
            if edit.value.is_none() && !matches!(at.get(key), Some(Nbt::Compound(_))) { present = false; break; }
            let entry = at.entry(key.clone()).or_insert_with(|| Nbt::Compound(NbtCompound::new()));
            if !matches!(entry, Nbt::Compound(_)) { *entry = Nbt::Compound(NbtCompound::new()); }
            let Nbt::Compound(inner) = entry else { unreachable!("made a compound just above") };
            at = inner;
        }
        if !present { continue; }
        match &edit.value { Some(value) => { at.insert(leaf.clone(), value.clone()); } None => { at.remove(leaf); } }
    }
    *data != before
}

/// The number the host calls a player's stack by (`Inventory` slot numbers): the selected hotbar slot for the main hand, 100 (feet) to 103
/// (head) for armor, -106 for the off hand.
pub(crate) fn carried_slot_number(slot: StackSlot, selected: u8) -> i32 {
    match slot {
        StackSlot::MainHand => i32::from(selected.min(8)),
        StackSlot::OffHand => -106,
        StackSlot::Feet => 100,
        StackSlot::Legs => 101,
        StackSlot::Chest => 102,
        StackSlot::Head => 103,
    }
}

/// The slot an armor slot name of `armor` lists (`feet`, `legs`, `chest`, `head`) stands for.
pub fn armor_stack_slot(name: &str) -> Option<StackSlot> {
    Some(match name { "feet" => StackSlot::Feet, "legs" => StackSlot::Legs, "chest" => StackSlot::Chest, "head" => StackSlot::Head, _ => return None })
}

/// One place a player carries a stack, as the wider events that report a player's whole carrying name them: a hand or an armor
/// slot, or a slot of the main inventory (0 to 35, the hotbar first).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CarriedSlot {
    /// A hand or an armor slot.
    Equipment(StackSlot),
    /// A main-inventory slot, 0 to 35.
    Inventory(u32),
}

/// A stack a player carries, with the place it sits in: what an event that enumerates a player's stacks reports for each non-empty
/// place (an item-damage callback sharing wear with the holder's other tools, an ammo callback looking for a quiver).
#[derive(Clone, Debug, PartialEq)]
pub struct CarriedStack {
    /// Where the stack sits.
    pub slot: CarriedSlot,
    /// The stack, with all its components.
    pub stack: ItemSlot,
}

/// A whole new stack for a place a player carries, which a callback queues in an event's `edits`: the host writes it after the
/// callbacks ran (a count of 0 empties the place). Writing a place the event did not list is allowed; the stack lands where it
/// names, if the host still finds the player there.
#[derive(Clone, Debug, PartialEq)]
pub struct CarriedEdit {
    /// The place to write.
    pub slot: CarriedSlot,
    /// The stack to put there.
    pub stack: ItemSlot,
}

/// The ticks a mob that burns for `current` ticks burns for after burning for `seconds` more, or None when that changes nothing (the new
/// fire is not longer, the seconds are not positive, or the target is fire-immune).
pub(crate) fn fire_ticks_after(current: i32, seconds: i32, fire_immune: bool) -> Option<i32> {
    let ticks = seconds.checked_mul(20)?;
    (!fire_immune && seconds > 0 && ticks > current.max(0)).then_some(ticks)
}

/// Whether a mob type can be frozen: the kinds of the game's `freeze_immune_entity_types` tag cannot.
pub(crate) fn can_freeze(kind: &str) -> bool {
    !matches!(kind, "minecraft:stray" | "minecraft:polar_bear" | "minecraft:snow_golem")
}

/// The ticks frozen a mob has after being frozen for `ticks` more, when it had `current`: a mob already frozen keeps its count, one below
/// the 140 ticks that freeze it starts from them. None for ticks that are not positive.
pub(crate) fn frozen_ticks_after(current: i32, ticks: i32) -> Option<i32> {
    (ticks > 0).then(|| current.max(140).saturating_add(ticks))
}

/// What a knockback adds to a mob's velocity, as `LivingEntity.knockback(strength, x, z)` does it: `strength` along the horizontal plane
/// against the direction (x, z), and up by the strength (at most 0.4) when the mob stands on the ground. None for no strength or no direction.
pub(crate) fn knockback_delta(strength: f64, x: f64, z: f64, on_ground: bool) -> Option<[f64; 3]> {
    let length = x.hypot(z);
    if !(strength.is_finite() && strength > 0.0 && length.is_finite() && length >= 1.0e-5) { return None; }
    Some([-x / length * strength, if on_ground { strength.min(0.4) } else { 0.0 }, -z / length * strength])
}

/// The direction (x, z) `Entity.knockback` takes for a push against the way something faces at `yaw` degrees: (-sin, cos).
pub(crate) fn facing_direction(yaw: f32) -> (f64, f64) {
    let angle = f64::from(yaw).to_radians();
    (-angle.sin(), angle.cos())
}

/// The health a living entity has after being healed by `amount`, which the host then cuts to the maximum. None for an amount that is
/// not a positive number.
pub(crate) fn healed_health(current: f32, amount: f32) -> Option<f32> {
    (amount.is_finite() && amount > 0.0 && current.is_finite()).then(|| current + amount)
}

/// A mob hurting the player, heard on a server as the hit starts (before armor wears and the damage lands).
#[derive(Clone, Debug)]
pub struct WearerHit {
    /// The mob's entity id and type id.
    pub attacker: i32,
    pub kind: String,
    /// The damage type id and the amount the game works the hit out as: not reported yet, so an empty id and 0.0 for now (the hit is
    /// heard before the game works either out).
    pub damage_type: String,
    pub amount: f32,
    /// What the mob was doing as the hit landed.
    pub attacker_state: EntityState,
    /// Where the mob stands (its feet position), as the game reported it; None when it could not tell.
    pub attacker_position: Option<[f64; 3]>,
    /// The armor the player wore as of their last tick, as (slot, stack): `feet`, `legs`, `chest`, `head`, only the pieces carrying custom data.
    pub armor: Vec<(String, StackData)>,
    /// The effects to put on the mob, which callbacks add to.
    pub effects: RefCell<Vec<EffectGrant>>,
    /// The effects to put on the wearer, which callbacks add to; they land on the player's next tick.
    pub self_effects: RefCell<Vec<EffectGrant>>,
    /// The damage to deal to the mob, which callbacks add to; it lands on the player's next tick, as hurt by the player.
    pub counter_damage: RefCell<Vec<CounterDamage>>,
    /// The actions to do to the mob (`Attacker`) or the wearer (`Victim`), which callbacks add to; they land on the player's next tick.
    pub actions: RefCell<Vec<EntityAction>>,
}

/// Damage about to be applied, which callbacks may change (NeoForge: `LivingDamageEvent.Pre`).
#[derive(Clone, Debug)]
pub struct DamageModification {
    /// What is known of the damage; its `amount` is the amount as the game works it out and does not change.
    pub damage: LivingDamage,
    /// The amount now, which callbacks may change with `set`; every callback is called in turn, each seeing the last one's.
    pub amount: Cell<f32>,
    /// When a player's attack caused it: the stack the player attacked with (damage, max damage, custom data); None otherwise.
    pub attacker_stack: Option<StackData>,
    /// The armor the damaged entity wore as of its last tick, as (slot, stack): `feet`, `legs`, `chest`, `head`, only the
    /// pieces carrying custom data (a mod's own armor keeps its state there). A player's pieces are remembered from
    /// their tick; another living entity's are the ones the hit's own report read from its saved data, when that
    /// report ran, so a hit no report covered (a mob's hit on a mob, the environment) names none.
    pub armor: Vec<(String, StackData)>,
    /// When a player's weapon hit a mob: what the player and the mob were doing as the hit landed; None otherwise.
    /// A mob's synchronous attack (a melee swing the game names the attacker of before the hit lands) also reports
    /// the mob's state here, from its attack snapshot; the fields the snapshot does not carry (its health, its
    /// equipment by slot) stay None.
    pub attacker_state: Option<EntityState>,
    pub target_state: Option<EntityState>,
    /// The entity type id of the entity that caused the hit, such as `minecraft:zombie`; empty when the game did
    /// not report it. A player's weapon hit reports `minecraft:player`, a mob's synchronous attack the mob's type.
    pub attacker_kind: String,
    /// Whether the entity that caused the hit is a player: `Some(true)` when the game names a player as the cause
    /// (a weapon hit, or a hit whose causing entity carries a player's identity, such as a player's explosion),
    /// `Some(false)` when it names a mob, None when the game did not say.
    pub attacker_is_player: Option<bool>,
    /// The stack in the causing entity's main hand, for an attacker of any kind; None for an empty hand or when
    /// the game did not report it. For a player's attack this is the same stack as `attacker_stack`.
    pub attacker_main_hand: Option<StackData>,
    /// The stack on the causing entity's chest, for an attacker of any kind; None for an empty chest slot or when
    /// the game did not report it. Only a mob's synchronous attack reports it.
    pub attacker_chest: Option<StackData>,
    /// The causing entity's `minecraft:attack_damage` attribute as the game reports it (a mob's, from its attack
    /// snapshot); None when not reported.
    pub attacker_attack_damage: Option<f32>,
    /// The damaged entity's health as the hit arrived, before this hit's damage; None when the game did not
    /// report it (a player victim's health is not reported at this stage).
    pub target_health: Option<f32>,
    /// The damaged entity's `minecraft:attack_damage` attribute as the game reports it (a mob victim's); None
    /// when not reported.
    pub target_attack_damage: Option<f32>,
    /// The actions to do to the damaged player (`Victim`), which callbacks add to; they land on the player's next tick (only for a player
    /// wearing armor with custom data, whose armor is listed in `armor`), and the other party's are dropped. On a player's attack
    /// (`attacker_stack` is set) the actions for `Attacker` land on the attacking player's next tick instead. An action for a
    /// party neither tick can place (a mob attacker, a mob victim) is kept for that entity's next tick, and a
    /// [`EntityAction::DropItem`] is handed to the game to spawn at its target entity.
    pub actions: RefCell<Vec<EntityAction>>,
}

/// Knockback about to be applied to a living entity, which callbacks may change or cancel (NeoForge: `LivingKnockBackEvent`).
#[derive(Clone, Debug)]
pub struct KnockbackModification {
    pub entity: i32,
    pub kind: String,
    /// The entity that caused it, or a negative id if none.
    pub attacker: i32,
    /// The stack in the attacker's main hand, with its damage, max damage and custom data; None for an empty hand or no attacker.
    pub attacker_main: Option<StackData>,
    /// The stack in the attacker's off hand; None when empty.
    pub attacker_off: Option<StackData>,
    /// The strength now, which callbacks may change with `set`; each callback sees the last one's.
    pub strength: Cell<f64>,
    /// The direction the target is pushed away along, on the horizontal plane.
    pub x: Cell<f64>,
    pub z: Cell<f64>,
    /// Set to true for no knockback at all.
    pub cancel: Cell<bool>,
}

/// A player's item about to lose durability, which callbacks may change or cancel (NeoForge has no event for it; the vanilla
/// source is `ItemStack.hurtAndBreak`). It is heard on a server, once for each wear, after Unbreaking has had its say, and not for
/// a creative player's tools.
#[derive(Clone, Debug)]
pub struct ItemDamage {
    /// The player's uuid and name.
    pub player: u128,
    pub name: String,
    /// The item id of the stack worn.
    pub item: String,
    /// The stack as the player holds or wears it (main hand first, then the armor slots), found by its item id; None when none matches.
    pub stack: Option<StackData>,
    /// The wear now, which callbacks may change with `set` (0 or less leaves the item whole); each callback sees the last one's.
    pub amount: Cell<i32>,
    /// Set to true for no wear at all.
    pub cancel: Cell<bool>,
    /// The worn stack as an item slot, with its components (tool, attribute modifiers, enchantments); None when `stack` is.
    pub slot: Option<ItemSlot>,
    /// A whole new stack for the worn one, which a callback may set; it is written to the main-inventory stack that carried the data
    /// in `stack`, and then the game wears it by 0. When the stack is not in the main inventory (a worn armor piece) it is not
    /// written and the game wears the stack by `amount`, so a callback that sets `replacement` also sets `amount` to the wear left.
    pub replacement: RefCell<Option<ItemSlot>>,
    /// What the player was doing as the stack wore (health, surroundings, the level of their effects); None when the game did not name them.
    pub holder: Option<EntityState>,
    /// The place the worn stack sits in, when the host found it in a hand or an armor slot; None when it was only matched in the
    /// main inventory (or not found at all).
    pub worn: Option<CarriedSlot>,
    /// Every other stack the holder carries, each with its place: the hands and armor the host could read and the main inventory's
    /// non-empty slots. The worn stack itself is listed too, at `worn`'s place when that is known; a callback sharing the wear
    /// (or a shield) with the holder's other tools picks from this list and skips that place. Empty when the host could read none.
    pub carried: Vec<CarriedStack>,
    /// Whole new stacks for places the holder carries, which callbacks add to: the host writes them after the callbacks ran, so a
    /// callback can hand a bonus (a shield's charge, kept in the stack's custom data) to one of the `carried` stacks. A count of 0
    /// empties the place.
    pub edits: RefCell<Vec<CarriedEdit>>,
}

/// How far a mob sees a player, about to be scaled for the kind of mob that is looking (NeoForge: `LivingVisibilityEvent`). It is
/// heard on a server each time a mob works out its range for a player who wears armor carrying custom data (a mod's own armor keeps its
/// state there); the game's own factors (sneaking, invisibility, worn heads) are already in the range and a callback only scales it
/// further. Nothing is heard for a player wearing no such armor.
#[derive(Clone, Debug)]
pub struct VisibilityModification {
    /// The wearer's uuid.
    pub wearer: u128,
    /// The type id of the mob that is looking, such as `minecraft:skeleton`.
    pub observer: String,
    /// What the wearer wore as of their last tick, as (slot, stack) for `feet`, `legs`, `chest` and `head` (only pieces carrying custom data).
    pub armor: Vec<(String, StackData)>,
    /// Changes whenever what any player wears changes, so a callback may keep what it worked out from `armor` for as long as it stays
    /// the same. It is never 0.
    pub armor_revision: u64,
    /// The factor on the mob's range now (a `Cell`: change it with `set`; 1 leaves the range whole, 0.5 halves it, 0 hides the wearer);
    /// each callback sees the last one's. NeoForge's `modifyVisibility` multiplies, so a callback usually sets it to its product with the
    /// factor it wants.
    pub multiplier: Cell<f64>,
}

static MODIFY_VISIBILITY: Event<VisibilityModification, ()> = Event::void();
static SHEAR: Event<ShearAttempt, ()> = Event::void();
static PLAYER_TICK: Event<PlayerTick, ()> = Event::void();
static WEAPON_HIT: Event<WeaponHit, ()> = Event::void();
static WEARER_HIT: Event<WearerHit, ()> = Event::void();
static MODIFY_KNOCKBACK: Event<KnockbackModification, ()> = Event::void();
static MODIFY_ITEM_DAMAGE: Event<ItemDamage, ()> = Event::void();
static MODIFY_DAMAGE: Event<DamageModification, ()> = Event::void();
static MODIFY_HURT: Event<DamageModification, ()> = Event::void();
static ALLOW_DAMAGE: Event<LivingDamage, bool> = Event::allow_all();
static AFTER_DEATH: Event<LivingDamage, ()> = Event::void();
static CRITICAL_HIT: Event<CriticalHit, ()> = Event::void();
static LIVING_JUMP: Event<LivingJump, ()> = Event::void();
static MOVEMENT_INPUT: Event<MovementInput, ()> = Event::void();

/// Fabric: `ServerLivingEntityEvents`.
pub struct ServerLivingEntityEvents;

impl ServerLivingEntityEvents {
    /// Fabric: `ALLOW_DAMAGE`: return `false` to cancel the damage.
    pub const ALLOW_DAMAGE: &'static Event<LivingDamage, bool> = &ALLOW_DAMAGE;
    /// NeoForge: `LivingDamageEvent.Pre`. Called for damage that goes ahead, before armor and effects take their share; change the amount
    /// with `amount.set`. For a player's attack on the server it carries the stack used.
    pub const MODIFY_DAMAGE: &'static Event<DamageModification, ()> = &MODIFY_DAMAGE;
    /// NeoForge: `LivingIncomingDamageEvent`. Called as damage that goes ahead arrives, before armor and enchantment protection
    /// take their share and before `MODIFY_DAMAGE`: the amount as it arrives, which callbacks may change with `amount.set`, and
    /// the same armor, states, attacking stack and action queueing as the damage event that follows in the same call.
    pub const MODIFY_HURT: &'static Event<DamageModification, ()> = &MODIFY_HURT;
    /// Fabric: `AFTER_DEATH`. Called when a living entity's death goes ahead, just before it is removed. Players are not reported yet.
    pub const AFTER_DEATH: &'static Event<LivingDamage, ()> = &AFTER_DEATH;
    /// A server player's tick, heard while they wear armor or hold a tool carrying custom data: take effects off them with `remove_effects`.
    /// The world scope is open while it runs, so callbacks may read and change blocks around the player.
    pub const PLAYER_TICK: &'static Event<PlayerTick, ()> = &PLAYER_TICK;
    /// A mob hurting the player, as the hit starts: push status effects on the mob with `effects`. Server only.
    pub const WEARER_HIT: &'static Event<WearerHit, ()> = &WEARER_HIT;
    /// A player's weapon hit on a mob, before its damage lands: push status effects on the mob with `effects`. Server only.
    pub const WEAPON_HIT: &'static Event<WeaponHit, ()> = &WEAPON_HIT;
    /// NeoForge: `LivingKnockBackEvent`. Change the strength or direction with `set`, or set `cancel`. Server only.
    pub const MODIFY_KNOCKBACK: &'static Event<KnockbackModification, ()> = &MODIFY_KNOCKBACK;
    /// A player's item losing durability: change the amount with `amount.set` or set `cancel`. Server only.
    pub const MODIFY_ITEM_DAMAGE: &'static Event<ItemDamage, ()> = &MODIFY_ITEM_DAMAGE;
    /// NeoForge: `LivingVisibilityEvent`. How far a mob sees a player in armor carrying custom data: scale it with `multiplier.set`. Server only.
    pub const MODIFY_VISIBILITY: &'static Event<VisibilityModification, ()> = &MODIFY_VISIBILITY;
    /// NeoForge: `CriticalHitEvent`. A player's melee hit on a living target: force or deny the critical with `critical.set` and change the
    /// damage multiplier with `damage_multiplier.set`. The attacker is the acting player, or -1 when the game names none (a guest's hit).
    pub const CRITICAL_HIT: &'static Event<CriticalHit, ()> = &CRITICAL_HIT;
    /// NeoForge: `LivingEvent.LivingJumpEvent`. A living entity jumped: a callback that needs to move it defers the push to its next tick.
    pub const LIVING_JUMP: &'static Event<LivingJump, ()> = &LIVING_JUMP;
    /// NeoForge: `MovementInputUpdateEvent`. The local player's movement input: scale the impulses with `forward.set` / `strafe.set`, or
    /// read the jump key state. Client only; the entity reach is not open here, so a motion change is deferred to the next tick.
    pub const MOVEMENT_INPUT: &'static Event<MovementInput, ()> = &MOVEMENT_INPUT;
}

/// The effects the callbacks take off the player on their tick.
pub(crate) fn player_tick(tick: &PlayerTick) -> Vec<String> {
    PLAYER_TICK.invoke(tick);
    tick.remove_effects.borrow_mut().drain(..).collect()
}

/// The actions the callbacks queued on the player's tick, taken after `player_tick`.
pub(crate) fn take_tick_actions(tick: &PlayerTick) -> Vec<EntityAction> {
    tick.actions.borrow_mut().drain(..).collect()
}

/// A player's melee hit about to be critical, or not: what the callbacks leave of `critical` and `damage_multiplier` is what the game goes on with.
pub(crate) fn critical_hit(hit: &CriticalHit) {
    CRITICAL_HIT.invoke(hit);
}

/// A living entity jumped.
pub(crate) fn living_jump(jump: &LivingJump) {
    LIVING_JUMP.invoke(jump);
}

/// The local player's movement input, after the game read it and before it applies it.
pub(crate) fn movement_input(input: &MovementInput) {
    MOVEMENT_INPUT.invoke(input);
}

static ATTRIBUTES: std::sync::Mutex<std::collections::BTreeMap<(u128, String), f64>> = std::sync::Mutex::new(std::collections::BTreeMap::new());
static TICKING: std::sync::Mutex<std::collections::BTreeSet<u128>> = std::sync::Mutex::new(std::collections::BTreeSet::new());

/// Whether the player's tick is heard: while they wear armor or hold a tool carrying custom data, and for one tick after the last
/// such piece goes away, so that what the callbacks kept for the equipment (an attribute amount) is let go.
pub(crate) fn tick_wanted(player: u128, equipped: bool) -> bool {
    let Ok(mut ticking) = TICKING.lock() else { return equipped };
    if equipped { ticking.insert(player); true } else { ticking.remove(&player) }
}

/// The changes to a player's attribute bases for the amounts `wanted` (summed per attribute): (attribute, change, the total then applied).
/// Nothing is recorded until `attribute_applied` is told the host took the change.
pub(crate) fn attribute_changes(player: u128, wanted: &[(String, f64)]) -> Vec<(String, f64, f64)> {
    let mut totals: std::collections::BTreeMap<&str, f64> = std::collections::BTreeMap::new();
    for (attribute, amount) in wanted { *totals.entry(attribute).or_insert(0.0) += amount; }
    let Ok(applied) = ATTRIBUTES.lock() else { return Vec::new() };
    let mut changes = Vec::new();
    for (attribute, total) in &totals {
        let held = applied.get(&(player, (*attribute).to_owned())).copied().unwrap_or(0.0);
        if *total != held { changes.push(((*attribute).to_owned(), total - held, *total)); }
    }
    for ((who, attribute), held) in applied.iter() {
        if *who == player && *held != 0.0 && !totals.contains_key(attribute.as_str()) { changes.push((attribute.clone(), -held, 0.0)); }
    }
    changes
}

/// Records that the host took a change: the player's total for the attribute is now `total`.
pub(crate) fn attribute_applied(player: u128, attribute: &str, total: f64) {
    if let Ok(mut applied) = ATTRIBUTES.lock() {
        if total == 0.0 { applied.remove(&(player, attribute.to_owned())); } else { applied.insert((player, attribute.to_owned()), total); }
    }
}

/// The total a player's attribute base currently holds from the additions, or 0.0 when none: what a fold must work around, so that an
/// addition landing on a folded base is unfolded with it instead of being multiplied by the unfold.
pub(crate) fn attribute_applied_total(player: u128, attribute: &str) -> f64 {
    ATTRIBUTES.lock().ok().and_then(|applied| applied.get(&(player, attribute.to_owned())).copied()).unwrap_or(0.0)
}

/// What a player who leaves holds on their attributes (to take off), forgotten.
pub(crate) fn forget_attributes(player: u128) -> Vec<(String, f64)> {
    if let Ok(mut ticking) = TICKING.lock() { ticking.remove(&player); }
    let Ok(mut applied) = ATTRIBUTES.lock() else { return Vec::new() };
    let held: Vec<(String, f64)> = applied.iter().filter(|((who, _), _)| *who == player).map(|((_, attribute), held)| (attribute.clone(), *held)).collect();
    applied.retain(|(who, _), _| *who != player);
    held
}

static MULTIPLY: std::sync::Mutex<std::collections::BTreeMap<(u128, String), f64>> = std::sync::Mutex::new(std::collections::BTreeMap::new());

/// The ratios to fold a player's attribute bases by for the multiply-total factors `wanted` (one per attribute): (attribute, ratio, the
/// total factor then folded in). The ratio is the wanted factor over the factor already folded in (1.0 when nothing is), so repeating the
/// action with the same factor changes nothing. Nothing is recorded until `multiply_factor_applied` is told the host took the ratio.
pub(crate) fn multiply_factor_changes(player: u128, wanted: &[(String, f64)]) -> Vec<(String, f64, f64)> {
    let mut totals: std::collections::BTreeMap<&str, f64> = std::collections::BTreeMap::new();
    for (attribute, factor) in wanted { *totals.entry(attribute).or_insert(1.0) *= factor; }
    let Ok(applied) = MULTIPLY.lock() else { return Vec::new() };
    let mut changes = Vec::new();
    for (attribute, total) in &totals {
        let held = applied.get(&(player, (*attribute).to_owned())).copied().unwrap_or(1.0);
        if *total != held {
            let ratio = if held == 0.0 { *total } else { total / held };
            changes.push(((*attribute).to_owned(), ratio, *total));
        }
    }
    for ((who, attribute), held) in applied.iter() {
        if *who == player && *held != 1.0 && !totals.contains_key(attribute.as_str()) { changes.push((attribute.clone(), 1.0 / held, 1.0)); }
    }
    changes
}

/// Records that the host took a fold: the player's total factor for the attribute is now `total`.
pub(crate) fn multiply_factor_applied(player: u128, attribute: &str, total: f64) {
    if let Ok(mut applied) = MULTIPLY.lock() {
        if total == 1.0 { applied.remove(&(player, attribute.to_owned())); } else { applied.insert((player, attribute.to_owned()), total); }
    }
}

/// What a player who leaves holds folded into their attributes (to unfold), forgotten.
pub(crate) fn forget_multiply_factors(player: u128) -> Vec<(String, f64)> {
    let Ok(mut applied) = MULTIPLY.lock() else { return Vec::new() };
    let held: Vec<(String, f64)> = applied.iter().filter(|((who, _), _)| *who == player).map(|((_, attribute), held)| (attribute.clone(), *held)).collect();
    applied.retain(|(who, _), _| *who != player);
    held
}

/// The effects the callbacks put on a mob that hurt the player (`wearer`, the entity id of the player the armor is attributed to); the ones for
/// the wearer and the damage for the mob are queued for that player's next tick, and dropped when no wearer is known.
pub(crate) fn wearer_hit(hit: &WearerHit, wearer: Option<i32>) -> Vec<EffectGrant> {
    WEARER_HIT.invoke(hit);
    let own: Vec<(i32, EffectGrant)> = hit.self_effects.borrow_mut().drain(..).filter_map(|grant| wearer.map(|wearer| (wearer, grant))).collect();
    if !own.is_empty() { if let Ok(mut queue) = SELF_EFFECTS.lock() { queue.extend(own); } }
    let damage: Vec<(i32, i32, CounterDamage)> = hit.counter_damage.borrow_mut().drain(..).filter_map(|damage| wearer.map(|wearer| (wearer, hit.attacker, damage))).collect();
    if !damage.is_empty() { if let Ok(mut queue) = COUNTER_DAMAGE.lock() { queue.extend(damage); } }
    let actions: Vec<(i32, i32, EntityAction)> = hit.actions.borrow_mut().drain(..).filter_map(|action| wearer.map(|wearer| (wearer, hit.attacker, action))).collect();
    if !actions.is_empty() { if let Ok(mut queue) = ENTITY_ACTIONS.lock() { queue.extend(actions); } }
    hit.effects.borrow_mut().drain(..).collect()
}

/// The actions for the parties of hits on wearers, with the wearer's entity id and the mob's.
static ENTITY_ACTIONS: std::sync::Mutex<Vec<(i32, i32, EntityAction)>> = std::sync::Mutex::new(Vec::new());

/// Takes the actions queued for the hits on the wearer `wearer`, as (mob, action); other players' stay queued.
pub(crate) fn take_wearer_actions(wearer: i32) -> Vec<(i32, EntityAction)> {
    let Ok(mut queue) = ENTITY_ACTIONS.lock() else { return Vec::new() };
    let (mine, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut *queue).into_iter().partition(|(owner, _, _)| *owner == wearer);
    *queue = rest;
    mine.into_iter().map(|(_, mob, action)| (mob, action)).collect()
}

/// The effects for a wearer that hits have queued, with the wearer's entity id.
static SELF_EFFECTS: std::sync::Mutex<Vec<(i32, EffectGrant)>> = std::sync::Mutex::new(Vec::new());

/// The damage for the mobs that hits have queued, with the wearer's entity id and the mob's.
static COUNTER_DAMAGE: std::sync::Mutex<Vec<(i32, i32, CounterDamage)>> = std::sync::Mutex::new(Vec::new());

/// Takes the damage queued for the mobs that hurt the wearer `wearer`, as (mob, damage); other players' stay queued.
pub(crate) fn take_counter_damage(wearer: i32) -> Vec<(i32, CounterDamage)> {
    let Ok(mut queue) = COUNTER_DAMAGE.lock() else { return Vec::new() };
    let (mine, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut *queue).into_iter().partition(|(owner, _, _)| *owner == wearer);
    *queue = rest;
    mine.into_iter().map(|(_, mob, damage)| (mob, damage)).collect()
}

/// Takes the effects queued for the wearer `wearer`; other players' stay queued.
pub(crate) fn take_self_effects(wearer: i32) -> Vec<EffectGrant> {
    let Ok(mut queue) = SELF_EFFECTS.lock() else { return Vec::new() };
    let (mine, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut *queue).into_iter().partition(|(owner, _)| *owner == wearer);
    *queue = rest;
    mine.into_iter().map(|(_, grant)| grant).collect()
}

/// The entity id of the only player wearing custom-data armor, which a hit on a player is attributed to.
pub(crate) fn only_wearer_entity() -> Option<i32> {
    ARMOR.lock().ok().and_then(|worn| if worn.len() == 1 { worn.keys().next().copied() } else { None })
}

/// What each player wore, by entity id, for a hit on them: the armor cached by `remember_armor`, for the entity the game names.
pub(crate) fn armor_of_the_only_wearer() -> Vec<(String, StackData)> {
    ARMOR.lock().ok().and_then(|worn| if worn.len() == 1 { worn.values().next().map(|(_, pieces)| pieces.clone()) } else { None }).unwrap_or_default()
}

/// The effects the callbacks put on the mob a weapon hit.
pub(crate) fn weapon_hit(hit: &WeaponHit) -> Vec<EffectGrant> {
    WEAPON_HIT.invoke(hit);
    hit.effects.borrow_mut().drain(..).collect()
}

/// What the callbacks decided about a shearing: whether it goes ahead, and the actions queued for the player.
pub(crate) fn shear(attempt: &ShearAttempt) -> (bool, Vec<EntityAction>) {
    SHEAR.invoke(attempt);
    (!attempt.cancel.get(), attempt.actions.borrow_mut().drain(..).collect())
}

/// The actions the callbacks queued on a weapon hit, taken after `weapon_hit`.
pub(crate) fn take_weapon_actions(hit: &WeaponHit) -> Vec<EntityAction> {
    hit.actions.borrow_mut().drain(..).collect()
}

/// The knockback after the callbacks as (strength, x, z), or None when a callback cancelled it.
pub(crate) fn modify_knockback(modification: &KnockbackModification) -> Option<(f64, f64, f64)> {
    MODIFY_KNOCKBACK.invoke(modification);
    if modification.cancel.get() { None } else { Some((modification.strength.get(), modification.x.get(), modification.z.get())) }
}

/// The wear after the callbacks, with the replacement stack a callback set and the edits to the holder's other stacks, or None
/// when a callback cancelled it (the edits are then dropped with the wear).
pub(crate) fn modify_item_damage(wear: &ItemDamage) -> Option<(i32, Option<ItemSlot>, Vec<CarriedEdit>)> {
    MODIFY_ITEM_DAMAGE.invoke(wear);
    if wear.cancel.get() { None } else { Some((wear.amount.get(), wear.replacement.borrow_mut().take(), wear.edits.borrow_mut().drain(..).collect())) }
}

pub(crate) fn entity_loaded(id: u32, kind: &str, dimension: &str) {
    ENTITY_LOAD.invoke(&EntityInfo { id, kind: kind.to_owned(), dimension: dimension.to_owned() });
}

pub(crate) fn entity_unloaded(id: u32, kind: &str, dimension: &str) {
    ENTITY_UNLOAD.invoke(&EntityInfo { id, kind: kind.to_owned(), dimension: dimension.to_owned() });
}

/// True when the entity's tick may run (no `ENTITY_TICK_PRE` callback cancelled it).
pub(crate) fn entity_tick_before(info: &EntityInfo) -> bool {
    ENTITY_TICK_PRE.invoke(info)
}

/// Reports that the entity's tick ran.
pub(crate) fn entity_tick_after(info: &EntityInfo) {
    ENTITY_TICK_POST.invoke(info);
}

/// Whether any callback hears entity ticks, so the host adapter skips its work when none does.
pub(crate) fn entity_tick_heard() -> bool {
    !ENTITY_TICK_PRE.is_empty() || !ENTITY_TICK_POST.is_empty()
}

/// What the game names of the entity behind a synchronous attack (a mob's melee swing) as the attack is
/// delivered: its type, what it holds and wears, its attack attribute and the state the hit hooks read. The
/// game names the attacker before the hit lands (its knockback scope), so the facts belong to the attack in
/// flight; a damage event reads them matched by the attacker's entity id, and only briefly.
#[derive(Clone, Debug)]
pub(crate) struct AttackerFacts {
    pub kind: String,
    pub main_hand: Option<StackData>,
    pub chest: Option<StackData>,
    pub attack_damage: f32,
    pub state: EntityState,
}

/// What the game names of a damaged entity as a hit arrives: its health before the hit, and its attack
/// attribute (a mob's; the ender dragon's is not named, and a player's health is not named at this stage).
#[derive(Clone, Copy, Debug)]
pub(crate) struct VictimFacts {
    pub health: f32,
    pub attack_damage: Option<f32>,
}

thread_local! {
    static ATTACKER_FACTS: RefCell<Option<(i32, AttackerFacts, std::time::Instant)>> = const { RefCell::new(None) };
    static VICTIM_FACTS: RefCell<Option<(i32, VictimFacts, std::time::Instant)>> = const { RefCell::new(None) };
    static DAMAGE_PARTIES: RefCell<Option<(i32, i32, bool, std::time::Instant)>> = const { RefCell::new(None) };
}

/// How long a named fact of a hit stays usable: the damage events follow in the same call, so this only has to
/// cover the calls the game makes between naming a fact and the events that read it.
const DAMAGE_FACTS_LIFETIME: std::time::Duration = std::time::Duration::from_millis(100);

/// Names the entity behind the synchronous attack being delivered: its type, the stacks in its main hand and
/// on its chest, its attack attribute and its state. The transport calls this as the game opens the attack;
/// the damage events of that attack read the facts.
pub(crate) fn note_attacker_facts(entity: i32, facts: AttackerFacts) {
    ATTACKER_FACTS.with(|pending| *pending.borrow_mut() = Some((entity, facts, std::time::Instant::now())));
}

/// Names the health (and attack attribute, for a mob) of the entity a hit is about to land on. The transport
/// calls this as the game opens the hit, and again for the ender dragon as its damage is worked out.
pub(crate) fn note_victim_facts(entity: i32, facts: VictimFacts) {
    VICTIM_FACTS.with(|pending| *pending.borrow_mut() = Some((entity, facts, std::time::Instant::now())));
}

/// Names the parties of a hit as the game works them out (its incoming-damage report, which runs inside the
/// hit, after the hurt stage and before the damage stage): the victim, the causing entity and whether that
/// entity is a player.
pub(crate) fn note_damage_parties(victim: i32, attacker: i32, attacker_is_player: bool) {
    DAMAGE_PARTIES.with(|pending| *pending.borrow_mut() = Some((victim, attacker, attacker_is_player, std::time::Instant::now())));
}

fn attacker_facts(entity: i32) -> Option<AttackerFacts> {
    ATTACKER_FACTS.with(|pending| match &*pending.borrow() {
        Some((who, facts, at)) if *who == entity && at.elapsed() < DAMAGE_FACTS_LIFETIME => Some(facts.clone()),
        _ => None,
    })
}

fn victim_facts(entity: i32) -> Option<VictimFacts> {
    VICTIM_FACTS.with(|pending| match &*pending.borrow() {
        Some((who, facts, at)) if *who == entity && at.elapsed() < DAMAGE_FACTS_LIFETIME => Some(*facts),
        _ => None,
    })
}

/// Fills a damage event's report of the causing entity and of the damaged entity's health from what the game
/// named of this hit: the attack snapshot, the parties report and the victim snapshot, over the states the
/// weapon-hit report brought (those stand where they exist).
fn fill_damage_facts(modification: &mut DamageModification) {
    let damage = modification.damage.clone();
    if let Some(facts) = attacker_facts(damage.attacker) {
        modification.attacker_kind = facts.kind;
        modification.attacker_main_hand = facts.main_hand;
        modification.attacker_chest = facts.chest;
        modification.attacker_attack_damage = Some(facts.attack_damage);
        if modification.attacker_state.is_none() { modification.attacker_state = Some(facts.state); }
        modification.attacker_is_player = Some(false);
    }
    if modification.attacker_main_hand.is_none() { modification.attacker_main_hand = modification.attacker_stack.clone(); }
    if modification.attacker_stack.is_some() { modification.attacker_is_player = Some(true); }
    // The parties report runs later in the hit than the hurt stage, so the damage stage reads it and the hurt
    // stage keeps what the snapshot said; a report names this hit by both of its entities.
    DAMAGE_PARTIES.with(|pending| {
        if let Some((victim, attacker, is_player, at)) = &*pending.borrow() {
            if *victim == damage.entity && *attacker == damage.attacker && at.elapsed() < DAMAGE_FACTS_LIFETIME {
                modification.attacker_is_player = Some(*is_player);
            }
        }
    });
    if modification.attacker_kind.is_empty() {
        if let Some(state) = &modification.attacker_state { modification.attacker_kind = state.kind.clone(); }
    }
    modification.target_health = modification.target_state.as_ref().and_then(|state| state.health)
        .or_else(|| victim_facts(damage.entity).map(|facts| facts.health));
    modification.target_attack_damage = victim_facts(damage.entity).and_then(|facts| facts.attack_damage);
}

/// True when the damage may be applied.
pub(crate) fn after_death(death: &LivingDamage) {
    AFTER_DEATH.invoke(death);
}

thread_local! {
    static PENDING_ATTACK: RefCell<Option<(i32, StackData)>> = const { RefCell::new(None) };
    static PENDING_STATES: RefCell<Option<(i32, Option<EntityState>, Option<EntityState>, std::time::Instant)>> = const { RefCell::new(None) };
}

/// How long the states of a hit stay good: the damage event follows in the same call, so a few milliseconds is plenty, and a longer gap
/// means the hit never reached it (cancelled, absorbed) and the states belong to something that is over.
const HIT_STATES_LIFETIME: std::time::Duration = std::time::Duration::from_millis(100);

/// A hit is landing on `entity`: what the attacker and the target were doing, remembered for the damage that follows in the same call.
/// A hit is reported more than once (the mob's hit and the damage event both describe it), and one report may know a side's
/// equipment where another does not: the reports of one hit complement each other there, and the rest of the newest report stands.
pub(crate) fn hit_states(entity: i32, attacker: Option<EntityState>, target: Option<EntityState>) {
    PENDING_STATES.with(|pending| {
        let mut pending = pending.borrow_mut();
        if let Some((old_entity, old_attacker, old_target, at)) = &*pending {
            if *old_entity == entity && at.elapsed() < HIT_STATES_LIFETIME {
                let keep_equipment = |new: &mut Option<EntityState>, old: &Option<EntityState>| {
                    if let (Some(new), Some(old)) = (new, old) {
                        if new.equipment.is_none() { new.equipment = old.equipment.clone(); }
                        if new.armor_stacks.is_empty() { new.armor_stacks = old.armor_stacks.clone(); }
                    }
                };
                let (mut attacker, mut target) = (attacker, target);
                keep_equipment(&mut attacker, old_attacker);
                keep_equipment(&mut target, old_target);
                *pending = Some((entity, attacker, target, std::time::Instant::now()));
                return;
            }
        }
        *pending = Some((entity, attacker, target, std::time::Instant::now()));
    });
}

/// The state of each player wearing armor that carries custom data, as of their last tick, by entity id: the damage event that
/// follows a hit on them reports it as `target_state` when the hit brought no state of its own.
static WEARER_STATES: std::sync::Mutex<std::collections::BTreeMap<i32, EntityState>> = std::sync::Mutex::new(std::collections::BTreeMap::new());

/// Records the state of the player `entity` as of this tick; None forgets it.
pub(crate) fn remember_wearer_state(entity: i32, state: Option<EntityState>) {
    let Ok(mut states) = WEARER_STATES.lock() else { return };
    match state { Some(state) => { states.insert(entity, state); } None => { states.remove(&entity); } }
}

fn wearer_state(entity: i32) -> Option<EntityState> { WEARER_STATES.lock().ok()?.get(&entity).cloned() }

/// What the vanilla biomes are like: (id without `minecraft:`, base temperature, whether it has precipitation). A data pack's biome is not here.
const BIOME_CLIMATES: &[(&str, f32, bool)] = &[
    ("badlands", 2.0, false), ("eroded_badlands", 2.0, false), ("wooded_badlands", 2.0, false), ("desert", 2.0, false), ("savanna", 2.0, false),
    ("savanna_plateau", 1.0, false), ("windswept_savanna", 1.1, false),
    ("bamboo_jungle", 0.95, true), ("jungle", 0.95, true), ("sparse_jungle", 0.95, true),
    ("beach", 0.8, true), ("snowy_beach", 0.05, true), ("stony_shore", 0.2, true),
    ("birch_forest", 0.6, true), ("old_growth_birch_forest", 0.6, true), ("dark_forest", 0.7, true), ("forest", 0.7, true), ("flower_forest", 0.7, true),
    ("pale_garden", 0.7, true), ("cherry_grove", 0.5, true), ("meadow", 0.5, true),
    ("cold_ocean", 0.5, true), ("deep_cold_ocean", 0.5, true), ("deep_ocean", 0.5, true), ("ocean", 0.5, true), ("lukewarm_ocean", 0.5, true),
    ("deep_lukewarm_ocean", 0.5, true), ("warm_ocean", 0.5, true), ("deep_frozen_ocean", 0.5, true), ("frozen_ocean", 0.0, true),
    ("dripstone_caves", 0.8, true), ("lush_caves", 0.5, true), ("deep_dark", 0.8, true),
    ("end_barrens", 0.5, false), ("end_highlands", 0.5, false), ("end_midlands", 0.5, false), ("small_end_islands", 0.5, false), ("the_end", 0.5, false), ("the_void", 0.5, false),
    ("nether_wastes", 2.0, false), ("crimson_forest", 2.0, false), ("warped_forest", 2.0, false), ("soul_sand_valley", 2.0, false), ("basalt_deltas", 2.0, false),
    ("frozen_peaks", -0.7, true), ("grove", -0.2, true), ("jagged_peaks", -0.7, true), ("snowy_slopes", -0.3, true), ("stony_peaks", 1.0, true),
    ("frozen_river", 0.0, true), ("river", 0.5, true), ("swamp", 0.8, true), ("mangrove_swamp", 0.8, true),
    ("ice_spikes", 0.0, true), ("snowy_plains", 0.0, true), ("snowy_taiga", -0.5, true), ("taiga", 0.25, true), ("old_growth_pine_taiga", 0.3, true),
    ("old_growth_spruce_taiga", 0.25, true), ("mushroom_fields", 0.9, true), ("plains", 0.8, true), ("sunflower_plains", 0.8, true),
    ("windswept_forest", 0.2, true), ("windswept_gravelly_hills", 0.2, true), ("windswept_hills", 0.2, true),
];

/// The base temperature and whether it has precipitation, for a vanilla biome id (`minecraft:plains`); None for any other.
pub(crate) fn biome_climate(biome: &str) -> Option<(f32, bool)> {
    let name = biome.strip_prefix("minecraft:")?;
    BIOME_CLIMATES.iter().find(|(id, _, _)| *id == name).map(|(_, temperature, rain)| (*temperature, *rain))
}

/// `Biome.getTemperature` at height `y`: above y 80 the air cools by 0.05 per 40 blocks. The game adds a noise term of at most about 0.01
/// to that (and a noise to frozen biomes), which is left out.
pub(crate) fn temperature_at(base: f32, y: i32) -> f32 {
    if y > 80 { base - (y - 80) as f32 * 0.05 / 40.0 } else { base }
}

/// Whether rain falls on a spot: it is raining, the sky light there is full (nothing above it) and the biome rains; a biome PlutoLib
/// does not know is taken to rain, and one that is cooler than 0.15 at that height snows instead.
pub(crate) fn rain_exposed(raining: bool, sky_light: u8, biome: Option<&str>, y: i32) -> bool {
    if !raining || sky_light < 15 { return false; }
    match biome.and_then(biome_climate) {
        Some((temperature, precipitation)) => precipitation && temperature_at(temperature, y) >= 0.15,
        None => true,
    }
}

/// Whether the block `id` holds water in its own right: water, a bubble column, or a plant that grows under water.
pub(crate) fn holds_water(id: &str, waterlogged: bool) -> bool {
    waterlogged || matches!(id, "minecraft:water" | "minecraft:bubble_column" | "minecraft:kelp" | "minecraft:kelp_plant" | "minecraft:seagrass" | "minecraft:tall_seagrass")
}

/// What an entity's saved data (`Entity.saveWithoutId`) says: its health, its maximum health, the share of its armor slots filled
/// and the names of the equipment slots that hold an item.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct SavedFacts { pub health: Option<f32>, pub max_health: Option<f32>, pub armor_coverage: Option<f32>, pub equipment: Option<Vec<String>>, pub armor_stacks: Vec<(String, StackData)> }

fn number(value: &super::base::Nbt) -> Option<f64> {
    use super::base::Nbt;
    Some(match value { Nbt::Float(v) => f64::from(*v), Nbt::Double(v) => *v, Nbt::Int(v) => f64::from(*v), Nbt::Short(v) => f64::from(*v), Nbt::Byte(v) => f64::from(*v), Nbt::Long(v) => *v as f64, _ => return None })
}

/// Reads an entity's saved data. The maximum health is the saved `minecraft:max_health` attribute: its base value with the saved modifiers
/// applied (add value, then multiplied base, then multiplied total); None when the data holds no such attribute.
pub(crate) fn saved_facts(data: &super::base::NbtCompound) -> SavedFacts {
    use super::base::Nbt;
    let health = data.get("Health").and_then(number).map(|health| health as f32);
    let max_health = data.get("attributes").and_then(|list| match list { Nbt::List(list) => Some(list), _ => None }).and_then(|list| {
        list.iter().find_map(|attribute| {
            let Nbt::Compound(attribute) = attribute else { return None };
            if !matches!(attribute.get("id"), Some(Nbt::String(id)) if id == "minecraft:max_health") { return None; }
            let base = attribute.get("base").and_then(number)?;
            let (mut add, mut base_multiplier, mut total_multiplier) = (0.0, 0.0, 1.0);
            if let Some(Nbt::List(modifiers)) = attribute.get("modifiers") {
                for modifier in modifiers {
                    let Nbt::Compound(modifier) = modifier else { continue };
                    let amount = modifier.get("amount").and_then(number).unwrap_or(0.0);
                    match modifier.get("operation") {
                        Some(Nbt::String(operation)) if operation == "add_multiplied_base" => base_multiplier += amount,
                        Some(Nbt::String(operation)) if operation == "add_multiplied_total" => total_multiplier *= 1.0 + amount,
                        _ => add += amount,
                    }
                }
            }
            Some(((base + add) * (1.0 + base_multiplier) * total_multiplier) as f32)
        })
    });
    // The `equipment` compound names a slot only while it holds something.
    let filled = |equipment: &super::base::NbtCompound, slot: &str| {
        matches!(equipment.get(slot), Some(Nbt::Compound(stack)) if !matches!(stack.get("id"), Some(Nbt::String(id)) if id == "minecraft:air"))
    };
    let (armor_coverage, equipment, armor_stacks) = match data.get("equipment") {
        Some(Nbt::Compound(equipment)) => (
            Some(armor_coverage(["head", "chest", "legs", "feet"].iter().filter(|slot| filled(equipment, slot)).count())),
            Some(["mainhand", "offhand", "head", "chest", "legs", "feet"].iter().filter(|slot| filled(equipment, slot)).map(|slot| slot.to_string()).collect()),
            ["feet", "legs", "chest", "head"].iter().filter_map(|slot| saved_armor_stack(equipment, slot)).collect(),
        ),
        _ => (None, None, Vec::new()),
    };
    SavedFacts { health, max_health, armor_coverage, equipment, armor_stacks }
}

/// One armor piece from an entity's saved `equipment` compound, in the game's saved stack layout (`id`, `count`,
/// `components`): the stack as a mod sees it, with its `minecraft:custom_data`. None for an empty slot or a piece
/// carrying no custom data — the armor the damage events report is the pieces a mod keeps state on, as the pieces
/// a player's tick reports are.
fn saved_armor_stack(equipment: &super::base::NbtCompound, slot: &str) -> Option<(String, StackData)> {
    use super::base::Nbt;
    let Nbt::Compound(stack) = equipment.get(slot)? else { return None };
    let Some(Nbt::String(id)) = stack.get("id") else { return None };
    if id == "minecraft:air" { return None; }
    let custom_data = match stack.get("components") {
        Some(Nbt::Compound(components)) => match components.get("minecraft:custom_data") {
            Some(Nbt::Compound(data)) => data.clone(),
            _ => return None,
        },
        _ => return None,
    };
    let count = stack.get("count").and_then(number).map(|count| count as u32).unwrap_or(1);
    Some((slot.to_owned(), StackData { item: id.clone(), count, custom_data, ..StackData::default() }))
}

/// `LivingEntity.getArmorCoverPercentage` for `filled` of the four armor slots.
pub(crate) fn armor_coverage(filled: usize) -> f32 { filled.min(4) as f32 / 4.0 }

/// A player is attacking `entity` with `stack`: remembered for the damage that follows in the same call.
pub(crate) fn attack_started(entity: i32, stack: Option<StackData>) {
    PENDING_ATTACK.with(|pending| *pending.borrow_mut() = stack.map(|s| (entity, s)));
}

/// Changes whenever the armor any player wears changes; see `VisibilityModification::armor_revision`.
static ARMOR_REVISION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// The armor the player with entity id `entity` wore as of their last tick; the game names the local player of a single-player world as
/// entity 0, which is the only wearer when one is known.
pub(crate) fn armor_worn_by(entity: i32) -> Vec<(String, StackData)> {
    let Ok(worn) = ARMOR.lock() else { return Vec::new() };
    if let Some((_, pieces)) = worn.get(&entity) { return pieces.clone(); }
    if entity == 0 && worn.len() == 1 { return worn.values().next().map(|(_, pieces)| pieces.clone()).unwrap_or_default(); }
    Vec::new()
}

/// What each player wears, by entity id, with the player's uuid so a logout can forget it.
static ARMOR: std::sync::Mutex<std::collections::BTreeMap<i32, (u128, Vec<(String, StackData)>)>> = std::sync::Mutex::new(std::collections::BTreeMap::new());

/// What each player holds in the off hand, by entity id, with the player's uuid: the hand the armor
/// memory does not cover, where a harvest enchantment may sit on a shield.
static OFFHAND: std::sync::Mutex<std::collections::BTreeMap<i32, (u128, Option<StackData>)>> = std::sync::Mutex::new(std::collections::BTreeMap::new());

/// Records what the player (`entity`) holds in the off hand; a stack with no custom data is
/// recorded as none, as the tick that reports it only names stacks carrying some.
pub(crate) fn remember_offhand(uuid: u128, entity: i32, stack: Option<StackData>) {
    let Ok(mut hands) = OFFHAND.lock() else { return };
    hands.insert(entity, (uuid, stack));
}

/// The equipment of the player `uuid`, as (slot, stack): the armor the armor memory kept for them
/// (in `feet`, `legs`, `chest`, `head` order), then the off hand it kept, under the name `offhand`.
/// Only pieces carrying custom data are ever recorded. Empty when the player is not known.
pub fn equipment_of(uuid: u128) -> Vec<(String, StackData)> {
    let mut out = Vec::new();
    if let Ok(worn) = ARMOR.lock() {
        for (owner, pieces) in worn.values() {
            if *owner == uuid { out.extend(pieces.iter().cloned()); }
        }
    }
    if let Ok(hands) = OFFHAND.lock() {
        for (owner, stack) in hands.values() {
            if *owner == uuid {
                if let Some(stack) = stack { out.push(("offhand".to_string(), stack.clone())); }
            }
        }
    }
    out
}

/// Records what the player (`entity`) wears; an empty list forgets it.
pub(crate) fn remember_armor(uuid: u128, entity: i32, armor: Vec<(String, StackData)>) {
    let Ok(mut worn) = ARMOR.lock() else { return };
    if armor.is_empty() {
        if worn.remove(&entity).is_some() { ARMOR_REVISION.fetch_add(1, std::sync::atomic::Ordering::Relaxed); }
    } else if worn.get(&entity).map(|(known, pieces)| (*known, pieces)) != Some((uuid, &armor)) {
        worn.insert(entity, (uuid, armor));
        ARMOR_REVISION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

/// The piece (`feet`, `legs`, `chest` or `head`) the player with `uuid` wore as of their last tick, when it carries custom data.
pub(crate) fn worn_piece(uuid: u128, slot: &str) -> Option<StackData> {
    let worn = ARMOR.lock().ok()?;
    worn.values().find(|(known, _)| *known == uuid)?.1.iter().find(|(name, _)| name == slot).map(|(_, stack)| stack.clone())
}

/// The factor on how far a mob of type `observer` sees the player `wearer`: 1 when the player wears no armor carrying custom data.
pub(crate) fn modify_visibility(wearer: u128, observer: &str) -> f64 {
    let Ok(worn) = ARMOR.lock() else { return 1.0 };
    let Some((_, armor)) = worn.values().find(|(known, _)| *known == wearer) else { return 1.0 };
    let armor = armor.clone();
    drop(worn);
    let modification = VisibilityModification { wearer, observer: observer.to_owned(), armor, armor_revision: ARMOR_REVISION.load(std::sync::atomic::Ordering::Relaxed), multiplier: Cell::new(1.0) };
    MODIFY_VISIBILITY.invoke(&modification);
    let factor = modification.multiplier.get();
    if factor.is_finite() { factor.max(0.0) } else { 1.0 }
}

/// Forgets a player who left.
pub(crate) fn forget_armor(uuid: u128) {
    if let Ok(mut worn) = ARMOR.lock() {
        let gone: Vec<i32> = worn.iter().filter(|(_, (known, _))| *known == uuid).map(|(entity, _)| *entity).collect();
        worn.retain(|_, (known, _)| *known != uuid);
        for entity in gone { remember_wearer_state(entity, None); }
    }
}

/// The amount after the callbacks; the stack of the attack that is being applied to `damage.entity`, if there is one.
pub(crate) fn modify_damage(damage: &LivingDamage) -> f32 {
    let attacker_stack = PENDING_ATTACK.with(|pending| {
        let mut pending = pending.borrow_mut();
        match pending.take() {
            Some((entity, stack)) if entity == damage.entity => Some(stack),
            other => {
                *pending = other;
                None
            }
        }
    });
    let armor = ARMOR.lock().ok().and_then(|worn| worn.get(&damage.entity).map(|(_, pieces)| pieces.clone())).unwrap_or_default();
    let (attacker_state, mut target_state) = PENDING_STATES.with(|pending| {
        let mut pending = pending.borrow_mut();
        match pending.take() {
            Some((entity, attacker, target, at)) if entity == damage.entity && at.elapsed() < HIT_STATES_LIFETIME => (attacker, target),
            Some((entity, _, _, at)) if entity == damage.entity || at.elapsed() >= HIT_STATES_LIFETIME => (None, None),
            other => { *pending = other; (None, None) }
        }
    });
    // A hit that brought no state of the damaged entity still has its wearer's, as of their last tick.
    if target_state.is_none() && !armor.is_empty() { target_state = wearer_state(damage.entity); }
    // A mob's worn pieces come with the hit's own report of it, when that report ran (a player's weapon hit reads the
    // mob's saved data); a player's pieces are the remembered ones above.
    let armor = if armor.is_empty() { target_state.as_ref().map(|state| state.armor_stacks.clone()).unwrap_or_default() } else { armor };
    let mut modification = DamageModification { damage: damage.clone(), amount: Cell::new(damage.amount), attacker_stack, armor, attacker_state, target_state, attacker_kind: String::new(), attacker_is_player: None, attacker_main_hand: None, attacker_chest: None, attacker_attack_damage: None, target_health: None, target_attack_damage: None, actions: RefCell::new(Vec::new()) };
    fill_damage_facts(&mut modification);
    MODIFY_DAMAGE.invoke(&modification);
    queue_damage_actions(damage, &modification);
    modification.amount.get()
}

/// The amount after the hurt callbacks. The hurt stage runs as the damage arrives, before armor takes its share, where the
/// damage stage runs after it; both read what the hit brought, so this stage only borrows the attacking stack and the states
/// and leaves them for the damage event that follows in the same call.
pub(crate) fn modify_hurt(damage: &LivingDamage) -> f32 {
    let attacker_stack = PENDING_ATTACK.with(|pending| match &*pending.borrow() { Some((entity, stack)) if *entity == damage.entity => Some(stack.clone()), _ => None });
    let armor = ARMOR.lock().ok().and_then(|worn| worn.get(&damage.entity).map(|(_, pieces)| pieces.clone())).unwrap_or_default();
    let (attacker_state, mut target_state) = PENDING_STATES.with(|pending| match &*pending.borrow() {
        Some((entity, attacker, target, at)) if *entity == damage.entity && at.elapsed() < HIT_STATES_LIFETIME => (attacker.clone(), target.clone()),
        _ => (None, None),
    });
    // A hit that brought no state of the damaged entity still has its wearer's, as of their last tick.
    if target_state.is_none() && !armor.is_empty() { target_state = wearer_state(damage.entity); }
    // A mob's worn pieces come with the hit's own report of it, when that report ran (a player's weapon hit reads the
    // mob's saved data); a player's pieces are the remembered ones above.
    let armor = if armor.is_empty() { target_state.as_ref().map(|state| state.armor_stacks.clone()).unwrap_or_default() } else { armor };
    let mut modification = DamageModification { damage: damage.clone(), amount: Cell::new(damage.amount), attacker_stack, armor, attacker_state, target_state, attacker_kind: String::new(), attacker_is_player: None, attacker_main_hand: None, attacker_chest: None, attacker_attack_damage: None, target_health: None, target_attack_damage: None, actions: RefCell::new(Vec::new()) };
    fill_damage_facts(&mut modification);
    MODIFY_HURT.invoke(&modification);
    queue_damage_actions(damage, &modification);
    modification.amount.get()
}

/// Queues the actions a damage callback asked for, at either stage: the ones for an attacking player land on that player's
/// next tick, the ones for the wearer on the wearer's, a drop is handed to the game for its target, and one for a party
/// neither tick can place (a mob attacker, a mob victim) is kept for that entity's next tick.
fn queue_damage_actions(damage: &LivingDamage, modification: &DamageModification) {
    for action in modification.actions.borrow_mut().drain(..) {
        // A drop lands at its target entity whatever the parties are; the game spawns it there.
        if let EntityAction::DropItem { target, stack } = &action {
            let entity = match target { ActionTarget::Attacker => damage.attacker, ActionTarget::Victim => damage.entity };
            if entity >= 0 { if let Ok(mut queue) = DROP_ACTIONS.lock() { queue.push((entity, stack.clone())); } }
            continue;
        }
        // The actions a player's attack asks for the attacker (a heal by the damage dealt) land on that player's next tick, with the damaged entity as the other party.
        if action_target(&action) == ActionTarget::Attacker && modification.attacker_stack.is_some() && damage.attacker >= 0 {
            if let Ok(mut queue) = ATTACKER_ACTIONS.lock() { queue.push((damage.attacker, damage.entity, action)); }
            continue;
        }
        // The actions for the wearer land on their next tick, with the attacker's entity id as the other party.
        if !modification.armor.is_empty() {
            if let Ok(mut queue) = ENTITY_ACTIONS.lock() { queue.push((damage.entity, damage.attacker, action)); }
            continue;
        }
        // Neither tick queue can place it: keep it for the target entity's next tick instead of dropping it.
        let entity = match action_target(&action) { ActionTarget::Attacker => damage.attacker, ActionTarget::Victim => damage.entity };
        if entity >= 0 { if let Ok(mut queue) = LATE_ACTIONS.lock() { queue.push((entity, damage.attacker, damage.entity, action)); } }
    }
}

/// Which party an action is done to.
fn action_target(action: &EntityAction) -> ActionTarget {
    match action {
        EntityAction::SetFire { target, .. } | EntityAction::Heal { target, .. } | EntityAction::Freeze { target, .. } | EntityAction::Knockback { target, .. }
        | EntityAction::AddEffect { target, .. } | EntityAction::EditStackData { target, .. } | EntityAction::Hurt { target, .. }
        | EntityAction::SetEffectDuration { target, .. } | EntityAction::Teleport { target, .. } | EntityAction::DropItem { target, .. }
        | EntityAction::Summon { target, .. } => *target,
    }
}

/// The stacks damage callbacks asked to drop at an entity, with the entity's id: the damage events have no
/// world to drop into, so the transport hands them to the game, which spawns them at the entity.
static DROP_ACTIONS: std::sync::Mutex<Vec<(i32, StackData)>> = std::sync::Mutex::new(Vec::new());

/// Takes the drops queued at entities, as (entity, stack); the transport calls this as the world ticks.
pub(crate) fn take_drop_actions() -> Vec<(i32, StackData)> {
    let Ok(mut queue) = DROP_ACTIONS.lock() else { return Vec::new() };
    std::mem::take(&mut *queue)
}

/// The actions for a party neither the attacker's nor the wearer's tick could place, kept by the entity they
/// are done to, with the hit's attacker and victim.
static LATE_ACTIONS: std::sync::Mutex<Vec<(i32, i32, i32, EntityAction)>> = std::sync::Mutex::new(Vec::new());

/// Takes the actions kept for the entity `entity`, as (attacker, victim, action); other entities' stay queued.
pub(crate) fn take_late_actions(entity: i32) -> Vec<(i32, i32, EntityAction)> {
    let Ok(mut queue) = LATE_ACTIONS.lock() else { return Vec::new() };
    let (mine, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut *queue).into_iter().partition(|(who, _, _, _)| *who == entity);
    *queue = rest;
    mine.into_iter().map(|(_, attacker, victim, action)| (attacker, victim, action)).collect()
}

/// The actions a player's attacks asked for the attacker, with the attacker's entity id and the damaged entity's.
static ATTACKER_ACTIONS: std::sync::Mutex<Vec<(i32, i32, EntityAction)>> = std::sync::Mutex::new(Vec::new());

/// Takes the actions queued for the attacks of the player `attacker`, as (damaged entity, action); other players' stay queued.
pub(crate) fn take_attacker_actions(attacker: i32) -> Vec<(i32, EntityAction)> {
    let Ok(mut queue) = ATTACKER_ACTIONS.lock() else { return Vec::new() };
    let (mine, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut *queue).into_iter().partition(|(who, _, _)| *who == attacker);
    *queue = rest;
    mine.into_iter().map(|(_, victim, action)| (victim, action)).collect()
}

pub(crate) fn allow_damage(damage: &LivingDamage) -> bool {
    ALLOW_DAMAGE.invoke(damage)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shearing_can_be_cancelled_and_queues_actions_for_the_player() {
        ServerEntityEvents::SHEAR.register(|attempt| {
            if attempt.entity != 8181 { return; }
            if attempt.tool.as_ref().is_some_and(|tool| tool.item == "example:cutters") {
                attempt.actions.borrow_mut().push(EntityAction::AddEffect { target: ActionTarget::Attacker, effect: "minecraft:speed".into(), amplifier: 0, duration: 40 });
            }
            if attempt.off_hand { attempt.cancel.set(true); }
        });
        let attempt = |item: Option<&str>, off_hand| ShearAttempt {
            player: 1, name: "shearer".into(), entity: 8181, off_hand,
            tool: item.map(|item| StackData { item: item.into(), count: 1, ..StackData::default() }),
            cancel: Cell::new(false), actions: RefCell::new(Vec::new()),
        };
        let (allowed, actions) = shear(&attempt(Some("example:cutters"), false));
        assert!(allowed);
        assert_eq!(actions, vec![EntityAction::AddEffect { target: ActionTarget::Attacker, effect: "minecraft:speed".into(), amplifier: 0, duration: 40 }]);
        let (allowed, actions) = shear(&attempt(Some("minecraft:shears"), true));
        assert!(!allowed && actions.is_empty(), "the off hand's shearing is cancelled and queues nothing");
    }

    #[test]
    fn modify_damage_carries_the_armor_a_player_wore_and_forgets_it_on_logout() {
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let log = seen.clone();
        ServerLivingEntityEvents::MODIFY_DAMAGE.register(move |hit| if hit.damage.entity == 4242 { log.lock().unwrap().push(hit.armor.iter().map(|(slot, stack)| format!("{slot}:{}", stack.item)).collect::<Vec<_>>()); });
        let hurt = LivingDamage { entity: 4242, kind: "minecraft:player".into(), damage_type: "minecraft:mob_attack".into(), attacker: 3, direct: 3, amount: 5.0 };
        let helmet = StackData { item: "example:helmet".into(), count: 1, ..StackData::default() };
        remember_armor(99, 4242, vec![("head".into(), helmet)]);
        modify_damage(&hurt);
        forget_armor(99);
        modify_damage(&hurt);
        assert_eq!(*seen.lock().unwrap(), vec![vec!["head:example:helmet".to_string()], Vec::<String>::new()]);
    }

    #[test]
    fn damage_events_carry_the_attack_snapshot_and_the_victims_health() {
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let log = seen.clone();
        ServerLivingEntityEvents::MODIFY_DAMAGE.register(move |hit| {
            if hit.damage.entity == 6060 {
                log.lock().unwrap().push((hit.attacker_kind.clone(), hit.attacker_is_player, hit.attacker_main_hand.as_ref().map(|stack| stack.item.clone()),
                    hit.attacker_chest.as_ref().map(|stack| stack.item.clone()), hit.attacker_attack_damage, hit.target_health, hit.target_attack_damage,
                    hit.attacker_state.as_ref().map(|state| state.kind.clone())));
            }
        });
        let sword = StackData { item: "example:sword".into(), count: 1, ..StackData::default() };
        let plate = StackData { item: "example:plate".into(), count: 1, ..StackData::default() };
        note_attacker_facts(77, AttackerFacts { kind: "minecraft:zombie".into(), main_hand: Some(sword), chest: Some(plate), attack_damage: 3.0,
            state: EntityState { kind: "minecraft:zombie".into(), on_ground: true, ..EntityState::default() } });
        note_victim_facts(6060, VictimFacts { health: 14.5, attack_damage: Some(2.0) });
        let hurt = LivingDamage { entity: 6060, kind: "minecraft:pig".into(), damage_type: "minecraft:mob_attack".into(), attacker: 77, direct: 77, amount: 4.0 };
        modify_damage(&hurt);
        // The parties report names the same hit's causer as a player (a tamed mob's owner credited, say): it stands over the snapshot.
        note_damage_parties(6060, 77, true);
        modify_damage(&hurt);
        assert_eq!(*seen.lock().unwrap(), vec![
            ("minecraft:zombie".to_string(), Some(false), Some("example:sword".to_string()), Some("example:plate".to_string()), Some(3.0), Some(14.5), Some(2.0), Some("minecraft:zombie".to_string())),
            ("minecraft:zombie".to_string(), Some(true), Some("example:sword".to_string()), Some("example:plate".to_string()), Some(3.0), Some(14.5), Some(2.0), Some("minecraft:zombie".to_string())),
        ]);
    }

    #[test]
    fn a_drop_item_action_is_queued_for_its_target_and_other_actions_for_a_mob_party_are_kept_for_its_tick() {
        ServerLivingEntityEvents::MODIFY_DAMAGE.register(|hit| {
            if hit.damage.entity != 7070 { return; }
            hit.actions.borrow_mut().push(EntityAction::DropItem { target: ActionTarget::Victim, stack: StackData { item: "example:scale".into(), count: 3, ..StackData::default() } });
            hit.actions.borrow_mut().push(EntityAction::Heal { target: ActionTarget::Attacker, amount: 2.0 });
            hit.actions.borrow_mut().push(EntityAction::SetFire { target: ActionTarget::Victim, seconds: 5 });
        });
        let hurt = LivingDamage { entity: 7070, kind: "minecraft:ender_dragon".into(), damage_type: "minecraft:explosion".into(), attacker: 88, direct: 88, amount: 10.0 };
        modify_damage(&hurt);
        assert_eq!(take_drop_actions(), vec![(7070, StackData { item: "example:scale".into(), count: 3, ..StackData::default() })]);
        assert!(take_drop_actions().is_empty(), "the drops were taken");
        // The heal for the (mob) attacker and the fire for the (mob) victim could go on no player's tick: both are kept, by target.
        assert_eq!(take_late_actions(88), vec![(88, 7070, EntityAction::Heal { target: ActionTarget::Attacker, amount: 2.0 })]);
        assert_eq!(take_late_actions(7070), vec![(88, 7070, EntityAction::SetFire { target: ActionTarget::Victim, seconds: 5 })]);
        assert!(take_late_actions(7070).is_empty());
    }

    #[test]
    fn visibility_scales_only_for_a_wearer_of_custom_data_armor_and_the_revision_follows_the_armor() {
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let log = seen.clone();
        ServerLivingEntityEvents::MODIFY_VISIBILITY.register(move |look| {
            if look.wearer != 31337 { return; }
            log.lock().unwrap().push(look.armor_revision);
            if look.observer == "minecraft:skeleton" && look.armor.iter().any(|(slot, piece)| slot == "head" && piece.item == "example:mask") {
                look.multiplier.set(look.multiplier.get() * 0.25);
            }
        });
        let piece = |item: &str| StackData { item: item.into(), count: 1, ..StackData::default() };
        assert_eq!(modify_visibility(31337, "minecraft:skeleton"), 1.0, "a player with no cached armor is not asked about");
        remember_armor(31337, 4141, vec![("head".into(), piece("example:mask"))]);
        assert_eq!(worn_piece(31337, "head").map(|stack| stack.item), Some("example:mask".to_string()));
        assert!(worn_piece(31337, "feet").is_none() && worn_piece(1, "head").is_none());
        assert_eq!(modify_visibility(31337, "minecraft:skeleton"), 0.25);
        assert_eq!(modify_visibility(31337, "minecraft:zombie"), 1.0);
        remember_armor(31337, 4141, vec![("head".into(), piece("example:mask"))]);
        assert_eq!(modify_visibility(31337, "minecraft:zombie"), 1.0);
        remember_armor(31337, 4141, vec![("head".into(), piece("example:hat"))]);
        assert_eq!(modify_visibility(31337, "minecraft:skeleton"), 1.0, "the armor changed");
        forget_armor(31337);
        assert_eq!(modify_visibility(31337, "minecraft:skeleton"), 1.0);
        let revisions = seen.lock().unwrap().clone();
        assert!(revisions.iter().all(|revision| *revision != 0));
        assert_eq!(revisions.len(), 4);
        assert_eq!(revisions[0], revisions[1], "the same armor keeps its revision");
        assert_eq!(revisions[1], revisions[2], "re-reporting identical armor keeps it too");
        assert!(revisions[3] > revisions[2], "changed armor takes a new one");
    }

    #[test]
    fn knockback_and_item_wear_can_be_changed_or_cancelled_by_callbacks() {
        ServerLivingEntityEvents::MODIFY_KNOCKBACK.register(|push| {
            if push.entity != 5151 { return; }
            if push.attacker_main.as_ref().is_some_and(|stack| stack.item == "example:hammer") { push.strength.set(push.strength.get() * 2.0); }
            if push.attacker_off.is_some() { push.cancel.set(true); }
        });
        ServerLivingEntityEvents::MODIFY_ITEM_DAMAGE.register(|wear| {
            if wear.name != "wearer" { return; }
            if wear.item == "example:shield" { wear.cancel.set(true); } else { wear.amount.set(wear.amount.get() - 1); }
        });
        let hammer = StackData { item: "example:hammer".into(), count: 1, ..StackData::default() };
        let push = |main: Option<StackData>, off: Option<StackData>| KnockbackModification { entity: 5151, kind: "minecraft:zombie".into(), attacker: 1, attacker_main: main, attacker_off: off, strength: Cell::new(0.4), x: Cell::new(1.0), z: Cell::new(0.0), cancel: Cell::new(false) };
        assert_eq!(modify_knockback(&push(Some(hammer.clone()), None)), Some((0.8, 1.0, 0.0)));
        assert_eq!(modify_knockback(&push(None, None)), Some((0.4, 1.0, 0.0)));
        assert_eq!(modify_knockback(&push(None, Some(hammer))), None);
        let wear = |item: &str, amount| ItemDamage { player: 1, name: "wearer".into(), item: item.into(), stack: None, slot: None, amount: Cell::new(amount), cancel: Cell::new(false), replacement: RefCell::new(None), holder: None, worn: None, carried: Vec::new(), edits: RefCell::new(Vec::new()) };
        assert_eq!(modify_item_damage(&wear("example:pickaxe", 3)), Some((2, None, Vec::new())));
        assert_eq!(modify_item_damage(&wear("example:shield", 3)), None);
    }

    #[test]
    fn item_wear_reports_the_holders_other_stacks_and_returns_their_edits() {
        use super::super::base::{Identifier, NbtCompoundExt};
        ServerLivingEntityEvents::MODIFY_ITEM_DAMAGE.register(|wear| {
            if wear.name != "sharer" { return; }
            // Share the wear: hand a charge (kept in custom data) to the first other tool the holder carries.
            let other = wear.carried.iter().find(|carried| carried.slot != CarriedSlot::Equipment(StackSlot::Chest)).expect("another stack is listed");
            let mut stack = other.stack.clone();
            stack.custom_data.insert("shield".into(), super::super::base::Nbt::Int(5));
            wear.edits.borrow_mut().push(CarriedEdit { slot: other.slot, stack });
        });
        let chest = ItemSlot::new(Identifier::parse("example:chestplate").unwrap(), 1);
        let pick = ItemSlot::new(Identifier::parse("example:pickaxe").unwrap(), 1);
        let wear = ItemDamage {
            player: 2, name: "sharer".into(), item: "example:chestplate".into(), stack: None, slot: Some(chest),
            amount: Cell::new(4), cancel: Cell::new(false), replacement: RefCell::new(None), holder: None,
            worn: Some(CarriedSlot::Equipment(StackSlot::Chest)),
            carried: vec![
                CarriedStack { slot: CarriedSlot::Equipment(StackSlot::Chest), stack: ItemSlot::new(Identifier::parse("example:chestplate").unwrap(), 1) },
                CarriedStack { slot: CarriedSlot::Inventory(3), stack: pick },
            ],
            edits: RefCell::new(Vec::new()),
        };
        let (amount, _, edits) = modify_item_damage(&wear).expect("not cancelled");
        assert_eq!(amount, 4);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].slot, CarriedSlot::Inventory(3), "the edit names the other stack's place, not the worn one's");
        assert_eq!(edits[0].stack.custom_data.get_int("shield", 0), 5);
    }

    #[test]
    fn a_weapon_hit_carries_what_the_attacker_and_the_target_were_doing() {
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let log = seen.clone();
        ServerLivingEntityEvents::MODIFY_DAMAGE.register(move |hit| if hit.damage.entity == 6262 {
            log.lock().unwrap().push((hit.attacker_state.as_ref().map(|state| state.sprinting), hit.target_state.as_ref().map(|state| state.fire_ticks)));
        });
        let hurt = LivingDamage { entity: 6262, kind: "minecraft:husk".into(), damage_type: "minecraft:player_attack".into(), attacker: 1, direct: 1, amount: 5.0 };
        hit_states(6262, Some(EntityState { sprinting: true, ..EntityState::default() }), Some(EntityState { fire_ticks: 40, ..EntityState::default() }));
        modify_damage(&hurt);
        modify_damage(&hurt);
        assert_eq!(*seen.lock().unwrap(), vec![(Some(true), Some(40)), (None, None)], "the states belong to one hit");
        hit_states(6262, Some(EntityState { sprinting: true, ..EntityState::default() }), None);
        std::thread::sleep(std::time::Duration::from_millis(150));
        modify_damage(&hurt);
        assert_eq!(seen.lock().unwrap().last(), Some(&(None, None)), "states that outlived their hit are dropped");
    }

    #[test]
    fn a_weapon_hit_collects_the_effects_callbacks_put_on_the_mob() {
        ServerLivingEntityEvents::WEAPON_HIT.register(|hit| {
            if hit.entity != 7373 { return; }
            if hit.weapon.as_ref().is_some_and(|weapon| weapon.item == "example:wand") {
                hit.effects.borrow_mut().push(EffectGrant { effect: "minecraft:slowness".into(), amplifier: 3, duration: 20 });
            }
        });
        let hit = |item: Option<&str>| WeaponHit { entity: 7373, kind: "minecraft:husk".into(), attacker: 1, weapon: item.map(|item| StackData { item: item.into(), count: 1, ..StackData::default() }), attacker_state: None, target_state: None, target_position: None, attacker_position: None, armor: Vec::new(), effects: RefCell::new(Vec::new()), actions: RefCell::new(Vec::new()) };
        assert_eq!(weapon_hit(&hit(Some("example:wand"))), vec![EffectGrant { effect: "minecraft:slowness".into(), amplifier: 3, duration: 20 }]);
        assert!(weapon_hit(&hit(Some("example:stick"))).is_empty());
        assert!(weapon_hit(&hit(None)).is_empty());
    }

    #[test]
    fn the_armor_a_weapon_hit_carries_is_the_attacking_players_as_of_their_last_tick() {
        let helmet = StackData { item: "example:helmet".into(), count: 1, ..StackData::default() };
        remember_armor(0xA11CE, 4343, vec![("head".into(), helmet.clone())]);
        assert_eq!(armor_worn_by(4343), vec![("head".to_owned(), helmet.clone())]);
        assert!(armor_worn_by(4344).is_empty(), "an unknown player wears nothing known");
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let log = seen.clone();
        ServerLivingEntityEvents::WEAPON_HIT.register(move |hit| if hit.entity == 4545 { log.lock().unwrap().push(hit.armor.len()); });
        let hit = WeaponHit { entity: 4545, kind: "minecraft:husk".into(), attacker: 4343, weapon: None, attacker_state: None, target_state: None, target_position: None, attacker_position: None, armor: armor_worn_by(4343), effects: RefCell::new(Vec::new()), actions: RefCell::new(Vec::new()) };
        assert!(weapon_hit(&hit).is_empty());
        assert_eq!(*seen.lock().unwrap(), vec![1]);
        forget_armor(0xA11CE);
        assert!(armor_worn_by(4343).is_empty(), "a player who left is forgotten");
    }

    #[test]
    fn a_hit_on_the_wearer_collects_the_effects_callbacks_put_on_the_attacker() {
        ServerLivingEntityEvents::WEARER_HIT.register(|hit| {
            if hit.attacker != 8484 { return; }
            if hit.armor.iter().any(|(slot, piece)| slot == "head" && piece.item == "example:helmet") {
                hit.effects.borrow_mut().push(EffectGrant { effect: "minecraft:weakness".into(), amplifier: 0, duration: 100 });
            }
        });
        let hit = |armor: Vec<(String, StackData)>| WearerHit { attacker: 8484, kind: "minecraft:zombie".into(), damage_type: "minecraft:mob_attack".into(), amount: 3.0, attacker_state: EntityState::default(), attacker_position: None, armor, effects: RefCell::new(Vec::new()), self_effects: RefCell::new(Vec::new()), counter_damage: RefCell::new(Vec::new()), actions: RefCell::new(Vec::new()) };
        let helmet = StackData { item: "example:helmet".into(), count: 1, ..StackData::default() };
        assert_eq!(wearer_hit(&hit(vec![("head".into(), helmet)]), Some(7)), vec![EffectGrant { effect: "minecraft:weakness".into(), amplifier: 0, duration: 100 }]);
        assert!(wearer_hit(&hit(Vec::new()), Some(7)).is_empty());
    }

    #[test]
    fn actions_on_a_hit_on_the_wearer_wait_for_the_next_tick_and_a_weapon_hit_hands_its_own_over() {
        ServerLivingEntityEvents::WEARER_HIT.register(|hit| {
            if hit.attacker == 6161 { hit.actions.borrow_mut().push(EntityAction::SetFire { target: ActionTarget::Attacker, seconds: 4 }); }
        });
        ServerLivingEntityEvents::WEAPON_HIT.register(|hit| {
            if hit.entity == 6262 { hit.actions.borrow_mut().push(EntityAction::Freeze { target: ActionTarget::Victim, ticks: 80 }); }
        });
        let hit = WearerHit { attacker: 6161, kind: "minecraft:zombie".into(), damage_type: String::new(), amount: 0.0, attacker_state: EntityState::default(), attacker_position: None, armor: Vec::new(), effects: RefCell::new(Vec::new()), self_effects: RefCell::new(Vec::new()), counter_damage: RefCell::new(Vec::new()), actions: RefCell::new(Vec::new()) };
        wearer_hit(&hit, Some(41));
        assert!(take_wearer_actions(42).is_empty(), "another player's tick takes nothing");
        assert_eq!(take_wearer_actions(41), vec![(6161, EntityAction::SetFire { target: ActionTarget::Attacker, seconds: 4 })]);
        assert!(take_wearer_actions(41).is_empty(), "the queue is emptied once taken");
        wearer_hit(&hit, None);
        assert!(take_wearer_actions(41).is_empty(), "a hit with no known wearer queues nothing");
        let weapon = WeaponHit { entity: 6262, kind: "minecraft:husk".into(), attacker: 1, weapon: None, attacker_state: None, target_state: None, target_position: None, attacker_position: None, armor: Vec::new(), effects: RefCell::new(Vec::new()), actions: RefCell::new(Vec::new()) };
        weapon_hit(&weapon);
        assert_eq!(take_weapon_actions(&weapon), vec![EntityAction::Freeze { target: ActionTarget::Victim, ticks: 80 }]);
        assert!(take_weapon_actions(&weapon).is_empty());
    }

    #[test]
    fn the_action_guards_follow_the_games_own_rules() {
        assert_eq!(fire_ticks_after(0, 5, false), Some(100));
        assert_eq!(fire_ticks_after(100, 5, false), None, "a fire that is not longer changes nothing");
        assert_eq!(fire_ticks_after(-1, 1, false), Some(20), "a mob that does not burn reports 0 or less");
        assert_eq!(fire_ticks_after(0, 5, true), None, "fire-immune mobs do not burn");
        assert_eq!(fire_ticks_after(0, 0, false), None);
        assert!(!can_freeze("minecraft:stray") && !can_freeze("minecraft:snow_golem") && can_freeze("minecraft:zombie"));
        assert_eq!(frozen_ticks_after(0, 80), Some(220), "a mob starts from the 140 ticks that freeze it");
        assert_eq!(frozen_ticks_after(200, 80), Some(280), "a frozen mob keeps what it has");
        assert_eq!(frozen_ticks_after(0, 0), None);
        assert_eq!(healed_health(5.0, 2.5), Some(7.5));
        assert_eq!(healed_health(5.0, 0.0), None);
        assert_eq!(healed_health(5.0, f32::NAN), None);
    }

    #[test]
    fn knockback_pushes_against_the_direction_and_lifts_a_mob_on_the_ground() {
        assert_eq!(knockback_delta(0.5, 0.0, 2.0, true), Some([0.0, 0.4_f64.min(0.5), -0.5]));
        assert_eq!(knockback_delta(0.3, 3.0, 0.0, false), Some([-0.3, 0.0, 0.0]));
        assert_eq!(knockback_delta(0.3, 3.0, 0.0, true).map(|delta| delta[1]), Some(0.3), "lifted by the strength below 0.4");
        assert!(knockback_delta(0.0, 1.0, 0.0, true).is_none() && knockback_delta(1.0, 0.0, 0.0, true).is_none());
        let (x, z) = facing_direction(0.0);
        assert!(x.abs() < 1e-12 && (z - 1.0).abs() < 1e-12, "yaw 0 faces south");
        let [dx, _, dz] = knockback_delta(1.0, x, z, false).unwrap();
        assert!(dx.abs() < 1e-12 && (dz + 1.0).abs() < 1e-12, "a mob facing south is pushed north");
        let (x, z) = facing_direction(90.0);
        assert!((x + 1.0).abs() < 1e-12 && z.abs() < 1e-9, "yaw 90 faces west");
    }

    #[test]
    fn effects_for_the_wearer_wait_for_the_next_tick() {
        ServerLivingEntityEvents::WEARER_HIT.register(|hit| {
            if hit.attacker == 9595 { hit.self_effects.borrow_mut().push(EffectGrant { effect: "minecraft:strength".into(), amplifier: 0, duration: 200 }); }
        });
        let hit = WearerHit { attacker: 9595, kind: "minecraft:zombie".into(), damage_type: "minecraft:mob_attack".into(), amount: 3.0, attacker_state: EntityState::default(), attacker_position: None, armor: Vec::new(), effects: RefCell::new(Vec::new()), self_effects: RefCell::new(Vec::new()), counter_damage: RefCell::new(Vec::new()), actions: RefCell::new(Vec::new()) };
        hit.counter_damage.borrow_mut().push(CounterDamage { damage_type: "minecraft:thorns".into(), amount: 2.0 });
        assert!(wearer_hit(&hit, Some(31)).is_empty(), "nothing goes on the attacker");
        assert!(take_counter_damage(32).is_empty() && take_self_effects(32).is_empty(), "another player's tick takes nothing");
        assert_eq!(take_counter_damage(31), vec![(9595, CounterDamage { damage_type: "minecraft:thorns".into(), amount: 2.0 })]);
        assert_eq!(take_self_effects(31), vec![EffectGrant { effect: "minecraft:strength".into(), amplifier: 0, duration: 200 }]);
        assert!(take_self_effects(31).is_empty(), "the queue is emptied once taken");
        let unattributed = WearerHit { attacker: 9595, kind: "minecraft:zombie".into(), damage_type: String::new(), amount: 0.0, attacker_state: EntityState::default(), attacker_position: None, armor: Vec::new(), effects: RefCell::new(Vec::new()), self_effects: RefCell::new(Vec::new()), counter_damage: RefCell::new(Vec::new()), actions: RefCell::new(Vec::new()) };
        wearer_hit(&unattributed, None);
        assert!(take_self_effects(31).is_empty(), "a hit with no known wearer queues nothing");
    }

    #[test]
    fn a_player_tick_collects_the_effects_callbacks_take_off() {
        ServerLivingEntityEvents::PLAYER_TICK.register(|tick| {
            if tick.entity != 5757 { return; }
            for (id, amplifier, _) in &tick.effects { if id == "minecraft:poison" && *amplifier == 0 { tick.remove_effects.borrow_mut().push(id.clone()); } }
        });
        let tick = |effects: Vec<(String, i32, i32)>| PlayerTick { player: 1, entity: 5757, dimension: "minecraft:overworld".to_owned(), position: [0.0; 3], on_ground: true, sneaking: false, gliding: false, spectator: false, in_water: false, armor: Vec::new(), hands: Vec::new(), block_light: 0, sky_light: 0, sky_darken: 0, effects, remove_effects: RefCell::new(Vec::new()), attribute_additions: RefCell::new(Vec::new()), actions: RefCell::new(Vec::new()) };
        assert_eq!(player_tick(&tick(vec![("minecraft:poison".into(), 0, 100), ("minecraft:speed".into(), 0, 100)])), vec!["minecraft:poison".to_string()]);
        assert!(player_tick(&tick(vec![("minecraft:poison".into(), 1, 100)])).is_empty());
    }

    #[test]
    fn attribute_amounts_are_applied_once_and_taken_off_when_no_callback_names_them() {
        let wanted = vec![("minecraft:knockback_resistance".to_string(), 0.1), ("minecraft:knockback_resistance".to_string(), 0.05)];
        let first = attribute_changes(31, &wanted);
        assert_eq!(first.len(), 1);
        assert!((first[0].1 - 0.15).abs() < 1e-12 && (first[0].2 - 0.15).abs() < 1e-12);
        attribute_applied(31, &first[0].0, first[0].2);
        assert!(attribute_changes(31, &wanted).is_empty(), "an amount already applied is not applied again");
        let less = attribute_changes(31, &wanted[..1]);
        assert!((less[0].1 + 0.05).abs() < 1e-12);
        attribute_applied(31, &less[0].0, less[0].2);
        let gone = attribute_changes(31, &[]);
        assert!((gone[0].1 + 0.1).abs() < 1e-12 && gone[0].2 == 0.0);
        attribute_applied(31, &gone[0].0, 0.0);
        assert!(attribute_changes(31, &[]).is_empty());
        attribute_applied(31, "minecraft:armor", 2.0);
        assert_eq!(forget_attributes(31), vec![("minecraft:armor".to_string(), 2.0)]);
        assert!(forget_attributes(31).is_empty());
    }

    #[test]
    fn a_player_is_ticked_while_armor_is_worn_and_once_after() {
        assert!(!tick_wanted(32, false));
        assert!(tick_wanted(32, true));
        assert!(tick_wanted(32, false), "one more tick to let go of what the armor kept");
        assert!(!tick_wanted(32, false));
    }

    #[test]
    fn after_death_reaches_every_callback_with_the_killer() {
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let log = seen.clone();
        ServerLivingEntityEvents::AFTER_DEATH.register(move |d| log.lock().unwrap().push((d.entity, d.attacker)));
        after_death(&LivingDamage { entity: 7, kind: "minecraft:zombie".into(), damage_type: "minecraft:player_attack".into(), attacker: 2, direct: -1, amount: 0.0 });
        assert_eq!(*seen.lock().unwrap(), [(7, 2)]);
    }

    #[test]
    fn any_veto_cancels_the_damage() {
        ServerLivingEntityEvents::ALLOW_DAMAGE.register(|d| d.amount < 100.0);
        let hit = |amount| LivingDamage { entity: 1, kind: "minecraft:zombie".into(), damage_type: "minecraft:generic".into(), attacker: -1, direct: -1, amount };
        assert!(allow_damage(&hit(3.0)));
        assert!(!allow_damage(&hit(500.0)));
    }
    #[test]
    fn stack_edits_set_nested_values_and_remove_them_and_report_whether_anything_changed() {
        let edit = |path: &[&str], value: Option<Nbt>| StackDataEdit { path: path.iter().map(|key| key.to_string()).collect(), value };
        let mut data = NbtCompound::new();
        assert!(edit_custom_data(&mut data, &[edit(&["persistent", "mod:bar"], Some(Nbt::Int(40)))]), "the compounds on the way are made");
        let Some(Nbt::Compound(persistent)) = data.get("persistent") else { panic!("persistent compound") };
        assert_eq!(persistent.get("mod:bar"), Some(&Nbt::Int(40)));
        assert!(!edit_custom_data(&mut data, &[edit(&["persistent", "mod:bar"], Some(Nbt::Int(40))), edit(&[], Some(Nbt::Int(1))), edit(&["absent", "key"], None)]), "the same value, no path and a missing entry change nothing");
        assert!(!data.contains_key("absent"), "removing does not make the compounds it would run through");
        assert!(edit_custom_data(&mut data, &[edit(&["persistent", "mod:bar"], Some(Nbt::Int(10))), edit(&["persistent", "mod:other"], Some(Nbt::Int(5))), edit(&["persistent", "mod:other"], None)]));
        let Some(Nbt::Compound(persistent)) = data.get("persistent") else { panic!("persistent compound") };
        assert_eq!((persistent.get("mod:bar"), persistent.contains_key("mod:other")), (Some(&Nbt::Int(10)), false));
        data.insert("flat".into(), Nbt::Int(1));
        assert!(edit_custom_data(&mut data, &[edit(&["flat", "inner"], Some(Nbt::Int(2)))]), "a value in the way becomes a compound");
        assert!(matches!(data.get("flat"), Some(Nbt::Compound(_))));
    }

    #[test]
    fn stack_slots_map_to_the_hosts_inventory_numbers() {
        assert_eq!(carried_slot_number(StackSlot::MainHand, 3), 3);
        assert_eq!(carried_slot_number(StackSlot::MainHand, 200), 8, "never past the hotbar");
        assert_eq!([StackSlot::Feet, StackSlot::Legs, StackSlot::Chest, StackSlot::Head].map(|slot| carried_slot_number(slot, 0)), [100, 101, 102, 103]);
        assert_eq!(carried_slot_number(StackSlot::OffHand, 0), -106);
        assert_eq!(armor_stack_slot("chest"), Some(StackSlot::Chest));
        assert_eq!(armor_stack_slot("body"), None);
    }

    #[test]
    fn a_damage_callbacks_actions_wait_for_the_wearers_next_tick() {
        remember_armor(77, 7777, vec![("chest".into(), StackData { item: "example:plate".into(), count: 1, ..StackData::default() })]);
        ServerLivingEntityEvents::MODIFY_DAMAGE.register(|hit| {
            if hit.damage.entity != 7777 { return; }
            hit.actions.borrow_mut().push(EntityAction::EditStackData { target: ActionTarget::Victim, slot: StackSlot::Chest, item: "example:plate".into(), edits: vec![StackDataEdit { path: vec!["bar".into()], value: Some(Nbt::Int(3)) }] });
        });
        let hit = |entity| LivingDamage { entity, kind: "minecraft:player".into(), damage_type: "minecraft:freeze".into(), attacker: -1, direct: -1, amount: 2.0 };
        modify_damage(&hit(7777));
        modify_damage(&hit(7778));
        assert!(take_wearer_actions(7778).is_empty(), "a callback that only listens for one wearer queues for that one");
        let queued = take_wearer_actions(7777);
        assert_eq!(queued.len(), 1);
        assert!(matches!(&queued[0], (-1, EntityAction::EditStackData { slot: StackSlot::Chest, .. })));
        assert!(take_wearer_actions(7777).is_empty());
        forget_armor(77);
    }

    #[test]
    fn a_players_attack_can_ask_for_an_action_on_the_attacker_that_waits_for_their_next_tick() {
        ServerLivingEntityEvents::MODIFY_DAMAGE.register(|hit| {
            if hit.damage.entity != 5151 { return; }
            hit.actions.borrow_mut().push(EntityAction::Heal { target: ActionTarget::Attacker, amount: hit.amount.get() * 0.5 });
            hit.actions.borrow_mut().push(EntityAction::Hurt { target: ActionTarget::Victim, amount: 1.0, damage_type: "example:echo".into() });
        });
        let hit = |attacker| LivingDamage { entity: 5151, kind: "minecraft:husk".into(), damage_type: "minecraft:player_attack".into(), attacker, direct: attacker, amount: 8.0 };
        let sword = || StackData { item: "example:sword".into(), count: 1, ..StackData::default() };
        // A hit with no player's stack behind it asks for nothing for an attacker (a mob's heal is not a lifesteal).
        modify_damage(&hit(9191));
        assert!(take_attacker_actions(9191).is_empty());
        attack_started(5151, Some(sword()));
        modify_damage(&hit(9191));
        assert!(take_attacker_actions(9192).is_empty(), "another player's tick takes nothing");
        assert_eq!(take_attacker_actions(9191), vec![(5151, EntityAction::Heal { target: ActionTarget::Attacker, amount: 4.0 })], "the victim's own action stays off the attacker's queue");
        assert!(take_attacker_actions(9191).is_empty(), "taken once");
        attack_started(5151, Some(sword()));
        modify_damage(&hit(-1));
        assert!(take_attacker_actions(-1).is_empty(), "no attacker, nothing queued");
    }

    #[test]
    fn modify_hurt_runs_as_the_damage_arrives_and_leaves_the_hit_for_the_damage_stage() {
        remember_armor(88, 8888, vec![("chest".into(), StackData { item: "example:plate".into(), count: 1, ..StackData::default() })]);
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let log = seen.clone();
        ServerLivingEntityEvents::MODIFY_HURT.register(move |hit| {
            if hit.damage.entity != 8888 { return; }
            log.lock().unwrap().push((hit.amount.get(), hit.attacker_state.as_ref().and_then(|state| state.equipment.clone())));
            hit.amount.set(hit.amount.get() + 1.0);
            hit.actions.borrow_mut().push(EntityAction::AddEffect { target: ActionTarget::Victim, effect: "minecraft:regeneration".into(), amplifier: 0, duration: 40 });
        });
        let hurt = LivingDamage { entity: 8888, kind: "minecraft:player".into(), damage_type: "minecraft:mob_attack".into(), attacker: 3, direct: 3, amount: 5.0 };
        // Two reports of the one hit: the first knows the attacker's equipment, the second does not; together they describe it.
        hit_states(8888, Some(EntityState { kind: "minecraft:zombie".into(), equipment: Some(vec!["mainhand".into()]), ..EntityState::default() }), None);
        hit_states(8888, Some(EntityState { kind: "minecraft:zombie".into(), ..EntityState::default() }), None);
        assert_eq!(modify_hurt(&hurt), 6.0, "the hurt callbacks changed the amount as it arrived");
        assert_eq!(*seen.lock().unwrap(), vec![(5.0, Some(vec!["mainhand".to_string()]))], "the reports together kept the equipment");
        let queued = take_wearer_actions(8888);
        assert_eq!(queued.len(), 1);
        assert!(matches!(&queued[0], (3, EntityAction::AddEffect { effect, .. }) if effect == "minecraft:regeneration"));
        // The damage stage that follows reads the same hit: the hurt stage borrowed the states, it did not spend them.
        let seen_damage = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let log_damage = seen_damage.clone();
        ServerLivingEntityEvents::MODIFY_DAMAGE.register(move |hit| if hit.damage.entity == 8888 { log_damage.lock().unwrap().push(hit.attacker_state.as_ref().and_then(|state| state.equipment.clone())); });
        modify_damage(&hurt);
        assert_eq!(*seen_damage.lock().unwrap(), vec![Some(vec!["mainhand".to_string()])]);
        forget_armor(88);
    }

    #[test]
    fn an_effect_level_is_the_amplifier_plus_one() {
        let state = EntityState { effects: vec!["minecraft:poison".into()], effect_levels: vec![("minecraft:poison".into(), 1)], ..EntityState::default() };
        assert_eq!((state.effect_level("minecraft:poison"), state.effect_level("minecraft:speed")), (2, 0));
    }

    #[test]
    fn the_climate_of_a_vanilla_biome_cools_with_height_and_decides_rain() {
        assert_eq!(biome_climate("minecraft:plains"), Some((0.8, true)));
        assert_eq!(biome_climate("minecraft:desert"), Some((2.0, false)));
        assert_eq!(biome_climate("example:glow_fields"), None);
        assert_eq!(temperature_at(0.8, 80), 0.8);
        assert!((temperature_at(0.8, 120) - 0.75).abs() < 1e-6, "0.05 cooler for 40 blocks above y 80");
        assert!(rain_exposed(true, 15, Some("minecraft:plains"), 64));
        assert!(!rain_exposed(true, 15, Some("minecraft:desert"), 64), "a desert has no rain");
        assert!(!rain_exposed(true, 15, Some("minecraft:snowy_plains"), 64), "a cold biome snows");
        assert!(!rain_exposed(true, 15, Some("minecraft:plains"), 700), "the air that high is too cold for rain");
        assert!(!rain_exposed(true, 14, Some("minecraft:plains"), 64), "a roof shuts the rain out");
        assert!(!rain_exposed(false, 15, Some("minecraft:plains"), 64));
        assert!(rain_exposed(true, 15, Some("example:glow_fields"), 64), "an unknown biome is taken to rain");
        assert!(holds_water("minecraft:kelp", false) && holds_water("minecraft:stone", true) && !holds_water("minecraft:air", false));
    }

    #[test]
    fn saved_data_gives_health_the_modified_maximum_and_armor_coverage() {
        use super::super::base::{Nbt, NbtCompound};
        let compound = |entries: Vec<(&str, Nbt)>| -> NbtCompound { entries.into_iter().map(|(key, value)| (key.to_owned(), value)).collect() };
        let stack = |id: &str| Nbt::Compound(compound(vec![("id", Nbt::String(id.into())), ("count", Nbt::Int(1))]));
        let data = compound(vec![
            ("Health", Nbt::Float(7.5)),
            ("attributes", Nbt::List(vec![
                Nbt::Compound(compound(vec![("id", Nbt::String("minecraft:movement_speed".into())), ("base", Nbt::Double(0.2))])),
                Nbt::Compound(compound(vec![("id", Nbt::String("minecraft:max_health".into())), ("base", Nbt::Double(20.0)), ("modifiers", Nbt::List(vec![
                    Nbt::Compound(compound(vec![("amount", Nbt::Double(4.0)), ("operation", Nbt::String("add_value".into()))])),
                    Nbt::Compound(compound(vec![("amount", Nbt::Double(0.5)), ("operation", Nbt::String("add_multiplied_base".into()))])),
                    Nbt::Compound(compound(vec![("amount", Nbt::Double(1.0)), ("operation", Nbt::String("add_multiplied_total".into()))])),
                ]))])),
            ])),
            ("equipment", Nbt::Compound(compound(vec![("head", stack("minecraft:iron_helmet")), ("feet", stack("minecraft:air")), ("mainhand", stack("minecraft:stick"))]))),
        ]);
        // (20 + 4) * (1 + 0.5) * (1 + 1).
        assert_eq!(saved_facts(&data), SavedFacts { health: Some(7.5), max_health: Some(72.0), armor_coverage: Some(0.25), equipment: Some(vec!["mainhand".into(), "head".into()]), armor_stacks: Vec::new() });
        assert_eq!(saved_facts(&NbtCompound::new()), SavedFacts::default(), "data that says nothing reports nothing");
        assert_eq!(armor_coverage(9), 1.0);
    }

    #[test]
    fn saved_data_gives_the_worn_armor_pieces_that_carry_custom_data() {
        use super::super::base::{Nbt, NbtCompound};
        let compound = |entries: Vec<(&str, Nbt)>| -> NbtCompound { entries.into_iter().map(|(key, value)| (key.to_owned(), value)).collect() };
        let tool_helmet = Nbt::Compound(compound(vec![
            ("id", Nbt::String("tconstruct:travelers_helmet".into())), ("count", Nbt::Int(1)),
            ("components", Nbt::Compound(compound(vec![("minecraft:custom_data", Nbt::Compound(compound(vec![("tconstruct:materials", Nbt::String("x".into()))])))]))),
        ]));
        let plain_plate = Nbt::Compound(compound(vec![("id", Nbt::String("minecraft:iron_chestplate".into())), ("count", Nbt::Int(1))]));
        let data = compound(vec![("equipment", Nbt::Compound(compound(vec![("head", tool_helmet), ("chest", plain_plate)])))]);
        let facts = saved_facts(&data);
        assert_eq!(facts.armor_stacks.len(), 1, "only the piece carrying custom data is reported: {:?}", facts.armor_stacks);
        assert_eq!(facts.armor_stacks[0].0, "head");
        assert_eq!(facts.armor_stacks[0].1.item, "tconstruct:travelers_helmet");
        assert!(facts.armor_stacks[0].1.custom_data.contains_key("tconstruct:materials"), "the piece's own data rides along");
    }

    #[test]
    fn modify_damage_carries_a_mobs_worn_armor_from_the_hits_own_report() {
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let log = seen.clone();
        ServerLivingEntityEvents::MODIFY_DAMAGE.register(move |hit| if hit.damage.entity == 7171 { log.lock().unwrap().push(hit.armor.iter().map(|(slot, stack)| format!("{slot}:{}", stack.item)).collect::<Vec<_>>()); });
        let helmet = StackData { item: "tconstruct:travelers_helmet".into(), count: 1, ..StackData::default() };
        let state = EntityState { kind: "tconstruct:sky_slime".into(), armor_stacks: vec![("head".into(), helmet)], ..EntityState::default() };
        hit_states(7171, None, Some(state));
        let hurt = LivingDamage { entity: 7171, kind: "tconstruct:sky_slime".into(), damage_type: "minecraft:player_attack".into(), attacker: 3, direct: 3, amount: 5.0 };
        modify_damage(&hurt);
        assert_eq!(*seen.lock().unwrap(), vec![vec!["head:tconstruct:travelers_helmet".to_string()]]);
    }

    #[test]
    fn a_hit_on_a_wearer_reports_the_state_remembered_at_their_last_tick() {
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let log = seen.clone();
        ServerLivingEntityEvents::MODIFY_DAMAGE.register(move |hit| {
            if hit.damage.entity == 515151 { log.lock().unwrap().push(hit.target_state.as_ref().and_then(|state| state.health)); }
        });
        let damage = LivingDamage { entity: 515151, kind: "minecraft:player".into(), damage_type: "minecraft:mob_attack".into(), attacker: 3, direct: 3, amount: 4.0 };
        let piece = StackData { item: "example:plate".into(), count: 1, ..StackData::default() };
        remember_armor(5151, 515151, vec![("chest".into(), piece)]);
        modify_damage(&damage);
        remember_wearer_state(515151, Some(EntityState { health: Some(12.0), max_health: Some(20.0), ..EntityState::default() }));
        modify_damage(&damage);
        // A state that came with the hit wins over the remembered one.
        hit_states(515151, None, Some(EntityState { health: Some(3.0), ..EntityState::default() }));
        modify_damage(&damage);
        forget_armor(5151);
        modify_damage(&damage);
        assert_eq!(*seen.lock().unwrap(), [None, Some(12.0), Some(3.0), None], "forgetting the wearer forgets the state");
    }

    #[test]
    fn the_sky_is_darkest_at_midnight_and_a_storm_darkens_the_day() {
        assert_eq!(sky_darken(0.0, 0.0, 6000), 0, "clear noon");
        assert_eq!(sky_darken(0.0, 0.0, 18000), 11, "clear midnight");
        assert_eq!(sky_darken(0.0, 0.0, 6000 + 24000 * 3), 0, "the day count does not matter");
        // 1 - 1.0 * (1 - 5/16) * (1 - 5/16) = 0.5273, times 11 and cut.
        assert_eq!(sky_darken(1.0, 1.0, 6000), 5, "a thunderstorm at noon");
        assert_eq!(sky_darken(1.0, 0.0, 6000), 3, "rain alone at noon");
        assert!((0..24000).step_by(500).all(|time| sky_darken(0.0, 0.0, time) <= 11));
    }

    #[test]
    fn effects_are_categorised_for_the_vanilla_ids_only() {
        assert_eq!(effect_category("minecraft:regeneration"), Some(EffectCategory::Beneficial));
        assert_eq!(effect_category("minecraft:poison"), Some(EffectCategory::Harmful));
        assert_eq!(effect_category("minecraft:glowing"), Some(EffectCategory::Neutral));
        assert_eq!(effect_category("tconstruct:momentum_armor"), None, "another mod's effect is not vanilla's to categorise");
    }

    #[test]
    fn a_tick_hears_its_callbacks_and_hands_back_the_actions_they_queued() {
        ServerLivingEntityEvents::PLAYER_TICK.register(|tick| {
            if tick.entity != 6262 { return; }
            tick.actions.borrow_mut().push(EntityAction::Hurt { target: ActionTarget::Attacker, amount: 2.0, damage_type: "example:tangled".into() });
        });
        let tick = PlayerTick { player: 1, entity: 6262, dimension: "minecraft:overworld".to_owned(), position: [0.0; 3], on_ground: true, sneaking: false, gliding: false, spectator: false, in_water: false, armor: Vec::new(), hands: Vec::new(), block_light: 0, sky_light: 0, sky_darken: 0, effects: Vec::new(), remove_effects: RefCell::new(Vec::new()), attribute_additions: RefCell::new(Vec::new()), actions: RefCell::new(Vec::new()) };
        player_tick(&tick);
        assert_eq!(take_tick_actions(&tick), vec![EntityAction::Hurt { target: ActionTarget::Attacker, amount: 2.0, damage_type: "example:tangled".into() }]);
        assert!(take_tick_actions(&tick).is_empty(), "taken once");
    }

    #[test]
    fn a_critical_hit_callback_may_force_the_crit_and_raise_the_multiplier() {
        ServerLivingEntityEvents::CRITICAL_HIT.register(|hit| {
            if hit.attacker_entity != 7701 { return; }
            if !hit.attacker_on_ground && !hit.attacker_in_water && !hit.critical.get() {
                hit.critical.set(true);
                hit.damage_multiplier.set(hit.damage_multiplier.get() + 0.5);
            }
        });
        let hit = CriticalHit {
            attacker_entity: 7701, attacker_on_ground: false, attacker_in_water: false,
            target_kind: "minecraft:zombie".into(), vanilla_critical: false,
            critical: Cell::new(false), damage_multiplier: Cell::new(1.0),
        };
        critical_hit(&hit);
        assert!(hit.critical.get(), "the airborne hit is forced critical");
        assert_eq!(hit.damage_multiplier.get(), 1.5);
        let grounded = CriticalHit {
            attacker_entity: 7701, attacker_on_ground: true, attacker_in_water: false,
            target_kind: "minecraft:zombie".into(), vanilla_critical: false,
            critical: Cell::new(false), damage_multiplier: Cell::new(1.0),
        };
        critical_hit(&grounded);
        assert!(!grounded.critical.get(), "a grounded hit is not forced");
        assert_eq!(grounded.damage_multiplier.get(), 1.0);
    }

    #[test]
    fn a_jump_is_reported_with_its_kind() {
        use std::sync::Mutex;
        static JUMPED: Mutex<Vec<(i32, String)>> = Mutex::new(Vec::new());
        ServerLivingEntityEvents::LIVING_JUMP.register(|jump| JUMPED.lock().unwrap().push((jump.entity, jump.kind.clone())));
        living_jump(&LivingJump { entity: 7702, kind: "minecraft:player".into() });
        assert_eq!(JUMPED.lock().unwrap().clone(), vec![(7702, "minecraft:player".to_owned())]);
    }

    #[test]
    fn a_movement_input_callback_may_scale_the_impulses() {
        ServerLivingEntityEvents::MOVEMENT_INPUT.register(|input| {
            if input.entity != 7703 || !input.using_item { return; }
            input.forward.set(input.forward.get() * 0.5);
            input.strafe.set(input.strafe.get() * 0.5);
        });
        let input = MovementInput {
            entity: 7703, forward: Cell::new(1.0), strafe: Cell::new(0.4),
            jump: Cell::new(false), sneak: Cell::new(false), using_item: true, using: None,
        };
        movement_input(&input);
        assert_eq!(input.forward.get(), 0.5);
        assert!((input.strafe.get() - 0.2).abs() < 1e-9);
        let idle = MovementInput {
            entity: 7703, forward: Cell::new(1.0), strafe: Cell::new(0.4),
            jump: Cell::new(false), sneak: Cell::new(false), using_item: false, using: None,
        };
        movement_input(&idle);
        assert_eq!(idle.forward.get(), 1.0, "not using an item, the impulses stand");
    }

    #[test]
    fn a_movement_input_carries_the_stack_being_used() {
        use std::sync::Mutex;
        static USED: Mutex<Vec<Option<StackData>>> = Mutex::new(Vec::new());
        ServerLivingEntityEvents::MOVEMENT_INPUT.register(|input| {
            if input.entity != 7704 { return; }
            USED.lock().unwrap().push(input.using.clone());
        });
        let bow = StackData { item: "tconstruct:longbow".into(), count: 1, ..StackData::default() };
        movement_input(&MovementInput {
            entity: 7704, forward: Cell::new(1.0), strafe: Cell::new(0.4),
            jump: Cell::new(false), sneak: Cell::new(false), using_item: true, using: Some(bow.clone()),
        });
        movement_input(&MovementInput {
            entity: 7704, forward: Cell::new(1.0), strafe: Cell::new(0.4),
            jump: Cell::new(false), sneak: Cell::new(false), using_item: false, using: None,
        });
        assert_eq!(USED.lock().unwrap().clone(), vec![Some(bow), None], "the used stack rides the event, and none when nothing is used");
    }

    #[test]
    fn a_multiply_total_fold_applies_as_a_ratio_and_unfolds() {
        let player = 77_001u128;
        let speed = "minecraft:movement_speed".to_owned();
        assert_eq!(multiply_factor_changes(player, &[(speed.clone(), 0.1)]), vec![(speed.clone(), 0.1, 0.1)], "folding from 1.0 to 0.1 is a ratio of 0.1");
        multiply_factor_applied(player, &speed, 0.1);
        assert!(multiply_factor_changes(player, &[(speed.clone(), 0.1)]).is_empty(), "the same factor again changes nothing");
        assert_eq!(multiply_factor_changes(player, &[(speed.clone(), 1.0)]), vec![(speed.clone(), 10.0, 1.0)], "unfolding is a ratio of 10");
        multiply_factor_applied(player, &speed, 1.0);
        assert!(multiply_factor_changes(player, &[(speed.clone(), 1.0)]).is_empty(), "unfolded, nothing to do");
        multiply_factor_applied(player, &speed, 0.1);
        assert_eq!(multiply_factor_changes(player, &[]), vec![(speed.clone(), 10.0, 1.0)], "a fold no longer wanted unfolds");
        multiply_factor_applied(player, &speed, 1.0);
        assert_eq!(forget_multiply_factors(player), Vec::<(String, f64)>::new(), "nothing held after the unfold");
        multiply_factor_applied(player, &speed, 0.1);
        assert_eq!(forget_multiply_factors(player), vec![(speed, 0.1)], "the fold is handed back on leaving");
        assert!(forget_multiply_factors(player).is_empty(), "forgotten once");
    }

    #[test]
    fn an_attribute_addition_total_is_read_for_the_fold() {
        let player = 77_002u128;
        assert_eq!(attribute_applied_total(player, "minecraft:armor"), 0.0, "nothing applied yet");
        attribute_applied(player, "minecraft:armor", 0.5);
        assert_eq!(attribute_applied_total(player, "minecraft:armor"), 0.5);
        attribute_applied(player, "minecraft:armor", 0.0);
        assert_eq!(attribute_applied_total(player, "minecraft:armor"), 0.0, "a zero total clears it");
    }

    #[test]
    fn an_entity_tick_can_be_cancelled_before_and_is_reported_after() {
        use std::sync::Mutex;
        static POSTED: Mutex<Vec<u32>> = Mutex::new(Vec::new());
        ServerEntityEvents::ENTITY_TICK_PRE.register(|info| info.kind != "test:frozen");
        ServerEntityEvents::ENTITY_TICK_POST.register(|info| POSTED.lock().unwrap().push(info.id));
        assert!(entity_tick_heard());
        let frozen = EntityInfo { id: 71, kind: "test:frozen".into(), dimension: "minecraft:overworld".into() };
        let moving = EntityInfo { id: 72, kind: "test:walker".into(), dimension: "minecraft:overworld".into() };
        assert!(!entity_tick_before(&frozen), "the frozen kind's tick is cancelled");
        assert!(entity_tick_before(&moving));
        entity_tick_after(&moving);
        assert_eq!(POSTED.lock().unwrap().clone(), vec![72], "only the tick that ran is reported");
    }
}
