//! ToolContextPredicate, the subset the original ships in modifier data (SlimeKnights, MIT; see LICENSE):
//! mantle:tag, mantle:set, mantle:inverted, mantle:and, mantle:or, tconstruct:has_modifier,
//! tconstruct:has_persistent_key and tconstruct:has_hook (the tool trait hooks). Any other predicate type is rejected, never treated as true.
use pluto_lib::Identifier;
use serde_json::{Map, Value};

/// Which modifiers HasModifierPredicate inspects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Check { All, Upgrades }

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Predicate {
    Any,
    /// The tool item is in this item tag.
    Tag(Identifier),
    /// The tool item is one of these (minecraft:air matches an empty hand, never a tool stack).
    Set(Vec<Identifier>),
    Not(Box<Predicate>),
    And(Vec<Predicate>),
    Or(Vec<Predicate>),
    HasModifier { modifier: Identifier, min: i32, max: i32, check: Check },
    /// The same over every modifier in a modifier tag.
    HasModifierTag { tag: Identifier, min: i32, max: i32, check: Check },
    HasPersistentKey(Identifier),
    /// tconstruct:has_hook: the tool's definition has this hook.
    HasHook(Identifier),
}

/// What a predicate may ask about a tool. With no item (previews outside a tool stack), item-based
/// predicates are false; callers that need "no context" semantics check `item` themselves.
pub(crate) struct Facts<'a> {
    pub item: Option<&'a Identifier>,
    pub in_tag: &'a dyn Fn(&Identifier, &Identifier) -> bool,
    /// Summed integer effective level of the modifier among the checked modifiers.
    pub level: &'a dyn Fn(&Identifier, Check) -> i32,
    /// Summed level of the checked modifiers that are in a modifier tag.
    pub tag_level: &'a dyn Fn(&Identifier, Check) -> i32,
    pub has_persistent_key: &'a dyn Fn(&Identifier) -> bool,
    /// The string stored in the tool's persistent data under a key; empty when absent or not a string (NBT getString).
    pub persistent_string: &'a dyn Fn(&Identifier) -> String,
    /// The numeric entries of the tool's persistent compound under a key, in key order (empty when absent).
    pub persistent_numbers: &'a dyn Fn(&Identifier) -> Vec<(String, f64)>,
    /// ToolTraitHook.addTraits of the tool's definition and materials for an alternate hook (rebalanced_trait, trim_trait).
    pub hook_traits: &'a dyn Fn(&Identifier) -> Vec<crate::material_traits::ModifierEntry>,
    /// HasToolHookPredicate: whether the tool's definition carries the hook.
    pub has_hook: &'a dyn Fn(&Identifier) -> bool,
}

fn id(value: &Value) -> Result<Identifier, String> {
    Identifier::parse(value.as_str().ok_or("expected an identifier string")?).map_err(|e| e.to_string())
}

impl Predicate {
    pub fn decode(value: &Value) -> Result<Self, String> {
        let object = value.as_object().ok_or("predicate must be an object")?;
        Self::decode_typed(object, object.get("type").ok_or("predicate type missing")?)
    }

    fn decode_typed(object: &Map<String, Value>, kind: &Value) -> Result<Self, String> {
        let kind = kind.as_str().ok_or("predicate type must be a string")?;
        let list = |key: &str| -> Result<Vec<Predicate>, String> {
            object.get(key).and_then(Value::as_array).filter(|values| !values.is_empty())
                .ok_or_else(|| format!("{key} must be a nonempty array"))?.iter().map(Predicate::decode).collect()
        };
        Ok(match kind {
            "mantle:tag" => Self::Tag(id(object.get("tag").ok_or("tag missing")?)?),
            "mantle:set" => Self::Set(object.get("items").and_then(Value::as_array).ok_or("items must be an array")?
                .iter().map(id).collect::<Result<_, _>>()?),
            "mantle:inverted" => {
                // The wrapped predicate's own fields sit beside inverted_type.
                let inner = object.get("inverted_type").ok_or("inverted_type missing")?;
                if inner.as_str() == Some("mantle:inverted") { return Err("nested inversion is not shipped".into()); }
                Self::Not(Box::new(Self::decode_typed(object, inner)?))
            }
            "mantle:and" => Self::And(list("predicates")?),
            "mantle:or" => Self::Or(list("predicates")?),
            "tconstruct:has_hook" => {
                let hook = id(object.get("hook").ok_or("hook missing")?)?;
                // The definition data records only the tool trait hooks; another hook cannot be answered.
                if !matches!(hook.to_string().as_str(), "tconstruct:tool_traits" | "tconstruct:rebalanced_trait" | "tconstruct:trim_trait") { return Err(format!("unimplemented tool predicate has_hook for {hook}")); }
                Self::HasHook(hook)
            }
            "tconstruct:has_persistent_key" => Self::HasPersistentKey(id(object.get("key").ok_or("key missing")?)?),
            "tconstruct:has_modifier" => {
                let matcher = object.get("modifier").and_then(Value::as_object).ok_or("modifier must be an object")?;
                let matcher_type = matcher.get("type").and_then(Value::as_str);
                if !matches!(matcher_type, Some("tconstruct:single" | "tconstruct:tag")) {
                    return Err("only single-modifier matches are implemented".into());
                }
                let (min, max) = match object.get("level") {
                    None | Some(Value::Null) => (1, 32767),
                    Some(Value::Number(n)) => { let n = crate::materials::number_int(&n.to_string()); (n, n) }
                    Some(Value::Object(range)) => (
                        range.get("min").and_then(Value::as_i64).map_or(1, |n| n as i32),
                        range.get("max").and_then(Value::as_i64).map_or(32767, |n| n as i32)),
                    _ => return Err("level must be a number or range".into()),
                };
                let check = match object.get("check").and_then(Value::as_str) {
                    Some("all") => Check::All, Some("upgrades") => Check::Upgrades,
                    _ => return Err("check must be all or upgrades".into()),
                };
                if matcher_type == Some("tconstruct:tag") {
                    Self::HasModifierTag { tag: id(matcher.get("tag").ok_or("tag missing")?)?, min, max, check }
                } else {
                    Self::HasModifier { modifier: id(matcher.get("modifier").ok_or("modifier missing")?)?, min, max, check }
                }
            }
            other => return Err(format!("unimplemented tool predicate {other}")),
        })
    }

    pub fn matches(&self, facts: &Facts) -> bool {
        match self {
            Self::Any => true,
            Self::Tag(tag) => facts.item.is_some_and(|item| (facts.in_tag)(item, tag)),
            Self::Set(items) => facts.item.is_some_and(|item| items.contains(item)),
            Self::Not(inner) => !inner.matches(facts),
            Self::And(all) => all.iter().all(|p| p.matches(facts)),
            Self::Or(any) => any.iter().any(|p| p.matches(facts)),
            Self::HasPersistentKey(key) => (facts.has_persistent_key)(key),
            Self::HasHook(hook) => (facts.has_hook)(hook),
            Self::HasModifierTag { tag, min, max, check } => {
                let level = (facts.tag_level)(tag, *check);
                if *min == 0 && *max == 0 { level == 0 } else if (*min, *max) == (1, 32767) { level > 0 } else { (*min..=*max).contains(&level) }
            }
            Self::HasModifier { modifier, min, max, check } => {
                // Level 0 means absent; the full valid range means present at any level.
                let level = (facts.level)(modifier, *check);
                if *min == 0 && *max == 0 { level == 0 } else if (*min, *max) == (1, 32767) { level > 0 } else { (*min..=*max).contains(&level) }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
    fn facts<'a>(item: Option<&'a Identifier>, in_tag: &'a dyn Fn(&Identifier, &Identifier) -> bool, level: &'a dyn Fn(&Identifier, Check) -> i32,
        key: &'a dyn Fn(&Identifier) -> bool) -> Facts<'a> { Facts { item, in_tag, level, tag_level: &|_, _| 0, has_persistent_key: key, persistent_string: &|_| String::new(), persistent_numbers: &|_| Vec::new(), hook_traits: &|_| Vec::new(), has_hook: &|_| false } }
    #[test]
    fn every_shipped_predicate_shape_decodes_and_evaluates_over_tool_facts() {
        let pickaxe = id("tconstruct:pickaxe");
        let in_tag = |item: &Identifier, tag: &Identifier| item.path() == "pickaxe" && tag.path() == "modifiable/harvest";
        let level = |modifier: &Identifier, check: Check| if modifier.path() == "long_fall" && check == Check::All { 2 } else { 0 };
        let key = |key: &Identifier| key.path() == "crossbow_ammo";
        let f = facts(Some(&pickaxe), &in_tag, &level, &key);
        let decode = |value: Value| Predicate::decode(&value).unwrap();
        assert!(decode(json!({"type":"mantle:tag","tag":"tconstruct:modifiable/harvest"})).matches(&f));
        assert!(!decode(json!({"type":"mantle:tag","tag":"tconstruct:modifiable/armor"})).matches(&f));
        assert!(!decode(json!({"type":"mantle:inverted","inverted_type":"mantle:tag","tag":"tconstruct:modifiable/harvest"})).matches(&f));
        assert!(decode(json!({"type":"mantle:and","predicates":[{"type":"mantle:tag","tag":"tconstruct:modifiable/harvest"},
            {"type":"mantle:inverted","inverted_type":"mantle:tag","tag":"tconstruct:modifiable/broad"}]})).matches(&f));
        assert!(decode(json!({"type":"mantle:or","predicates":[{"type":"mantle:set","items":["minecraft:air"]},
            {"type":"mantle:tag","tag":"tconstruct:modifiable/harvest"}]})).matches(&f));
        assert!(decode(json!({"type":"mantle:set","items":["tconstruct:pickaxe"]})).matches(&f));
        assert!(decode(json!({"type":"tconstruct:has_persistent_key","key":"tconstruct:crossbow_ammo"})).matches(&f));
        let has = |level: Value| decode(json!({"type":"tconstruct:has_modifier","check":"all","level":level,"modifier":{"type":"tconstruct:single","modifier":"tconstruct:long_fall"}}));
        assert!(!has(json!(1)).matches(&f), "exactly level 1: the tool has level 2");
        assert!(has(json!(2)).matches(&f));
        assert!(has(json!({"min":2})).matches(&f) && !has(json!({"min":3})).matches(&f));
        // Level 0 means absent.
        let none = |_: &Identifier, _: Check| 0;
        let g = facts(Some(&pickaxe), &in_tag, &none, &key);
        assert!(has(json!(0)).matches(&g) && !has(json!({"min":1})).matches(&g));
    }
    #[test]
    fn missing_item_context_and_unknown_predicates_never_become_true() {
        let in_tag = |_: &Identifier, _: &Identifier| true;
        let level = |_: &Identifier, _: Check| 0;
        let key = |_: &Identifier| false;
        let none = facts(None, &in_tag, &level, &key);
        assert!(!Predicate::decode(&json!({"type":"mantle:tag","tag":"x:y"})).unwrap().matches(&none));
        assert!(Predicate::Any.matches(&none));
        // A tag match asks for the summed level of the modifiers in that tag.
        let tagged = Facts { tag_level: &|tag, _| if tag.path() == "friends" { 2 } else { 0 }, ..facts(None, &in_tag, &level, &key) };
        let has_tag = |tag: &str, level: Value| Predicate::decode(&json!({"type":"tconstruct:has_modifier","check":"all","level":level,"modifier":{"type":"tconstruct:tag","tag":tag}})).unwrap();
        assert!(has_tag("test:friends", json!(null)).matches(&tagged) && !has_tag("test:strangers", json!(null)).matches(&tagged));
        assert!(has_tag("test:strangers", json!(0)).matches(&tagged) && !has_tag("test:friends", json!(0)).matches(&tagged));
        for bad in [json!({"type":"mantle:block","block":"x:y"}), json!({"type":"mantle:and","predicates":[]}),
                    json!({"type":"mantle:inverted","inverted_type":"mantle:inverted"}), json!({}), json!("tag"),
                    json!({"type":"tconstruct:has_modifier","check":"sometimes","modifier":{"type":"tconstruct:single","modifier":"a:b"}}),
                    json!({"type":"tconstruct:has_modifier","check":"all","modifier":{"type":"tconstruct:unknown","tag":"a:b"}})] {
            assert!(Predicate::decode(&bad).is_err(), "{bad}");
        }
    }
}
