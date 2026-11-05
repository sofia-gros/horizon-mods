use serde_json::{Value, json};
use std::collections::BTreeMap;

pub(super) type Raw = BTreeMap<String, Vec<Vec<u8>>>;
const LOOT: &str = "data/tconstruct/loot_modifiers/chrysophilite_modifier.json";
const GLOBAL: &str = "data/forge/loot_modifiers/global_loot_modifiers.json";
const ID: &str = "tconstruct:chrysophilite_modifier";

pub(super) fn merge_pack(
    raw: &mut Raw,
    files: impl IntoIterator<Item = String>,
    read: impl Fn(&str) -> Option<Vec<u8>>,
) -> Result<(), String> {
    for file in files {
        if file != LOOT && file != GLOBAL {
            continue;
        }
        let bytes =
            read(&file).ok_or_else(|| format!("unreadable Chrysophilite resource {file}"))?;
        raw.entry(file).or_default().push(bytes);
    }
    Ok(())
}

pub(super) fn enabled(raw: &Raw) -> bool {
    let Some(layers) = raw.get(GLOBAL) else {
        return false;
    };
    let mut enabled = false;
    for bytes in layers {
        let Ok(value) = serde_json::from_slice::<Value>(bytes) else {
            return false;
        };
        let Some(object) = value.as_object() else {
            return false;
        };
        if object
            .keys()
            .any(|key| key != "replace" && key != "entries")
        {
            return false;
        }
        let Some(replace) = value.get("replace").map_or(Some(false), Value::as_bool) else {
            return false;
        };
        let Some(entries) = value.get("entries").and_then(Value::as_array) else {
            return false;
        };
        if replace {
            enabled = false;
        }
        for entry in entries {
            let Some(entry) = entry.as_str() else {
                return false;
            };
            if entry == ID {
                enabled = true;
            }
        }
    }
    enabled
        && raw
            .get(LOOT)
            .and_then(|layers| layers.last())
            .and_then(|bytes| serde_json::from_slice::<Value>(bytes).ok())
            .is_some_and(|v| v == canonical())
}

fn canonical() -> Value {
    json!({
            "type":"mantle:add_entry",
            "conditions":[{"condition":"mantle:block_tag","tag":"tconstruct:chrysophilite_ores"},{"condition":"tconstruct:has_chrysophilite"}],
            "entry":{"type":"minecraft:item","name":"minecraft:gold_nugget"},
            "functions":[
                {"add":false,"count":{"type":"minecraft:uniform","max":6,"min":2},"function":"minecraft:set_count"},
                {"formula":"minecraft:ore_drops","function":"tconstruct:chrysophilite_bonus","include_base":false},
                {"function":"minecraft:explosion_decay"}
            ],
            "post_conditions":[{"type":"mantle:inverted","condition":{"type":"mantle:contains_item","ingredient":{"tag":"tconstruct:chrysophilite_ores"}}}]
    })
}

// Attribute = 1 + gold armor count, then include_base=false removes one.
// No Fortune enchantment is consulted by ChrysophiliteBonusFunction.
pub(super) fn nuggets(gold: u8, mut next_int: impl FnMut(i32) -> i32) -> Option<u32> {
    if gold > 4 {
        return None;
    }
    let base = next_int(5);
    if !(0..5).contains(&base) {
        return None;
    }
    let multiplier = if gold == 0 {
        1
    } else {
        let bound = i32::from(gold) + 2;
        let bonus = next_int(bound);
        if !(0..bound).contains(&bonus) {
            return None;
        }
        (bonus - 1).max(0) + 1
    };
    Some(((base + 2) * multiplier) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn count_rng_order_and_ore_multiplier() {
        let mut bounds = Vec::new();
        assert_eq!(
            nuggets(0, |bound| {
                bounds.push(bound);
                4
            }),
            Some(6)
        );
        assert_eq!(bounds, vec![5]);
        bounds.clear();
        assert_eq!(
            nuggets(4, |bound| {
                bounds.push(bound);
                bound - 1
            }),
            Some(30)
        );
        assert_eq!(bounds, vec![5, 6]);
        assert_eq!(nuggets(1, |_| 0), Some(2));
        assert_eq!(
            nuggets(5, |_| panic!("invalid gold count consumes no RNG")),
            None
        );
    }
    #[test]
    fn loot_override_and_global_replace_are_fail_closed() {
        let mut raw = Raw::new();
        raw.insert(LOOT.into(), vec![serde_json::to_vec(&canonical()).unwrap()]);
        raw.insert(
            GLOBAL.into(),
            vec![br#"{"replace":false,"entries":["tconstruct:chrysophilite_modifier"]}"#.to_vec()],
        );
        assert!(enabled(&raw));
        raw.get_mut(GLOBAL)
            .unwrap()
            .push(br#"{"replace":true,"entries":[]}"#.to_vec());
        assert!(!enabled(&raw));
        raw.get_mut(GLOBAL)
            .unwrap()
            .push(br#"{"entries":["tconstruct:chrysophilite_modifier"]}"#.to_vec());
        assert!(enabled(&raw));
        raw.get_mut(LOOT).unwrap().push(b"{}".to_vec());
        assert!(!enabled(&raw));
    }
}
