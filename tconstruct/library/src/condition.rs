//! Data conditions (`ICondition` of Forge, Mantle and Tinkers) as they appear in the original's data files: `forge:or`, `forge:and`,
//! `forge:not`, `forge:true`, `forge:false`, `forge:mod_loaded`, `mantle:tag_filled` and `tconstruct:config`.
use serde_json::Value;

/// What a condition asks the game.
pub trait ConditionContext {
    /// Whether another mod is installed. The port has no mod integration, so the game answers false for all of them.
    fn mod_loaded(&self, mod_id: &str) -> bool;
    /// Whether the item tag (`forge:ingots/aluminum`) has any members.
    fn tag_filled(&self, tag: &str) -> bool;
    /// The value of a Tinkers config switch (`force_integration_materials`).
    fn config(&self, prop: &str) -> bool;
}

/// A context in which nothing is installed, no tag has members and every switch is off.
pub struct EmptyContext;

impl ConditionContext for EmptyContext {
    fn mod_loaded(&self, _: &str) -> bool {
        false
    }
    fn tag_filled(&self, _: &str) -> bool {
        false
    }
    fn config(&self, _: &str) -> bool {
        false
    }
}

/// Evaluates a condition object. An unknown type or a malformed object is an error, as Forge refuses to parse it.
pub fn test(condition: &Value, context: &dyn ConditionContext) -> Result<bool, String> {
    let object = condition.as_object().ok_or("a condition must be an object")?;
    let kind = object.get("type").and_then(Value::as_str).ok_or("a condition needs a type")?;
    let text = |key: &str| object.get(key).and_then(Value::as_str).ok_or_else(|| format!("condition {kind} needs '{key}'"));
    match kind {
        "forge:true" => Ok(true),
        "forge:false" => Ok(false),
        "forge:mod_loaded" => Ok(context.mod_loaded(text("modid")?)),
        "mantle:tag_filled" => Ok(context.tag_filled(text("tag")?)),
        "tconstruct:config" => Ok(context.config(text("prop")?)),
        "forge:not" => Ok(!test(object.get("value").ok_or("forge:not needs 'value'")?, context)?),
        "forge:and" | "forge:or" => {
            let values = object.get("values").and_then(Value::as_array).ok_or_else(|| format!("{kind} needs 'values'"))?;
            let mut results = Vec::with_capacity(values.len());
            for value in values {
                results.push(test(value, context)?);
            }
            Ok(if kind == "forge:and" { results.iter().all(|r| *r) } else { results.iter().any(|r| *r) })
        }
        other => Err(format!("unknown condition type '{other}'")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct Switches;
    impl ConditionContext for Switches {
        fn mod_loaded(&self, id: &str) -> bool {
            id == "present"
        }
        fn tag_filled(&self, tag: &str) -> bool {
            tag == "forge:ingots/aluminum"
        }
        fn config(&self, prop: &str) -> bool {
            prop == "on"
        }
    }

    #[test]
    fn the_combinators_follow_forge() {
        let or = json!({"type": "forge:or", "values": [{"type": "tconstruct:config", "prop": "off"}, {"type": "mantle:tag_filled", "tag": "forge:ingots/aluminum"}]});
        assert!(test(&or, &Switches).unwrap());
        assert!(!test(&or, &EmptyContext).unwrap());
        let and = json!({"type": "forge:and", "values": [{"type": "forge:true"}, {"type": "forge:not", "value": {"type": "forge:mod_loaded", "modid": "absent"}}]});
        assert!(test(&and, &Switches).unwrap());
        assert!(test(&json!({"type": "forge:and", "values": []}), &Switches).unwrap(), "an empty and holds");
        assert!(!test(&json!({"type": "forge:or", "values": []}), &Switches).unwrap(), "an empty or does not");
    }

    #[test]
    fn unknown_and_broken_conditions_are_errors() {
        assert!(test(&json!({"type": "nobody:knows"}), &Switches).is_err());
        assert!(test(&json!({"type": "forge:mod_loaded"}), &Switches).is_err());
        assert!(test(&json!("not an object"), &Switches).is_err());
    }
}
