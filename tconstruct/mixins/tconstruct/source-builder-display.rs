use serde_json::Value;

/// Order the common stat fields as in the source material tooltip.
pub(super) fn field_order(stat: &str) -> &'static [&'static str] {
    match stat {
        "tconstruct:head" => &["durability", "mining_tier", "mining_speed", "melee_attack"],
        "tconstruct:handle" => &["durability", "melee_damage", "melee_speed", "mining_speed"],
        "tconstruct:grip" => &["durability", "accuracy", "melee_damage"],
        "tconstruct:limb" => &["durability", "draw_speed", "velocity", "accuracy"],
        _ => &[],
    }
}

/// Match source percent boosts and signed ranged bonuses, keeping ordinary
/// numeric stats and registry values as data rather than guessed numbers.
pub(super) fn value(stat: &str, field: &str, raw: &Value) -> Option<String> {
    if let Some(number) = raw.as_f64() {
        let percent = stat == "tconstruct:handle" || stat == "tconstruct:grip" && field == "durability";
        let bonus = stat == "tconstruct:limb" && matches!(field, "draw_speed" | "velocity" | "accuracy")
            || stat == "tconstruct:grip" && field == "accuracy";
        if percent { return Some(format!("{:+.0}%", number * 100.0)); }
        if bonus { return Some(format!("{:+}", number)); }
        return Some(number.to_string());
    }
    raw.as_str().map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_percent_boosts_and_bonuses_are_not_raw_multipliers() {
        assert_eq!(value("tconstruct:handle", "durability", &serde_json::json!(0.0)).as_deref(), Some("+0%"));
        assert_eq!(value("tconstruct:handle", "melee_speed", &serde_json::json!(0.25)).as_deref(), Some("+25%"));
        assert_eq!(value("tconstruct:limb", "velocity", &serde_json::json!(-0.2)).as_deref(), Some("-0.2"));
        assert_eq!(value("tconstruct:head", "durability", &serde_json::json!(60)).as_deref(), Some("60"));
        assert_eq!(field_order("tconstruct:head")[1], "mining_tier");
    }
}
