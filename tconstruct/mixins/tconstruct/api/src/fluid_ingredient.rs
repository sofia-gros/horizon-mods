//! The fluid and item stacks that smeltery recipes name, and the fluid tags they name them by: Mantle's FluidIngredient
//! and FluidOutput, Forge/Mantle ItemOutput and the fluid tag files (SlimeKnights Tinkers' Construct and Mantle, MIT;
//! see LICENSE). Decoding is strict: a field this port does not read is reported, never skipped.
#![allow(dead_code)]
use pluto_lib::Identifier;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

/// A tag lookup: whether a fluid belongs to a fluid tag.
pub(crate) type InTag<'a> = &'a dyn Fn(&Identifier, &Identifier) -> bool;

pub(crate) fn id_of(value: &Value) -> Result<Identifier, String> {
    Identifier::parse(value.as_str().ok_or("expected an identifier string")?).map_err(|e| e.to_string())
}

/// An integer field that must be at least `min`; `default` stands in for a missing field.
pub(crate) fn int_field(fields: &Map<String, Value>, key: &str, min: i32, default: Option<i32>) -> Result<i32, String> {
    match fields.get(key) {
        None => default.ok_or_else(|| format!("{key} missing")),
        Some(Value::Number(n)) => {
            let n = crate::materials::number_int(&n.to_string());
            if n >= min { Ok(n) } else { Err(format!("{key} must be at least {min}")) }
        }
        Some(_) => Err(format!("{key} must be a number")),
    }
}

/// Fields that make an object a known shape: any other key is an error naming it.
pub(crate) fn only_keys(fields: &Map<String, Value>, what: &str, allowed: &[&str]) -> Result<(), String> {
    match fields.keys().find(|key| !allowed.contains(&key.as_str())) {
        Some(key) => Err(format!("unimplemented {what} field {key}")),
        None => Ok(()),
    }
}

/// A registry entry (a fluid or an item), or every entry of a tag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Source { Id(Identifier), Tag(Identifier) }

impl Source {
    fn decode(fields: &Map<String, Value>) -> Result<Self, String> {
        match (fields.get("fluid"), fields.get("tag")) {
            (Some(fluid), None) => Ok(Self::Id(id_of(fluid)?)),
            (None, Some(tag)) => Ok(Self::Tag(id_of(tag)?)),
            _ => Err("a fluid names exactly one of fluid or tag".into()),
        }
    }
    fn matches(&self, fluid: &Identifier, in_tag: InTag) -> bool {
        match self { Self::Id(own) => own == fluid, Self::Tag(tag) => in_tag(fluid, tag) }
    }
}

/// FluidIngredient: one fluid or tag with the amount it takes, or an array of those (any one of them).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FluidIngredient { pub options: Vec<(Source, i32)> }

impl FluidIngredient {
    pub(crate) fn decode(value: &Value) -> Result<Self, String> {
        match value {
            Value::Array(members) if !members.is_empty() => {
                let mut options = Vec::new();
                for member in members { options.extend(Self::decode(member)?.options); }
                Ok(Self { options })
            }
            Value::Object(fields) => {
                only_keys(fields, "fluid ingredient", &["fluid", "tag", "amount"])?;
                Ok(Self { options: vec![(Source::decode(fields)?, int_field(fields, "amount", 1, None)?)] })
            }
            _ => Err("fluid ingredient must be an object or a nonempty array".into()),
        }
    }
    /// FluidIngredient.test(Fluid): the fluid is one the ingredient names, whatever the amount.
    pub(crate) fn matches(&self, fluid: &Identifier, in_tag: InTag) -> bool {
        self.options.iter().any(|(source, _)| source.matches(fluid, in_tag))
    }
    /// getAmount(Fluid): the amount of the first option that names the fluid, 0 for another fluid.
    pub(crate) fn amount(&self, fluid: &Identifier, in_tag: InTag) -> i32 {
        self.options.iter().find(|(source, _)| source.matches(fluid, in_tag)).map_or(0, |(_, amount)| *amount)
    }
    /// FluidIngredient.test(FluidStack): the right fluid and at least the amount it takes.
    pub(crate) fn test(&self, fluid: &Identifier, amount: i32, in_tag: InTag) -> bool {
        self.matches(fluid, in_tag) && amount >= self.amount(fluid, in_tag)
    }
}

/// FluidOutput: a fixed fluid, or a tag standing for the fluid the pack prefers, with an amount and optional NBT.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FluidOutput { pub source: Source, pub amount: i32, pub nbt: Option<Value> }

impl FluidOutput {
    /// `extra` names the fields a recipe type reads from the same object (damageable melting's unit_size, ore rates).
    pub(crate) fn decode(value: &Value, extra: &[&str]) -> Result<Self, String> {
        let fields = value.as_object().ok_or("fluid output must be an object")?;
        let mut allowed = vec!["fluid", "tag", "amount", "nbt"];
        allowed.extend_from_slice(extra);
        only_keys(fields, "fluid output", &allowed)?;
        let nbt = match fields.get("nbt") { None | Some(Value::Null) => None, Some(nbt) => Some(nbt.clone()) };
        Ok(Self { source: Source::decode(fields)?, amount: int_field(fields, "amount", 1, None)?, nbt })
    }
    /// The fluid this output makes. A tag resolves to its first member: Mantle's tag preference picks by the pack's
    /// mod priority, and with no other mod's fluids the first entry (the source fluid) is the preferred one.
    pub(crate) fn resolve(&self, tags: &FluidTags) -> Option<Identifier> {
        match &self.source {
            Source::Id(fluid) => Some(fluid.clone()),
            Source::Tag(tag) => tags.members(tag).and_then(|members| members.first().cloned()),
        }
    }
}

/// ItemOutput: the item a recipe makes, a fixed one or the preferred member of an item tag.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ItemOutput { pub source: Source, pub count: i32, pub nbt: Option<Value> }

impl ItemOutput {
    /// A bare identifier string is one item; an object may carry a count (when `stack`) and NBT.
    pub(crate) fn decode(value: &Value, stack: bool) -> Result<Self, String> {
        if let Value::String(item) = value {
            return Ok(Self { source: Source::Id(Identifier::parse(item).map_err(|e| e.to_string())?), count: 1, nbt: None });
        }
        let fields = value.as_object().ok_or("item output must be an identifier or an object")?;
        only_keys(fields, "item output", if stack { &["item", "tag", "count", "nbt"] } else { &["item", "tag", "nbt"] })?;
        let source = match (fields.get("item"), fields.get("tag")) {
            (Some(item), None) => Source::Id(id_of(item)?),
            (None, Some(tag)) => Source::Tag(id_of(tag)?),
            _ => return Err("an item output names exactly one of item or tag".into()),
        };
        let nbt = match fields.get("nbt") { None | Some(Value::Null) => None, Some(nbt) => Some(nbt.clone()) };
        Ok(Self { source, count: int_field(fields, "count", 1, Some(1))?, nbt })
    }
}

/// Fluid tags with their nested tags flattened, in the order the tag files list them.
#[derive(Clone, Debug, Default)]
pub(crate) struct FluidTags { tags: BTreeMap<Identifier, Vec<Identifier>>, pub problems: Vec<String> }

impl FluidTags {
    /// Loads tag files (`{"values": [...]}`: fluid ids, `#tag` references, or `{"id", "required"}` objects).
    /// A missing required member is a problem; an optional one is left out. A tag that refers to itself is a problem.
    pub(crate) fn load(files: &[(Identifier, Value)]) -> Self {
        let raw: BTreeMap<&Identifier, &Value> = files.iter().map(|(id, value)| (id, value)).collect();
        let mut result = Self::default();
        for id in raw.keys() {
            let mut visiting = BTreeSet::new();
            match flatten(id, &raw, &mut visiting) {
                Ok(members) => { result.tags.insert((*id).clone(), members); }
                Err(error) => result.problems.push(format!("fluid tag {id}: {error}")),
            }
        }
        result
    }
    pub(crate) fn members(&self, tag: &Identifier) -> Option<&Vec<Identifier>> { self.tags.get(tag) }
    pub(crate) fn contains(&self, fluid: &Identifier, tag: &Identifier) -> bool { self.members(tag).is_some_and(|members| members.contains(fluid)) }
    pub(crate) fn len(&self) -> usize { self.tags.len() }
}

fn flatten(id: &Identifier, raw: &BTreeMap<&Identifier, &Value>, visiting: &mut BTreeSet<Identifier>) -> Result<Vec<Identifier>, String> {
    let value = raw.get(id).ok_or_else(|| format!("unknown tag {id}"))?;
    if !visiting.insert(id.clone()) { return Err(format!("{id} contains itself")); }
    let values = value.get("values").and_then(Value::as_array).ok_or("values must be an array")?;
    let mut members: Vec<Identifier> = Vec::new();
    for entry in values {
        let (text, required) = match entry {
            Value::String(text) => (text.as_str(), true),
            Value::Object(object) => (object.get("id").and_then(Value::as_str).ok_or("tag entry needs an id")?,
                object.get("required").and_then(Value::as_bool).unwrap_or(true)),
            _ => return Err("tag entry must be a string or an object".into()),
        };
        if let Some(tag) = text.strip_prefix('#') {
            let tag = Identifier::parse(tag).map_err(|e| e.to_string())?;
            match flatten(&tag, raw, visiting) {
                Ok(inner) => for member in inner { if !members.contains(&member) { members.push(member); } },
                Err(_) if !required && !raw.contains_key(&tag) => {}
                Err(error) => return Err(error),
            }
        } else {
            let fluid = Identifier::parse(text).map_err(|e| e.to_string())?;
            if !members.contains(&fluid) { members.push(fluid); }
        }
    }
    visiting.remove(id);
    Ok(members)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
    fn tags() -> FluidTags {
        FluidTags::load(&[
            (id("forge:molten_iron"), json!({"values": ["#tconstruct:molten_iron"]})),
            (id("tconstruct:molten_iron"), json!({"values": ["tconstruct:molten_iron", "tconstruct:flowing_molten_iron"]})),
            (id("mantle:soup"), json!({"values": [{"id": "#tconstruct:none", "required": false}, "tconstruct:meat_soup"]})),
            (id("a:loop"), json!({"values": ["#a:loop"]})),
        ])
    }

    #[test]
    fn nested_tags_flatten_in_order_and_optional_missing_members_are_left_out() {
        let tags = tags();
        assert_eq!(tags.members(&id("forge:molten_iron")).unwrap(), &vec![id("tconstruct:molten_iron"), id("tconstruct:flowing_molten_iron")]);
        assert_eq!(tags.members(&id("mantle:soup")).unwrap(), &vec![id("tconstruct:meat_soup")]);
        assert_eq!(tags.problems.len(), 1, "{:?}", tags.problems);
        assert!(tags.problems[0].contains("contains itself"));
    }

    #[test]
    fn an_ingredient_matches_its_fluid_or_tag_and_wants_its_amount() {
        let tags = tags();
        let in_tag = |fluid: &Identifier, tag: &Identifier| tags.contains(fluid, tag);
        let ingredient = FluidIngredient::decode(&json!([{"tag": "forge:molten_iron", "amount": 90}, {"fluid": "minecraft:lava", "amount": 50}])).unwrap();
        assert!(ingredient.matches(&id("tconstruct:flowing_molten_iron"), &in_tag), "a tag names its flowing fluid too");
        assert_eq!(ingredient.amount(&id("minecraft:lava"), &in_tag), 50);
        assert_eq!(ingredient.amount(&id("minecraft:water"), &in_tag), 0);
        assert!(ingredient.test(&id("minecraft:lava"), 50, &in_tag) && !ingredient.test(&id("minecraft:lava"), 49, &in_tag));
        for bad in [json!({"fluid": "a:b"}), json!({"fluid": "a:b", "tag": "a:c", "amount": 1}), json!({"fluid": "a:b", "amount": 0}),
            json!({"fluid": "a:b", "amount": 1, "nbt": {}}), json!([]), json!(3)] {
            assert!(FluidIngredient::decode(&bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn outputs_resolve_a_tag_to_its_first_fluid_and_item_outputs_take_a_bare_id() {
        let tags = tags();
        let output = FluidOutput::decode(&json!({"tag": "forge:molten_iron", "amount": 90}), &[]).unwrap();
        assert_eq!(output.resolve(&tags), Some(id("tconstruct:molten_iron")));
        assert_eq!(FluidOutput::decode(&json!({"tag": "x:unknown", "amount": 1}), &[]).unwrap().resolve(&tags), None);
        assert!(FluidOutput::decode(&json!({"fluid": "a:b", "amount": 1, "unit_size": 5}), &[]).is_err());
        assert!(FluidOutput::decode(&json!({"fluid": "a:b", "amount": 1, "unit_size": 5}), &["unit_size"]).is_ok());
        let nbt = ItemOutput::decode(&json!({"item": "tconstruct:repair_kit", "nbt": {"Material": "tconstruct:ancient"}}), false).unwrap();
        assert_eq!(nbt.nbt, Some(json!({"Material": "tconstruct:ancient"})));
        assert_eq!(ItemOutput::decode(&json!("minecraft:potion"), false).unwrap().count, 1);
        assert_eq!(ItemOutput::decode(&json!({"item": "a:b", "count": 4}), true).unwrap().count, 4);
        assert!(ItemOutput::decode(&json!({"item": "a:b", "count": 4}), false).is_err());
    }
}
