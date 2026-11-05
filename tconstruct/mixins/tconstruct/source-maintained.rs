// Maintained's attribute module, independent from its mining/velocity modules.
pub(super) fn supported_value(value: &serde_json::Value) -> bool {
    value == &serde_json::from_str::<serde_json::Value>(r###"{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:conditional_mining_speed","formula":["$max_durability",0.5,"*","duplicate","$durability","!-","non-negative","!/","$level","*",6,"*","$multiplier","*","$speed","+"],"percent":false,"require_effective":true,"variables":{"durability":"tconstruct:tool_durability","max_durability":{"type":"tconstruct:tool_stat","stat":"tconstruct:durability"}}},{"type":"tconstruct:conditional_stat","formula":["$max_durability",0.5,"*","duplicate","$durability","!-","non-negative","!/","$level","*",0.08,"*","$multiplier","*","$value","+"],"percent":false,"stat":"tconstruct:velocity","variables":{"durability":"tconstruct:tool_durability","max_durability":{"type":"tconstruct:tool_stat","stat":"tconstruct:durability"}}},{"type":"tconstruct:attribute","attribute":"minecraft:generic.armor_toughness","formula":["$max_durability",0.5,"*","duplicate","$durability","!-","non-negative","!/","$level","*",2,"*"],"operation":"addition","slots":["mainhand","offhand","feet","legs","chest","head"],"tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/armor"},"variables":{"durability":"tconstruct:tool_durability","max_durability":{"type":"tconstruct:tool_stat","stat":"tconstruct:durability"}}}]}"###)
        .expect("retained Maintained definition")
}

/// The source maximum is a float stat; current durability truncates that stat
/// to an integer before subtracting damage. Do not round maximum first.
pub(super) fn toughness(raw_maximum: f32, maximum: u32, damage: u32, level: i32, broken: bool) -> Option<f64> {
    if !raw_maximum.is_finite() || raw_maximum < 1.0 || maximum == 0
        || raw_maximum as u32 != maximum || level < 0 { return None; }
    if broken || damage >= maximum || level == 0 { return Some(0.0); }
    let half = raw_maximum * 0.5f32;
    let remaining = maximum.saturating_sub(damage) as f32;
    let amount = ((remaining - half).max(0.0) / half) * level as f32 * 2.0f32;
    amount.is_finite().then_some(f64::from(amount))
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn falls_to_zero_at_half_and_never_multiplies_base_toughness() {
        assert_eq!(toughness(100.0,100,0,2,false),Some(4.0));
        assert_eq!(toughness(100.0,100,25,2,false),Some(2.0));
        assert_eq!(toughness(100.0,100,50,2,false),Some(0.0));
        assert_eq!(toughness(100.0,100,99,2,false),Some(0.0));
        assert_eq!(toughness(100.0,100,0,2,true),Some(0.0));
    }
    #[test] fn fractional_maximum_and_invalid_data() {
        let expected = f64::from(((100.0f32 - 50.25) / 50.25) * 2.0);
        assert_eq!(toughness(100.5,100,0,1,false),Some(expected));
        assert!(expected < 2.0);
        assert_eq!(toughness(100.5,101,0,1,false),None);
        assert_eq!(toughness(f32::NAN,100,0,1,false),None);
        assert_eq!(toughness(100.0,100,0,-1,false),None);
    }
    #[test] fn repairs_and_level_changes_are_recomputed_not_accumulated() {
        assert_eq!(toughness(100.0,100,75,1,false),Some(0.0));
        assert_eq!(toughness(100.0,100,25,1,false),Some(1.0));
        assert_eq!(toughness(100.0,100,25,3,false),Some(3.0));
        assert_eq!(toughness(100.0,100,25,0,false),Some(0.0));
        assert_eq!(toughness(100.0,100,100,1,false),Some(0.0));
    }
}
