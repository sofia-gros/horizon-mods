// Pending actual-native fixture. No substitute Game or actor store is defined.
// The fixture must import unchanged main08afa rules/math and actual native Tag.
use crate::tconstruct_source_mining_rules as rules;
use horizon_world::nbt::Tag;
use std::collections::HashSet;

fn tool(modifiers: &[(&str, i32)]) -> Tag {
    Tag::compound([
        ("tic_max_damage", Tag::Int(100)),
        ("tic_mining_traits", Tag::List(modifiers.iter().map(|(id, level)| {
            Tag::compound([("id", Tag::String((*id).into())), ("level", Tag::Int(*level))])
        }).collect())),
    ])
}

fn apply(custom: &Tag, damage: u32, incoming: f32, env: f32,
    position: Option<(i32, i32)>, effective: bool, broad: Option<bool>,
    previous_water: Option<bool>, ground: bool, armor: f32) -> f32 {
    // This is a scalar-policy composition oracle, NOT an invocation of Game/apply.
    let speed = rules::conditional_with_water(custom, damage, incoming, env,
        position, effective, false, broad, Some(6.0), previous_water).unwrap_or(incoming);
    rules::finish_mining_speed(speed * rules::airborne_multiplier(custom, damage, ground), armor)
}

#[test]
fn normal_incoming_speed_is_the_base_not_a_recomputed_tool_speed() {
    let c = tool(&[("tconstruct:blasting", 20)]);
    for (incoming, expected) in [(4.0, 5.0), (8.0, 9.0), (0.0, 1.0)] {
        assert_eq!(apply(&c, 0, incoming, 1.0, None, true, Some(false), None, true, 1.0), expected);
    }
}

#[test]
fn native_attribute_does_not_scale_the_source_flat_addition() {
    let c = tool(&[("tconstruct:blasting", 20)]);
    let env = rules::environment_multiplier(Some(0), None, None, false, false, true);
    for native_attribute in [0.0, 0.5, 1.0, 2.0] {
        let incoming = (4.0 * env) * native_attribute;
        assert_eq!(apply(&c, 0, incoming, env, None, true, Some(false), None, true, 1.0), incoming + env);
    }
}

#[test]
fn source_environment_preserves_fatigue_water_ground_order() {
    for (fatigue, penalty) in [(None, 1.0), (Some(0), 0.3), (Some(1), 0.09),
        (Some(2), 0.0027), (Some(3), 0.00081), (Some(8), 0.00081)] {
        for water in [false, true] { for affinity in [false, true] { for ground in [false, true] {
            let mut oracle = 1.6f32 * penalty;
            if water && !affinity { oracle /= 5.0; }
            if !ground { oracle /= 5.0; }
            assert_eq!(rules::environment_multiplier(Some(0), Some(2), fatigue, water, affinity, ground).to_bits(), oracle.to_bits());
        }}}
    }
}

#[test]
fn current_water_multiplier_and_previous_water_hydraulic_are_distinct() {
    let c = tool(&[("tconstruct:hydraulic", 36)]);
    for current_water in [false, true] { for affinity in [false, true] {
        let env = rules::environment_multiplier(None, None, None, current_water, affinity, true);
        let incoming = 4.0 * env;
        let dry_previous = apply(&c, 0, incoming, env, None, true, Some(false), None, true, 1.0);
        let wet_previous = apply(&c, 0, incoming, env, None, true, Some(false), Some(affinity), true, 1.0);
        assert_eq!(dry_previous.to_bits(), incoming.to_bits());
        assert_eq!(wet_previous.to_bits(), (incoming + env * if affinity { 8.0 } else { 40.0 }).to_bits());
        assert_ne!(dry_previous, wet_previous);
    }}
}

#[test]
fn airborne_then_armor_follows_flat_bonus() {
    let c = tool(&[("tconstruct:maintained", 1), ("tconstruct:airborne", 1)]);
    let env = rules::environment_multiplier(None, None, None, false, false, false);
    let incoming = 4.0 / 5.0;
    let flat = incoming + 6.0 * env;
    assert_eq!(apply(&c, 0, incoming, env, None, true, Some(false), None, false, 1.1).to_bits(), ((flat * 5.0) * 1.1).to_bits());
}

#[test]
fn broken_tool_preserves_incoming_before_independent_armor() {
    let c = tool(&[("tconstruct:blasting", 20), ("tconstruct:airborne", 1)]);
    assert_eq!(apply(&c, 100, 3.0, 1.0, None, true, Some(false), None, false, 1.1).to_bits(), (3.0f32 * 1.1).to_bits());
    let flagged = Tag::compound([
        ("tic_max_damage", Tag::Int(100)), ("tic_broken", Tag::Byte(1)),
        ("tic_mining_traits", c.get("tic_mining_traits").unwrap().clone()),
    ]);
    assert_eq!(apply(&flagged, 0, 3.0, 1.0, None, true, Some(false), None, false, 1.1).to_bits(), (3.0f32 * 1.1).to_bits());
}

#[test]
fn malformed_tool_and_valid_ineffective_tool_remain_distinct() {
    assert_eq!(rules::evaluated_speed(&Tag::Int(1), "minecraft:stone", |_, _| false), None);
    let valid = Tag::compound([("rules", Tag::List(vec![Tag::compound([
        ("blocks", Tag::String("minecraft:dirt".into())), ("speed", Tag::Float(7.0f32.to_bits())),
    ])]))]);
    assert_eq!(rules::evaluated_speed(&valid, "minecraft:stone", |_, _| false), Some(None));
    assert_eq!(rules::evaluated_speed(&valid, "minecraft:dirt", |_, _| false), Some(Some(7.0)));
}

#[test]
fn unavailable_broad_is_not_a_known_empty_tag() {
    let empty = HashSet::new();
    assert_eq!(rules::broad_or_unknown(None, "tconstruct:pickaxe"), None);
    assert_eq!(rules::broad_or_unknown(Some(&empty), "tconstruct:pickaxe"), Some(false));
    let c = Tag::compound([
        ("tic_max_damage", Tag::Int(100)), ("tic_overslime", Tag::Int(2)),
        ("tic_mining_traits", tool(&[("tconstruct:overwield", 2)]).get("tic_mining_traits").unwrap().clone()),
    ]);
    assert_eq!(apply(&c, 0, 3.0, 1.0, None, true, None, None, true, 1.0), 3.0);
    assert!(apply(&c, 0, 3.0, 1.0, None, true, Some(false), None, true, 1.0) > 3.0);
}

#[test]
fn queried_position_changes_dwarven_policy_without_a_mining_cache() {
    let c = tool(&[("tconstruct:dwarven", 1)]);
    let at = |y| apply(&c, 0, 3.0, 1.0, Some((y, 320)), true, Some(false), None, true, 1.0);
    assert_ne!(at(0).to_bits(), at(256).to_bits());
    assert_eq!(at(0).to_bits(), crate::tconstruct_source_dwarven::boost(0.0, 320, 1, 3.0, 6.0).to_bits());
}

#[test]
fn official_progress_keeps_harvest_division() {
    assert_eq!(crate::official_fragments::progress(6.0, 2.0, true), 0.1);
    assert_eq!(crate::official_fragments::progress(6.0, 2.0, false), 0.03);
}

#[test]
fn zero_hardness_nan_is_caller_specific_not_normalized() {
    let p = crate::official_fragments::progress(0.0, 0.0, true);
    assert!(p.is_nan());
    assert!(!crate::official_fragments::tick_threshold(p));
    assert!(crate::official_fragments::delayed_completion(Some(p)));
    assert!(!crate::official_fragments::delayed_completion(None));
    assert!(!crate::official_fragments::delayed_completion(Some(0.5)));
    assert!(crate::official_fragments::delayed_completion(Some(1.0)));
}

#[test]
fn official_nonfinite_and_negative_values_are_not_clamped() {
    for speed in [f32::NEG_INFINITY, -1.0, 0.0, 1.0, f32::INFINITY, f32::NAN] {
        let p = crate::official_fragments::progress(speed, 1.0, true);
        if speed.is_nan() { assert!(p.is_nan()); }
        else { assert_eq!(p.to_bits(), (speed / 1.0 / 30.0).to_bits()); }
    }
    assert_eq!(crate::official_fragments::progress(1.0, 0.0, true), f32::INFINITY);
    assert_eq!(crate::official_fragments::progress(-1.0, 0.0, true), f32::NEG_INFINITY);
}
