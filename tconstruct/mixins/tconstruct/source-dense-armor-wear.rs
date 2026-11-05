// Canonical definition: Copyright (c) 2022 SlimeKnights, MIT.
// TConstruct f4e77d43f39ba1047bf93ae816b7e1beaab514c2.
// Full notice: tconstruct/third_party/tconstruct/LICENSE.
const DEFINITION: &str = r###"{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:repair","formula":["$factor",1.0,0.025,"$level","*",11.0,"$level","-","*","-","*"],"modifier_level":{"max":5}},{"type":"tconstruct:repair","each_level":-0.05,"flat":-0.5,"modifier_level":{"min":6}},{"type":"tconstruct:reduce_tool_damage","formula":[1.0,1.0,1.5,"$level","^","/","-"]}]}"###;

pub(super) fn supported(value: &serde_json::Value) -> bool {
    static EXPECTED: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    value == EXPECTED.get_or_init(|| serde_json::from_str(DEFINITION).expect("retained Dense definition"))
}

/// PostFixOperator POWER widens float operands to Math.pow, then narrows the
/// result before the separate float division and subtraction. Do not replace
/// it with powi(-level), which changes those rounding boundaries.
pub(super) fn probability(level: i32) -> f32 {
    if level <= 0 { return 0.0; }
    let power = 1.5f64.powf(f64::from(level as f32)) as f32;
    1.0f32 - 1.0f32 / power
}

pub(super) fn reduce(amount: u32, level: i32, mut next: impl FnMut() -> f32) -> u32 {
    let chance = probability(level);
    if amount == 0 || chance <= 0.0 { return amount; }
    if chance >= 1.0 { return 0; }
    (0..amount).filter(|_| next() >= chance).count() as u32
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn full_definition_is_required() {
        let mut value: serde_json::Value = serde_json::from_str(DEFINITION).unwrap();
        assert!(supported(&value));
        value["modules"][2]["formula"][2] = serde_json::json!(2.0);
        assert!(!supported(&value));
    }
    #[test] fn source_power_then_float_division_order() {
        assert_eq!(probability(0), 0.0);
        assert_eq!(probability(1), 1.0f32 - 1.0f32 / 1.5f32);
        assert_eq!(probability(2), 1.0f32 - 1.0f32 / 2.25f32);
        assert_eq!(probability(100), 1.0);
    }
    #[test] fn equality_keeps_the_point_and_no_op_ranges_draw_nothing() {
        let chance = probability(1);
        let values = [0.0, chance, 0.999];
        let mut calls = 0;
        assert_eq!(reduce(3, 1, || { let value = values[calls]; calls += 1; value }), 2);
        assert_eq!(calls, 3);
        assert_eq!(reduce(0, 1, || panic!("no point to roll")), 0);
        assert_eq!(reduce(4, 0, || panic!("no active Dense")), 4);
        assert_eq!(reduce(4, 100, || panic!("full prevention")), 0);
    }
}
