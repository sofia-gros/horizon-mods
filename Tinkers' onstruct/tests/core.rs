use serde_json::{Value, json};
use tconstruct_horizon::{Catalog, ToolState, diagnostic, stats::Accumulator, tool::formula};

fn close(left: f32, right: f32) {
    assert!((left - right).abs() < 0.00001, "{left} != {right}");
}

#[test]
fn wood_pickaxe_uses_part_stats_tool_trait_and_merged_material_traits() {
    let c = Catalog::embedded().unwrap();
    let e = c.evaluate(&ToolState::pickaxe(["wood"; 3])).unwrap();
    assert_eq!(e.stats.durability().unwrap(), 60);
    close(e.stats.mining_speed, 2.0);
    close(e.stats.attack_damage, 1.0);
    close(e.stats.attack_speed, 1.2);
    assert_eq!(e.stats.harvest_tier, "minecraft:wood");
    assert_eq!(
        e.modifiers
            .iter()
            .map(|m| (m.id.as_str(), m.level))
            .collect::<Vec<_>>(),
        [("tconstruct:pierce", 1), ("tconstruct:cultivated", 3)]
    );
    close(e.repair_factor(&c).unwrap(), 2.5);
    let effects = e.hit_effects(&c).unwrap();
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].effect, "tconstruct:pierce");
    close(effects[0].level, 1.0);
    close(effects[0].ticks, 80.0);
    assert!(effects[0].before_melee);
}

#[test]
fn flint_head_and_handle_percentage_are_distinct_operations() {
    let c = Catalog::embedded().unwrap();
    let e = c
        .evaluate(&ToolState::pickaxe(["flint", "wood", "wood"]))
        .unwrap();
    assert_eq!(e.stats.durability().unwrap(), 85);
    close(e.stats.attack_damage, 2.25);
    close(e.repair_factor(&c).unwrap(), 2.0);
    let all = c.evaluate(&ToolState::pickaxe(["flint"; 3])).unwrap();
    close(all.stats.durability, 72.25);
    assert_eq!(all.stats.durability().unwrap(), 72); // intValue, not rounding
    close(all.stats.attack_damage, 2.475);
    assert_eq!(all.modifiers[1].level, 3);
}

#[test]
fn damage_repair_and_state_roundtrip_keep_the_tool() {
    let c = Catalog::embedded().unwrap();
    let mut state = ToolState::pickaxe(["wood"; 3]);
    state.damage_points(&c, u32::MAX).unwrap();
    assert_eq!(state.damage, 60);
    assert!(state.broken);
    assert_eq!(state.repair_points(&c, 0).unwrap(), 0);
    assert!(state.broken);
    let e = c.evaluate(&state).unwrap();
    assert_eq!(e.mining_speed(&c, &state, true).unwrap(), 0.0);
    let saved = state.to_json().unwrap();
    assert_eq!(ToolState::from_json(&c, &saved).unwrap(), state);
    assert_eq!(state.repair_points(&c, 10).unwrap(), 10);
    assert_eq!(state.damage, 50);
    assert!(!state.broken);
    assert_eq!(state.repair_points(&c, u32::MAX).unwrap(), 50);
    assert_eq!(state.damage, 0);
}

#[test]
fn mining_base_wear_preserves_broken_and_independent_states() {
    let c = Catalog::embedded().unwrap();
    let mut first = ToolState::pickaxe(["wood"; 3]);
    let second = first.clone();
    first.damage_after_mining(&c, 0.0).unwrap();
    first.damage_after_mining(&c, -0.0).unwrap();
    assert_eq!(first.damage, 0);
    for _ in 0..59 {
        first.damage_after_mining(&c, 1.5).unwrap();
    }
    assert_eq!(first.damage, 59);
    assert!(!first.broken);
    first.damage_after_mining(&c, 1.5).unwrap();
    assert_eq!(first.damage, 60);
    assert!(first.broken);
    first.damage_after_mining(&c, 1.5).unwrap();
    assert_eq!(first.damage, 60);
    assert_eq!(second.damage, 0);
    assert_eq!(
        ToolState::from_json(&c, &first.to_json().unwrap()).unwrap(),
        first
    );
    let before = first.clone();
    assert!(first.damage_after_mining(&c, f32::NAN).is_err());
    assert_eq!(first, before);
}

#[test]
fn mining_base_wear_matches_pinned_java_get_damage() {
    let c = Catalog::embedded().unwrap();
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/mining-wear-oracle.json")).unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let mut state = ToolState::pickaxe(["wood"; 3]);
        state
            .damage_after_mining(&c, case["hardness"].as_f64().unwrap() as f32)
            .unwrap();
        assert_eq!(u64::from(state.damage), case["damage"].as_u64().unwrap());
    }
}

#[test]
fn jagged_uses_stat_multiplier_not_maximum_durability() {
    let c = Catalog::embedded().unwrap();
    let mut state = ToolState::pickaxe(["flint", "wood", "wood"]);
    state.damage_points(&c, 25).unwrap();
    let e = c.evaluate(&state).unwrap();
    close(e.mining_speed(&c, &state, true).unwrap(), 3.325);
    close(e.mining_speed(&c, &state, false).unwrap(), 3.5);
    close(e.melee_damage(&c, &state).unwrap(), 2.30625);
}

#[test]
fn stale_evaluation_is_rejected_when_materials_change() {
    let c = Catalog::embedded().unwrap();
    let e = c.evaluate(&ToolState::pickaxe(["wood"; 3])).unwrap();
    assert!(
        e.mining_speed(&c, &ToolState::pickaxe(["flint"; 3]), true)
            .is_err()
    );
}

#[test]
fn serialized_state_rejects_unknown_schema_fields_and_invalid_damage() {
    let c = Catalog::embedded().unwrap();
    let state = ToolState::pickaxe(["wood"; 3]);
    for (key, value) in [
        ("schema", json!(2)),
        ("damage", json!(61)),
        ("broken", json!(true)),
        ("extra", json!(1)),
    ] {
        let mut changed = serde_json::to_value(&state).unwrap();
        changed[key] = value;
        assert!(ToolState::from_json(&c, &changed.to_string()).is_err());
    }
    assert!(c.evaluate(&ToolState::pickaxe(["stone"; 3])).is_err());
}

#[test]
fn accumulator_preserves_default_cancellation_and_double_narrowing() {
    let mut stat = Accumulator::new(9.0);
    stat.add(0.1);
    stat.update(10.0);
    stat.percent(-0.15);
    stat.multiply(1.1);
    let expected = ((0.0 + 0.1_f32 + 10.0) * (1.0 + (-0.15_f64 as f32))) * (1.1_f64 as f32);
    assert_eq!(stat.build().to_bits(), expected.to_bits());
}

#[test]
fn postfix_is_bounded_and_diagnoses_invalid_programs() {
    assert_eq!(
        formula(&json!([3, 0, "/"]), |_| unreachable!()).unwrap(),
        0.0
    );
    assert_eq!(
        formula(&json!([9, "sqrt"]), |_| unreachable!()).unwrap(),
        3.0
    );
    for code in [
        json!([]),
        json!(["+"]),
        json!([1, 2]),
        json!([-1, "sqrt"]),
        json!([2, "exec"]),
        json!(["$missing"]),
        json!(vec![1; 65]),
    ] {
        assert!(
            formula(&code, |_| Err("unknown variable".into())).is_err(),
            "{code}"
        );
    }
}

#[test]
fn diagnostic_does_not_claim_to_create_playable_items() {
    let c = Catalog::embedded().unwrap();
    assert!(
        diagnostic(&c, "stats wood wood wood")
            .unwrap()
            .contains("diagnostics only")
    );
    assert!(diagnostic(&c, "stats stone wood wood").is_err());
    assert!(diagnostic(&c, "give pickaxe").is_err());
}

// Load modified documents through the same public path as imported data.
fn modified_catalog(change: impl FnOnce(&mut Value, &mut Value)) -> Result<Catalog, String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("third_party/tconstruct/src/generated/resources/data/tconstruct/tinkering");
    let read = |path: &str| std::fs::read_to_string(root.join(path)).unwrap();
    let mut tool: Value = serde_json::from_str(&read("tool_definitions/pickaxe.json")).unwrap();
    let mut pierce: Value = serde_json::from_str(&read("modifiers/pierce.json")).unwrap();
    change(&mut tool, &mut pierce);
    Catalog::from_documents(
        &[
            (
                "tconstruct:wood",
                &read("materials/definition/wood.json"),
                &read("materials/stats/wood.json"),
                &read("materials/traits/wood.json"),
            ),
            (
                "tconstruct:flint",
                &read("materials/definition/flint.json"),
                &read("materials/stats/flint.json"),
                &read("materials/traits/flint.json"),
            ),
        ],
        &tool.to_string(),
        &[
            ("tconstruct:pierce", &pierce.to_string()),
            ("tconstruct:cultivated", &read("modifiers/cultivated.json")),
            ("tconstruct:jagged", &read("modifiers/jagged.json")),
        ],
    )
}

#[test]
fn loader_rejects_unknown_modules_and_modified_deferred_configuration() {
    assert!(
        modified_catalog(|tool, _| tool["modules"][0]["type"] = json!("tconstruct:new_api"))
            .unwrap_err()
            .contains("unsupported module")
    );
    assert!(
        modified_catalog(|_, m| m["modules"][0]["condition"] = json!(true))
            .unwrap_err()
            .contains("unsupported field")
    );
    assert!(
        modified_catalog(|tool, _| tool["modules"][7]["expansion_direction"] = json!("unknown"))
            .unwrap_err()
            .contains("unsupported configuration")
    );
    assert!(modified_catalog(|_, m| m["modules"][1]["effect"] = json!("unknown")).is_err());
}
