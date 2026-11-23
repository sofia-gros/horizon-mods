//! ReplaceBlockWalkerModule (SlimeKnights, MIT; see LICENSE): walking replaces the block below an
//! empty cell with another block; the shipped form is frost walker, whose boots turn still water
//! under air into frosted ice. A replacement's target is a Mantle block predicate over a block
//! state; this file is the port's first block-state predicate decoder, covering the forms the
//! shipped data uses (`mantle:block_properties`, and the default `mantle:any`).
use crate::{modifier_modules::Condition, wearer_modules::Leveling};
use pluto_lib::{BlockState, Identifier};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// The properties a block state predicate or a replacement state names, by name; a property left
/// out is unconstrained (for a predicate) or keeps the block's default (for a state). Mantle's
/// block state data spells every property value as a string.
fn properties(object: &Map<String, Value>) -> Result<BTreeMap<String, String>, String> {
    let mut named = BTreeMap::new();
    if let Some(listed) = object.get("properties").filter(|value| !value.is_null()) {
        let listed = listed.as_object().ok_or("block properties must be an object")?;
        for (name, value) in listed {
            let value = value.as_str().ok_or("block property values must be strings")?;
            named.insert(name.clone(), value.to_owned());
        }
    }
    Ok(named)
}

/// A Mantle block predicate over a block state (BlockPredicate): any state, or one named block
/// whose listed properties all match (BlockPropertiesPredicate).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BlockPredicate {
    Any,
    Properties { block: Identifier, properties: BTreeMap<String, String> },
}

impl BlockPredicate {
    /// `Ok(None)` is a predicate form this port cannot honor, which leaves the module unbound;
    /// `Err` is malformed data for a form it does implement. An absent target is the loader's
    /// default, any (BlockReplacement's target is a defaultField).
    pub(crate) fn decode(value: Option<&Value>) -> Result<Option<Self>, String> {
        let Some(value) = value.filter(|value| !value.is_null()) else { return Ok(Some(Self::Any)); };
        // A bare string names its type directly, the shorthand Mantle's predicate loaders accept.
        let kind = match value {
            Value::String(kind) => kind.as_str(),
            Value::Object(object) => object.get("type").and_then(Value::as_str).ok_or("a block predicate needs a type")?,
            _ => return Err("a block predicate must be a type name or an object".into()),
        };
        match kind {
            "mantle:any" => Ok(Some(Self::Any)),
            "mantle:block_properties" => {
                let object = value.as_object().ok_or("block_properties must be an object")?;
                let block = object.get("block").and_then(Value::as_str).ok_or("block_properties needs block")?;
                let block = Identifier::parse(block).map_err(|error| error.to_string())?;
                Ok(Some(Self::Properties { block, properties: properties(object)? }))
            }
            _ => Ok(None),
        }
    }
    /// Whether the state is the predicate's block with every listed property at its listed value.
    pub(crate) fn matches(&self, state: &BlockState) -> bool {
        match self {
            Self::Any => true,
            Self::Properties { block, properties } => state.block == *block && properties.iter().all(|(name, value)| state.property(name) == Some(value.as_str())),
        }
    }
}

/// One entry of the module's `replace` list (BlockReplacement): a state matching `target` is
/// replaced by `state`, while the modifier's level lies in the entry's own range.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Replacement {
    pub(crate) target: BlockPredicate,
    pub(crate) state: BlockState,
    pub(crate) min: i32,
    pub(crate) max: i32,
}

impl Replacement {
    /// `Ok(None)` is a target form this port cannot honor (the module stays unbound).
    fn decode(value: &Value) -> Result<Option<Self>, String> {
        let object = value.as_object().ok_or("a replacement must be an object")?;
        // BlockStateLoadable.DIFFERENCE pulls the replacement state from this object directly:
        // `block` and `properties` are the entry's own keys, beside `target` and `modifier_level`.
        let block = object.get("block").and_then(Value::as_str).ok_or("a replacement needs block")?;
        let mut state = BlockState::new(Identifier::parse(block).map_err(|error| error.to_string())?);
        state.properties = properties(object)?;
        let Some(target) = BlockPredicate::decode(object.get("target"))? else { return Ok(None); };
        let (min, max) = crate::modifiers::range(object.get("modifier_level"))?;
        Ok(Some(Self { target, state, min, max }))
    }
    /// IntRange.test against the modifier's level (walkOn reads entry.getLevel()).
    pub(crate) fn accepts(&self, level: i32) -> bool { self.min <= level && level <= self.max }
}

/// ReplaceBlockWalkerModule: the replacements, the walk radius and the module's condition.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ReplaceFluid {
    pub(crate) replacements: Vec<Replacement>,
    pub(crate) radius: Leveling,
    pub(crate) condition: Condition,
}

impl ReplaceFluid {
    /// `Ok(None)` is a replacement or predicate form this port cannot honor (unbound);
    /// `Err` is malformed data for a form it does implement.
    pub(crate) fn decode(fields: &Map<String, Value>, condition: Condition) -> Result<Option<Self>, String> {
        let listed = fields.get("replace").and_then(Value::as_array).ok_or("replace_fluid needs its replace list")?;
        let mut replacements = Vec::with_capacity(listed.len());
        for entry in listed {
            let Some(replacement) = Replacement::decode(entry)? else { return Ok(None); };
            replacements.push(replacement);
        }
        Ok(Some(Self { replacements, radius: Leveling::decode(fields.get("radius"), "radius")?, condition }))
    }
    /// The replacements whose level range accepts `level`, in data order (walkOn tests each in
    /// order and stops at the first that matches).
    pub(crate) fn at_level(&self, level: i32) -> Vec<Replacement> {
        self.replacements.iter().filter(|replacement| replacement.accepts(level)).cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn state(text: &str) -> BlockState { BlockState::parse(text).expect("a state string") }

    #[test]
    fn the_shipped_target_matches_only_still_water() {
        // frost_walker's replacement target: water whose level property is 0.
        let target = BlockPredicate::decode(Some(&json!({"type": "mantle:block_properties", "block": "minecraft:water", "properties": {"level": "0"}}))).unwrap().expect("a form this port honors");
        assert!(target.matches(&state("minecraft:water[level=0]")));
        assert!(!target.matches(&state("minecraft:water[level=1]")), "flowing water is not replaced");
        assert!(!target.matches(&state("minecraft:lava[level=0]")));
        assert!(!target.matches(&state("minecraft:air")));
    }

    #[test]
    fn an_absent_or_any_target_matches_everything() {
        for value in [None, Some(json!("mantle:any")), Some(json!({"type": "mantle:any"}))] {
            let target = BlockPredicate::decode(value.as_ref()).unwrap().expect("any is honored");
            assert!(target.matches(&state("minecraft:water[level=7]")));
            assert!(target.matches(&state("minecraft:stone")));
        }
    }

    #[test]
    fn an_unknown_predicate_form_stays_unbound() {
        assert!(BlockPredicate::decode(Some(&json!({"type": "mantle:tag", "tag": "minecraft:base_stone_overworld"}))).unwrap().is_none());
    }

    #[test]
    fn the_shipped_replacement_decodes_with_its_state_and_full_range() {
        let entry = json!({"block": "minecraft:frosted_ice", "target": {"type": "mantle:block_properties", "block": "minecraft:water", "properties": {"level": "0"}}});
        let replacement = Replacement::decode(&entry).unwrap().expect("the shipped form is honored");
        assert_eq!(replacement.state, state("minecraft:frosted_ice"));
        assert!(replacement.accepts(1) && replacement.accepts(10), "no modifier_level names every level");
        // A replacement state may name its own properties beside its block.
        let aged = Replacement::decode(&json!({"block": "minecraft:frosted_ice", "properties": {"age": "2"}})).unwrap().unwrap();
        assert_eq!(aged.state, state("minecraft:frosted_ice[age=2]"));
        assert_eq!(aged.target, BlockPredicate::Any, "an absent target is any");
    }

    #[test]
    fn a_replacement_level_range_gates_by_the_modifier_level() {
        let gated = Replacement::decode(&json!({"block": "minecraft:ice", "modifier_level": {"min": 2, "max": 3}})).unwrap().unwrap();
        assert!(!gated.accepts(1));
        assert!(gated.accepts(2) && gated.accepts(3));
        assert!(!gated.accepts(4));
        let exact = Replacement::decode(&json!({"block": "minecraft:ice", "modifier_level": 2})).unwrap().unwrap();
        assert!(!exact.accepts(1) && exact.accepts(2) && !exact.accepts(3), "a bare number is an exact level");
    }
}
