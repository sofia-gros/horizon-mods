use tconstruct_horizon::{Catalog, ToolState, specimen};

#[test]
fn specimen_keeps_core_state_separate_from_vanilla_damage() {
    let catalog = Catalog::embedded().unwrap();
    let command = specimen::command(&catalog, "specimen flint wood wood 85").unwrap();
    let quoted = command
        .split_once("state_json:")
        .unwrap()
        .1
        .strip_suffix("}}] 1")
        .unwrap();
    let source = quoted
        .strip_prefix('\'')
        .unwrap()
        .strip_suffix('\'')
        .unwrap();
    let state = ToolState::from_json(&catalog, source).unwrap();
    assert_eq!(
        state.materials,
        ["tconstruct:flint", "tconstruct:wood", "tconstruct:wood"]
    );
    assert_eq!(state.damage, 85);
    assert!(state.broken);
    assert!(command.starts_with("give @s wooden_pickaxe["));
    assert!(command.chars().count() <= 256, "host command limit");
    assert!(!command.contains("minecraft:damage="));
    assert!(specimen::FEEDBACK.contains("Gameplay remains vanilla"));
}

#[test]
fn specimen_rejects_unsupported_materials_and_command_fragments() {
    let catalog = Catalog::embedded().unwrap();
    for args in [
        "specimen stone wood wood",
        "specimen wood wood wood -1",
        "specimen wood wood wood 4294967296",
        "specimen wood wood wood 1 extra",
        "specimen wood\" wood wood",
    ] {
        assert!(specimen::command(&catalog, args).is_err(), "{args}");
    }
}
