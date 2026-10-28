use tconstruct_horizon::{
    Catalog, ToolState,
    crafting::{PartKind, plan_part, plan_pickaxe},
};

#[test]
fn pinned_part_costs_and_order_make_a_pickaxe() {
    let catalog = Catalog::embedded().unwrap();
    let head = plan_part(&catalog, PartKind::Head, "flint", 2, true).unwrap();
    let handle = plan_part(&catalog, PartKind::Handle, "wood", 1, true).unwrap();
    let binding = plan_part(&catalog, PartKind::Binding, "tconstruct:wood", 1, true).unwrap();
    assert_eq!(
        (
            head.material_items,
            handle.material_items,
            binding.material_items
        ),
        (2, 1, 1)
    );
    let parts = [head.output, handle.output, binding.output];
    assert_eq!(
        plan_pickaxe(&catalog, &parts).unwrap(),
        ToolState::pickaxe(["flint", "wood", "wood"])
    );
    let mut wrong = parts.clone();
    wrong.swap(0, 1);
    assert!(plan_pickaxe(&catalog, &wrong).is_err());
}

#[test]
fn insufficient_or_unknown_inputs_are_rejected_without_mutation() {
    let catalog = Catalog::embedded().unwrap();
    assert!(plan_part(&catalog, PartKind::Head, "wood", 1, true).is_err());
    assert!(plan_part(&catalog, PartKind::Handle, "wood", 1, false).is_err());
    assert!(plan_part(&catalog, PartKind::Binding, "stone", 64, true).is_err());
}

#[test]
fn station_repair_checks_all_slots_and_consumes_in_order() {
    use tconstruct_horizon::crafting::plan_repair_slots;
    let catalog = Catalog::embedded().unwrap();
    let mut tool = ToolState::pickaxe(["flint", "wood", "wood"]);
    tool.damage_points(&catalog, 85).unwrap();
    let split = plan_repair_slots(
        &catalog,
        &tool,
        [Some(("flint", 1)), None, Some(("tconstruct:flint", 3))],
    )
    .unwrap();
    assert_eq!(split.consumed, [1, 0, 1]);
    assert_eq!(split.output.damage, 0);
    let partial = plan_repair_slots(&catalog, &tool, [None, Some(("flint", 1)), None]).unwrap();
    assert_eq!(partial.consumed, [0, 1, 0]);
    assert_eq!(partial.output.damage, 29);
    let excess = plan_repair_slots(
        &catalog,
        &tool,
        [Some(("flint", 3)), Some(("flint", 2)), None],
    )
    .unwrap();
    assert_eq!(excess.consumed, [2, 0, 0]);
    assert!(
        plan_repair_slots(
            &catalog,
            &tool,
            [Some(("flint", 3)), None, Some(("wood", 1))]
        )
        .is_err()
    );
    assert!(
        plan_repair_slots(
            &catalog,
            &tool,
            [Some(("flint", 3)), None, Some(("stone", 1))]
        )
        .is_err()
    );
    assert!(plan_repair_slots(&catalog, &tool, [None, None, None]).is_err());
    assert_eq!(tool.damage, 85);
    assert!(plan_repair_slots(&catalog, &split.output, [Some(("flint", 1)), None, None]).is_err());
    let mut wood = ToolState::pickaxe(["wood", "wood", "wood"]);
    wood.damage_points(&catalog, 60).unwrap();
    let wood_plan = plan_repair_slots(
        &catalog,
        &wood,
        [None, Some(("wood", 1)), Some(("tconstruct:wood", 2))],
    )
    .unwrap();
    assert_eq!(wood_plan.consumed, [0, 1, 1]);
    assert_eq!(wood_plan.output.damage, 0);
}

#[test]
fn station_repair_truncates_each_stack_before_the_next() {
    use tconstruct_horizon::crafting::plan_repair_slots;
    let catalog = Catalog::embedded().unwrap();
    let mut tool = ToolState::pickaxe(["flint", "wood", "flint"]);
    tool.damage_points(&catalog, 85).unwrap();
    let one_stack = plan_repair_slots(&catalog, &tool, [Some(("flint", 2)), None, None]).unwrap();
    let split = plan_repair_slots(
        &catalog,
        &tool,
        [Some(("flint", 1)), Some(("flint", 1)), None],
    )
    .unwrap();
    assert_eq!(
        (one_stack.repaired_points, one_stack.output.damage),
        (85, 0)
    );
    assert_eq!((split.repaired_points, split.output.damage), (84, 1));
    let third = plan_repair_slots(
        &catalog,
        &tool,
        [Some(("flint", 1)), Some(("flint", 1)), Some(("flint", 1))],
    )
    .unwrap();
    assert_eq!(third.consumed, [1, 1, 1]);
    assert_eq!(third.output.damage, 0);
}

#[test]
fn replacement_rebuilds_stats_and_preserves_or_clamps_damage() {
    use tconstruct_horizon::crafting::{Part, plan_part_replacement};
    let catalog = Catalog::embedded().unwrap();
    let mut tool = ToolState::pickaxe(["flint", "wood", "wood"]);
    tool.damage_points(&catalog, 70).unwrap();
    let replacement = Part {
        kind: PartKind::Head,
        material: "tconstruct:wood".into(),
    };
    let result = plan_part_replacement(&catalog, &tool, &replacement).unwrap();
    let maximum = catalog
        .evaluate(&result)
        .unwrap()
        .stats
        .durability()
        .unwrap();
    assert_eq!(result.damage, 70.min(maximum));
    assert_eq!(result.broken, 70 >= maximum);
    assert_eq!(
        result.materials,
        ["tconstruct:wood", "tconstruct:wood", "tconstruct:wood"]
    );
    assert_eq!(tool.materials[0], "tconstruct:flint");
    assert_eq!(tool.damage, 70);
    let unknown = Part {
        kind: PartKind::Binding,
        material: "tconstruct:unknown".into(),
    };
    assert!(plan_part_replacement(&catalog, &tool, &unknown).is_err());
}

#[test]
fn broken_tool_increased_maximum_is_rejected_without_losing_broken_state() {
    use tconstruct_horizon::crafting::{Part, plan_part_replacement};
    let catalog = Catalog::embedded().unwrap();
    let mut tool = ToolState::pickaxe(["wood", "wood", "wood"]);
    tool.damage_points(&catalog, u32::MAX).unwrap();
    let part = Part {
        kind: PartKind::Head,
        material: "tconstruct:flint".into(),
    };
    assert!(plan_part_replacement(&catalog, &tool, &part).is_err());
    assert!(tool.broken);
}

#[test]
fn part_costs_match_pinned_java_method() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/part-cost-oracle.json")).unwrap();
    let catalog = Catalog::embedded().unwrap();
    for (case, kind) in fixture["cases"].as_array().unwrap().iter().zip([
        PartKind::Head,
        PartKind::Handle,
        PartKind::Binding,
    ]) {
        for material in ["wood", "flint"] {
            let plan = plan_part(&catalog, kind, material, 64, true).unwrap();
            assert_eq!(
                u64::from(plan.material_items),
                case["items"].as_u64().unwrap()
            );
        }
    }
}

#[test]
fn material_repair_plans_head_material_and_clamps_at_full() {
    use tconstruct_horizon::crafting::plan_repair;
    let catalog = Catalog::embedded().unwrap();
    let mut tool = ToolState::pickaxe(["flint", "wood", "wood"]);
    tool.damage_points(&catalog, 85).unwrap();
    let one = plan_repair(&catalog, &tool, "flint", 1).unwrap();
    // Flint head durability 85 / 3, multiplied by cultivated factor 2.
    assert_eq!(
        (
            one.material_items,
            one.repaired_points,
            one.output.damage,
            one.output.broken
        ),
        (1, 56, 29, false)
    );
    let two = plan_repair(&catalog, &tool, "flint", 64).unwrap();
    assert_eq!(
        (two.material_items, two.repaired_points, two.output.damage),
        (2, 85, 0)
    );
    assert!(plan_repair(&catalog, &tool, "wood", 1).is_err());
    assert!(plan_repair(&catalog, &tool, "flint", 0).is_err());
    assert!(plan_repair(&catalog, &two.output, "flint", 1).is_err());
    assert_eq!((tool.damage, tool.broken), (85, true));
}

#[test]
fn material_repair_matches_pinned_java_arithmetic() {
    use tconstruct_horizon::crafting::plan_repair;
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/repair-oracle.json")).unwrap();
    let catalog = Catalog::embedded().unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let head = case["head"].as_str().unwrap();
        let mut tool = ToolState::pickaxe([head, "wood", "wood"]);
        tool.damage_points(&catalog, case["damage"].as_u64().unwrap() as u32)
            .unwrap();
        let plan = plan_repair(
            &catalog,
            &tool,
            head,
            case["available"].as_u64().unwrap() as u32,
        )
        .unwrap();
        assert_eq!(
            u64::from(plan.material_items),
            case["items"].as_u64().unwrap(),
            "{case}"
        );
        assert_eq!(
            u64::from(plan.repaired_points),
            case["repaired"].as_u64().unwrap(),
            "{case}"
        );
    }
}
