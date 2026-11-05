// Pinned SlingKnockbackModule's ordinary Battlesign path.
pub(super) fn known(id: &str, bytes: Option<&[u8]>) -> bool {
    let expected = match id {
        "bonking" => {
            r#"{"level_display":"tconstruct:single_level","modules":[{"type":"tconstruct:sling_knockback","damage_multiplier":1.5,"drawtime_multiplier":1.5,"force_multiplier":{"each_level":2.0,"flat":1.0}}]}"#
        }
        "knockback" => {
            r#"{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:knockback","allow":"projectile","each_level":0.5},{"type":"tconstruct:attribute","attribute":"minecraft:generic.attack_knockback","each_level":1.0,"operation":"addition","slots":["mainhand","feet","legs","chest","head"]},{"type":"tconstruct:sling_force","formula":[0.25,"$level","*","$multiplier","*",1.0,"$knockback_resistance","-","*","$knockback_multiplier","*","$value","+"],"percent":false,"sling":{"type":"tconstruct:tag","tag":"tconstruct:knockback_slings/self"},"variables":{"knockback_multiplier":{"type":"tconstruct:entity","attribute":"tconstruct:generic.knockback_multiplier","entity_type":"tconstruct:attribute","fallback":1.0},"knockback_resistance":{"type":"tconstruct:entity","attribute":"minecraft:generic.knockback_resistance","entity_type":"tconstruct:attribute","fallback":0.0}}},{"type":"tconstruct:sling_force","formula":[0.25,"$level","*","$multiplier","*","$value","+"],"percent":false,"sling":{"type":"tconstruct:tag","tag":"tconstruct:knockback_slings/target"}}]}"#
        }
        "tool" => {
            r#"{"modules":[{"type":"tconstruct:material_stats","primary_part":0,"stat_types":["tconstruct:head","tconstruct:plating_shield"]},{"type":"tconstruct:default_materials","materials":[{"type":"tconstruct:random","allow_hidden":true},{"type":"tconstruct:random","allow_hidden":true}]},{"type":"tconstruct:material_traits","hooks":["tconstruct:rebalanced_trait"],"material_index":1,"stat_type":"tconstruct:plating_shield"},{"type":"tconstruct:base_stats","stats":{"tconstruct:attack_damage":2.0,"tconstruct:attack_speed":1.2,"tconstruct:block_amount":50.0,"tconstruct:block_angle":120.0}},{"type":"tconstruct:modifier_slots","slots":{"abilities":1,"defense":3}},{"type":"tconstruct:traits","traits":[{"level":1,"name":"tconstruct:blocking"},{"level":1,"name":"tconstruct:bonking"},{"level":1,"name":"tconstruct:knockback"}]},{"type":"tconstruct:melee_particle","particle":"tconstruct:bonk"}]}"#
        }
        _ => return false,
    };
    bytes.and_then(|b| serde_json::from_slice::<serde_json::Value>(b).ok())
        == serde_json::from_str::<serde_json::Value>(expected).ok()
}
pub(super) fn charge(ticks: i32, draw: i32) -> f32 {
    if ticks <= 0 {
        return 0.0;
    }
    let c = ticks as f32 / draw as f32;
    ((c * c + 2.0 * c) / 3.0).min(1.0)
}
pub(super) fn draw_ticks(attack_speed: f32) -> Option<i32> {
    (attack_speed.is_finite() && attack_speed >= 0.0).then(|| (30.0 / attack_speed).ceil() as i32)
}
pub(super) fn force(charge: f32, knockback: i32) -> f32 {
    let multiplier = charge * 3.0;
    // Canonical Knockback target-sling hook, after the default power of one.
    multiplier + 0.25 * knockback as f32 * multiplier
}
pub(super) fn safe_module(value: &serde_json::Value) -> bool {
    value
        .get("modules")
        .and_then(|v| v.as_array())
        .is_some_and(|modules| {
            modules.iter().all(|m| {
                matches!(
                    m.get("type").and_then(|v| v.as_str()),
                    Some(
                        "tconstruct:repair"
                            | "tconstruct:reduce_tool_damage"
                            | "tconstruct:tool_damage_range"
                            | "tconstruct:rarity"
                            | "tconstruct:stat_boost"
                            | "tconstruct:set_stat"
                            | "tconstruct:modifier_slot"
                            | "tconstruct:volatile_flag"
                            | "tconstruct:volatile_int"
                    )
                ) && m.get("hooks").is_none()
                    && m.get("flag").and_then(|v| v.as_str()) != Some("tconstruct:no_melee")
            })
        })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_charge_and_attack_speed_draw() {
        assert_eq!(draw_ticks(1.2), Some(25));
        assert_eq!(draw_ticks(0.0), Some(i32::MAX));
        assert_eq!(draw_ticks(f32::NAN), None);
        assert_eq!(charge(0, 25), 0.0);
        assert!(charge(1, 25) > 0.0);
        assert_eq!(charge(25, 25), 1.0);
        assert_eq!(force(1.0, 1), 3.75);
        assert_eq!(force(0.5, 1), 1.875);
    }
    #[test]
    fn unknown_or_interacting_modules_fail_closed() {
        assert!(!safe_module(
            &serde_json::json!({"modules":[{"type":"tconstruct:fishing"}]})
        ));
        assert!(!safe_module(
            &serde_json::json!({"modules":[{"type":"tconstruct:repair","hooks":["tconstruct:sling_launch"]}]})
        ));
        assert!(safe_module(
            &serde_json::json!({"modules":[{"type":"tconstruct:repair"}]})
        ));
        assert!(!known("bonking", None));
    }
}
