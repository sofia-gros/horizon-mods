// Canonical Solar Powered definition: Copyright (c) 2022 SlimeKnights, MIT.
// TConstruct f4e77d43f39ba1047bf93ae816b7e1beaab514c2.
// Full notice: tconstruct/third_party/tconstruct/LICENSE.
const DEFINITION: &str = r###"{"level_display":"tconstruct:single_level","modules":[{"type":"tconstruct:reduce_tool_damage","formula":[0.01,"$level","*",0.04,"+","$light","*"],"reinforced_tooltip":true,"variables":{"light":{"type":"tconstruct:entity","entity_type":"tconstruct:light","fallback":15.0,"light_layer":"sky"}}}],"priority":185}"###;
pub(super) fn supported(value: &serde_json::Value) -> bool {
    static EXPECTED: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    value
        == EXPECTED.get_or_init(|| {
            serde_json::from_str(DEFINITION).expect("retained Solar Powered definition")
        })
}

/// EntityLightVariable's SKY path clamps negative effective skylight to zero.
/// Keep the postfix formula's separate f32 multiply, add and multiply order.
pub(super) fn probability(level: i32, sky: i32, darken: i32) -> Option<f32> {
    if level < 0 || !(0..=15).contains(&sky) || !(0..=15).contains(&darken) {
        return None;
    }
    if level == 0 {
        return Some(0.0);
    }
    Some((0.01f32 * level as f32 + 0.04f32) * (sky - darken).max(0) as f32)
}
/// ReduceToolDamageModule draws once per remaining point. Certain and absent
/// prevention consume no RNG, and equality with the chance retains the point.
pub(super) fn reduce(amount: u32, chance: f32, mut next: impl FnMut() -> f32) -> u32 {
    if amount == 0 || chance <= 0.0 {
        return amount;
    }
    if chance >= 1.0 {
        return 0;
    }
    (0..amount).filter(|_| next() >= chance).count() as u32
}
/// Minecraft's blockPosition floors all axes, including negative coordinates.
pub(super) fn block_position(position: [f64; 3]) -> Option<[i32; 3]> {
    if !position.into_iter().all(|v| {
        v.is_finite() && v.floor() >= f64::from(i32::MIN) && v.floor() <= f64::from(i32::MAX)
    }) {
        return None;
    }
    Some(position.map(|v| v.floor() as i32))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_exact_source_definition_is_supported() {
        let mut value: serde_json::Value = serde_json::from_str(DEFINITION).unwrap();
        assert!(supported(&value));
        value["priority"] = serde_json::json!(150);
        assert!(!supported(&value));
    }
    #[test]
    fn daylight_night_cover_and_zero_level_use_source_order() {
        assert_eq!(probability(1, 15, 0), Some((0.01f32 + 0.04f32) * 15.0));
        assert_eq!(probability(1, 15, 11), Some((0.01f32 + 0.04f32) * 4.0));
        assert_eq!(probability(1, 4, 11), Some(0.0));
        assert_eq!(probability(0, 15, 0), Some(0.0));
        assert_eq!(probability(1, 16, 0), None);
        assert_eq!(probability(-1, 15, 0), None);
        assert_eq!(probability(1, 15, -1), None);
    }
    #[test]
    fn equality_keeps_the_point_and_certain_results_do_not_roll() {
        let chance = probability(1, 15, 0).unwrap();
        let rolls = [0.0, chance, 0.99];
        let mut count = 0;
        assert_eq!(
            reduce(3, chance, || {
                let value = rolls[count];
                count += 1;
                value
            }),
            2
        );
        assert_eq!(count, 3);
        assert_eq!(reduce(0, chance, || panic!("zero amount")), 0);
        assert_eq!(reduce(7, 0.0, || panic!("dark")), 7);
        assert_eq!(
            reduce(7, probability(3, 15, 0).unwrap(), || panic!(
                "certain prevention"
            )),
            0
        );
    }
    #[test]
    fn feet_position_floors_and_rejects_invalid_coordinates() {
        assert_eq!(block_position([-0.1, 64.9, 15.999]), Some([-1, 64, 15]));
        assert_eq!(block_position([f64::NAN, 64.0, 0.0]), None);
        assert_eq!(block_position([f64::from(i32::MAX) + 1.0, 64.0, 0.0]), None);
    }
}
