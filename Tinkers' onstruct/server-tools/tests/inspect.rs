use serde_json::Value;
use tconstruct_horizon::{Catalog, ToolState};
use tconstruct_server_tools::{edit_state, inspect, melee_projection, mining_projection};

#[test]
fn flint_repair_consumes_only_needed_material_and_rejects_wrong_inputs() {
    use tconstruct_server_tools::repair_flint;
    let catalog = Catalog::embedded().unwrap();
    let mut tool = ToolState::pickaxe(["flint", "wood", "wood"]);
    tool.damage_points(&catalog, 85).unwrap();
    let source = tool.to_json().unwrap();
    let (result, used) = repair_flint(
        &catalog,
        "minecraft:wooden_pickaxe",
        1,
        &source,
        "minecraft:flint",
        3,
    )
    .unwrap();
    assert_eq!(used, 2);
    assert_eq!(ToolState::from_json(&catalog, &result).unwrap().damage, 0);
    assert!(
        repair_flint(
            &catalog,
            "minecraft:wooden_pickaxe",
            1,
            &source,
            "minecraft:stone",
            3
        )
        .is_err()
    );
    assert!(
        repair_flint(
            &catalog,
            "minecraft:wooden_pickaxe",
            1,
            &source,
            "minecraft:flint",
            0
        )
        .is_err()
    );
    assert!(
        repair_flint(
            &catalog,
            "minecraft:wooden_pickaxe",
            1,
            &result,
            "minecraft:flint",
            3
        )
        .is_err()
    );
}

#[test]
fn mining_projection_matches_original_java_speed_bits() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/mining-speed-oracle.json")).unwrap();
    let catalog = Catalog::embedded().unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let mut state = ToolState::pickaxe([case["head"].as_str().unwrap(), "wood", "wood"]);
        state
            .damage_points(&catalog, case["damage"].as_u64().unwrap() as u32)
            .unwrap();
        // This Java fixture uses identity modifiers; compare the base hook.
        let speed = if state.broken {
            0.3_f32
        } else if case["effective"].as_bool().unwrap() {
            catalog
                .evaluate(&state)
                .unwrap()
                .stats
                .mining_speed
                .max(1.0)
        } else {
            1.0
        };
        assert_eq!(
            u64::from(speed.to_bits()),
            case["expected_bits"].as_u64().unwrap(),
            "{case}"
        );
    }
}

#[test]
fn projection_uses_the_selected_materials_and_broken_state() {
    let catalog = Catalog::embedded().unwrap();
    for (head, damage, speed, tag, broken) in [
        ("wood", 0, 2.0, "minecraft:incorrect_for_wooden_tool", false),
        (
            "flint",
            25,
            3.325,
            "minecraft:incorrect_for_stone_tool",
            false,
        ),
        ("flint", 85, 0.3, "minecraft:incorrect_for_stone_tool", true),
    ] {
        let mut state = ToolState::pickaxe([head, "wood", "wood"]);
        state.damage_points(&catalog, damage).unwrap();
        let profile = mining_projection(
            &catalog,
            "minecraft:wooden_pickaxe",
            1,
            &state.to_json().unwrap(),
        )
        .unwrap();
        assert_eq!(
            (profile.speed, profile.incorrect_tag, profile.broken),
            (speed, tag, broken)
        );
    }
}

#[test]
fn test_edits_keep_materials_and_roundtrip_broken_and_repair_boundaries() {
    let catalog = Catalog::embedded().unwrap();
    let original = ToolState::pickaxe(["flint", "wood", "wood"]);
    let broken = edit_state(
        &catalog,
        "minecraft:wooden_pickaxe",
        1,
        &original.to_json().unwrap(),
        "test-damage",
        u32::MAX,
    )
    .unwrap();
    let state = ToolState::from_json(&catalog, &broken).unwrap();
    assert_eq!(state.materials, original.materials);
    assert_eq!((state.damage, state.broken), (85, true));
    let repaired = edit_state(
        &catalog,
        "minecraft:wooden_pickaxe",
        1,
        &broken,
        "test-repair",
        20,
    )
    .unwrap();
    let state = ToolState::from_json(&catalog, &repaired).unwrap();
    assert_eq!((state.damage, state.broken), (65, false));
    let restored = edit_state(
        &catalog,
        "minecraft:wooden_pickaxe",
        1,
        &repaired,
        "test-repair",
        u32::MAX,
    )
    .unwrap();
    assert_eq!(ToolState::from_json(&catalog, &restored).unwrap(), original);
    assert!(edit_state(&catalog, "minecraft:diamond", 1, &broken, "test-repair", 1).is_err());
    assert!(
        edit_state(
            &catalog,
            "minecraft:wooden_pickaxe",
            1,
            "{}",
            "test-repair",
            1
        )
        .is_err()
    );
}

#[test]
fn server_inspection_keeps_each_specimens_state_and_vanilla_damage_distinct() {
    let catalog = Catalog::embedded().unwrap();
    for (head, damage, durability, broken) in [
        ("wood", 0, 60, false),
        ("flint", 17, 85, false),
        ("flint", 85, 85, true),
    ] {
        let mut state = ToolState::pickaxe([head, "wood", "wood"]);
        state.damage_points(&catalog, damage).unwrap();
        let reply = inspect(
            &catalog,
            "minecraft:wooden_pickaxe",
            1,
            &state.to_json().unwrap(),
            3,
            &"a".repeat(32),
        )
        .unwrap();
        assert!(reply.len() <= 256);
        let value: Value = serde_json::from_str(&reply).unwrap();
        assert_eq!(value["parts"], serde_json::json!([head, "wood", "wood"]));
        assert_eq!(value["damage"], damage);
        assert_eq!(value["broken"], broken);
        assert_eq!(value["durability"], durability);
        assert_eq!(
            value["mining"].as_f64(),
            Some(if head == "wood" { 2.0 } else { 3.5 })
        );
        assert_eq!(
            value["attack"].as_f64(),
            Some(if head == "wood" { 1.0 } else { 2.25 })
        );
        assert_eq!(value["speed"].as_f64(), Some(1.2));
        assert_eq!(value["vanilla_damage"], 3);
        assert_eq!(value["hooks"], false);
    }
}

#[test]
fn server_inspection_rejects_invalid_carriers_and_states() {
    let catalog = Catalog::embedded().unwrap();
    let state = ToolState::pickaxe(["wood", "wood", "wood"])
        .to_json()
        .unwrap();
    assert!(inspect(&catalog, "minecraft:diamond", 1, &state, 0, "test").is_err());
    assert!(inspect(&catalog, "minecraft:wooden_pickaxe", 2, &state, 0, "test").is_err());
    assert!(inspect(&catalog, "minecraft:wooden_pickaxe", 1, "{}", 0, "test").is_err());
    assert!(
        inspect(
            &catalog,
            "minecraft:wooden_pickaxe",
            1,
            &state,
            0,
            "bad request"
        )
        .is_err()
    );
}

#[test]
fn melee_projection_tracks_material_jagged_and_breakage() {
    let catalog = Catalog::embedded().unwrap();
    let mut state = ToolState::pickaxe(["flint", "wood", "wood"]);
    let project = |state: &ToolState| {
        melee_projection(
            &catalog,
            "minecraft:wooden_pickaxe",
            1,
            &state.to_json().unwrap(),
        )
        .unwrap()
    };
    let fresh = project(&state);
    assert_eq!(
        (fresh.damage_bonus, fresh.attack_speed, fresh.enabled),
        (2.25, 1.2, true)
    );
    assert!((fresh.damage_multiplier - 1.0).abs() < 0.000001);
    state.damage_points(&catalog, 25).unwrap();
    assert!((project(&state).damage_multiplier - 1.025).abs() < 0.000001);
    state.damage_points(&catalog, 60).unwrap();
    assert!(!project(&state).enabled);
    state.repair_points(&catalog, 85).unwrap();
    assert_eq!(project(&state), fresh);
}
