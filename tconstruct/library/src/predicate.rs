//! Mantle's JSON predicates as the original's data writes them: `{"type": "mantle:and", "predicates": [...]}`, `mantle:or`,
//! `mantle:inverted` (with `inverted_type` and the inverted predicate's own fields beside it), a bare string for a predicate with no
//! fields (`"mantle:can_protect"`), and any other type, which the caller decides (`mantle:tag`, `mantle:mob_type`, ...).
use serde_json::{Map, Value};

/// Decides a predicate of a type that is not a combinator: its type id and its fields.
pub type Leaf<'a> = &'a dyn Fn(&str, &Map<String, Value>) -> Result<bool, String>;

/// Evaluates a predicate.
pub fn test(predicate: &Value, leaf: Leaf<'_>) -> Result<bool, String> {
    match predicate {
        Value::String(kind) => test_typed(kind, &Map::new(), leaf),
        Value::Object(object) => {
            let kind = object.get("type").and_then(Value::as_str).ok_or("a predicate needs a type")?;
            test_typed(kind, object, leaf)
        }
        _ => Err("a predicate must be a string or an object".to_owned()),
    }
}

fn list(object: &Map<String, Value>, kind: &str, leaf: Leaf<'_>) -> Result<Vec<bool>, String> {
    let predicates = object.get("predicates").and_then(Value::as_array).ok_or_else(|| format!("{kind} needs 'predicates'"))?;
    predicates.iter().map(|p| test(p, leaf)).collect()
}

fn test_typed(kind: &str, object: &Map<String, Value>, leaf: Leaf<'_>) -> Result<bool, String> {
    match kind {
        "mantle:and" => Ok(list(object, kind, leaf)?.iter().all(|r| *r)),
        "mantle:or" => Ok(list(object, kind, leaf)?.iter().any(|r| *r)),
        "mantle:inverted" => {
            // the inverted predicate is either nested under 'predicate', or named by 'inverted_type' with its fields beside
            if let Some(inner) = object.get("predicate") {
                return Ok(!test(inner, leaf)?);
            }
            let inner_kind = object.get("inverted_type").and_then(Value::as_str).ok_or("mantle:inverted needs 'inverted_type' or 'predicate'")?;
            let mut inner = object.clone();
            inner.remove("type");
            inner.remove("inverted_type");
            Ok(!test_typed(inner_kind, &inner, leaf)?)
        }
        other => leaf(other, object),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn leaf(kind: &str, object: &Map<String, Value>) -> Result<bool, String> {
        match kind {
            "mantle:tag" => Ok(object.get("tag").and_then(Value::as_str) == Some("minecraft:is_projectile")),
            "mantle:can_protect" => Ok(true),
            "mantle:has_entity" => Ok(false),
            other => Err(format!("unknown predicate {other}")),
        }
    }

    #[test]
    fn combinators_and_the_inverted_shape_of_the_data() {
        let and = json!({"type": "mantle:and", "predicates": ["mantle:can_protect", {"type": "mantle:inverted", "inverted_type": "mantle:tag", "tag": "minecraft:is_projectile"}]});
        assert!(!test(&and, &leaf).unwrap(), "the tag matches, so its inverse fails the and");
        let other = json!({"type": "mantle:or", "predicates": ["mantle:has_entity", {"type": "mantle:tag", "tag": "minecraft:is_projectile"}]});
        assert!(test(&other, &leaf).unwrap());
        assert!(test(&json!({"type": "mantle:inverted", "inverted_type": "mantle:has_entity"}), &leaf).unwrap());
        assert!(!test(&json!({"type": "mantle:inverted", "predicate": "mantle:can_protect"}), &leaf).unwrap());
    }

    #[test]
    fn unknown_types_and_bad_shapes_are_errors() {
        assert!(test(&json!("nobody:knows"), &leaf).is_err());
        assert!(test(&json!(5), &leaf).is_err());
        assert!(test(&json!({"type": "mantle:and"}), &leaf).is_err());
        assert!(test(&json!({"type": "mantle:inverted"}), &leaf).is_err());
    }
}
