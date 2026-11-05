// Pinned Spectral modules; custom definitions must not inherit these effects.
pub(super) fn known(source: Option<&[u8]>) -> bool {
    let Some(value) = source.and_then(|raw| serde_json::from_slice::<serde_json::Value>(raw).ok())
    else {
        return false;
    };
    value
        == serde_json::json!({
            "level_display":"tconstruct:default", "priority":10,
            "modules":[
                {"type":"tconstruct:weapon_mob_effect","effect":"minecraft:glowing","level":{"flat":1.0},"time":{"per_level":200.0}},
                {"type":"tconstruct:counter_mob_effect","chance":{"flat":1.0},"durability_usage":0,"effect":"minecraft:glowing","level":{"flat":1.0},"time":{"per_level":200.0}},
                {"type":"tconstruct:projectile_place_glow","blocks":true,"entities":false,"tool_damage":5}
            ]
        })
}

pub(super) fn duration(level: i32) -> Option<i32> {
    (level > 0).then(|| (200.0f32 * level as f32) as i32)
}
pub(super) fn charged(scale: f32) -> bool {
    scale.is_finite() && scale > 0.9
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_duration_and_charge_threshold() {
        assert_eq!(duration(0), None);
        assert_eq!(duration(-1), None);
        assert_eq!(duration(1), Some(200));
        assert_eq!(duration(3), Some(600));
        assert_eq!(duration(i32::MAX), Some(i32::MAX));
        assert!(!charged(0.9));
        assert!(charged(0.91));
    }
    #[test]
    fn unknown_or_incomplete_definitions_are_closed() {
        assert!(!known(None));
        assert!(!known(Some(b"{}")));
        assert!(!known(Some(b"{broken")));
    }
}
