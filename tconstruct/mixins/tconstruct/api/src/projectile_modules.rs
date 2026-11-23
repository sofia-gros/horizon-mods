// SPDX-License-Identifier: MIT
//! The launcher modules that act when a tool fires (SlimeKnights, MIT; see LICENSE): DamageOnShootModule
//! and InfinityModule. Both work from what the shot reports about the launcher itself. The modules that
//! act on the projectile in flight (pierce, bounce, gravity, fuse, explosion, attraction, angle, glow)
//! are decoded here too and bound in `flight_hook`, on PlutoLib's projectile spawn and flight tellings:
//! the spawn pass sets a shot up, the flight pass steers it tick by tick, and the impact pass lands the
//! rest, as the original's shot, scheduled-task, hit and fuse hooks divide the work.
use crate::modifier_entries::Entry;
use pluto_lib::{Identifier, ItemSlot, Nbt};
use serde_json::{Map, Value};

fn number(fields: &Map<String, Value>, key: &str) -> Result<f32, String> {
    match fields.get(key).filter(|value| !value.is_null()) {
        None => Ok(0.0),
        Some(value) => value.as_f64().map(|value| value as f32).ok_or_else(|| format!("{key} must be a number")),
    }
}

fn identifier(fields: &Map<String, Value>, key: &str) -> Result<Identifier, String> {
    Identifier::parse(fields.get(key).and_then(Value::as_str).ok_or_else(|| format!("{key} must be an ID string"))?).map_err(|e| e.to_string())
}

/// DamageOnShootModule: firing the tool hurts its holder by the damage, of the named type.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DamageOnShoot { damage: f32, damage_type: Identifier }

impl DamageOnShoot {
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Self, String> {
        let damage = number(fields, "damage")?;
        if damage < 0.0 { return Err("damage must not be negative".into()); }
        Ok(Self { damage, damage_type: identifier(fields, "damage_type")? })
    }

    /// onProjectileLaunch: the damage type and the amount (the damage times the effective level) the
    /// holder is hurt by; None for none.
    pub(crate) fn hurt(&self, entry: &Entry) -> Option<(String, f32)> {
        let amount = self.damage * entry.effective_level();
        (amount > 0.0).then(|| (self.damage_type.to_string(), amount))
    }
}

/// InfinityModule: a launcher that fires its named ammo without the shooter carrying any, paid for with
/// the tool's durability. The arrows it supplies are marked `tic_infinity`, and the modifier's variant
/// stored on the tool is copied onto them under the variant tag, as findAmmo does. What the module does
/// on shoot in the original — set the fired arrow to creative-only pickup — has no surface here: the
/// launch and shot events report the arrow but cannot change it, so a supplied arrow that lands can be
/// picked up like a normal one.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Infinity { ammo: Identifier, variant_tag: String, durability_usage: i32, check_standard_arrows: bool }

impl Infinity {
    /// `None` for an ammo stack form this port cannot honor (an item stack carrying NBT of its own).
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        let ammo = match fields.get("ammo").filter(|value| !value.is_null()) {
            Some(Value::String(text)) => Identifier::parse(text).map_err(|e| e.to_string())?,
            Some(Value::Object(object)) => {
                if object.keys().any(|key| key != "item") { return Ok(None); }
                identifier(object, "item")?
            }
            _ => return Err("ammo must be an item stack".into()),
        };
        let variant_tag = match fields.get("variant_tag").filter(|value| !value.is_null()) {
            None => String::new(),
            Some(Value::String(text)) => text.clone(),
            Some(_) => return Err("variant_tag must be a string".into()),
        };
        let durability_usage = match fields.get("durability_usage").filter(|value| !value.is_null()) {
            Some(Value::Number(number)) => { let value = crate::materials::number_int(&number.to_string()); if value < 0 { return Err("durability_usage must not be negative".into()); } value }
            _ => return Err("durability_usage must be an integer".into()),
        };
        let check_standard_arrows = match fields.get("check_standard_arrows").filter(|value| !value.is_null()) {
            None => true,
            Some(Value::Bool(value)) => *value,
            Some(_) => return Err("check_standard_arrows must be a boolean".into()),
        };
        Ok(Some(Self { ammo, variant_tag, durability_usage, check_standard_arrows }))
    }

    /// Whether the module leaves a shot with standard ammo alone (findAmmo returns empty for it).
    pub(crate) fn skips_standard(&self, standard_present: bool) -> bool { self.check_standard_arrows && standard_present }

    /// findAmmo's count: how many of the ammo the tool's remaining durability pays for, at most a stack;
    /// free when the usage is 0. 0 when the durability pays for none.
    pub(crate) fn count(&self, durability: i32) -> u32 {
        if self.durability_usage <= 0 { return 64; }
        let paid = durability.max(0).saturating_add(self.durability_usage - 1) / self.durability_usage;
        paid.min(64) as u32
    }

    /// The durability one shot costs (shrinkAmmo damages the tool by the usage per arrow consumed).
    pub(crate) fn usage(&self) -> i32 { self.durability_usage }

    /// onRemoved: the persistent key the tool's variant is stored under (the modifier's own ID), removed
    /// when the module names a variant tag; None when it stores nothing.
    pub(crate) fn variant_key(&self, modifier: &Identifier) -> Option<String> {
        (!self.variant_tag.is_empty()).then(|| modifier.to_string())
    }

    /// findAmmo's stack: `count` of the ammo, marked as infinity, with the tool's stored variant copied
    /// under the variant tag when both the module and the tool name one.
    pub(crate) fn supply(&self, count: u32, variant: &str) -> ItemSlot {
        let mut stack = ItemSlot::new(self.ammo.clone(), count);
        stack.custom_data.insert("tic_infinity".into(), Nbt::Byte(1));
        if !self.variant_tag.is_empty() && !variant.is_empty() {
            stack.custom_data.insert(self.variant_tag.clone(), Nbt::String(variant.to_owned()));
        }
        stack
    }
}

/// A leveling value as the ranged modules carry it: a bare number, or an object with `flat` and
/// `each_level` parts (the original's `LevelingValue`). Returns (flat, each_level); absent is (0, 0).
fn leveling(fields: &Map<String, Value>, key: &str) -> Result<(f32, f32), String> {
    match fields.get(key).filter(|value| !value.is_null()) {
        None => Ok((0.0, 0.0)),
        Some(Value::Number(value)) => Ok((value.as_f64().ok_or_else(|| format!("{key} must be a number"))? as f32, 0.0)),
        Some(Value::Object(object)) => Ok((number(object, "flat")?, number(object, "each_level")?)),
        Some(_) => Err(format!("{key} must be a leveling value")),
    }
}

/// A leveling value at the module's top level (the module record itself is the value: ArrowPierceModule,
/// ProjectileBounceModule, ProjectileGravityModule).
fn leveling_top(fields: &Map<String, Value>) -> Result<(f32, f32), String> {
    Ok((number(fields, "flat")?, number(fields, "each_level")?))
}

fn at(leveling: (f32, f32), entry: &Entry) -> f32 {
    leveling.0 + leveling.1 * entry.effective_level()
}

fn boolean(fields: &Map<String, Value>, key: &str, fallback: bool) -> Result<bool, String> {
    match fields.get(key).filter(|value| !value.is_null()) {
        None => Ok(fallback),
        Some(Value::Bool(value)) => Ok(*value),
        Some(_) => Err(format!("{key} must be a boolean")),
    }
}

/// ArrowPierceModule: on shot, the arrow's pierce level is set to the leveling value.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ArrowPierce { leveling: (f32, f32) }

impl ArrowPierce {
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Self, String> {
        Ok(Self { leveling: leveling_top(fields)? })
    }

    pub(crate) fn pierce(&self, entry: &Entry) -> u8 {
        (at(self.leveling, entry) as i32).clamp(0, 255) as u8
    }
}

/// ProjectileBounceModule: on a block hit, while the bounces this modifier has used on the projectile
/// are under its leveling limit and the motion into the surface is fast enough, the hit is consumed and
/// the motion bounces (scaled 0.9, the hit axis negated).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProjectileBounce { leveling: (f32, f32) }

impl ProjectileBounce {
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Self, String> {
        Ok(Self { leveling: leveling_top(fields)? })
    }

    /// The most bounces this module grants one projectile (LevelingInt).
    pub(crate) fn limit(&self, entry: &Entry) -> i32 {
        at(self.leveling, entry) as i32
    }
}

/// ProjectileGravityModule: on shot the projectile flies without gravity; a scheduled task restores
/// gravity after the leveling delay in ticks (the hover).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProjectileGravity { leveling: (f32, f32) }

impl ProjectileGravity {
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Self, String> {
        Ok(Self { leveling: leveling_top(fields)? })
    }

    pub(crate) fn delay(&self, entry: &Entry) -> u32 {
        at(self.leveling, entry).round().max(0.0) as u32
    }
}

/// ProjectileFuseModule: on shot it schedules the fuse task; when the leveling time in ticks passes,
/// the projectile's fuse hooks fire (explosion, attraction), it sheds particles, and a single-use
/// projectile is discarded.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProjectileFuse { particle: String, time: (f32, f32) }

impl ProjectileFuse {
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Self, String> {
        let particle = match fields.get("particle").filter(|value| !value.is_null()) {
            None => "minecraft:flame".to_owned(),
            Some(Value::String(text)) => text.clone(),
            Some(_) => return Err("particle must be a string".into()),
        };
        Ok(Self { particle, time: leveling(fields, "time")? })
    }

    pub(crate) fn ticks(&self, entry: &Entry) -> u32 {
        at(self.time, entry).round().max(0.0) as u32
    }

    pub(crate) fn particle(&self) -> &str { &self.particle }
}

/// ProjectileExplosionModule: on a hit, and when a fuse ends, the projectile explodes where it is, if
/// its radius passes 0.5; the blast is the game's own, so the module's knockback multiplier rides the
/// game's blast knockback unscaled (a reduction against the original's custom explosion).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProjectileExplosion {
    radius: (f32, f32),
    place_fire: bool,
    destroy: bool,
    efln_bonus: f32,
}

impl ProjectileExplosion {
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Self, String> {
        let destroy = match fields.get("block_interaction").filter(|value| !value.is_null()) {
            None => false,
            Some(Value::String(text)) => text == "destroy",
            Some(_) => return Err("block_interaction must be a string".into()),
        };
        Ok(Self {
            radius: leveling(fields, "radius")?,
            place_fire: boolean(fields, "place_fire", false)?,
            destroy,
            efln_bonus: number(fields, "efln_bonus")?,
        })
    }

    /// The blast radius for a projectile; `efln` adds the bonus, as a fins-marked projectile's power
    /// feeds its EFLN blast. 0.5 or under does not explode.
    pub(crate) fn radius(&self, entry: &Entry, efln: bool) -> f32 {
        at(self.radius, entry) + if efln { self.efln_bonus } else { 0.0 }
    }

    pub(crate) fn place_fire(&self) -> bool { self.place_fire }
    pub(crate) fn destroy(&self) -> bool { self.destroy }
}

/// ProjectileAttractMobsModule: on a hit, and when a fuse ends, living entities near the point are
/// pulled toward it. Priority 125 in the shipped data, so it runs before the explosion that ends the
/// projectile.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProjectileAttractMobs { radius: (f32, f32), strength: (f32, f32) }

impl ProjectileAttractMobs {
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Self, String> {
        Ok(Self { radius: leveling(fields, "radius")?, strength: leveling(fields, "strength")? })
    }

    pub(crate) fn radius(&self, entry: &Entry) -> f32 { at(self.radius, entry) }
    pub(crate) fn strength(&self, entry: &Entry) -> f32 { at(self.strength, entry) }
}

/// ProjectilePlaceGlowModule: on a hit, a glow block is placed where the projectile landed (at a hit
/// entity's feet facing down, or in the cell on a hit block's face); placing one on a block hit
/// discards the projectile.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProjectilePlaceGlow { blocks: bool, entities: bool, tool_damage: i32 }

impl ProjectilePlaceGlow {
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Self, String> {
        let tool_damage = match fields.get("tool_damage").filter(|value| !value.is_null()) {
            None => 0,
            Some(Value::Number(value)) => crate::materials::number_int(&value.to_string()),
            Some(_) => return Err("tool_damage must be an integer".into()),
        };
        Ok(Self { blocks: boolean(fields, "blocks", true)?, entities: boolean(fields, "entities", true)?, tool_damage })
    }

    pub(crate) fn blocks(&self) -> bool { self.blocks }
    pub(crate) fn entities(&self) -> bool { self.entities }
    pub(crate) fn tool_damage(&self) -> i32 { self.tool_damage }
}

/// RestrictAngleModule: on shot, the projectile's direction is clamped to a fan of fixed angles:
/// `max(4, 2^(6-level))` directions around the circle.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RestrictAngle;

impl RestrictAngle {
    pub(crate) fn decode(_fields: &Map<String, Value>) -> Result<Self, String> {
        Ok(Self)
    }

    pub(crate) fn directions(&self, entry: &Entry) -> u32 {
        let level = entry.effective_level() as i32;
        let fan = if level >= 6 { 1 } else { 1u32 << (6 - level) };
        fan.max(4)
    }

    /// The motion with its horizontal direction snapped to the fan's nearest angle, its speed and
    /// vertical motion kept.
    pub(crate) fn clamp(&self, entry: &Entry, motion: [f64; 3]) -> [f64; 3] {
        clamp_to(self.directions(entry), motion)
    }
}

/// [`RestrictAngle::clamp`] with the fan size already computed.
pub(crate) fn clamp_to(directions: u32, motion: [f64; 3]) -> [f64; 3] {
    let horizontal = (motion[0] * motion[0] + motion[2] * motion[2]).sqrt();
    if horizontal < 1e-9 || directions == 0 { return motion; }
    let step = std::f64::consts::TAU / f64::from(directions);
    let yaw = (-motion[0]).atan2(motion[2]);
    let snapped = (yaw / step).round() * step;
    [-snapped.sin() * horizontal, motion[1], snapped.cos() * horizontal]
}

/// What a fireball option matches in the shooter's inventory (the match half of a
/// FireballType): one item, or an item tag. Another ingredient form leaves the module
/// unbound.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum AmmoMatch {
    Item(Identifier),
    Tag(Identifier),
}

impl AmmoMatch {
    fn decode(value: &Value) -> Result<Option<Self>, String> {
        let Value::Object(object) = value else { return Err("fireball match must be an object".into()) };
        if object.contains_key("item") {
            return Ok(Some(Self::Item(identifier(object, "item")?)));
        }
        if object.contains_key("tag") {
            return Ok(Some(Self::Tag(identifier(object, "tag")?)));
        }
        Ok(None)
    }

    /// Whether a carried stack is this ammo; `in_tag` answers item tag membership.
    pub(crate) fn matches(&self, item: &Identifier, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> bool {
        match self {
            Self::Item(wanted) => wanted == item,
            Self::Tag(tag) => in_tag(item, tag),
        }
    }
}

/// A damage multiplier field, 1 when absent (the fireball loaders' default).
fn defaulted_multiplier(fields: &Map<String, Value>, key: &str) -> Result<f32, String> {
    match fields.get(key).filter(|value| !value.is_null()) {
        None => Ok(1.0),
        Some(_) => number(fields, key),
    }
}

/// A damage type pair (ranged, melee); the melee half doubles as the fireball's
/// enderference override type (CustomFireball.setDamageType's quirk).
fn damage_pair(value: &Value) -> Result<Option<(Identifier, Identifier)>, String> {
    let Value::Object(object) = value else { return Err("damage_type must be an object".into()) };
    Ok(Some((identifier(object, "ranged")?, identifier(object, "melee")?)))
}

/// A list of modifier entries (`{name, level}`), as the ammo modifier lists carry them.
fn modifier_list(fields: &Map<String, Value>, key: &str) -> Result<Vec<(Identifier, i32)>, String> {
    match fields.get(key).filter(|value| !value.is_null()) {
        None => Ok(Vec::new()),
        Some(Value::Array(list)) => list
            .iter()
            .map(|entry| {
                let object = entry.as_object().ok_or("modifier entry must be an object")?;
                let level = match object.get("level") {
                    Some(Value::Number(number)) => crate::materials::number_int(&number.to_string()),
                    _ => return Err("modifier entry level must be an integer".into()),
                };
                Ok((identifier(object, "name")?, level))
            })
            .collect(),
        Some(_) => Err(format!("{key} must be a list")),
    }
}

/// One FireballType: the ammo it matches, the modifiers it adds to the shot, and its own
/// damage type pair and damage multiplier where they override the module's.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FireballType {
    pub matches: AmmoMatch,
    pub ammo_modifiers: Vec<(Identifier, i32)>,
    /// The (ranged, melee) pair, when this type names its own.
    pub damage_type: Option<(Identifier, Identifier)>,
    pub damage_multiplier: f32,
}

impl FireballType {
    fn decode(value: &Value) -> Result<Option<Self>, String> {
        let Value::Object(object) = value else { return Err("fireball option must be an object".into()) };
        let Some(matches) = (match object.get("match") {
            Some(match_value) => AmmoMatch::decode(match_value)?,
            None => return Err("fireball option match is required".into()),
        }) else {
            return Ok(None);
        };
        let damage_type = match object.get("damage_type").filter(|value| !value.is_null()) {
            None => None,
            Some(value) => damage_pair(value)?,
        };
        Ok(Some(Self {
            matches,
            ammo_modifiers: modifier_list(object, "ammo_modifiers")?,
            damage_type,
            damage_multiplier: defaulted_multiplier(object, "damage_multiplier")?,
        }))
    }
}

/// FireballModule: using the tool shoots a fireball carrying one of the option ammos the
/// shooter carries (the firing itself is `fireball_hook`'s). The durability is a
/// LevelingInt, (flat, each_level).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Fireball {
    pub options: Vec<FireballType>,
    /// The module's (ranged, melee) damage type pair.
    pub damage_type: (Identifier, Identifier),
    pub damage_multiplier: f32,
    pub ammo_modifiers: Vec<(Identifier, i32)>,
    pub durability: (i32, i32),
    pub sound: Identifier,
}

impl Fireball {
    /// `None` when an option matches an ingredient form this port cannot honor.
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        let options = match fields.get("options").filter(|value| !value.is_null()) {
            Some(Value::Array(list)) => {
                let mut options = Vec::with_capacity(list.len());
                for option in list {
                    match FireballType::decode(option)? {
                        Some(option) => options.push(option),
                        None => return Ok(None),
                    }
                }
                options
            }
            _ => return Err("options must be a list".into()),
        };
        let damage_type = match fields.get("damage_type").filter(|value| !value.is_null()) {
            Some(value) => damage_pair(value)?.ok_or("damage_type must name ranged and melee")?,
            None => return Err("damage_type is required".into()),
        };
        let durability = match fields.get("durability").filter(|value| !value.is_null()) {
            Some(Value::Object(object)) => {
                let part = |key: &str| match object.get(key).filter(|value| !value.is_null()) {
                    None => Ok(0),
                    Some(Value::Number(number)) => Ok(crate::materials::number_int(&number.to_string())),
                    Some(_) => Err("durability must be a leveling int".to_string()),
                };
                (part("flat")?, part("each_level")?)
            }
            Some(Value::Number(number)) => (0, crate::materials::number_int(&number.to_string())),
            _ => return Err("durability is required".into()),
        };
        let sound = match fields.get("sound").filter(|value| !value.is_null()) {
            None => Identifier::parse("minecraft:entity.blaze.shoot").map_err(|e| e.to_string())?,
            Some(_) => identifier(fields, "sound")?,
        };
        Ok(Some(Self {
            options,
            damage_type,
            damage_multiplier: defaulted_multiplier(fields, "damage_multiplier")?,
            ammo_modifiers: modifier_list(fields, "ammo_modifiers")?,
            durability,
            sound,
        }))
    }

    /// The first option matching a carried stack (getFireballType over the found ammo).
    pub(crate) fn option_for(&self, item: &Identifier, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Option<&FireballType> {
        self.options.iter().find(|option| option.matches.matches(item, in_tag))
    }

    /// The durability one shot costs at the entry's level (LevelingInt.compute).
    pub(crate) fn usage(&self, entry: &Entry) -> i32 {
        crate::modifier_modules::leveling_compute(self.durability, entry.effective_level())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fields(value: Value) -> Map<String, Value> { value.as_object().unwrap().clone() }
    fn entry(level: i32) -> Entry { Entry::new(Identifier::parse("tconstruct:spiny").unwrap(), level) }

    #[test]
    fn fireball_decodes_the_shipped_slimeball_module() {
        // The shipped slimeball module, cut down to two of its five options.
        let module = Fireball::decode(&fields(json!({
            "ammo_modifiers": [{"level": 1, "name": "tconstruct:bounce"}],
            "damage_multiplier": 1.5,
            "damage_type": {"melee": "tconstruct:fluid_impact_melee", "ranged": "tconstruct:fluid_impact_ranged"},
            "durability": {"flat": 1},
            "options": [
                {"ammo_modifiers": [{"level": 2, "name": "tconstruct:drawback"}], "damage_multiplier": 0.67, "match": {"tag": "forge:slimeball/earth"}},
                {"ammo_modifiers": [{"level": 1, "name": "tconstruct:explosive"}], "damage_type": {"melee": "tconstruct:mob_explosion_melee", "ranged": "tconstruct:mob_explosion_ranged"}, "match": {"item": "minecraft:magma_cream"}}
            ],
            "sound": "tconstruct:slimy_bounce"
        }))).unwrap().unwrap();
        assert_eq!(module.damage_multiplier, 1.5);
        assert_eq!(module.usage(&entry(1)), 1);
        assert_eq!(module.sound.to_string(), "tconstruct:slimy_bounce");
        let magma = Identifier::parse("minecraft:magma_cream").unwrap();
        let option = module.option_for(&magma, &|_, _| false).unwrap();
        assert_eq!(option.damage_multiplier, 1.0, "an option without a multiplier defaults to 1");
        assert_eq!(option.damage_type.as_ref().unwrap().0.to_string(), "tconstruct:mob_explosion_ranged");
        let earth = Identifier::parse("tconstruct:earth_slimeball").unwrap();
        let in_tag = |item: &Identifier, tag: &Identifier| item == &earth && tag.to_string() == "forge:slimeball/earth";
        assert_eq!(module.option_for(&earth, &in_tag).unwrap().damage_multiplier, 0.67);
    }

    #[test]
    fn damage_on_shoot_hurts_the_holder_by_the_damage_times_the_level() {
        // The shipped spiny: 1 damage of the spiny type.
        let module = DamageOnShoot::decode(&fields(json!({"damage":1.0,"damage_type":"tconstruct:spiny"}))).unwrap();
        assert_eq!(module.hurt(&entry(1)), Some(("tconstruct:spiny".to_owned(), 1.0)));
        assert_eq!(module.hurt(&entry(3)), Some(("tconstruct:spiny".to_owned(), 3.0)));
        let none = DamageOnShoot::decode(&fields(json!({"damage":0.0,"damage_type":"tconstruct:spiny"}))).unwrap();
        assert_eq!(none.hurt(&entry(2)), None);
        assert!(DamageOnShoot::decode(&fields(json!({"damage":-1.0,"damage_type":"tconstruct:spiny"}))).is_err());
        assert!(DamageOnShoot::decode(&fields(json!({"damage":1.0}))).is_err(), "the damage type is required");
    }

    #[test]
    fn flight_modules_decode_the_shipped_shapes_and_compute_their_values() {
        // impaling: arrow_pierce, 1 per level at the top level.
        let pierce = ArrowPierce::decode(&fields(json!({"each_level":1}))).unwrap();
        assert_eq!((pierce.pierce(&entry(1)), pierce.pierce(&entry(3))), (1, 3));
        // bounce: 2 per level, -1 flat: level 1 grants a single bounce.
        let bounce = ProjectileBounce::decode(&fields(json!({"each_level":2,"flat":-1}))).unwrap();
        assert_eq!((bounce.limit(&entry(1)), bounce.limit(&entry(2))), (1, 3));
        // hover: gravity returns after a flat 20 ticks.
        let gravity = ProjectileGravity::decode(&fields(json!({"flat":20}))).unwrap();
        assert_eq!(gravity.delay(&entry(1)), 20);
        // fuse: flame particles after a flat 10 ticks.
        let fuse = ProjectileFuse::decode(&fields(json!({"particle":"minecraft:flame","time":{"flat":10}}))).unwrap();
        assert_eq!((fuse.ticks(&entry(1)), fuse.particle()), (10, "minecraft:flame"));
        // explosive: radius 1 + 1 per level, destroy interaction, an efln bonus of 0.5.
        let boom = ProjectileExplosion::decode(&fields(json!({"block_interaction":"destroy","efln_bonus":0.5,"knockback":{"flat":1.0},"place_fire":false,"radius":{"each_level":1.0,"flat":1.0}}))).unwrap();
        assert_eq!((boom.radius(&entry(1), false), boom.radius(&entry(2), false), boom.radius(&entry(1), true)), (2.0, 3.0, 2.5));
        assert!(boom.destroy() && !boom.place_fire());
        // attractive: radius 3 per level, strength a flat 0.5.
        let attract = ProjectileAttractMobs::decode(&fields(json!({"radius":{"each_level":3.0},"strength":{"flat":0.5}}))).unwrap();
        assert_eq!((attract.radius(&entry(2)), attract.strength(&entry(2))), (6.0, 0.5));
        // glowing places on both, spectral on blocks only.
        let glow = ProjectilePlaceGlow::decode(&fields(json!({"blocks":true,"entities":true,"tool_damage":5}))).unwrap();
        assert!(glow.blocks() && glow.entities() && glow.tool_damage() == 5);
        let spectral = ProjectilePlaceGlow::decode(&fields(json!({"blocks":true,"entities":false,"tool_damage":5}))).unwrap();
        assert!(spectral.blocks() && !spectral.entities());
        // crystalbound: the fan halves per level and never goes under 4 directions.
        let angle = RestrictAngle::decode(&fields(json!({}))).unwrap();
        assert_eq!((angle.directions(&entry(1)), angle.directions(&entry(2)), angle.directions(&entry(5))), (32, 16, 4));
        // A direction already on a fan step (-90 degrees is a multiple of the 11.25 degree step) is kept.
        let kept = angle.clamp(&entry(1), [1.0, 0.25, 0.0]);
        assert!((kept[0] - 1.0).abs() < 1e-9 && kept[1] == 0.25 && kept[2].abs() < 1e-9, "kept on its step: {kept:?}");
        // One between steps snaps to the nearest: yaw -95.7 degrees goes to -101.25 (step -9).
        let clamped = angle.clamp(&entry(1), [0.995, 0.25, -0.0995]);
        assert!((clamped[0] - 0.9808).abs() < 0.001 && (clamped[2] + 0.1951).abs() < 0.001, "snapped to step -9: {clamped:?}");
        assert_eq!(clamped[1], 0.25, "the vertical motion is kept");
    }

    #[test]
    fn infinity_supplies_a_stack_paid_for_with_durability_and_marks_it() {
        // The shipped crystalshot: crystalshot arrows, 4 durability a shot, the variant copied.
        let module = Infinity::decode(&fields(json!({"ammo":"tconstruct:crystalshot","check_standard_arrows":true,"durability_usage":4,"variant_tag":"variant"}))).unwrap().unwrap();
        assert!(module.skips_standard(true));
        assert!(!module.skips_standard(false));
        assert_eq!((module.count(100), module.count(9), module.count(4), module.count(3), module.count(0)), (25, 3, 1, 1, 0), "the count rounds up and stops at a stack");
        assert_eq!(module.count(10_000), 64);
        assert_eq!(module.usage(), 4);
        let stack = module.supply(3, "rose_gold");
        assert_eq!((stack.item.to_string(), stack.count), ("tconstruct:crystalshot".to_owned(), 3));
        assert_eq!(stack.custom_data.get("tic_infinity"), Some(&Nbt::Byte(1)));
        assert_eq!(stack.custom_data.get("variant"), Some(&Nbt::String("rose_gold".to_owned())));
        let plain = module.supply(1, "");
        assert!(!plain.custom_data.contains_key("variant"), "no variant stored, none copied");
        let free = Infinity::decode(&fields(json!({"ammo":"minecraft:arrow","durability_usage":0}))).unwrap().unwrap();
        assert_eq!(free.count(0), 64, "a usage of 0 fires for free");
        assert!(Infinity::decode(&fields(json!({"ammo":{"item":"minecraft:arrow","nbt":{}},"durability_usage":1}))).unwrap().is_none(), "an ammo stack with its own NBT stays unbound");
        assert!(Infinity::decode(&fields(json!({"ammo":{"item":"minecraft:arrow"},"durability_usage":1}))).unwrap().is_some());
    }
}
