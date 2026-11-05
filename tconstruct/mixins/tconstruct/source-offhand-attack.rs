// OffhandAttackModifier + Mantle OffhandCooldownTracker, pinned source.
pub(super) fn known(id: &str, bytes: Option<&[u8]>) -> bool {
    let expected = match id {
        "tool" => {
            r#"{"modules":[{"type":"tconstruct:part_stats","parts":["tconstruct:small_blade","tconstruct:tool_handle"],"primary_part":0},{"type":"tconstruct:default_materials","materials":[{"type":"tconstruct:random","tier":1},{"type":"tconstruct:random","tier":1}]},{"type":"tconstruct:base_stats","stats":{"tconstruct:attack_damage":3.0,"tconstruct:attack_speed":2.0,"tconstruct:block_amount":10.0,"tconstruct:use_item_speed":1.0}},{"type":"tconstruct:multiply_stats","multipliers":{"tconstruct:attack_damage":0.65,"tconstruct:durability":0.75,"tconstruct:mining_speed":0.75}},{"type":"tconstruct:modifier_slots","slots":{"abilities":1,"upgrades":3}},{"type":"tconstruct:traits","traits":[{"level":1,"name":"tconstruct:padded"},{"level":1,"name":"tconstruct:offhand_attack"},{"level":1,"name":"tconstruct:silky_shears"}]},{"type":"tconstruct:tool_actions","tool_actions":["sword_dig","hoe_dig"]},{"type":"tconstruct:is_effective","predicate_type":"mantle:tag","tag":"tconstruct:mineable/dagger"},{"type":"tconstruct:mining_speed_modifier","blocks":["minecraft:cobweb"],"modifier":7.5,"predicate_type":"mantle:set"},{"type":"tconstruct:fixed_material_name","index":0}]}"#
        }
        "silky_shears" => {
            r#"{"level_display":"tconstruct:pluses","modules":[{"type":"tconstruct:shears","expanded":1.0,"per_level":1.0,"tool":{"type":"tconstruct:has_modifier","check":"all","modifier":{"type":"tconstruct:single","modifier":"tconstruct:silky"}}},{"type":"tconstruct:show_interaction_source"}],"priority":70}"#
        }
        "cultivated" => {
            r#"{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:repair","each_level":0.5}]}"#
        }
        "dense" => {
            r#"{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:repair","formula":["$factor",1.0,0.025,"$level","*",11.0,"$level","-","*","-","*"],"modifier_level":{"max":5}},{"type":"tconstruct:repair","each_level":-0.05,"flat":-0.5,"modifier_level":{"min":6}},{"type":"tconstruct:reduce_tool_damage","formula":[1.0,1.0,1.5,"$level","^","/","-"]}]}"#
        }
        "stringy" => {
            r#"{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:material_repair","durability":140,"material":"tconstruct:string"}]}"#
        }
        "preserved" => {
            r#"{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:stat_boost","each_level":0.15,"operation":"multiply_base","stat":"tconstruct:durability"},{"type":"tconstruct:repair","each_level":0.15}]}"#
        }
        "tanned" => {
            r#"{"level_display":"tconstruct:no_levels","modules":[{"type":"tconstruct:tool_damage_range","cause":{"type":"mantle:inverted","inverted_type":"tconstruct:tag","tag":"tconstruct:durability/bypass_tanned"},"max":1},{"type":"tconstruct:tool_damage_range","cause":{"type":"tconstruct:tag","tag":"tconstruct:durability/secondary"},"max":0}],"priority":200}"#
        }
        "reinforced" => {
            r#"{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:reduce_tool_damage","cause":{"type":"mantle:inverted","inverted_type":"tconstruct:tag","tag":"tconstruct:durability/bypass_reinforced"},"formula":[0.025,"$level","*",11.0,"$level","-","*"],"modifier_level":{"max":5},"tool":{"type":"tconstruct:has_modifier","check":"all","level":0,"modifier":{"type":"tconstruct:single","modifier":"tconstruct:unbreakable"}}},{"type":"tconstruct:reduce_tool_damage","cause":{"type":"mantle:inverted","inverted_type":"tconstruct:tag","tag":"tconstruct:durability/bypass_reinforced"},"each_level":0.05,"flat":0.5,"modifier_level":{"min":6},"tool":{"type":"tconstruct:has_modifier","check":"all","level":0,"modifier":{"type":"tconstruct:single","modifier":"tconstruct:unbreakable"}}}]}"#
        }
        "lightweight" => {
            r#"{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:stat_boost","each_level":0.08,"operation":"multiply_base","stat":"tconstruct:attack_speed"},{"type":"tconstruct:stat_boost","each_level":0.08,"operation":"multiply_base","stat":"tconstruct:mining_speed"},{"type":"tconstruct:stat_boost","each_level":0.03,"operation":"add","stat":"tconstruct:draw_speed"},{"type":"tconstruct:stat_boost","each_level":0.03,"operation":"add","stat":"tconstruct:velocity"}]}"#
        }
        "ductile" => {
            r#"{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:stat_boost","each_level":0.1,"operation":"multiply_base","stat":"tconstruct:durability"},{"type":"tconstruct:stat_boost","each_level":0.05,"operation":"multiply_base","stat":"tconstruct:attack_damage"},{"type":"tconstruct:stat_boost","each_level":0.25,"operation":"add","stat":"tconstruct:projectile_damage"},{"type":"tconstruct:stat_boost","each_level":1.0,"operation":"add","stat":"tconstruct:armor_toughness"}]}"#
        }
        "sharpness" => {
            r#"{"level_display":{"type":"tconstruct:unique","first_unique":true,"unique_until":5},"modules":[{"type":"tconstruct:stat_boost","each_level":0.75,"operation":"add","stat":"tconstruct:attack_damage"}]}"#
        }
        "swiftstrike" => {
            r#"{"level_display":{"type":"tconstruct:unique","first_unique":false,"unique_until":5},"modules":[{"type":"tconstruct:stat_boost","each_level":0.05,"operation":"multiply_base","stat":"tconstruct:attack_speed"}]}"#
        }
        _ => return false,
    };
    bytes.and_then(|b| serde_json::from_slice::<serde_json::Value>(b).ok())
        == serde_json::from_str::<serde_json::Value>(expected).ok()
}
/// Mantle uses Math.round(20 / speed), not ceil or the main-hand half-tick.
pub(super) fn cooldown(speed: f32) -> Option<i32> {
    (speed.is_finite() && speed >= 0.0).then(|| (20.0 / speed).round() as i32)
}
/// The source permits another attack after one tick, with partial damage.
/// A fresh player has full charge. A replaced player's backwards tick resets it.
pub(super) fn charge(now: i32, last: Option<(i32, i32)>) -> Option<f32> {
    let Some((at, duration)) = last else {
        return Some(1.0);
    };
    let elapsed = i64::from(now) - i64::from(at);
    if elapsed < 0 {
        return Some(1.0);
    }
    if elapsed == 0 {
        return None;
    }
    Some(if duration <= 0 {
        1.0
    } else {
        (elapsed as f32 / duration as f32).clamp(0.0, 1.0)
    })
}
pub(super) fn hurt_delay(current: i32, weapon_ticks: i32) -> i32 {
    if weapon_ticks < current {
        ((i64::from(current) + i64::from(weapon_ticks)) / 2) as i32
    } else {
        current
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_cooldown_rejects_same_tick_but_allows_partial_hits() {
        assert_eq!(charge(10, None), Some(1.0));
        assert_eq!(charge(10, Some((10, 10))), None);
        assert_eq!(charge(11, Some((10, 10))), Some(0.1));
        assert_eq!(charge(20, Some((10, 10))), Some(1.0));
        assert_eq!(charge(21, Some((10, 10))), Some(1.0));
        assert_eq!(charge(0, Some((10, 10))), Some(1.0));
        assert_eq!(charge(11, Some((10, 0))), Some(1.0));
    }
    #[test]
    fn source_speed_rounding_and_pack_gates() {
        assert_eq!(hurt_delay(20, 10), 15);
        assert_eq!(hurt_delay(10, 10), 10);
        assert_eq!(hurt_delay(20, 30), 20);
        assert_eq!(cooldown(2.0), Some(10));
        assert_eq!(cooldown(3.0), Some(7));
        assert_eq!(cooldown(0.0), Some(i32::MAX));
        assert_eq!(cooldown(f32::NAN), None);
        assert!(!known("tool", None));
        assert!(!known("silky_shears", Some(b"{}")));
        assert!(!known("custom", Some(b"{}")));
    }
}
