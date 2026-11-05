// Pinned GoldenAttributeModule: once per wearer, not once per piece or level.
use serde_json::{Value, json};
pub(crate) const ID: &str = "tconstruct:gold_guard";
pub(crate) const ATTRIBUTE: &str = "minecraft:max_health";
pub(crate) const MODIFIER: &str = "tconstruct:gold_guard";
pub(crate) fn known_guard(bytes: Option<&[u8]>) -> bool {
    bytes.and_then(|b|serde_json::from_slice::<Value>(b).ok()).is_some_and(|v| v==json!({
        "level_display":"tconstruct:single_level", "modules":[
            {"type":"tconstruct:golden_attribute","attribute":"minecraft:generic.max_health","each_level":4.0,"flat":4.0,"operation":"addition"},
            {"type":"tconstruct:volatile_flag","flag":"tconstruct:piglin_neutral","modifier_level":{"min":2}}
        ]
    }))
}
pub(crate) fn known_golden(bytes: Option<&[u8]>) -> bool {
    bytes.and_then(|b|serde_json::from_slice::<Value>(b).ok()).is_some_and(|v|v==json!({
        "level_display":"tconstruct:no_levels","modules":[{"type":"tconstruct:volatile_flag","flag":"tconstruct:piglin_neutral"}]
    }))
}
pub(crate) fn neutral(golden: i32, guard: i32) -> Option<bool> {
    (golden>=0 && guard>=0).then_some(golden>0 || guard>=2)
}
pub(crate) fn bonus(slots: &[(i32,bool)]) -> Option<f64> {
    if slots.len()>4 || slots.iter().any(|(level,_)|*level<0) {return None;}
    slots.iter().any(|(level,_)|*level>0).then(||4.0+4.0*slots.iter().filter(|(_,gold)|*gold).count() as f64)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn count_neutral_slots_once_not_modifier_levels() {
        assert_eq!(bonus(&[(1,false)]),Some(4.0));
        assert_eq!(bonus(&[(2,true)]),Some(8.0));
        assert_eq!(bonus(&[(99,true),(1,false),(0,true),(0,true)]),Some(16.0));
        assert_eq!(bonus(&[(2,true);4]),Some(20.0));
        assert_eq!(bonus(&[(0,true);4]),None);
        assert_eq!(bonus(&[(-1,true)]),None);
        assert_eq!(bonus(&[(2,true);5]),None);
    }
    #[test] fn golden_and_guard_neutrality_thresholds() {
        assert_eq!(neutral(0,1),Some(false));assert_eq!(neutral(0,2),Some(true));
        assert_eq!(neutral(1,0),Some(true));assert_eq!(neutral(1,1),Some(true));
        assert_eq!(neutral(-1,2),None);
    }
    #[test] fn canonical_definition_reload_is_fail_closed() {
        let guard=br#"{"level_display":"tconstruct:single_level","modules":[{"type":"tconstruct:golden_attribute","attribute":"minecraft:generic.max_health","each_level":4.0,"flat":4.0,"operation":"addition"},{"type":"tconstruct:volatile_flag","flag":"tconstruct:piglin_neutral","modifier_level":{"min":2}}]}"#;
        let golden=br#"{"level_display":"tconstruct:no_levels","modules":[{"type":"tconstruct:volatile_flag","flag":"tconstruct:piglin_neutral"}]}"#;
        assert!(known_guard(Some(guard)));assert!(known_golden(Some(golden)));
        for invalid in [None,Some(&b"{}"[..]),Some(&b"null"[..])] {assert!(!known_guard(invalid));assert!(!known_golden(invalid));}
        let mut changed:Value=serde_json::from_slice(guard).unwrap();changed["modules"][0]["each_level"]=8.into();
        assert!(!known_guard(Some(&serde_json::to_vec(&changed).unwrap())));
        assert!(known_guard(Some(guard)));
    }
}
