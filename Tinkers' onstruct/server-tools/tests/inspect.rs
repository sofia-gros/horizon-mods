use serde_json::Value;
use tconstruct_horizon::{Catalog, ToolState};
use tconstruct_server_tools::inspect;

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
