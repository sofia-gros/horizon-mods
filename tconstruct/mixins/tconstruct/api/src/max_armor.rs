//! MaxArmorAttributeModule and ArmorLevelModule, from SlimeKnights (MIT; see LICENSE). A max armor attribute gives the wearer an attribute
//! value worked from the LARGEST level of its modifier among the worn pieces, never a sum; an armor level adds up the level of its modifier
//! over the worn pieces under a key other modules read. Only a vanilla attribute with the `addition` operation is honored (the host can
//! only move a player's base value, which is exact for an addition); a held tool (`held_tag`) is not counted, as the player tick lists
//! worn armor only. Both skip a broken piece unless `allow_broken`.
use crate::modifier_entries::Entry;
use pluto_lib::Identifier;
use serde_json::{Map, Value};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MaxArmorAttribute { attribute: String, flat: f32, each: f32, allow_broken: bool }

/// One worn piece's claim on an attribute: its level, keyed by the module that makes it so pieces of the same modifier compete.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Candidate { key: String, attribute: String, level: f32, flat: f32, each: f32 }

fn allow_broken(fields: &Map<String, Value>) -> Result<bool, String> {
    match fields.get("allow_broken").filter(|value| !value.is_null()) {
        None => Ok(false),
        Some(Value::Bool(value)) => Ok(*value),
        Some(_) => Err("allow_broken must be boolean".into()),
    }
}

impl MaxArmorAttribute {
    /// `None` for an attribute the host lacks (a Tinkers or Forge one) or an operation other than `addition`.
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        let name = fields.get("attribute").and_then(Value::as_str).ok_or("attribute missing")?;
        let operation = fields.get("operation").and_then(Value::as_str).ok_or("operation missing")?;
        let Some(path) = name.strip_prefix("minecraft:") else { return Ok(None); };
        if operation != "addition" { return Ok(None); }
        let path = path.strip_prefix("generic.").or_else(|| path.strip_prefix("player.")).unwrap_or(path);
        let number = |key: &str| match fields.get(key).filter(|value| !value.is_null()) {
            None => Ok(0.0),
            Some(value) => value.as_f64().map(|value| value as f32).ok_or_else(|| format!("{key} must be a number")),
        };
        Ok(Some(Self { attribute: format!("minecraft:{path}"), flat: number("flat")?, each: number("each_level")?, allow_broken: allow_broken(fields)? }))
    }

    /// The claim a worn piece makes, when the modifier's conditions hold and the piece is not broken (or the module allows it).
    pub(crate) fn candidate(&self, entry: &Entry, condition: bool, broken: bool) -> Option<Candidate> {
        (condition && (!broken || self.allow_broken)).then(|| Candidate {
            key: format!("{}/{}", entry.id, self.attribute), attribute: self.attribute.clone(), level: entry.effective_level(), flat: self.flat, each: self.each,
        })
    }
}

/// What the wearer gets for the claims of every worn piece, as (attribute, amount) with the amounts of one attribute added up: for each
/// key the largest level counts once. A zero amount is not applied (the original adds no modifier for it).
pub(crate) fn strongest(pieces: impl IntoIterator<Item = Candidate>) -> Vec<(String, f64)> {
    let mut best: BTreeMap<String, Candidate> = BTreeMap::new();
    for piece in pieces {
        match best.get(&piece.key) { Some(held) if held.level >= piece.level => {} _ => { best.insert(piece.key.clone(), piece); } }
    }
    let mut totals: BTreeMap<String, f64> = BTreeMap::new();
    for piece in best.into_values() {
        if piece.level <= 0.0 { continue; }
        let amount = f64::from(piece.flat + piece.each * piece.level);
        if amount != 0.0 { *totals.entry(piece.attribute).or_insert(0.0) += amount; }
    }
    totals.into_iter().collect()
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ArmorLevel { key: Identifier, allow_broken: bool }

impl ArmorLevel {
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Self, String> {
        let key = Identifier::parse(fields.get("key").and_then(Value::as_str).ok_or("key missing")?).map_err(|e| e.to_string())?;
        Ok(Self { key, allow_broken: allow_broken(fields)? })
    }

    /// The key and the levels this piece adds (`intEffectiveLevel`), when the piece is not broken or the module allows it.
    pub(crate) fn level(&self, entry: &Entry, broken: bool) -> Option<(Identifier, i32)> {
        (!broken || self.allow_broken).then(|| (self.key.clone(), entry.int_effective_level()))
    }
}

/// The wearer's total for each key over their pieces; a key whose total is not above zero is absent, as the original removes it.
pub(crate) fn totals(pieces: impl IntoIterator<Item = (Identifier, i32)>) -> BTreeMap<Identifier, i32> {
    let mut sums: BTreeMap<Identifier, i32> = BTreeMap::new();
    for (key, amount) in pieces { let sum = sums.entry(key).or_insert(0); *sum = sum.wrapping_add(amount); }
    sums.retain(|_, sum| *sum > 0);
    sums
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn attribute(value: Value) -> MaxArmorAttribute { MaxArmorAttribute::decode(value.as_object().unwrap()).unwrap().unwrap() }
    fn entry(level: i32) -> Entry { Entry::new(Identifier::parse("tconstruct:projectile_protection").unwrap(), level) }
    fn knockback() -> MaxArmorAttribute {
        attribute(json!({"attribute":"minecraft:generic.knockback_resistance","operation":"addition","each_level":0.05}))
    }

    #[test]
    fn the_largest_level_counts_once_and_not_their_sum() {
        let module = knockback();
        let claims = [1, 3, 2].map(|level| module.candidate(&entry(level), true, false).unwrap());
        let amounts = strongest(claims);
        assert_eq!(amounts.len(), 1);
        assert_eq!(amounts[0].0, "minecraft:knockback_resistance");
        assert!((amounts[0].1 - 0.15).abs() < 1e-6, "{amounts:?}");
    }

    #[test]
    fn a_broken_piece_or_a_failed_condition_claims_nothing_unless_broken_is_allowed() {
        let module = knockback();
        assert!(module.candidate(&entry(2), true, true).is_none());
        assert!(module.candidate(&entry(2), false, false).is_none());
        let lenient = attribute(json!({"attribute":"minecraft:generic.knockback_resistance","operation":"addition","each_level":0.05,"allow_broken":true}));
        assert!(lenient.candidate(&entry(2), true, true).is_some());
        assert!(strongest(Vec::new()).is_empty(), "no pieces, no attribute");
    }

    #[test]
    fn different_modifiers_each_count_their_own_largest_and_add_up_on_one_attribute() {
        let other = Entry::new(Identifier::parse("tconstruct:other").unwrap(), 4);
        let claims = vec![knockback().candidate(&entry(2), true, false).unwrap(), knockback().candidate(&other, true, false).unwrap()];
        let amounts = strongest(claims);
        assert!((amounts[0].1 - 0.3).abs() < 1e-6, "{amounts:?}");
    }

    #[test]
    fn only_a_vanilla_addition_is_honored() {
        let decode = |value: Value| MaxArmorAttribute::decode(value.as_object().unwrap()).unwrap();
        assert!(decode(json!({"attribute":"tconstruct:player.use_item_speed","operation":"addition","each_level":0.05})).is_none());
        assert!(decode(json!({"attribute":"minecraft:generic.knockback_resistance","operation":"multiply_base","each_level":0.05})).is_none());
    }

    #[test]
    fn armor_levels_add_over_pieces_and_a_key_at_zero_is_gone() {
        let key = Identifier::parse("tconstruct:magnet").unwrap();
        let module = ArmorLevel::decode(json!({"key":"tconstruct:magnet"}).as_object().unwrap()).unwrap();
        assert!(module.level(&entry(1), true).is_none(), "a broken piece adds nothing");
        let pieces = [entry(1), entry(2)].iter().filter_map(|e| module.level(e, false)).collect::<Vec<_>>();
        assert_eq!(totals(pieces)[&key], 3);
        assert!(totals(vec![(key.clone(), 1), (key, -1)]).is_empty());
        let belt = ArmorLevel::decode(json!({"key":"tconstruct:soul_belt","allow_broken":true}).as_object().unwrap()).unwrap();
        assert_eq!(belt.level(&entry(1), true).unwrap().1, 1);
    }
}
