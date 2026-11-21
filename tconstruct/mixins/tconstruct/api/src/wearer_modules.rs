// SPDX-License-Identifier: MIT
//! The armor modules that act on their wearer, ported from SlimeKnights (MIT; see LICENSE): DepthProtectionModule, OvershieldModule,
//! RestoreLostHealthModule, KineticModule, DamageOnUnequipModule and ReduceEffectOnUnequipModule. The wearer is the `target` of the hit as PlutoLib reported
//! them as of their last tick; what the modules do to the wearer (an effect, a hurt, an effect's time, the armor's own data) goes back as
//! `EntityAction`s. The wearer's stack is the one their armor listing carries, so an edit lands on the piece the module sits on.
use crate::{
    melee_damage::TargetCondition,
    modifier_entries::Entry,
    protection::{DamageSource, SourceCondition},
    tool_stack,
};
use pluto_lib::{EffectCategory, Identifier, NbtCompound, StackDataEdit};
use serde_json::{Map, Value};

/// A LevelingValue: `flat` plus `each_level` times the level.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Leveling { flat: f32, each: f32 }

impl Leveling {
    /// A `{flat, each_level}` object (either part may be missing); the whole value may also be absent when `required` is false.
    fn decode(value: Option<&Value>, name: &str) -> Result<Self, String> {
        let object = value.and_then(Value::as_object).ok_or_else(|| format!("{name} must be a leveling value"))?;
        Ok(Self { flat: number(object, "flat")?, each: number(object, "each_level")? })
    }
    /// The same fields read straight from a module's own object (LevelingValue.directField).
    fn direct(fields: &Map<String, Value>) -> Result<Self, String> { Ok(Self { flat: number(fields, "flat")?, each: number(fields, "each_level")? }) }
    fn compute(self, level: f32) -> f32 { self.flat + self.each * level }
    /// LevelingInt.compute: the float value cut to an int.
    fn int(self, level: f32) -> i32 { self.compute(level) as i32 }
}

fn number(fields: &Map<String, Value>, key: &str) -> Result<f32, String> {
    match fields.get(key).filter(|value| !value.is_null()) {
        None => Ok(0.0),
        Some(value) => value.as_f64().map(|value| value as f32).ok_or_else(|| format!("{key} must be a number")),
    }
}

fn source(fields: &Map<String, Value>) -> Result<Option<SourceCondition>, String> {
    match fields.get("damage_source") { None | Some(Value::Null) => Ok(Some(SourceCondition::Any)), Some(value) => SourceCondition::decode(value) }
}

/// DepthProtectionModule: protection that grows below the baseline height and turns into a penalty above it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DepthProtection { source: SourceCondition, wearer: TargetCondition, baseline: f32, neutral_range: f32, amount: Leveling }

impl DepthProtection {
    /// `None` for a damage source or a wearer condition this port cannot evaluate.
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        let (Some(source), Some(wearer)) = (source(fields)?, TargetCondition::decode_for("target", fields.get("wearing_entity"))?) else { return Ok(None); };
        let neutral_range = number(fields, "neutral_range")?;
        if neutral_range < 0.0 { return Err("neutral_range must not be negative".into()); }
        if !fields.contains_key("baseline_height") { return Err("baseline_height missing".into()); }
        Ok(Some(Self { source, wearer, baseline: number(fields, "baseline_height")?, neutral_range, amount: Leveling::direct(fields)? }))
    }

    /// getBonusMultiplier: 0 to 2 below the baseline (linear in the depth), 0 in the neutral range above it, then 0 to -1 above that.
    pub(crate) fn bonus_multiplier(y: f32, baseline: f32, neutral_range: f32) -> f32 {
        if y < baseline { return ((baseline - y) / baseline).min(2.0); }
        let debuff_height = baseline + neutral_range;
        if y > debuff_height { return ((debuff_height - y) / baseline).max(-1.0); }
        0.0
    }

    /// ProtectionModifierHook: the running protection after this module. A wearer whose height was not reported gets nothing.
    pub(crate) fn protect(&self, entry: &Entry, source: &DamageSource, value: f32) -> f32 {
        if !self.source.matches(source) || !self.wearer.matches(&|tag| source.wearer_in_tag(tag)) { return value; }
        let Some(y) = source.entities.target.and_then(|wearer| wearer.y) else { return value; };
        value + Self::bonus_multiplier(y as f32, self.baseline, self.neutral_range) * self.amount.compute(entry.effective_level())
    }
}

/// OvershieldModule: armor that spends the overslime on it for protection.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Overshield { protection: Leveling, consumed: Leveling }

impl Overshield {
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Self, String> {
        Ok(Self { protection: Leveling::decode(fields.get("protection"), "protection")?, consumed: Leveling::decode(fields.get("consumed"), "consumed")? })
    }

    /// ProtectionModifierHook: the running protection after this module, with the overslime it spent taken off `work` (the piece's data).
    /// Nothing happens for damage that bypasses invulnerability, with no overslime on the piece, or when the module would spend none.
    pub(crate) fn protect(&self, entry: &Entry, work: &mut NbtCompound, source: &DamageSource, value: f32) -> f32 {
        if !SourceCondition::CanProtect.matches(source) { return value; }
        let overslime = crate::overslime::amount(work);
        if overslime <= 0 { return value; }
        let level = entry.effective_level();
        let target = self.consumed.int(level);
        if target <= 0 { return value; }
        let consumed = overslime.min(target);
        // Scaled by how much of the wanted overslime was there; the rest of the shield stays.
        crate::overslime::set_amount(work, overslime - consumed);
        value + self.protection.compute(level) * consumed as f32 / target as f32
    }
}

/// What RestoreLostHealthModule does when it lands: the regeneration it gives and the wear it costs the piece.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Restore {
    /// The ticks of Regeneration I the wearer gets.
    pub duration: i32,
    /// The wear on the piece.
    pub wear: i32,
}

/// RestoreLostHealthModule: armor that turns part of the damage its wearer takes into slow healing.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RestoreLostHealth { percent: Leveling, effect_level: Leveling, durability_usage: Leveling, chance: Leveling, attacker: TargetCondition, defender: TargetCondition }

impl RestoreLostHealth {
    /// The module's priority: it runs after the modules that change the damage.
    pub(crate) const PRIORITY: i32 = 10;

    /// `None` for an attacker or defender condition this port cannot evaluate.
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        let (Some(attacker), Some(defender)) = (TargetCondition::decode_for("attacker", fields.get("attacker"))?, TargetCondition::decode_for("target", fields.get("defender"))?) else { return Ok(None); };
        Ok(Some(Self {
            percent: Leveling::decode(fields.get("percentage"), "percentage")?, effect_level: Leveling::decode(fields.get("effect_level"), "effect_level")?,
            durability_usage: Leveling::decode(fields.get("durability_usage"), "durability_usage")?, chance: Leveling::decode(fields.get("chance"), "chance")?,
            attacker, defender,
        }))
    }

    /// modifyDamageTaken: what the module does for `amount` of damage taken, or None when it does nothing. `roll` is a uniform value in
    /// [0, 1), drawn only when the effect level is positive, as the original draws it.
    pub(crate) fn restore(&self, entry: &Entry, source: &DamageSource, amount: f32, roll: &mut dyn FnMut() -> f32) -> Option<Restore> {
        if !self.defender.matches(&|tag| source.wearer_in_tag(tag)) || !self.attacker.matches(source.attacker_in_tag) { return None; }
        let level = entry.effective_level();
        let effect_level = self.effect_level.int(level);
        if effect_level <= 0 || roll() >= self.chance.compute(level) { return None; }
        let heal = (self.percent.compute(level) * amount) as i32;
        if heal <= 0 { return None; }
        // Regeneration restores 1 health every 50 ticks at level 1 and faster above (Java masks the shift count to five bits).
        let duration = heal.wrapping_mul(50_i32.wrapping_shr((effect_level - 1) as u32));
        Some(Restore { duration, wear: self.durability_usage.int(level).max(0) })
    }
}

/// KineticModule: armor that feeds on being hit. Each hit an entity lands on the wearer raises the
/// insatiable (armor) effect a level, up to the total kinetic level of the worn pieces; the effect
/// itself carries a point of attack damage per level. Only the slot in charge applies it
/// (SlotInChargeModule), and its level is the total over the equipped pieces, so the grant is worked
/// out over the pieces a hit reports, as armor levels are.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Kinetic;

impl Kinetic {
    /// The effect's duration: 10 seconds, as the module applies it.
    pub(crate) const DURATION: i32 = 10 * 20;

    /// OnAttackedModifierHook through InsatiableModifier.applyEffect: the grant for one entity-caused
    /// hit, or None when the total kinetic level of the worn pieces is nothing. The new amplifier is
    /// one above the wearer's current one (`current`, -1 when they lack the effect), capped at one
    /// below the total level.
    pub(crate) fn grant(total_level: i32, current: i32) -> Option<crate::weapon_effect::Grant> {
        if total_level <= 0 { return None; }
        let amplifier = (total_level - 1).min(current + 1);
        Some(crate::weapon_effect::Grant { effect: Identifier::new("tconstruct", "insatiable_armor").expect("valid effect id"), amplifier, duration: Self::DURATION })
    }
}

/// DamageOnUnequipModule: armor that hurts its wearer when it comes off.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DamageOnUnequip { damage: f32, damage_type: Identifier }

impl DamageOnUnequip {
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Self, String> {
        let damage = number(fields, "damage")?;
        if damage < 0.0 { return Err("damage must not be negative".into()); }
        let damage_type = match fields.get("damage_type").filter(|value| !value.is_null()) {
            None => Identifier::new("tconstruct", "entangled").map_err(|e| e.to_string())?,
            Some(value) => Identifier::parse(value.as_str().ok_or("damage_type must be an ID string")?).map_err(|e| e.to_string())?,
        };
        Ok(Self { damage, damage_type })
    }
    /// The damage type and the amount (the damage times the level) the wearer is hurt by; None for none.
    pub(crate) fn hurt(&self, entry: &Entry) -> Option<(String, f32)> {
        let amount = self.damage * entry.effective_level();
        (amount > 0.0).then(|| (self.damage_type.to_string(), amount))
    }
}

/// UpdateHealthModule: armor that refreshes its wearer's health when it goes on or comes off, by the bonus the
/// piece grants (the revitalizing attribute raises the maximum by the same amount, so going on heals and coming
/// off hurts by that much). Only armor slots are ported: the tick that reports equipment changes reports no
/// held tools, so a module naming a hand slot stays unbound.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct UpdateHealth { bonus: Leveling, slots: Vec<String> }

impl UpdateHealth {
    /// `None` for a module naming a hand slot, whose changes no event reports. An unknown slot name rejects.
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        // LevelingValue.ZERO when the bonus field is absent.
        let bonus = match fields.get("bonus").filter(|value| !value.is_null()) {
            None => Leveling { flat: 0.0, each: 0.0 },
            Some(value) => Leveling::decode(Some(value), "bonus")?,
        };
        let slots = fields.get("slots").and_then(Value::as_array).ok_or("slots must be an array")?.iter()
            .map(|slot| match slot.as_str() {
                Some(name @ ("mainhand" | "offhand" | "feet" | "legs" | "chest" | "head")) => Ok(name.to_owned()),
                _ => Err("unknown equipment slot".to_owned()),
            })
            .collect::<Result<Vec<_>, _>>()?;
        if slots.iter().any(|slot| matches!(slot.as_str(), "mainhand" | "offhand")) { return Ok(None); }
        Ok(Some(Self { bonus, slots }))
    }

    /// updateHealth for a change in `slot`: the bonus going on, its negation coming off, and nothing for a slot
    /// the module does not name. A positive result heals (the game cuts a heal to the maximum, as the original
    /// caps the raised health); a negative one is dealt as damage of the `tconstruct:update_health` type.
    pub(crate) fn delta(&self, entry: &Entry, slot: &str, equipped: bool) -> f32 {
        if !self.slots.iter().any(|named| named == slot) { return 0.0; }
        let bonus = self.bonus.compute(entry.effective_level());
        if equipped { bonus } else { -bonus }
    }
}

/// ReduceEffectOnUnequipModule: armor that shortens the effects of one category on its wearer when it comes off.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ReduceEffectOnUnequip { category: EffectCategory, percent: Leveling }

impl ReduceEffectOnUnequip {
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Self, String> {
        let category = match fields.get("category").and_then(Value::as_str) {
            Some("beneficial") => EffectCategory::Beneficial,
            Some("harmful") => EffectCategory::Harmful,
            Some("neutral") => EffectCategory::Neutral,
            _ => return Err("category must be beneficial, harmful or neutral".into()),
        };
        Ok(Self { category, percent: Leveling::decode(fields.get("percent"), "percent")? })
    }
    pub(crate) fn category(&self) -> EffectCategory { self.category }
    /// The share of an effect's time the module takes off; 0 for none.
    pub(crate) fn percent(&self, entry: &Entry) -> f32 { self.percent.compute(entry.effective_level()) }
}

/// The duration the module leaves an effect with: at least 1 tick, the time less the share. None for an effect that never ends or has run out.
pub(crate) fn shortened(duration: i32, percent: f32) -> Option<i32> {
    (duration > 0).then(|| ((duration as f32 * (1.0 - percent)) as i32).max(1))
}

/// The edits that turn the custom data `before` into `after`: one for each top-level entry that changed (a compound is put whole), except the
/// persistent data, which is edited key by key so another edit of one of its keys in the same tick is not undone.
pub(crate) fn edits_between(before: &NbtCompound, after: &NbtCompound) -> Vec<StackDataEdit> {
    let skip = |key: &String| key.as_str() == tool_stack::TAG_PERSISTENT;
    let mut edits: Vec<StackDataEdit> = after.iter().filter(|(key, value)| !skip(key) && before.get(*key) != Some(*value)).map(|(key, value)| StackDataEdit { path: vec![key.clone()], value: Some(value.clone()) }).collect();
    edits.extend(before.keys().filter(|key| !skip(key) && !after.contains_key(*key)).map(|key| StackDataEdit { path: vec![key.clone()], value: None }));
    edits.extend(crate::capacity::persistent_edits(before, after));
    edits
}

/// The edits of the wear a piece takes (ToolDamageUtil.damage with `cause` the modifier that costs it): its own damage and broken state, and
/// the shields that soak part of it. Empty when nothing is worn.
pub(crate) fn wear_edits(nbt: &NbtCompound, env: &tool_stack::Environment, cause: &Identifier, amount: i32, random: &mut dyn FnMut() -> f32) -> Vec<StackDataEdit> {
    let mut work = nbt.clone();
    crate::tool_damage::damage_tool_reduced(&mut work, amount, true, true, &mut |data, amount| {
        let amount = crate::capacity::absorb(data, env, Some(cause), amount, random);
        tool_stack::reduce_damage(data, env, Some(cause), amount, random)
    });
    edits_between(nbt, &work)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{damage_tags::DamageTags, entity_facts::Entities};
    use pluto_lib::{EntityState, Nbt};
    use serde_json::json;

    fn fields(value: Value) -> Map<String, Value> { value.as_object().unwrap().clone() }
    fn entry(level: i32) -> Entry { Entry::new(Identifier::parse("tconstruct:armor_module").unwrap(), level) }
    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
    fn with_source<R>(wearer: Option<&EntityState>, run: impl FnOnce(&DamageSource) -> R) -> R { with_damage(wearer, "minecraft:mob_attack", run) }
    fn with_damage<R>(wearer: Option<&EntityState>, damage_type: &str, run: impl FnOnce(&DamageSource) -> R) -> R {
        let tags = DamageTags::load(&[]).0;
        let damage_type = id(damage_type);
        let none = |_: &Identifier| false;
        run(&DamageSource { damage_type: &damage_type, indirect: false, tags: &tags, attacker_in_tag: &none, entities: Entities { attacker: None, target: wearer } })
    }

    #[test]
    fn depth_protection_grows_below_the_baseline_and_turns_into_a_penalty_above_the_neutral_range() {
        // The shipped numbers: baseline 64, neutral range 32, 1.25 per level.
        let at = |y: f32| DepthProtection::bonus_multiplier(y, 64.0, 32.0);
        assert_eq!((at(0.0), at(32.0), at(64.0), at(96.0)), (1.0, 0.5, 0.0, 0.0), "linear to 1 at the bottom of the world, none from the baseline up to the end of the neutral range");
        assert_eq!(at(-64.0), 2.0, "capped at 2");
        assert_eq!((at(112.0), at(400.0)), (-0.25, -1.0), "above the range it is a penalty down to -1");
        let module = DepthProtection::decode(&fields(json!({"baseline_height":64.0,"damage_source":"mantle:can_protect","each_level":1.25,"neutral_range":32.0}))).unwrap().unwrap();
        let wearer = |y: f64| EntityState { y: Some(y), ..EntityState::default() };
        // Level 2 at y 32: half of 2.5 on top of the 3 the armor already gave.
        assert_eq!(with_source(Some(&wearer(32.0)), |source| module.protect(&entry(2), source, 3.0)), 4.25);
        assert_eq!(with_source(Some(&wearer(112.0)), |source| module.protect(&entry(2), source, 3.0)), 3.0 - 0.25 * 2.5);
        assert_eq!(with_source(None, |source| module.protect(&entry(2), source, 3.0)), 3.0, "no wearer reported, no change");
        assert_eq!(with_source(Some(&EntityState::default()), |source| module.protect(&entry(2), source, 3.0)), 3.0, "no height reported, no change");
        assert!(DepthProtection::decode(&fields(json!({"baseline_height":64.0,"neutral_range":32.0,"each_level":1.0,"wearing_entity":"mantle:crouching"}))).unwrap().is_some(), "a wearer condition on a reported state is kept");
        assert!(DepthProtection::decode(&fields(json!({"baseline_height":64.0,"neutral_range":32.0,"each_level":1.0,"wearing_entity":{"type":"mantle:raining"}}))).unwrap().is_none());
        assert!(DepthProtection::decode(&fields(json!({"baseline_height":64.0,"neutral_range":-1.0}))).is_err());
    }

    fn overslime_piece(amount: i32) -> NbtCompound {
        let mut persistent = NbtCompound::new();
        persistent.insert(crate::overslime::id().to_string(), Nbt::Int(amount));
        let mut nbt = NbtCompound::new();
        nbt.insert(tool_stack::TAG_PERSISTENT.into(), Nbt::Compound(persistent));
        let mut stats = NbtCompound::new();
        stats.insert("tconstruct:overslime".into(), Nbt::Float(100.0));
        nbt.insert(tool_stack::TAG_STATS.into(), Nbt::Compound(stats));
        nbt
    }

    #[test]
    fn overshield_spends_the_overslime_it_converts_and_scales_with_what_was_there() {
        // The shipped overshield: 1.25 protection and 2 overslime per level.
        let module = Overshield::decode(&fields(json!({"consumed":{"each_level":2},"protection":{"each_level":1.25}}))).unwrap();
        let mut full = overslime_piece(10);
        assert_eq!(with_source(None, |source| module.protect(&entry(2), &mut full, source, 1.0)), 1.0 + 2.5, "level 2 wants 4 and has 10");
        assert_eq!(crate::overslime::amount(&full), 6, "4 spent");
        let mut short = overslime_piece(1);
        assert_eq!(with_source(None, |source| module.protect(&entry(2), &mut short, source, 1.0)), 1.0 + 2.5 * 0.25, "1 of the 4 wanted gives a quarter");
        assert_eq!(crate::overslime::amount(&short), 0);
        let mut none = overslime_piece(0);
        assert_eq!(with_source(None, |source| module.protect(&entry(2), &mut none, source, 1.0)), 1.0);
        // Damage that bypasses invulnerability is not protected against and costs nothing.
        let mut void = overslime_piece(10);
        assert_eq!(with_damage(None, "minecraft:out_of_world", |source| module.protect(&entry(2), &mut void, source, 1.0)), 1.0);
        assert_eq!(crate::overslime::amount(&void), 10);
        let broken = Overshield::decode(&fields(json!({"consumed":{"flat":0.0},"protection":{"flat":5.0}}))).unwrap();
        let mut piece = overslime_piece(5);
        assert_eq!(with_source(None, |source| broken.protect(&entry(1), &mut piece, source, 1.0)), 1.0, "a module that consumes nothing adds nothing (the original would divide by zero)");
        assert_eq!(crate::overslime::amount(&piece), 5);
        assert!(Overshield::decode(&fields(json!({"protection":{"flat":1.0}}))).is_err());
    }

    fn restore() -> RestoreLostHealth {
        // The shipped restore: 15% chance, 25% of the damage, level I regeneration, one wear per level.
        RestoreLostHealth::decode(&fields(json!({"chance":{"flat":0.15},"durability_usage":{"each_level":1},"effect_level":{"flat":1},"percentage":{"flat":0.25}}))).unwrap().unwrap()
    }

    #[test]
    fn restore_lost_health_turns_a_share_of_the_damage_into_regeneration_when_the_chance_lands() {
        let module = restore();
        let mut rolls = [0.5_f32, 0.1].into_iter();
        let mut roll = || rolls.next().unwrap();
        assert_eq!(with_source(None, |source| module.restore(&entry(1), source, 8.0, &mut roll)), None, "a roll of 0.5 misses the 15% chance");
        // 25% of 8 is 2 health, 50 ticks each at effect level 1.
        assert_eq!(with_source(None, |source| module.restore(&entry(1), source, 8.0, &mut roll)), Some(Restore { duration: 100, wear: 1 }));
        // A hit too small to heal a whole point does nothing, and so does a level-0 effect, without drawing a roll.
        assert_eq!(with_source(None, |source| module.restore(&entry(1), source, 3.0, &mut || 0.0)), None, "25% of 3 is not a whole health point");
        let rebuff = RestoreLostHealth::decode(&fields(json!({"chance":{"flat":1.0},"durability_usage":{"each_level":1},"effect_level":{"each_level":1},"percentage":{"flat":0.5}}))).unwrap().unwrap();
        assert_eq!(with_source(None, |source| rebuff.restore(&entry(3), source, 10.0, &mut || 0.99)), Some(Restore { duration: 5 * (50 >> 2), wear: 3 }), "level 3 regenerates four times as fast");
        let never = RestoreLostHealth::decode(&fields(json!({"chance":{"flat":1.0},"durability_usage":{},"effect_level":{},"percentage":{"flat":1.0}}))).unwrap().unwrap();
        assert_eq!(with_source(None, |source| never.restore(&entry(1), source, 10.0, &mut || panic!("no roll when there is no effect"))), None);
        // A defender condition reads the wearer, and one on a fact that is not reported is not decoded.
        let crouching = RestoreLostHealth::decode(&fields(json!({"chance":{"flat":1.0},"durability_usage":{},"effect_level":{"flat":1},"percentage":{"flat":1.0},"defender":"mantle:crouching"}))).unwrap().unwrap();
        let sneaking = EntityState { crouching: true, ..EntityState::default() };
        assert!(with_source(Some(&sneaking), |source| crouching.restore(&entry(1), source, 4.0, &mut || 0.0)).is_some());
        assert!(with_source(Some(&EntityState::default()), |source| crouching.restore(&entry(1), source, 4.0, &mut || 0.0)).is_none());
        assert!(RestoreLostHealth::decode(&fields(json!({"chance":{},"durability_usage":{},"effect_level":{},"percentage":{},"defender":{"type":"mantle:raining"}}))).unwrap().is_none());
    }

    #[test]
    fn unequip_modules_hurt_by_the_level_and_shorten_a_category() {
        let entangled = DamageOnUnequip::decode(&fields(json!({"damage":1.0,"damage_type":"tconstruct:entangled"}))).unwrap();
        assert_eq!(entangled.hurt(&entry(1)), Some(("tconstruct:entangled".to_owned(), 1.0)));
        let entwined = DamageOnUnequip::decode(&fields(json!({"damage":2.0}))).unwrap();
        assert_eq!(entwined.hurt(&entry(3)), Some(("tconstruct:entangled".to_owned(), 6.0)), "the damage type defaults to entangled and the damage scales with the level");
        assert_eq!(DamageOnUnequip::decode(&fields(json!({"damage":0.0}))).unwrap().hurt(&entry(1)), None);
        assert!(DamageOnUnequip::decode(&fields(json!({"damage":-1.0}))).is_err());
        let sssss = ReduceEffectOnUnequip::decode(&fields(json!({"category":"beneficial","percent":{"each_level":0.2}}))).unwrap();
        assert_eq!((sssss.category(), sssss.percent(&entry(2))), (EffectCategory::Beneficial, 0.4));
        assert_eq!(shortened(200, 0.4), Some(120));
        assert_eq!(shortened(1, 0.9), Some(1), "never below one tick");
        assert_eq!((shortened(-1, 0.4), shortened(0, 0.4)), (None, None), "an endless effect is left alone");
        assert!(ReduceEffectOnUnequip::decode(&fields(json!({"category":"good","percent":{}}))).is_err());
    }

    #[test]
    fn update_health_heals_by_the_bonus_going_on_and_hurts_by_it_coming_off() {
        // The shipped revitalizing: no bonus for the armor slots (the attribute alone raises the maximum there),
        // 2 per level for the hands.
        let armor = UpdateHealth::decode(&fields(json!({"slots":["feet","legs","chest","head"]}))).unwrap().unwrap();
        assert_eq!((armor.delta(&entry(3), "chest", true), armor.delta(&entry(3), "chest", false)), (0.0, 0.0), "no bonus, no change");
        let hands = UpdateHealth::decode(&fields(json!({"bonus":{"each_level":2.0},"slots":["mainhand","offhand"]}))).unwrap();
        assert!(hands.is_none(), "held tools are not reported, so a hand module stays unbound");
        let mixed = UpdateHealth::decode(&fields(json!({"bonus":{"each_level":2.0},"slots":["feet","legs","chest","head"]}))).unwrap().unwrap();
        assert_eq!(mixed.delta(&entry(2), "feet", true), 4.0);
        assert_eq!(mixed.delta(&entry(2), "feet", false), -4.0);
        assert_eq!(mixed.delta(&entry(2), "mainhand", true), 0.0, "a slot the module does not name");
        assert!(UpdateHealth::decode(&fields(json!({"slots":["pocket"]}))).is_err());
        assert!(UpdateHealth::decode(&fields(json!({"bonus":2.0,"slots":["feet"]}))).is_err(), "the bonus is a leveling object");
    }

    #[test]
    fn kinetic_raises_insatiable_one_level_per_hit_up_to_the_total() {
        // The shipped kinetic: insatiable (armor) for 10 seconds.
        let grant = |total: i32, current: i32| Kinetic::grant(total, current).map(|grant| (grant.effect.to_string(), grant.amplifier, grant.duration));
        assert_eq!(grant(2, -1), Some(("tconstruct:insatiable_armor".to_owned(), 0, 200)), "a first hit grants level I");
        assert_eq!(grant(2, 0), Some(("tconstruct:insatiable_armor".to_owned(), 1, 200)), "the next hit raises it, capped at the total");
        assert_eq!(grant(2, 1), Some(("tconstruct:insatiable_armor".to_owned(), 1, 200)), "and stays there");
        assert_eq!(grant(3, 1), Some(("tconstruct:insatiable_armor".to_owned(), 2, 200)), "a higher total raises the cap");
        assert_eq!(grant(1, 4), Some(("tconstruct:insatiable_armor".to_owned(), 0, 200)), "a weaker total pulls the cap below the current level");
        assert_eq!(grant(0, -1), None, "no kinetic pieces, no grant");
    }

    #[test]
    fn the_edits_between_two_states_of_a_piece_put_what_changed_and_remove_what_went() {
        let mut before = NbtCompound::new();
        before.insert("Damage".into(), Nbt::Int(3));
        before.insert("Gone".into(), Nbt::Int(1));
        before.insert("Same".into(), Nbt::Int(7));
        let mut after = before.clone();
        after.insert("Damage".into(), Nbt::Int(5));
        after.remove("Gone");
        let edits = edits_between(&before, &after);
        assert!(edits.contains(&StackDataEdit { path: vec!["Damage".into()], value: Some(Nbt::Int(5)) }));
        assert!(edits.contains(&StackDataEdit { path: vec!["Gone".into()], value: None }));
        assert_eq!(edits.len(), 2);
        assert!(edits_between(&before, &before).is_empty());
        // The persistent data is edited by key.
        let persistent = |amount: i32| { let mut data = NbtCompound::new(); data.insert("tconstruct:overslime".into(), Nbt::Int(amount)); data.insert("other".into(), Nbt::Int(1)); let mut nbt = NbtCompound::new(); nbt.insert(tool_stack::TAG_PERSISTENT.into(), Nbt::Compound(data)); nbt };
        assert_eq!(edits_between(&persistent(4), &persistent(2)), vec![StackDataEdit { path: vec![tool_stack::TAG_PERSISTENT.into(), "tconstruct:overslime".into()], value: Some(Nbt::Int(2)) }]);
    }
}
