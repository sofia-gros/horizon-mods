// SPDX-License-Identifier: MIT
//! The launcher modules that act when a tool fires (SlimeKnights, MIT; see LICENSE): DamageOnShootModule
//! and InfinityModule. Both work from what the shot reports about the launcher itself; the modules that
//! act on the projectile in flight (pierce, bounce, gravity, fuse, explosion, attraction, angle, glow)
//! stay unbound, as no event here reports a projectile's modifiers or motion or lets a callback change
//! them (the same gap `punch` records).
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fields(value: Value) -> Map<String, Value> { value.as_object().unwrap().clone() }
    fn entry(level: i32) -> Entry { Entry::new(Identifier::parse("tconstruct:spiny").unwrap(), level) }

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
