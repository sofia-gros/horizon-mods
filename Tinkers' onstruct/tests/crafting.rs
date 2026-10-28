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
