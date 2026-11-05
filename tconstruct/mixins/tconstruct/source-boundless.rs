pub(super) fn supported_value(value: &serde_json::Value) -> bool {
    value == &serde_json::json!({"level_display":"tconstruct:default","modules":[{
        "type":"tconstruct:attribute","attribute":"tconstruct:generic.protection_cap",
        "each_level":0.05,"flat":0.05,"operation":"addition",
        "slots":["mainhand","offhand","feet","legs","chest","head"],
        "tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/armor"},
        "tooltip_style":"percent"
    }]})
}

/// Each equipped modifier owns a slot-specific addition. Float amounts widen
/// to the double attribute, then ProtectionModifierHook converts it to EPF.
pub(super) fn cap(levels: impl IntoIterator<Item = i32>) -> Option<f32> {
    let mut value = 0.8f64;
    for level in levels {
        if level < 0 { return None; }
        if level > 0 { value += f64::from(0.05f32 * level as f32 + 0.05f32); }
    }
    if !value.is_finite() { return None; }
    let attribute = value.clamp(0.0,f64::from(0.95f32));
    Some((attribute * 25.0).min(f64::from(25.0f32 * 0.95f32)) as f32)
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn flat_bonus_requires_a_modifier_and_slots_add() {
        assert_eq!(cap([]),Some(20.0));
        assert_eq!(cap([0,0]),Some(20.0));
        assert_eq!(cap([1]),Some(22.5));
        assert_eq!(cap([2]),Some(23.75));
        assert_eq!(cap([1,1]),Some(23.75));
        assert_eq!(cap([i32::MAX]),Some(23.75));
        assert_eq!(cap([-1]),None);
    }
    #[test] fn exact_source_contract_requires_armor_and_flat_term() {
        let mut value = serde_json::json!({"level_display":"tconstruct:default","modules":[{
            "type":"tconstruct:attribute","attribute":"tconstruct:generic.protection_cap",
            "each_level":0.05,"flat":0.05,"operation":"addition",
            "slots":["mainhand","offhand","feet","legs","chest","head"],
            "tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/armor"},"tooltip_style":"percent"
        }]});
        assert!(supported_value(&value));
        value["modules"][0]["flat"] = serde_json::json!(0);
        assert!(!supported_value(&value));
    }
}
