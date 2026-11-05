// BlockingModule and InteractionHandler shield rules from the pinned source.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Stats {
    pub amount: f32,
    pub angle: f32,
    pub speed: f32,
}
pub(super) fn stock(item: &str) -> bool {
    matches!(
        item,
        "tconstruct:travelers_shield" | "tconstruct:plate_shield" | "tconstruct:battlesign"
    )
}
pub(super) fn stats(bytes: &[u8], blockade: f32) -> Option<Stats> {
    if !blockade.is_finite() || blockade < 0.0 {
        return None;
    }
    let definition: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let modules = definition.get("modules")?.as_array()?;
    // Do not silently ignore a replacement tool's additional stat machinery.
    if modules.iter().any(|m| {
        !matches!(
            m.get("type").and_then(|v| v.as_str()),
            Some(
                "tconstruct:material_stats"
                    | "tconstruct:default_materials"
                    | "tconstruct:material_traits"
                    | "tconstruct:base_stats"
                    | "tconstruct:modifier_slots"
                    | "tconstruct:traits"
                    | "tconstruct:melee_particle"
                    | "tconstruct:tool_parts"
                    | "tconstruct:attack_interaction"
                    | "tconstruct:fixed_material_name"
                    | "tconstruct:statless_part_repair"
            )
        )
    }) {
        return None;
    }
    let mut bases = modules
        .iter()
        .filter(|m| m.get("type").and_then(|v| v.as_str()) == Some("tconstruct:base_stats"));
    let base = bases.next()?.get("stats")?.as_object()?;
    if bases.next().is_some() {
        return None;
    }
    let number = |name: &str, default: f32| -> Option<f32> {
        let value = match base.get(name) {
            Some(v) => v.as_f64()? as f32,
            None => default,
        };
        value.is_finite().then_some(value)
    };
    Some(Stats {
        amount: (number("tconstruct:block_amount", 5.0)? * (1.0 + blockade)).clamp(0.0, 2048.0),
        angle: number("tconstruct:block_angle", 120.0)?.clamp(0.0, 180.0),
        speed: number("tconstruct:use_item_speed", 0.2)?.clamp(0.0, 1.0),
    })
}
pub(super) fn known_blocking(bytes: Option<&[u8]>) -> bool {
    bytes.and_then(|b| serde_json::from_slice::<serde_json::Value>(b).ok()).is_some_and(|v|
        v == serde_json::json!({"level_display":"tconstruct:no_levels","modules":[{"type":"tconstruct:blocking"}]}))
}
pub(super) fn known_blockade(bytes: Option<&[u8]>) -> bool {
    bytes.and_then(|b| serde_json::from_slice::<serde_json::Value>(b).ok()).is_some_and(|v|
        v == serde_json::json!({"level_display":"tconstruct:default","modules":[
            {"type":"tconstruct:stat_boost","each_level":1.0,"operation":"multiply_base","stat":"tconstruct:block_amount"},
            {"type":"tconstruct:requirements","display_modifiers":[{"level":1,"name":"tconstruct:blocking"}],
             "requirement":{"type":"mantle:or","predicates":[
                {"type":"tconstruct:has_modifier","check":"all","modifier":{"type":"tconstruct:single","modifier":"tconstruct:blocking"}},
                {"type":"tconstruct:has_modifier","check":"all","modifier":{"type":"tconstruct:single","modifier":"tconstruct:parrying"}}]},
             "translation_key":"modifier.tconstruct.blockade.requirements"}]}))
}
/// Source uses the pitched view against a horizontal source direction.
pub(super) fn faces(
    position: [f64; 3],
    source: Option<[f64; 3]>,
    yaw: f32,
    pitch: f32,
    full_angle: f32,
) -> bool {
    let Some(source) = source else {
        return false;
    };
    if !position.into_iter().chain(source).all(f64::is_finite)
        || !yaw.is_finite()
        || !pitch.is_finite()
    {
        return false;
    }
    let (dx, dz) = (source[0] - position[0], source[2] - position[2]);
    let length = dx.hypot(dz);
    if length < 1.0e-4 {
        return false;
    }
    let yaw = f64::from(-yaw).to_radians();
    let pitch = f64::from(pitch).to_radians();
    let dot = (dx * yaw.sin() + dz * yaw.cos()) * pitch.cos() / length;
    f64::from(full_angle) * 0.5 >= dot.clamp(-1.0, 1.0).acos().to_degrees()
}
pub(super) fn wear(blocked: f32) -> u32 {
    if !blocked.is_finite() || blocked < 3.0 {
        0
    } else {
        1 + blocked.floor().min(2048.0) as u32
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_cones_use_pitch_and_require_a_position() {
        assert!(faces([0.0; 3], Some([0.0, 8.0, 2.0]), 0.0, 0.0, 90.0));
        assert!(!faces([0.0; 3], Some([0.0, 8.0, 2.0]), 0.0, 60.0, 90.0));
        assert!(!faces([0.0; 3], Some([0.0, 8.0, -2.0]), 0.0, 0.0, 180.0));
        assert!(!faces([0.0; 3], Some([0.0, 8.0, 0.0]), 0.0, 0.0, 180.0));
        assert!(!faces([0.0; 3], None, 0.0, 0.0, 180.0));
    }
    #[test]
    fn wear_uses_blocked_damage_and_threshold() {
        assert_eq!(
            (wear(2.99), wear(3.0), wear(10.9), wear(f32::NAN)),
            (0, 4, 11, 0)
        );
    }
    #[test]
    fn block_amount_is_flat_capped_and_blockade_multiplies_base() {
        let b=br#"{"modules":[{"type":"tconstruct:base_stats","stats":{"tconstruct:block_amount":10.0,"tconstruct:block_angle":90.0,"tconstruct:use_item_speed":0.8}}]}"#;
        assert_eq!(
            stats(b, 2.0),
            Some(Stats {
                amount: 30.0,
                angle: 90.0,
                speed: 0.8
            })
        );
        assert_eq!(stats(b, 0.5).unwrap().amount, 15.0);
        assert!(stats(b, f32::NAN).is_none());
        assert_eq!(stats(b, 1000.0).unwrap().amount, 2048.0);
        assert!(stats(b, -1.0).is_none());
    }
}
