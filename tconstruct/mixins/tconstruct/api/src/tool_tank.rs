//! The tool tank: a fluid stored in a tool's own persistent data, and the modifier modules that
//! work through it (SlimeKnights ToolTankHelper, TankModule and the tank modules of
//! slimeknights.tconstruct.tools.modules, MIT; see LICENSE).
//!
//! The tank itself is PlutoLib's stack-data tank (`pluto_lib::tool_tank_api_v1`) under the key
//! `tconstruct:tank_fluid` of the tool's persistent data, in the original's FluidStack NBT shape.
//! Its capacity is the `tconstruct:tank_capacity` stat (ToolTankHelper.CAPACITY_STAT), fed by the
//! stat boosts of the tank trait's family; the tank exists on a tool exactly when the tool's
//! modifier list carries the `tconstruct:tank_handler` trait, whose own module is the TankModule
//! ported here as [`Module::Tank`](crate::modifier_modules::Module::Tank) (the handler is a
//! code-defined modifier in the original too: ModuleHookMap with a TankModule, priority 300,
//! no-levels display).
//!
//! What is wired in game today: the capacity stat and its boosts, the tank handler trait, the
//! validate clamp on rebuild (TankModule.validate), the removal cleanup, and melting's process
//! loot on block drops (block_break_hook). The rest is the modules' logic over the tool's data,
//! ready for its consumers, with the same standing as the overgrowth module: the effects
//! themselves run through the fluid effects engine, which needs a live world context the
//! interaction and hit events do not carry, and the inventory tick event
//! (`PlayerInventoryEvents::TICK`) reports neither the holder's tick count nor the level's game
//! time that overburn's gating reads, nor a way to write the stack back. Each module below says
//! exactly what its consumer still owes.
use crate::{
    fluid_effects::{LevelingInt, LevelingValue},
    fluid_values::{OreRateType, BOTTLE, BUCKET},
    modifier_entries::Entry,
    modifiers::Modifier,
    overslime,
    tool_stack::{saved_modifiers, stat, with_persistent, Environment, TAG_PERSISTENT},
    tool_stats::Stat,
};
use pluto_lib::{FluidStack, Identifier, Nbt, NbtCompound};
use serde_json::{json, Map, Value};

/// ToolTankHelper's fluid key in the tool's persistent data.
pub(crate) const FLUID_KEY: &str = "tconstruct:tank_fluid";
/// The trait modifier that gives a tool its tank (ToolTankHelper.TANK_HANDLER).
pub(crate) fn handler_id() -> Identifier { Identifier::new("tconstruct", "tank_handler").expect("valid id") }

/// The code-defined modifiers of the tank: just the tank handler (TinkerModifiers.tankHandler:
/// a ModuleHookMap holding one TankModule, level display no-levels, priority 300).
pub(crate) fn code_modifiers() -> Result<Vec<Modifier>, String> {
    let handler = Modifier::decode(handler_id(), &json!({"priority": 300, "level_display": "tconstruct:no_levels",
        "modules": [{"type": "tconstruct:tank", "hooks": ["tconstruct:volatile_data", "tconstruct:validate", "tconstruct:remove", "tconstruct:slot_stack", "tconstruct:display_name"]}]}))?;
    Ok(vec![handler])
}

/// ToolTankHelper.getCapacity: the tank capacity stat as an int.
pub(crate) fn capacity(nbt: &NbtCompound) -> i32 { stat(nbt, Stat::TankCapacity) as i32 }

/// Whether the tool has a tank: its saved modifier list carries the tank handler trait.
pub(crate) fn has_tank(nbt: &NbtCompound, env: &Environment) -> bool {
    saved_modifiers(nbt, env.modifiers).iter().any(|(entry, _)| entry.id == handler_id())
}

fn persistent(nbt: &NbtCompound) -> NbtCompound {
    match nbt.get(TAG_PERSISTENT) { Some(Nbt::Compound(data)) => data.clone(), _ => NbtCompound::new() }
}

/// ToolTankHelper.getFluid: the fluid in the tool's tank.
pub(crate) fn fluid(nbt: &NbtCompound) -> Option<FluidStack> { pluto_lib::tool_tank_api_v1::read(&persistent(nbt), FLUID_KEY) }

/// ToolTankHelper.setFluid: stores the fluid (clamped to the capacity); empty removes it.
/// Returns what is stored afterwards.
pub(crate) fn set_fluid(nbt: &mut NbtCompound, fluid: Option<&FluidStack>) -> Option<FluidStack> {
    let capacity = capacity(nbt).max(0) as u32;
    with_persistent(nbt, |data| pluto_lib::tool_tank_api_v1::write(data, FLUID_KEY, fluid, capacity))
}

/// TankModule.fill through the tool's tank: how much of `resource` is taken.
pub(crate) fn fill(nbt: &mut NbtCompound, resource: &FluidStack, execute: bool) -> i32 {
    let capacity = capacity(nbt).max(0) as u32;
    with_persistent(nbt, |data| pluto_lib::tool_tank_api_v1::fill(data, FLUID_KEY, capacity, resource, execute) as i32)
}

/// TankModule.drain(FluidStack) through the tool's tank.
pub(crate) fn drain_fluid(nbt: &mut NbtCompound, wanted: &FluidStack, execute: bool) -> Option<FluidStack> {
    with_persistent(nbt, |data| pluto_lib::tool_tank_api_v1::drain(data, FLUID_KEY, wanted, execute))
}

/// TankModule.drain(int) through the tool's tank.
pub(crate) fn drain(nbt: &mut NbtCompound, max: i32, execute: bool) -> Option<FluidStack> {
    if max <= 0 { return None; }
    with_persistent(nbt, |data| pluto_lib::tool_tank_api_v1::drain_amount(data, FLUID_KEY, max as u32, execute))
}

/// TankModule.validate: an overfull tank is clamped to the capacity (the fluid itself is kept).
/// Runs on rebuild (tool_stack::rebuild_stats), as the original's validate hook runs when a
/// tool's data is rebuilt. Returns whether the stored stack changed.
pub(crate) fn validate_clamp(nbt: &mut NbtCompound) -> bool {
    let capacity = capacity(nbt).max(0) as u32;
    with_persistent(nbt, |data| pluto_lib::tool_tank_api_v1::validate(data, FLUID_KEY, capacity))
}

/// Shrinks the tank by `amount` (the modules' `fluid.shrink(consumed)` before setFluid, when a
/// fluid effect consumed some of the tank). Returns how much left the tank.
pub(crate) fn consume(nbt: &mut NbtCompound, amount: i32) -> i32 {
    if amount <= 0 { return 0; }
    with_persistent(nbt, |data| pluto_lib::tool_tank_api_v1::drain_amount(data, FLUID_KEY, amount as u32, true).map_or(0, |drained| drained.amount as i32))
}

/// MeltingModule: melts dropped items and hit entities into the tank.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Melting {
    temperature: LevelingInt,
    nuggets_per_metal: LevelingInt,
    shards_per_gem: LevelingInt,
}

impl Melting {
    /// All three fields are required leveling ints (MeltingModule.LOADER).
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Self, String> {
        let leveling = |key: &str| -> Result<LevelingInt, String> {
            LevelingInt::decode(fields.get(key).ok_or_else(|| format!("{key} must be a leveling int"))?)
        };
        Ok(Self { temperature: leveling("temperature")?, nuggets_per_metal: leveling("nuggets_per_metal")?, shards_per_gem: leveling("shards_per_gem")? })
    }

    /// The hottest recipe this entry melts (MeltingModule.meltItem's temperature gate).
    pub(crate) fn temperature(&self, level: f32) -> i32 { self.temperature.compute(level) }

    /// MeltingModule.applyOreBoost (the module is its own melting container's ore rate): both
    /// rates divide by 9 here — the melter's Config.OreRate divides gems by 4 instead, which is
    /// a different rate for a different container.
    pub(crate) fn ore_boost(&self, level: f32, rate: OreRateType, amount: i32) -> i32 {
        match rate {
            OreRateType::Metal => amount.wrapping_mul(self.nuggets_per_metal.compute(level)) / 9,
            OreRateType::Gem => amount.wrapping_mul(self.shards_per_gem.compute(level)) / 9,
            OreRateType::None | OreRateType::Default => amount,
        }
    }

    /// MeltingModule.meltTarget's amount for one hit: the recipe output scaled by the damage
    /// dealt over the recipe's damage per output, capped at twice the output.
    pub(crate) fn entity_amount(output: i32, damage_per_output: i32, damage_dealt: f32) -> i32 {
        if damage_per_output <= 0 { return 0; }
        if damage_dealt < (damage_per_output * 2) as f32 { (output as f32 * damage_dealt / damage_per_output as f32) as i32 } else { output.wrapping_mul(2) }
    }

    /// EntityMeltingModule.getDefaultFluid: liquid soul, a fifth of a glass pane (50 mb), for
    /// targets with no entity melting recipe (their damage per output is 2, in meltTarget).
    pub(crate) fn default_entity_fluid() -> FluidStack {
        FluidStack::new(Identifier::new("tconstruct", "liquid_soul").expect("valid id"), (crate::fluid_values::GLASS_PANE / 5) as u32)
    }
}

/// What a worn piece's timed drink or charge does on this tick (SlurpingModule and
/// SpittingModule's inventory tick halves share the timing).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ArmorTick {
    /// No drink is running, or it is not a sound tick.
    Idle,
    /// A sound (and particles) tick: every 4 ticks while the timer runs.
    Sound,
    /// The timer expired on this tick: the caller runs the finish and the stored key is gone.
    Finished,
}

/// The persistent timer both armor forms keep under the modifier's own ID: the holder tick at
/// which the drink or charge finishes (SlurpingModule.startInteract, SpittingModule's helmet
/// startInteract through HelmetChargingEffect).
pub(crate) fn armor_charge_start(persistent: &mut NbtCompound, key: &Identifier, finish_tick: i32) {
    persistent.insert(key.to_string(), Nbt::Int(finish_tick));
}

/// One tick of the armor timer; an expired timer clears its own key, as both modules remove it
/// when they finish.
pub(crate) fn armor_charge_tick(persistent: &mut NbtCompound, key: &Identifier, now: i32) -> ArmorTick {
    let finish = crate::modifier_entries::nbt_int(persistent.get(&key.to_string()));
    if finish <= 0 { return ArmorTick::Idle; }
    let left = finish.wrapping_sub(now);
    if left < 0 {
        persistent.remove(&key.to_string());
        ArmorTick::Finished
    } else if left % 4 == 0 {
        ArmorTick::Sound
    } else {
        ArmorTick::Idle
    }
}

/// SlurpingModule: drinking the tank's fluid applies its entity effects to the holder.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Slurping {
    strength: LevelingValue,
    duration: LevelingInt,
}

impl Slurping {
    /// Both fields are required (SlurpingModule.LOADER).
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Self, String> {
        Ok(Self {
            strength: LevelingValue::decode(fields.get("strength").ok_or("strength must be a leveling value")?)?,
            duration: LevelingInt::decode(fields.get("duration").ok_or("duration must be a leveling int")?)?,
        })
    }

    /// The effect level a drink applies at (FluidEffects.apply's level for the entry).
    pub(crate) fn strength(&self, entry: &Entry) -> f32 { self.strength.compute(entry.effective_level()) }

    /// GeneralInteractionModifierHook.getUseDuration for a drink, and the armor form's timer
    /// length: the duration at the entry's effective level.
    pub(crate) fn duration(&self, entry: &Entry) -> i32 { self.duration.compute(entry.effective_level()) }

    /// KeybindInteractModifierHook.startInteract's store: the finish tick under the modifier ID.
    /// Whether the drink may start at all (the SIMULATE slurp) is the caller's, as it applies
    /// the fluid's effects to the holder.
    pub(crate) fn armor_start(&self, persistent: &mut NbtCompound, entry: &Entry, now: i32) {
        armor_charge_start(persistent, &entry.id, now.wrapping_add(self.duration(entry)));
    }

    /// The worn piece's inventory tick (with isCorrectSlot and the worn armor tag, which are the
    /// caller's gates): on Finished the caller finishes the drink (SlurpingModule.finishDrinking:
    /// the effects, then the tank shrinks by what they consumed, through [`consume`]).
    pub(crate) fn armor_tick(&self, persistent: &mut NbtCompound, entry: &Entry, now: i32) -> ArmorTick {
        armor_charge_tick(persistent, &entry.id, now)
    }

    /// stopInteract and onUnequip's clear (the unequip half only when the tool actually
    /// changed, which is the caller's comparison).
    pub(crate) fn stop(&self, persistent: &mut NbtCompound, entry: &Entry) { persistent.remove(&entry.id.to_string()); }
}

/// SpillingModule: a hit applies the tank fluid's entity effects to the target. The module's
/// own JSON is the leveling value itself (LevelingValue.LOADABLE.directField), so the level is
/// read from the module's root fields.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Spilling {
    level: LevelingValue,
}

impl Spilling {
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Self, String> {
        let part = |key: &str| -> Result<f32, String> {
            match fields.get(key).filter(|value| !value.is_null()) {
                None => Ok(0.0),
                Some(value) => value.as_f64().map(|number| number as f32).ok_or_else(|| format!("{key} must be a number")),
            }
        };
        Ok(Self { level: LevelingValue { flat: part("flat")?, each_level: part("each_level")? } })
    }

    /// The effect level a spill applies at (applyToEntity's level argument).
    pub(crate) fn level(&self, entry: &Entry) -> f32 { self.level.compute(entry.effective_level()) }
}

/// SpittingModule: charging and releasing fires the tank's fluid as projectiles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Spitting {
    shots: LevelingInt,
}

/// One shot plan of SpittingModule.spit: how many projectiles, how much fluid each carries,
/// and the power and velocity they are fired with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SpitPlan {
    pub shots: i32,
    pub amount_per_shot: i32,
    pub power: f32,
    pub velocity: f32,
}

impl Spitting {
    /// The shots field is a required leveling int (SpittingModule.LOADER).
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Self, String> {
        Ok(Self { shots: LevelingInt::decode(fields.get("shots").ok_or("shots must be a leveling int")?)? })
    }

    /// The number of projectiles at the entry's effective level.
    pub(crate) fn shots(&self, entry: &Entry) -> i32 { self.shots.compute(entry.effective_level()) }

    /// onToolUse's gate (the broken check is the caller's): the tank must hold at least the
    /// entry's plain level of a fluid that has effects.
    pub(crate) fn can_start(entry: &Entry, fluid: Option<&FluidStack>, has_effects: bool) -> bool {
        has_effects && fluid.is_some_and(|fluid| fluid.amount as i32 >= entry.level)
    }

    /// KeybindInteractModifierHook.startInteract's store for the helmet form: the finish tick
    /// under the modifier ID, `charge_time` being HelmetChargingEffect's draw time for the tool.
    pub(crate) fn armor_start(&self, persistent: &mut NbtCompound, entry: &Entry, now: i32, charge_time: i32) {
        armor_charge_start(persistent, &entry.id, now.wrapping_add(charge_time));
    }

    /// The helmet form's inventory tick: on Finished the breath sound and particles play and
    /// the key is gone (the spit itself happens in stopInteract, with its charge time).
    pub(crate) fn armor_tick(&self, persistent: &mut NbtCompound, entry: &Entry, now: i32) -> ArmorTick {
        armor_charge_tick(persistent, &entry.id, now)
    }

    /// stopInteract's clear.
    pub(crate) fn stop(&self, persistent: &mut NbtCompound, entry: &Entry) { persistent.remove(&entry.id.to_string()); }

    /// SpittingModule.spit's arithmetic. `charge` is the tool charge for the release
    /// (GeneralInteractionModifierHook.getToolCharge, the caller's), `amount_per_level` the
    /// fluid effect ingredient's amount for the tank's fluid, and the two stats the tool's
    /// modified projectile damage and velocity. None when nothing would be fired (no shots, or
    /// less than a millibucket per shot). Firing the projectiles and the tool damage of
    /// `shots` are the caller's; the tank then shrinks by `amount_per_shot * shots` through
    /// [`consume`] when the shooter consumes resources.
    pub(crate) fn plan(&self, entry: &Entry, fluid_amount: i32, amount_per_level: i32, charge: f32, projectile_damage: f32, velocity: f32) -> Option<SpitPlan> {
        let shots = self.shots(entry);
        if shots <= 0 { return None; }
        let power = charge * projectile_damage;
        let amount = fluid_amount.min((amount_per_level as f32 * power) as i32 * shots) / shots;
        if amount <= 0 { return None; }
        Some(SpitPlan { shots, amount_per_shot: amount, power, velocity: velocity * charge * 3.0 })
    }
}

/// SplashingModule: using the tool on an entity or block applies the tank fluid's effects,
/// over the tool's area of effect.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Splashing {
    strength: LevelingValue,
}

impl Splashing {
    /// The strength field is a required leveling value (SplashingModule.LOADER).
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Self, String> {
        Ok(Self { strength: LevelingValue::decode(fields.get("strength").ok_or("strength must be a leveling value")?)? })
    }

    /// The effect level a splash applies at.
    pub(crate) fn strength(&self, entry: &Entry) -> f32 { self.strength.compute(entry.effective_level()) }

    /// The item cooldown both forms apply before the effect: (int)(20 / draw speed).
    pub(crate) fn cooldown(draw_speed: f32) -> i32 { (20.0 / draw_speed) as i32 }

    /// The tool damage both forms charge afterwards: ceil(targets * level).
    pub(crate) fn damage(num_targets: i32, level: f32) -> i32 { (num_targets as f32 * level).ceil() as i32 }
}

/// MeltingFuelLookup's answer for a fluid, as OverburnModule reads it: the ingredient amount
/// one burn consumes, how long a burn lasts at the reference rate, and the burn's rate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Fuel {
    pub amount: i32,
    pub duration: i32,
    pub rate: i32,
}

/// OverburnModule.FuelInfo: the burn in progress, kept under the modifier's own ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FuelInfo {
    expiration: i64,
    rate: i32,
}

fn read_fuel_info(persistent: &NbtCompound, key: &str) -> Option<FuelInfo> {
    let Some(Nbt::Compound(info)) = persistent.get(key) else { return None };
    let expiration = match info.get("expiration") { Some(value) => value.as_i64().unwrap_or(0), None => 0 };
    Some(FuelInfo { expiration, rate: crate::modifier_entries::nbt_int(info.get("rate")) })
}

fn write_fuel_info(persistent: &mut NbtCompound, key: &str, info: FuelInfo) {
    let mut tag = NbtCompound::new();
    tag.insert("expiration".to_owned(), Nbt::Long(info.expiration));
    tag.insert("rate".to_owned(), Nbt::Int(info.rate));
    persistent.insert(key.to_owned(), Nbt::Compound(tag));
}

/// OverburnModule.onInventoryTick for one overburn entry: burns the tank's fluid as melting
/// fuel to regrow overslime. The caller gates the tick itself (server side, and the holder not
/// using this stack, which the original checks against the holder's use item); `holder_tick` is
/// the holder's tick count and `game_time` the level's game time, both of which the original
/// reads from the holder and the level. `fuel` answers MeltingFuelLookup for a fluid; `rolls`
/// gives one uniform value in [0, 1) when a burn's rate has a remainder. Returns whether the
/// tool changed. Nothing calls this in game yet: the inventory tick event reports neither the
/// holder's tick count nor the game time, and has no channel to write the stack back — the
/// same gap the overgrowth driver documents.
pub(crate) fn overburn_tick(nbt: &mut NbtCompound, env: &Environment, entry: &Entry, holder_tick: i32, game_time: i64, using_this: bool,
    fuel: &dyn Fn(&Identifier) -> Option<Fuel>, rolls: &mut dyn FnMut() -> f32) -> bool {
    // Levels above 6 leave the integer interval behind; the original caps the level at 6.
    let level = entry.level.min(6);
    let interval = 5 << (6 - level);
    if using_this || holder_tick % interval != 0 { return false; }
    if overslime::amount(nbt) >= overslime::capacity(nbt) { return false; }
    let before = nbt.clone();
    let key = entry.id.to_string();
    let mut info = read_fuel_info(&persistent(nbt), &key);
    let needed_fuel = info.is_none();
    if needed_fuel || info.is_some_and(|info| info.expiration < game_time) {
        info = None;
        if let Some(current) = fluid(nbt) {
            if let Some(found) = fuel(&current.fluid).filter(|found| found.amount > 0) {
                // Scale the duration by the interval so every level gets the same overslime
                // per fuel unit; a partial tank burns what is left for a scaled-down time.
                let mut duration = found.duration.wrapping_mul(interval) / 5;
                if found.amount > current.amount as i32 {
                    duration = duration.wrapping_mul(current.amount as i32) / found.amount;
                    set_fluid(nbt, None);
                } else {
                    let left = current.amount as i32 - found.amount;
                    if left > 0 {
                        let mut rest = current.clone();
                        rest.amount = left as u32;
                        set_fluid(nbt, Some(&rest));
                    } else {
                        set_fluid(nbt, None);
                    }
                }
                info = Some(FuelInfo { expiration: game_time + duration as i64, rate: found.rate });
            }
        }
        // Keep the burn only if it outlives the next interval; a burn that is already spent
        // is not worth the write, and a stale one is cleared.
        match info {
            Some(info) if info.expiration >= game_time + interval as i64 => with_persistent(nbt, |data| write_fuel_info(data, &key, info)),
            _ if !needed_fuel => { with_persistent(nbt, |data| data.remove(&key)); }
            _ => {}
        }
    }
    if let Some(info) = info {
        // Restore rate/10 per interval; the remainder is a chance of one more.
        let mut restore = info.rate / 10;
        let remainder = info.rate % 10;
        if remainder > 0 && (rolls() * 10.0) as i32 < remainder { restore += 1; }
        overslime::add_amount(nbt, env, restore);
    }
    *nbt != before
}

/// SmashingModule's keys in persistent data (its tank is not the tool tank: the amount is
/// derived from the fluid effect ingredient, never stored).
pub(crate) const SMASHING_FLUID: &str = "tconstruct:smashing_fluid";
pub(crate) const SMASHING_TAG: &str = "tconstruct:smashing_fluid_tag";
pub(crate) const SMASHING_AMOUNT: &str = "tconstruct:smashing_amount";
pub(crate) const SMASHING_VALIDATE: &str = "tconstruct:smashing_validate";
pub(crate) const SMASHING_USED: &str = "tconstruct:smashing_used";

/// The fluid a smashing tool (or projectile) holds: its id and tag, without the amount.
pub(crate) fn smashing_stored(data: &NbtCompound) -> Option<(Identifier, Option<NbtCompound>)> {
    let Some(Nbt::String(name)) = data.get(SMASHING_FLUID) else { return None };
    let fluid = Identifier::parse(name).ok()?;
    let tag = match data.get(SMASHING_TAG) { Some(Nbt::Compound(tag)) => Some(tag.clone()), _ => None };
    Some((fluid, tag))
}

/// SmashingModule.clearFluid: the fluid, its tag, the amount and the validation level go.
pub(crate) fn smashing_clear(data: &mut NbtCompound) {
    for key in [SMASHING_FLUID, SMASHING_VALIDATE, SMASHING_AMOUNT, SMASHING_TAG] { data.remove(key); }
}

/// SmashingModule.getAmount(modifier, fluid): the fluid effect ingredient's amount for the
/// fluid, times the entry's plain level. `amount_of` answers the ingredient amount, 0 for a
/// fluid without effects (the manager's no-entry answer).
pub(crate) fn smashing_amount(entry: &Entry, fluid: &Identifier, amount_of: &dyn Fn(&Identifier) -> i32) -> i32 {
    amount_of(fluid).wrapping_mul(entry.level)
}

/// SmashingModule.getFluidInTank: the stored fluid at its derived amount; a fluid whose
/// effects data packs took away reads as empty here (the drain forms clear it, as the
/// original's drains do).
pub(crate) fn smashing_fluid_in_tank(persistent: &NbtCompound, entry: &Entry, amount_of: &dyn Fn(&Identifier) -> i32) -> Option<FluidStack> {
    let (fluid, tag) = smashing_stored(persistent)?;
    let amount = smashing_amount(entry, &fluid, amount_of);
    if amount <= 0 { return None; }
    Some(FluidStack { fluid, amount: amount as u32, tag })
}

/// SmashingModule.getTankCapacity: the derived amount for the stored fluid, or a bottle
/// (FluidValues.BOTTLE) while empty — the original notes the empty number is meaningless.
pub(crate) fn smashing_capacity(persistent: &NbtCompound, entry: &Entry, amount_of: &dyn Fn(&Identifier) -> i32) -> i32 {
    match smashing_stored(persistent) {
        Some((fluid, _)) => smashing_amount(entry, &fluid, amount_of),
        None => BOTTLE,
    }
}

/// SmashingModule.fill: all or nothing — an empty store takes a fluid with effects when at
/// least the full derived amount is offered, and keeps the validation level (the caller's
/// craft-count-adjusted effective level, SmashingModule.getValidationAmount) against part
/// swaps. Returns the amount taken.
pub(crate) fn smashing_fill(persistent: &mut NbtCompound, entry: &Entry, resource: &FluidStack, execute: bool,
    amount_of: &dyn Fn(&Identifier) -> i32, validation: f32) -> i32 {
    if resource.amount == 0 || smashing_stored(persistent).is_some() { return 0; }
    let amount = smashing_amount(entry, &resource.fluid, amount_of);
    if amount <= 0 || (resource.amount as i32) < amount { return 0; }
    if execute {
        persistent.insert(SMASHING_FLUID.to_owned(), Nbt::String(resource.fluid.to_string()));
        persistent.insert(SMASHING_VALIDATE.to_owned(), Nbt::Float(validation));
        match &resource.tag {
            Some(tag) => { persistent.insert(SMASHING_TAG.to_owned(), Nbt::Compound(tag.clone())); }
            None => { persistent.remove(SMASHING_TAG); }
        }
    }
    amount
}

/// SmashingModule.drain(int): the whole stored amount, when it fits in `max`. A stored fluid
/// whose amount collapsed to 0 is cleared even by a simulation, as in the original.
pub(crate) fn smashing_drain(persistent: &mut NbtCompound, entry: &Entry, max: i32, execute: bool,
    amount_of: &dyn Fn(&Identifier) -> i32) -> Option<FluidStack> {
    if max <= 0 { return None; }
    let (fluid, tag) = smashing_stored(persistent)?;
    let amount = smashing_amount(entry, &fluid, amount_of);
    if amount <= 0 {
        smashing_clear(persistent);
        return None;
    }
    if amount > max { return None; }
    let result = FluidStack { fluid, amount: amount as u32, tag };
    if execute { smashing_clear(persistent); }
    Some(result)
}

/// SmashingModule.drain(FluidStack): the whole stored amount, when the request names the
/// stored fluid with an equal tag and covers the amount.
pub(crate) fn smashing_drain_matching(persistent: &mut NbtCompound, entry: &Entry, resource: &FluidStack, execute: bool,
    amount_of: &dyn Fn(&Identifier) -> i32) -> Option<FluidStack> {
    if resource.amount == 0 { return None; }
    let (fluid, tag) = smashing_stored(persistent)?;
    if resource.fluid != fluid { return None; }
    let amount = smashing_amount(entry, &fluid, amount_of);
    if amount <= 0 {
        smashing_clear(persistent);
        return None;
    }
    if amount > resource.amount as i32 || tag != resource.tag { return None; }
    let result = FluidStack { fluid, amount: amount as u32, tag };
    if execute { smashing_clear(persistent); }
    Some(result)
}

/// SmashingModule.validate: a validation level that would rise (a part swap made the entry
/// stronger than when the fluid was stored) is the `empty_to_swap` error; otherwise the
/// stored level follows the tool down. The original compares with getInt, so the stored
/// float is read truncated.
pub(crate) fn smashing_validate(persistent: &mut NbtCompound, validation: f32) -> Option<&'static str> {
    if smashing_stored(persistent).is_none() { return None; }
    let stored = match persistent.get(SMASHING_VALIDATE) {
        Some(Nbt::Float(value)) => *value as i32,
        Some(Nbt::Double(value)) => *value as i32,
        Some(value) => value.as_i64().unwrap_or(0) as i32,
        None => 0,
    };
    if (stored as f32) < validation { return Some("modifier.tconstruct.smashing.empty_to_swap"); }
    persistent.insert(SMASHING_VALIDATE.to_owned(), Nbt::Float(validation));
    None
}

/// SmashingModule.onProjectileShoot's data half: the tool's stored fluid and its derived
/// amount are copied into the projectile's persistent data. The original copies only when
/// the shot is not a multishot extra that may not be picked up: no arrow at all, an arrow
/// with pickup allowed, or a creative shooter.
pub(crate) fn smashing_shoot_copy(tool: &NbtCompound, projectile: &mut NbtCompound, entry: &Entry,
    amount_of: &dyn Fn(&Identifier) -> i32, no_arrow: bool, pickup_allowed: bool, creative: bool) {
    if !(no_arrow || pickup_allowed || creative) { return; }
    let Some((fluid, tag)) = smashing_stored(tool) else { return };
    let amount = smashing_amount(entry, &fluid, amount_of);
    if amount <= 0 { return; }
    projectile.insert(SMASHING_FLUID.to_owned(), Nbt::String(fluid.to_string()));
    projectile.insert(SMASHING_AMOUNT.to_owned(), Nbt::Int(amount));
    if let Some(tag) = tag { projectile.insert(SMASHING_TAG.to_owned(), Nbt::Compound(tag)); }
}

/// The fluid a smashing projectile carries, at its stored amount.
pub(crate) fn smashing_projectile_fluid(projectile: &NbtCompound) -> Option<FluidStack> {
    let (fluid, tag) = smashing_stored(projectile)?;
    let amount = crate::modifier_entries::nbt_int(projectile.get(SMASHING_AMOUNT));
    if amount <= 0 { return None; }
    Some(FluidStack { fluid, amount: amount as u32, tag })
}

/// Spends `drained` of a projectile's stored amount (the hit handlers' drain step): what is
/// left stays, an empty store is cleared.
fn smashing_spend(projectile: &mut NbtCompound, drained: i32) {
    let amount = crate::modifier_entries::nbt_int(projectile.get(SMASHING_AMOUNT));
    let remaining = amount - drained;
    if remaining > 0 { projectile.insert(SMASHING_AMOUNT.to_owned(), Nbt::Int(remaining)); } else { smashing_clear(projectile); }
}

/// A projectile whose stored amount ran out before a hit clears its store, as both hit
/// handlers do before anything else. Returns whether there was a store to clear.
fn smashing_clear_spent(projectile: &mut NbtCompound) -> bool {
    if smashing_stored(projectile).is_some() && crate::modifier_entries::nbt_int(projectile.get(SMASHING_AMOUNT)) <= 0 {
        smashing_clear(projectile);
        true
    } else {
        false
    }
}

/// SmashingModule.onProjectileHitEntity's data half, after the effects consumed `drained`:
/// the store shrinks, and the projectile is discarded — unless it is a piercing arrow, which
/// is marked used instead so it cannot drop its fluid later. Returns whether to discard.
pub(crate) fn smashing_entity_hit(projectile: &mut NbtCompound, drained: i32, piercing: bool) -> bool {
    smashing_clear_spent(projectile);
    if drained <= 0 { return false; }
    smashing_spend(projectile, drained);
    if piercing {
        projectile.insert(SMASHING_USED.to_owned(), Nbt::Byte(1));
        false
    } else {
        true
    }
}

/// SmashingModule.onProjectileHitBlock and onProjectileFuseFinish's data half: discard when
/// the effects consumed fluid, or when the projectile was already marked used.
pub(crate) fn smashing_block_hit(projectile: &mut NbtCompound, drained: i32) -> bool {
    smashing_clear_spent(projectile);
    let used = matches!(projectile.get(SMASHING_USED), Some(Nbt::Byte(1)));
    if drained > 0 {
        smashing_spend(projectile, drained);
        return true;
    }
    used
}

/// Mantle's FluidPredicate, the subset the original's data forms use: any fluid, one fluid,
/// or a fluid tag. Another form leaves the module unbound.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum FluidPredicate {
    Any,
    Id(Identifier),
    Tag(Identifier),
}

impl FluidPredicate {
    pub(crate) fn decode(value: Option<&Value>) -> Result<Option<Self>, String> {
        match value {
            None | Some(Value::Null) => Ok(Some(Self::Any)),
            Some(Value::String(name)) => Identifier::parse(name).map(|id| Some(Self::Id(id))).map_err(|error| error.to_string()),
            Some(Value::Object(fields)) => {
                if let Some(tag) = fields.get("tag").filter(|value| !value.is_null()) {
                    return Ok(Some(Self::Tag(Identifier::parse(tag.as_str().ok_or("tag must be an ID string")?).map_err(|error| error.to_string())?)));
                }
                if let Some(fluid) = fields.get("fluid").filter(|value| !value.is_null()) {
                    return Ok(Some(Self::Id(Identifier::parse(fluid.as_str().ok_or("fluid must be an ID string")?).map_err(|error| error.to_string())?)));
                }
                Ok(None)
            }
            Some(_) => Ok(None),
        }
    }

    pub(crate) fn matches(&self, fluid: &Identifier, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> bool {
        match self {
            Self::Any => true,
            Self::Id(id) => id == fluid,
            Self::Tag(tag) => in_tag(fluid, tag),
        }
    }
}

/// BucketModule: the tool acts as a bucket — sneaking places a bucket of the tank's fluid,
/// a plain use picks a bucket up. The world halves (the fluid block, the ray trace, the
/// vaporize rules) are the consumer's; what is here is the module's logic over the tank.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Bucket {
    pub fluids: FluidPredicate,
}

impl Bucket {
    /// The fluids field defaults to any fluid (FluidPredicate.LOADER.defaultField).
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        match FluidPredicate::decode(fields.get("fluids"))? {
            Some(fluids) => Ok(Some(Self { fluids })),
            None => Ok(None),
        }
    }

    /// afterBlockUse's tank gates: at least a bucket volume of a fluid the predicate allows.
    /// That the fluid flows (a FlowingFluid) and where it may go are world facts.
    pub(crate) fn can_place(&self, fluid: Option<&FluidStack>, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> bool {
        fluid.is_some_and(|fluid| fluid.amount as i32 >= BUCKET && self.fluids.matches(&fluid.fluid, in_tag))
    }

    /// Placing's tank half: one bucket volume leaves the tank.
    pub(crate) fn place_consume(nbt: &mut NbtCompound) -> bool { consume(nbt, BUCKET) == BUCKET }

    /// onToolUse's tank gates: the stored fluid must carry no tag, a bucket's worth of free
    /// space must remain, and a stored fluid must match the predicate.
    pub(crate) fn can_pickup(&self, nbt: &NbtCompound, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> bool {
        let current = fluid(nbt);
        if let Some(current) = &current {
            if current.tag.is_some() || !self.fluids.matches(&current.fluid, in_tag) { return false; }
        }
        capacity(nbt) - current.map_or(0, |current| current.amount as i32) >= BUCKET
    }

    /// pickupBlock's tank half, server side: set when empty, grow when the same fluid; a
    /// different fluid cannot happen (the caller matched the fluid state) and changes nothing.
    pub(crate) fn pickup(&self, nbt: &mut NbtCompound, picked: &Identifier) -> bool {
        match fluid(nbt) {
            None => {
                set_fluid(nbt, Some(&FluidStack::new(picked.clone(), BUCKET as u32)));
                true
            }
            Some(mut current) if current.fluid == *picked => {
                current.amount += BUCKET as u32;
                set_fluid(nbt, Some(&current));
                true
            }
            Some(_) => false,
        }
    }
}

/// InteractionSource: where an interaction came from (the filter TankInteractionModule
/// carries, and the sources the interaction hooks report).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InteractionSource {
    RightClick,
    LeftClick,
    Armor,
}

impl InteractionSource {
    /// The serialized names are the enum's lowercase names (TinkerLoadables.INTERACTION_SOURCE).
    pub(crate) fn decode(value: Option<&Value>) -> Result<Option<Option<Self>>, String> {
        match value {
            None | Some(Value::Null) => Ok(Some(None)),
            Some(Value::String(name)) => Ok(Some(Some(match name.as_str() {
                "right_click" => Self::RightClick,
                "left_click" => Self::LeftClick,
                "armor" => Self::Armor,
                _ => return Ok(None),
            }))),
            Some(_) => Err("interaction_source must be a source name".into()),
        }
    }
}

/// What TankInteractionModule.beforeBlockUse asks the clicked block's fluid handler, in the
/// original's order.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum TankTransfer {
    /// Sneaking, with fluid in the tool: fill the target from the tool.
    FillTarget,
    /// Not sneaking and the tool is empty: drain up to the capacity into the tool.
    DrainToTool { max: i32 },
    /// Not sneaking with fluid in the tool: drain more of the same fluid, up to the free space.
    TopUp { wanted: FluidStack },
}

/// TankInteractionModule: a use on a block with a fluid handler trades fluid with the tool's
/// tank. Whether the block has a handler on the clicked face, and running the transfer
/// against it, are the consumer's; the choice and the tool half are here.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TankInteraction {
    pub source: Option<InteractionSource>,
}

impl TankInteraction {
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        match InteractionSource::decode(fields.get("interaction_source"))? {
            Some(source) => Ok(Some(Self { source })),
            None => Ok(None),
        }
    }

    /// The transfer for one use: a source filter that does not match answers nothing, as does
    /// a full tool asked to top up. Sneaking with an empty tool also plans nothing (the
    /// original still answers success when a handler exists; that result is the caller's).
    pub(crate) fn plan(&self, nbt: &NbtCompound, source: InteractionSource, sneaking: bool) -> Option<TankTransfer> {
        if self.source.is_some_and(|filter| filter != source) { return None; }
        let current = fluid(nbt);
        if sneaking {
            return current.map(|_| TankTransfer::FillTarget);
        }
        match current {
            None => Some(TankTransfer::DrainToTool { max: capacity(nbt) }),
            Some(current) => {
                let free = capacity(nbt) - current.amount as i32;
                if free <= 0 { None } else { Some(TankTransfer::TopUp { wanted: FluidStack { amount: free as u32, ..current } }) }
            }
        }
    }

    /// The tool half after the target accepted `added` of the tool's fluid.
    pub(crate) fn filled_target(nbt: &mut NbtCompound, added: i32) -> bool { added > 0 && consume(nbt, added) > 0 }

    /// The tool half after draining `drained` out of the target: set when the tool was empty,
    /// grow when it is the same fluid (the original checks isFluidEqual after the drain).
    pub(crate) fn drained_to_tool(nbt: &mut NbtCompound, drained: &FluidStack) -> bool {
        match fluid(nbt) {
            None => {
                set_fluid(nbt, Some(drained));
                true
            }
            Some(mut current) => {
                if !drained.same_fluid(&current) { return false; }
                current.amount += drained.amount;
                set_fluid(nbt, Some(&current));
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        materials::Definitions as Materials,
        modifiers::Registry,
        tool_stack::{TAG_MODIFIERS, TAG_STATS},
    };

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
    fn stack(fluid: &str, amount: u32) -> FluidStack { FluidStack::new(id(fluid), amount) }
    fn entry(name: &str, level: i32) -> Entry { Entry::new(id(name), level) }

    /// A tool whose stats give it the tank capacity (and overslime capacity, when asked).
    fn tool(capacity: i32, overslime_capacity: Option<i32>) -> NbtCompound {
        let mut nbt = NbtCompound::new();
        let mut stats = NbtCompound::new();
        stats.insert("tconstruct:tank_capacity".into(), Nbt::Float(capacity as f32));
        if let Some(overslime) = overslime_capacity { stats.insert("tconstruct:overslime".into(), Nbt::Float(overslime as f32)); }
        nbt.insert(TAG_STATS.into(), Nbt::Compound(stats));
        nbt
    }

    fn environment<'a>(materials: &'a Materials, modifiers: &'a Registry) -> Environment<'a> {
        Environment { definition: None, has_materials: false, materials, modifiers, item_tags_loaded: false, item: None, item_tags: None }
    }

    #[test]
    fn the_tank_fills_drains_and_clamps_like_the_original() {
        let mut nbt = tool(1000, None);
        assert_eq!(fluid(&nbt), None);
        assert_eq!(fill(&mut nbt, &stack("minecraft:lava", 400), false), 400, "a simulation reports the take");
        assert_eq!(fluid(&nbt), None, "a simulation stores nothing");
        assert_eq!(fill(&mut nbt, &stack("minecraft:lava", 1400), true), 1000);
        assert_eq!(fluid(&nbt), Some(stack("minecraft:lava", 1000)));
        assert_eq!(fill(&mut nbt, &stack("minecraft:water", 10), true), 0, "another fluid is refused");
        assert_eq!(drain(&mut nbt, 250, true), Some(stack("minecraft:lava", 250)));
        assert_eq!(drain_fluid(&mut nbt, &stack("minecraft:water", 10), true), None);
        assert_eq!(drain_fluid(&mut nbt, &stack("minecraft:lava", 100), true), Some(stack("minecraft:lava", 100)));
        assert_eq!(fluid(&nbt), Some(stack("minecraft:lava", 650)));
        // Setting past the capacity clamps; setting empty clears.
        set_fluid(&mut nbt, Some(&stack("minecraft:lava", 5000)));
        assert_eq!(fluid(&nbt), Some(stack("minecraft:lava", 1000)));
        set_fluid(&mut nbt, None);
        assert_eq!(fluid(&nbt), None);
        // The validate clamp follows a capacity that shrank under the content.
        set_fluid(&mut nbt, Some(&stack("minecraft:lava", 900)));
        let mut stats = NbtCompound::new();
        stats.insert("tconstruct:tank_capacity".into(), Nbt::Float(300.0));
        nbt.insert(TAG_STATS.into(), Nbt::Compound(stats));
        assert!(validate_clamp(&mut nbt));
        assert_eq!(fluid(&nbt), Some(stack("minecraft:lava", 300)));
    }

    #[test]
    fn the_handler_is_a_code_modifier_with_the_tank_module() {
        let modifiers = code_modifiers().unwrap();
        assert_eq!(modifiers.len(), 1);
        let handler = &modifiers[0];
        assert_eq!(handler.id, handler_id());
        assert_eq!(handler.priority, 300);
        assert!(handler.unbound.is_empty(), "the tank module binds: {:?}", handler.unbound);
        assert!(handler.modules().iter().any(|module| module.is_tank()));
    }

    #[test]
    fn a_tool_with_the_handler_trait_has_a_tank() {
        let materials = Materials::default();
        let mut registry = Registry::default();
        for modifier in code_modifiers().unwrap() { registry.register_static(modifier).unwrap(); }
        let env = environment(&materials, &registry);
        let mut nbt = tool(1000, None);
        assert!(!has_tank(&nbt, &env));
        let entries = vec![Nbt::Compound(entry("tconstruct:tank_handler", 1).to_nbt())];
        nbt.insert(TAG_MODIFIERS.into(), Nbt::List(entries));
        assert!(has_tank(&nbt, &env));
    }

    #[test]
    fn melting_boosts_ore_by_its_own_rates_and_scales_entity_melts() {
        let melting = Melting::decode(
            json!({"temperature": {"flat": 500, "each_level": 500}, "nuggets_per_metal": {"flat": 9, "each_level": 3}, "shards_per_gem": {"flat": 6, "each_level": 2}})
                .as_object().unwrap(),
        )
        .unwrap();
        assert_eq!(melting.temperature(1.0), 1000);
        assert_eq!(melting.temperature(2.0), 1500);
        // Level 1: 12 nuggets per metal, 8 shards per gem, both over 9.
        assert_eq!(melting.ore_boost(1.0, OreRateType::Metal, 90), 120);
        assert_eq!(melting.ore_boost(1.0, OreRateType::Gem, 90), 80);
        assert_eq!(melting.ore_boost(1.0, OreRateType::Default, 90), 90);
        assert_eq!(Melting::entity_amount(100, 4, 2.0), 50, "half the recipe damage gives half the output");
        assert_eq!(Melting::entity_amount(100, 4, 8.0), 200, "twice the recipe damage caps the gain");
        assert_eq!(Melting::entity_amount(100, 4, 40.0), 200);
        assert_eq!(Melting::default_entity_fluid(), stack("tconstruct:liquid_soul", 50));
        assert!(Melting::decode(json!({"temperature": 1}).as_object().unwrap()).is_err(), "all three fields are required");
    }

    #[test]
    fn overburn_burns_fuel_on_its_interval_and_restores_overslime() {
        let materials = Materials::default();
        let modifiers = Registry::default();
        let env = environment(&materials, &modifiers);
        let fuel = |_: &Identifier| Some(Fuel { amount: 250, duration: 100, rate: 15 });
        let mut nbt = tool(1000, Some(100));
        set_fluid(&mut nbt, Some(&stack("minecraft:lava", 1000)));
        let overburn = entry("tconstruct:overburn", 1);
        // Level 1: the interval is 5 << 5 = 160 ticks.
        assert!(!overburn_tick(&mut nbt, &env, &overburn, 159, 1000, false, &fuel, &mut || 0.0), "off the interval nothing runs");
        let mut rolls = || 0.4;
        assert!(overburn_tick(&mut nbt, &env, &overburn, 160, 1000, false, &fuel, &mut rolls));
        assert_eq!(fluid(&nbt), Some(stack("minecraft:lava", 750)), "one fuel amount left the tank");
        assert_eq!(overslime::amount(&nbt), 2, "rate 15 restores 1, plus the remainder roll of 4 < 5");
        // The burn (100 * 160 / 5 = 3200 long) is stored under the modifier's own ID.
        let info = read_fuel_info(&persistent(&nbt), "tconstruct:overburn").unwrap();
        assert_eq!(info, FuelInfo { expiration: 4200, rate: 15 });
        // While the burn lasts, no more fuel is consumed; the restore runs again.
        assert!(overburn_tick(&mut nbt, &env, &overburn, 320, 1160, false, &fuel, &mut || 0.9));
        assert_eq!(fluid(&nbt), Some(stack("minecraft:lava", 750)));
        assert_eq!(overslime::amount(&nbt), 3, "a roll of 9 misses the remainder: only the flat 1");
        // A full overslime bar stops the module entirely.
        overslime::set_amount(&mut nbt, 100);
        assert!(!overburn_tick(&mut nbt, &env, &overburn, 480, 1320, false, &fuel, &mut || 0.0));
    }

    #[test]
    fn overburn_scales_a_partial_tanks_burn_and_clears_a_spent_one() {
        let materials = Materials::default();
        let modifiers = Registry::default();
        let env = environment(&materials, &modifiers);
        let fuel = |_: &Identifier| Some(Fuel { amount: 250, duration: 100, rate: 10 });
        let mut nbt = tool(1000, Some(100));
        set_fluid(&mut nbt, Some(&stack("minecraft:lava", 100)));
        let overburn = entry("tconstruct:overburn", 1);
        // 100 of a 250 recipe: the tank empties and the burn lasts 3200 * 100 / 250 = 1280.
        assert!(overburn_tick(&mut nbt, &env, &overburn, 160, 0, false, &fuel, &mut || 0.0));
        assert_eq!(fluid(&nbt), None);
        assert_eq!(read_fuel_info(&persistent(&nbt), "tconstruct:overburn"), Some(FuelInfo { expiration: 1280, rate: 10 }));
        // After it expires with an empty tank, the stored info is cleared.
        assert!(overburn_tick(&mut nbt, &env, &overburn, 160 * 9, 2000, false, &fuel, &mut || 0.0));
        assert_eq!(read_fuel_info(&persistent(&nbt), "tconstruct:overburn"), None);
        assert_eq!(overslime::amount(&nbt), 1, "the first burn restored its flat 1; the expired tick found no fuel");
    }

    #[test]
    fn slurping_and_spitting_armor_timers_share_the_timing() {
        let slurping = Slurping::decode(json!({"strength": {"each_level": 1.0}, "duration": {"flat": 21}}).as_object().unwrap()).unwrap();
        let slurp = entry("tconstruct:slurping", 2);
        assert_eq!(slurping.duration(&slurp), 21);
        assert_eq!(slurping.strength(&slurp), 2.0);
        let mut persistent = NbtCompound::new();
        slurping.armor_start(&mut persistent, &slurp, 100);
        assert_eq!(armor_charge_tick(&mut persistent, &slurp.id, 116), ArmorTick::Idle, "5 ticks left is not a sound tick");
        assert_eq!(armor_charge_tick(&mut persistent, &slurp.id, 117), ArmorTick::Sound, "4 ticks left is");
        assert_eq!(armor_charge_tick(&mut persistent, &slurp.id, 122), ArmorTick::Finished);
        assert_eq!(armor_charge_tick(&mut persistent, &slurp.id, 200), ArmorTick::Idle, "the finished timer cleared itself");
        slurping.armor_start(&mut persistent, &slurp, 0);
        slurping.stop(&mut persistent, &slurp);
        assert_eq!(armor_charge_tick(&mut persistent, &slurp.id, 10), ArmorTick::Idle);
    }

    #[test]
    fn spitting_plans_shots_from_the_charge_and_the_ingredient() {
        let spitting = Spitting::decode(json!({"shots": {"each_level": 1}}).as_object().unwrap()).unwrap();
        let spit = entry("tconstruct:spitting", 2);
        assert_eq!(spitting.shots(&spit), 2);
        // Power 0.5 * 2 = 1: per shot (int)(250 * 1) * 2 = 500 offered over 2 shots, tank holds 300.
        let plan = spitting.plan(&spit, 300, 250, 0.5, 2.0, 1.5).unwrap();
        assert_eq!(plan, SpitPlan { shots: 2, amount_per_shot: 150, power: 1.0, velocity: 2.25 });
        assert!(spitting.plan(&spit, 1, 250, 0.5, 2.0, 1.5).is_none(), "less than a millibucket per shot fires nothing");
        assert!(Spitting::can_start(&spit, Some(&stack("minecraft:lava", 2)), true), "the tank must hold the plain level");
        assert!(!Spitting::can_start(&spit, Some(&stack("minecraft:lava", 1)), true));
        assert!(!Spitting::can_start(&spit, Some(&stack("minecraft:lava", 5)), false), "a fluid without effects cannot be spat");
    }

    #[test]
    fn smashing_stores_no_amount_and_drains_all_or_nothing() {
        let amounts = |fluid: &Identifier| if fluid.to_string() == "tconstruct:blazing_blood" { 250 } else { 0 };
        let smash = entry("tconstruct:smashing", 2);
        let mut persistent = NbtCompound::new();
        assert_eq!(smashing_capacity(&persistent, &smash, &amounts), 250, "an empty smashing tank reports a bottle");
        assert_eq!(smashing_fill(&mut persistent, &smash, &stack("minecraft:lava", 1000), true, &amounts, 2.0), 0, "a fluid without effects is refused");
        assert_eq!(smashing_fill(&mut persistent, &smash, &stack("tconstruct:blazing_blood", 499), true, &amounts, 2.0), 0, "less than the full amount is refused");
        assert_eq!(smashing_fill(&mut persistent, &smash, &stack("tconstruct:blazing_blood", 500), false, &amounts, 2.0), 500, "a simulation reports the take");
        assert_eq!(smashing_stored(&persistent), None, "a simulation stores nothing");
        assert_eq!(smashing_fill(&mut persistent, &smash, &stack("tconstruct:blazing_blood", 900), true, &amounts, 2.0), 500, "the take is the derived amount, not the offer");
        assert_eq!(smashing_fluid_in_tank(&persistent, &smash, &amounts), Some(stack("tconstruct:blazing_blood", 500)));
        assert_eq!(smashing_fill(&mut persistent, &smash, &stack("tconstruct:blazing_blood", 500), true, &amounts, 2.0), 0, "a filled store takes no more");
        assert_eq!(smashing_drain(&mut persistent, &smash, 499, true, &amounts), None, "a drain below the amount takes nothing");
        assert_eq!(smashing_drain_matching(&mut persistent, &smash, &stack("tconstruct:blazing_blood", 500), true, &amounts), Some(stack("tconstruct:blazing_blood", 500)));
        assert_eq!(smashing_stored(&persistent), None);
    }

    #[test]
    fn smashing_validate_rejects_a_risen_level_and_follows_a_fallen_one() {
        let amounts = |_: &Identifier| 250;
        let smash = entry("tconstruct:smashing", 1);
        let mut persistent = NbtCompound::new();
        smashing_fill(&mut persistent, &smash, &stack("tconstruct:blazing_blood", 250), true, &amounts, 1.5);
        assert_eq!(smashing_validate(&mut persistent, 2.0), Some("modifier.tconstruct.smashing.empty_to_swap"));
        assert_eq!(smashing_validate(&mut persistent, 1.0), None);
        match persistent.get(SMASHING_VALIDATE) { Some(Nbt::Float(value)) => assert_eq!(*value, 1.0), other => panic!("{other:?}") }
        // The stored float is read truncated, as the original's getInt reads it: stored as
        // 1.5, it compares as 1, so a validation of 1.25 already counts as a rise.
        assert_eq!(smashing_drain(&mut persistent, &smash, 250, true, &amounts), Some(stack("tconstruct:blazing_blood", 250)));
        smashing_fill(&mut persistent, &smash, &stack("tconstruct:blazing_blood", 250), true, &amounts, 1.5);
        assert_eq!(smashing_validate(&mut persistent, 1.25), Some("modifier.tconstruct.smashing.empty_to_swap"), "getInt truncates 1.5 to 1, and 1 < 1.25 errors");
    }

    #[test]
    fn smashing_projectiles_carry_and_spend_their_fluid() {
        let amounts = |_: &Identifier| 250;
        let smash = entry("tconstruct:smashing", 1);
        let mut tool_data = NbtCompound::new();
        smashing_fill(&mut tool_data, &smash, &stack("tconstruct:blazing_blood", 250), true, &amounts, 1.0);
        let mut projectile = NbtCompound::new();
        smashing_shoot_copy(&tool_data, &mut projectile, &smash, &amounts, false, false, false);
        assert_eq!(smashing_projectile_fluid(&projectile), None, "a multishot extra that may not be picked up gets no fluid");
        smashing_shoot_copy(&tool_data, &mut projectile, &smash, &amounts, false, true, false);
        assert_eq!(smashing_projectile_fluid(&projectile), Some(stack("tconstruct:blazing_blood", 250)));
        // A piercing arrow spends and is marked used, not discarded.
        assert!(!smashing_entity_hit(&mut projectile, 100, true));
        assert_eq!(smashing_projectile_fluid(&projectile), Some(stack("tconstruct:blazing_blood", 150)));
        assert!(matches!(projectile.get(SMASHING_USED), Some(Nbt::Byte(1))));
        // A block hit with nothing consumed still discards a used projectile.
        assert!(smashing_block_hit(&mut projectile, 0));
        // A plain arrow is discarded by its first consuming hit.
        let mut other = NbtCompound::new();
        smashing_shoot_copy(&tool_data, &mut other, &smash, &amounts, true, false, false);
        assert!(smashing_entity_hit(&mut other, 250, false));
        assert_eq!(smashing_projectile_fluid(&other), None, "a fully spent store is cleared");
    }

    #[test]
    fn bucket_gates_and_pickup_follow_the_original() {
        let no_tag = |_: &Identifier, _: &Identifier| false;
        let bucket = Bucket::decode(json!({}).as_object().unwrap()).unwrap().unwrap();
        assert_eq!(bucket.fluids, FluidPredicate::Any, "no fluids field means any fluid");
        assert!(bucket.can_place(Some(&stack("minecraft:lava", 1000)), &no_tag));
        assert!(!bucket.can_place(Some(&stack("minecraft:lava", 999)), &no_tag), "placing needs a full bucket");
        let mut nbt = tool(1000, None);
        assert!(bucket.can_pickup(&nbt, &no_tag));
        assert!(bucket.pickup(&mut nbt, &id("minecraft:lava")));
        assert_eq!(fluid(&nbt), Some(stack("minecraft:lava", 1000)));
        assert!(!bucket.can_pickup(&nbt, &no_tag), "a full tank has no room for another bucket");
        assert!(!bucket.pickup(&mut nbt, &id("minecraft:water")), "a different fluid cannot be picked up");
        assert!(Bucket::place_consume(&mut nbt));
        assert_eq!(fluid(&nbt), None);
        // A tagged stored fluid blocks pickup entirely.
        let mut tagged = tool(2000, None);
        let mut tag = NbtCompound::new();
        tag.insert("x".into(), Nbt::Int(1));
        set_fluid(&mut tagged, Some(&FluidStack { fluid: id("minecraft:lava"), amount: 100, tag: Some(tag) }));
        assert!(!bucket.can_pickup(&tagged, &no_tag));
    }

    #[test]
    fn tank_interaction_plans_the_three_trades() {
        let interaction = TankInteraction::decode(json!({"interaction_source": "armor"}).as_object().unwrap()).unwrap().unwrap();
        let mut nbt = tool(1000, None);
        assert_eq!(interaction.plan(&nbt, InteractionSource::RightClick, false), None, "the source filter answers only armor");
        assert_eq!(interaction.plan(&nbt, InteractionSource::Armor, false), Some(TankTransfer::DrainToTool { max: 1000 }));
        assert_eq!(interaction.plan(&nbt, InteractionSource::Armor, true), None, "sneaking with an empty tool moves nothing");
        assert!(TankInteraction::drained_to_tool(&mut nbt, &stack("minecraft:lava", 400)));
        assert_eq!(interaction.plan(&nbt, InteractionSource::Armor, false), Some(TankTransfer::TopUp { wanted: stack("minecraft:lava", 600) }));
        assert_eq!(interaction.plan(&nbt, InteractionSource::Armor, true), Some(TankTransfer::FillTarget));
        assert!(!TankInteraction::drained_to_tool(&mut nbt, &stack("minecraft:water", 100)), "another fluid is not topped up");
        assert!(TankInteraction::filled_target(&mut nbt, 150));
        assert_eq!(fluid(&nbt), Some(stack("minecraft:lava", 250)));
        assert!(TankInteraction::decode(json!({"interaction_source": "nowhere"}).as_object().unwrap()).unwrap().is_none(), "an unknown source stays unbound");
    }
}
