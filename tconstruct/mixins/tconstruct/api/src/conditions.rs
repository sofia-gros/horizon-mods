//! Forge/Mantle/Tinkers data-condition tree; SlimeKnights port, MIT.
//! Decode every branch before evaluating; evaluation alone short-circuits.
use pluto_lib::{Identifier, RegistryLookup};
use serde_json::Value;
use std::collections::BTreeMap;

pub(crate) struct Context {
    pub items: Option<RegistryLookup>,
    pub config: BTreeMap<String, bool>,
}

pub(crate) enum Condition {
    Constant(bool), Mod(String), Item(Identifier), TagFilled(Identifier), TagEmpty(Identifier),
    Config(String), Not(Box<Self>), And(Vec<Self>), Or(Vec<Self>),
    /// mantle:tag_combination_filled: some entry is in every `matches` tag and not in `ignore`.
    TagCombination { registry: Identifier, matches: Vec<Identifier>, ignore: Option<Identifier> },
}
impl Condition {
    fn decode(value: &Value) -> Result<Self, String> {
        let object = value.as_object().ok_or("condition must be an object")?;
        let kind = object.get("type").and_then(Value::as_str).ok_or("condition needs a type")?;
        let text = |field: &str| object.get(field).and_then(Value::as_str)
            .ok_or_else(|| format!("{kind} needs {field}"));
        let id = |field: &str| Identifier::parse(text(field)?).map_err(|e| e.to_string());
        match kind {
            "forge:true" => Ok(Self::Constant(true)),
            "forge:false" => Ok(Self::Constant(false)),
            "forge:mod_loaded" => Ok(Self::Mod(text("modid")?.into())),
            "forge:item_exists" => Ok(Self::Item(id("item")?)),
            "mantle:tag_filled" => Ok(Self::TagFilled(id("tag")?)),
            "forge:tag_empty" => Ok(Self::TagEmpty(id("tag")?)),
            "tconstruct:config" => {
                let name = text("prop")?.to_lowercase();
                if !crate::config::known(&name) { return Err(format!("unknown Tinkers config property {name}")); }
                Ok(Self::Config(name))
            }
            "mantle:tag_combination_filled" => {
                // The registry defaults to items; match is one tag or an array of them.
                let registry = match object.get("registry") {
                    None => Identifier::new("minecraft", "item").expect("valid id"),
                    Some(value) => Identifier::parse(value.as_str().ok_or("registry must be an ID string")?).map_err(|e| e.to_string())?,
                };
                let tag = |value: &Value| Identifier::parse(value.as_str().ok_or("tag must be an ID string")?).map_err(|e| e.to_string());
                let matches = match object.get("match").ok_or("tag_combination_filled needs match")? {
                    Value::Array(values) => values.iter().map(tag).collect::<Result<Vec<_>, _>>()?,
                    single => vec![tag(single)?],
                };
                if matches.is_empty() { return Err("must match at least 1 tag".into()); }
                let ignore = object.get("ignore").map(tag).transpose()?;
                Ok(Self::TagCombination { registry, matches, ignore })
            }
            "forge:not" => Ok(Self::Not(Box::new(Self::decode(object.get("value").ok_or("not needs value")?)?))),
            "forge:and" | "forge:or" => {
                let children = object.get("values").and_then(Value::as_array).ok_or("condition needs values")?;
                if children.is_empty() { return Err("and/or needs at least one child".into()); }
                let children = children.iter().map(Self::decode).collect::<Result<_, _>>()?;
                Ok(if kind == "forge:and" { Self::And(children) } else { Self::Or(children) })
            }
            _ => Err(format!("unsupported condition type {kind}")),
        }
    }
    fn evaluate(&self, context: &Context) -> Result<bool, String> {
        let items = || context.items.as_ref().ok_or_else(|| "item registry unavailable during reload".to_owned());
        match self {
            Self::Constant(value) => Ok(*value),
            // This port intentionally has no other-mod integration. These are the
            // two original content identities present; Forge/Mantle are not installed mods.
            Self::Mod(id) => Ok(matches!(id.as_str(), "minecraft" | "tconstruct")),
            Self::Item(id) => Ok(items()?.contains(id)),
            Self::TagFilled(id) => Ok(items()?.get_tag(id).is_some_and(|members| !members.is_empty())),
            Self::TagEmpty(id) => Ok(items()?.get_tag(id).is_none_or(|members| members.is_empty())),
            Self::TagCombination { registry, matches, ignore } => {
                // The host captures no instrument registry; the generated table from the pinned data answers for it.
                if registry.to_string() == "minecraft:instrument" {
                    let members = |id: &Identifier| crate::instrument_tags::TAGS.iter()
                        .find(|(name, _)| *name == id.to_string()).map(|(_, members)| members.iter().copied().collect::<Vec<_>>()).unwrap_or_default();
                    let tags: Vec<Vec<&str>> = matches.iter().map(members).collect();
                    let ignored = ignore.as_ref().map_or_else(Vec::new, members);
                    if tags.iter().any(|members| members.is_empty()) { return Ok(false); }
                    return Ok(tags[0].iter().any(|entry| !ignored.contains(entry) && tags[1..].iter().all(|members| members.contains(entry))));
                }
                // Only the item registry is captured; another registry is not guessed at.
                if registry.to_string() != "minecraft:item" { return Err(format!("registry {registry} unavailable during reload")); }
                let items = items()?;
                let empty = std::collections::BTreeSet::new();
                let tag = |id: &Identifier| items.get_tag(id).unwrap_or(&empty);
                let tags: Vec<_> = matches.iter().map(tag).collect();
                let ignored = ignore.as_ref().map_or(&empty, tag);
                if tags.len() == 1 && ignored.is_empty() { return Ok(!tags[0].is_empty()); }
                if tags[1..].iter().any(|members| members.is_empty()) { return Ok(false); }
                Ok(tags[0].iter().any(|entry| !ignored.contains(entry) && tags[1..].iter().all(|members| members.contains(entry))))
            }
            Self::Config(name) => context.config.get(name).copied().ok_or_else(|| format!("config unavailable: {name}")),
            Self::Not(child) => Ok(!child.evaluate(context)?),
            Self::And(children) => {
                for child in children { if !child.evaluate(context)? { return Ok(false); } }
                Ok(true)
            }
            Self::Or(children) => {
                for child in children { if child.evaluate(context)? { return Ok(true); } }
                Ok(false)
            }
        }
    }
}
impl crate::materials::Conditions for Context {
    type Parsed = Condition;
    fn decode(&self, json: &Value) -> Result<Condition, String> { Condition::decode(json) }
    fn test(&self, condition: &Condition) -> Result<bool, String> { condition.evaluate(self) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn validates_unused_children_but_does_not_query_their_context() {
        let context = Context { items: None, config: crate::config::parse("").unwrap() };
        let valid = json!({"type":"forge:or","values":[{"type":"forge:true"},{"type":"forge:item_exists","item":"minecraft:stick"}]});
        assert!(Condition::decode(&valid).unwrap().evaluate(&context).unwrap());
        let invalid = json!({"type":"forge:or","values":[{"type":"forge:true"},{"type":"tconstruct:config","prop":"missing"}]});
        assert!(Condition::decode(&invalid).is_err());
        assert!(Condition::decode(&json!({"type":"forge:and","values":[]})).is_err());
        let needs_items = Condition::decode(&json!({"type":"forge:item_exists","item":"minecraft:stick"})).unwrap();
        assert!(needs_items.evaluate(&context).is_err());
        // tag_combination_filled: one tag or an array, an optional ignore tag, items by default.
        for ok in [json!({"type":"mantle:tag_combination_filled","match":"minecraft:logs"}),
            json!({"type":"mantle:tag_combination_filled","match":["a:b","c:d"],"ignore":"e:f"}),
            json!({"type":"mantle:tag_combination_filled","registry":"minecraft:instrument","match":"minecraft:goat_horns"})] {
            assert!(Condition::decode(&ok).is_ok(), "{ok}");
        }
        for bad in [json!({"type":"mantle:tag_combination_filled"}), json!({"type":"mantle:tag_combination_filled","match":[]}),
            json!({"type":"mantle:tag_combination_filled","match":3})] {
            assert!(Condition::decode(&bad).is_err(), "{bad}");
        }
        // The instrument registry is answered by the generated table: every goat horn is a variant horn, so
        // that combination is unfilled, but the screaming horns sit outside the regular tag, so that one is.
        let unfilled = Condition::decode(&json!({"type":"mantle:tag_combination_filled","registry":"minecraft:instrument","match":"minecraft:goat_horns","ignore":"tconstruct:variant_horns"})).unwrap();
        assert!(!unfilled.evaluate(&context).unwrap());
        let filled = Condition::decode(&json!({"type":"mantle:tag_combination_filled","registry":"minecraft:instrument","match":"minecraft:goat_horns","ignore":"minecraft:regular_goat_horns"})).unwrap();
        assert!(filled.evaluate(&context).unwrap());
        let unknown = Condition::decode(&json!({"type":"mantle:tag_combination_filled","registry":"minecraft:instrument","match":"x:y"})).unwrap();
        assert!(!unknown.evaluate(&context).unwrap(), "an unknown instrument tag is empty, not an error");
        // Another registry is never guessed at.
        let other = Condition::decode(&json!({"type":"mantle:tag_combination_filled","registry":"minecraft:painting_variant","match":"x:y"})).unwrap();
        assert!(other.evaluate(&context).is_err());
        let uppercase = Condition::decode(&json!({"type":"tconstruct:config","prop":"SPAWN_WITH_BOOK"})).unwrap();
        assert!(uppercase.evaluate(&context).unwrap());
    }
}
