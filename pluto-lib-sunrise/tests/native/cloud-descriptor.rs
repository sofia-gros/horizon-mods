use super::*;

fn request() -> Descriptor {
    Descriptor {
        position: [1.25, -32.5, 9.75], radius: 1.0, radius_per_tick: -1.0 / 600.0,
        radius_on_use: -0.5, timing: Timing { age: 0, duration_ticks: 600,
            wait_ticks: 10, reapplication_ticks: 20, duration_on_use: 0,
            effect_duration_scale: 0.25 },
        color_rgb: 0x123456, label: Label::Literal("fixture source".into()),
        attribution: Attribution::Unowned,
        effects: vec![Effect { id: "minecraft:speed".into(), duration_ticks: 400,
            amplifier: 2, visible: false, ambient: true }],
    }
}

#[test]
fn validated_request_preserves_policy_values_order_and_duplicates() {
    let mut d = request();
    d.effects.push(Effect { id: "poison".into(), duration_ticks: 31, amplifier: 0,
        visible: true, ambient: false });
    d.effects.push(d.effects[0].clone());
    let validated = validate(d.clone()).unwrap();
    assert_eq!(validated.descriptor(), &d);
    assert_eq!(validated.effects().iter().map(|e| e.raw_id).collect::<Vec<_>>(), ["speed", "poison", "speed"]);
    assert_eq!(validated.effects()[0], ResolvedEffect { raw_id: "speed", duration_ticks: 400,
        amplifier: 2, visible: false, ambient: true });
    assert_eq!(validated.effects()[1].duration_ticks, 31);
}

#[test]
fn attribution_is_an_explicit_request_not_inferred_actor_authority() {
    for attribution in [Attribution::Unowned, Attribution::Player { guest_connection: None },
        Attribution::Player { guest_connection: Some(17) }, Attribution::Mob { entity_id: 42 }] {
        let mut d = request(); d.attribution = attribution;
        assert_eq!(validate(d).unwrap().descriptor().attribution, attribution);
    }
}

#[test]
fn position_radius_and_timing_reject_nonfinite_and_out_of_range() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, f64::from(i32::MIN) - 1.0, f64::from(i32::MAX) + 1.0] {
        let mut d = request(); d.position[1] = value;
        assert_eq!(validate(d).unwrap_err(), Error::InvalidPosition);
    }
    for value in [f32::NAN, f32::INFINITY, -0.1, 32.1] {
        let mut d = request(); d.radius = value;
        assert_eq!(validate(d).unwrap_err(), Error::InvalidRadius);
    }
    for field in 0..2 {
        let mut d = request();
        if field == 0 { d.radius_per_tick = f32::NAN; } else { d.radius_on_use = f32::INFINITY; }
        assert_eq!(validate(d).unwrap_err(), Error::InvalidRadius);
    }
    for field in 0..5 {
        let mut d = request();
        match field { 0 => d.timing.age = -1, 1 => d.timing.duration_ticks = -2,
            2 => d.timing.wait_ticks = -1, 3 => d.timing.reapplication_ticks = -1,
            _ => d.timing.effect_duration_scale = -0.25 }
        assert_eq!(validate(d).unwrap_err(), Error::InvalidTiming);
    }
    for scale in [f32::NAN, f32::INFINITY] {
        let mut d = request(); d.timing.effect_duration_scale = scale;
        assert_eq!(validate(d).unwrap_err(), Error::InvalidTiming);
    }
}

#[test]
fn color_labels_and_effect_budget_are_bounded() {
    let mut d = request(); d.color_rgb = 0xff123456;
    assert_eq!(validate(d).unwrap_err(), Error::InvalidColor);
    for label in [Label::Literal("x".repeat(MAX_LABEL_BYTES + 1)), Label::Translation(String::new())] {
        let mut d = request(); d.label = label;
        assert_eq!(validate(d).unwrap_err(), Error::InvalidLabel);
    }
    let mut d = request(); d.effects.clear();
    assert_eq!(validate(d).unwrap_err(), Error::EffectBudget);
    let mut d = request(); d.effects = vec![d.effects[0].clone(); MAX_EFFECTS + 1];
    assert_eq!(validate(d).unwrap_err(), Error::EffectBudget);
}

#[test]
fn unknown_invalid_and_unrepresentable_effects_fail_without_partial_result() {
    let mut d = request(); d.effects.push(Effect { id: "fixture:unknown".into(), ..d.effects[0].clone() });
    assert_eq!(validate(d).unwrap_err(), Error::UnknownEffect { index: 1 });
    for (duration, amplifier) in [(-2, 0), (1, -1), (1, 256)] {
        let mut d = request(); d.effects[0].duration_ticks = duration; d.effects[0].amplifier = amplifier;
        assert_eq!(validate(d).unwrap_err(), Error::InvalidEffect { index: 0 });
    }
    for flags in [(true, true), (false, false)] {
        let mut d = request(); d.effects.push(Effect { id: "speed".into(), visible: flags.0,
            ambient: flags.1, ..d.effects[0].clone() });
        assert_eq!(validate(d).unwrap_err(), Error::ConflictingFlags { index: 1 });
    }
}

#[test]
fn legal_edges_are_preserved_without_defaulting_clamping_or_scaling() {
    for radius in [0.0, 32.0] {
        let mut d = request(); d.radius = radius;
        d.position = [f64::from(i32::MIN), f64::from(i32::MAX), 0.0];
        d.timing.duration_ticks = -1; d.timing.wait_ticks = 0;
        d.timing.reapplication_ticks = 0; d.timing.effect_duration_scale = 0.0;
        d.timing.duration_on_use = -13; d.effects[0].duration_ticks = -1;
        d.effects[0].amplifier = 255; d.label = Label::Literal(String::new());
        let valid = validate(d.clone()).unwrap();
        assert_eq!(valid.descriptor(), &d);
        assert_eq!(valid.effects()[0].duration_ticks, -1);
    }
}
