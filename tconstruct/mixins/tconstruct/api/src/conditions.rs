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
        let uppercase = Condition::decode(&json!({"type":"tconstruct:config","prop":"SPAWN_WITH_BOOK"})).unwrap();
        assert!(uppercase.evaluate(&context).unwrap());
    }
}
