//! In-memory integration of recipe plans and the host transaction primitive.
//! This deliberately does not claim a live game or a registered station.
use horizon_items::{
    inventory_transaction::{self, SlotChange, TransactionError},
    items::ItemStack,
};
use tconstruct_horizon::{
    Catalog,
    crafting::{Part, PartKind, plan_part, plan_pickaxe, plan_repair},
};

fn item(id: &str, count: u32) -> Option<ItemStack> {
    Some(ItemStack::new(id, count))
}
fn allowed(stack: &ItemStack) -> bool {
    matches!(
        &*stack.item,
        "example:pattern"
            | "minecraft:flint"
            | "example:pick_head"
            | "example:handle"
            | "example:binding"
            | "example:tool"
    ) && stack.count <= 64
}
fn change(slot: usize, expected: Option<ItemStack>, replacement: Option<ItemStack>) -> SlotChange {
    SlotChange {
        slot,
        expected,
        replacement,
    }
}

#[test]
fn two_players_cannot_consume_the_same_material_or_duplicate_the_part() {
    let catalog = Catalog::embedded().unwrap();
    let plan = plan_part(&catalog, PartKind::Head, "flint", 2, true).unwrap();
    let mut slots = vec![item("example:pattern", 1), item("minecraft:flint", 2), None];
    let changes = [
        change(0, slots[0].clone(), slots[0].clone()), // reusable pattern
        change(1, slots[1].clone(), None),
        change(2, None, item("example:pick_head", 1)),
    ];
    assert_eq!(plan.material_items, 2);
    let mut revision = 0;
    inventory_transaction::apply(&mut slots, &mut revision, 0, &changes, allowed).unwrap();
    let after = slots.clone();
    assert_eq!(
        inventory_transaction::apply(&mut slots, &mut revision, 0, &changes, allowed),
        Err(TransactionError::StaleRevision)
    );
    assert_eq!(slots, after);
    assert_eq!(
        (
            slots[0].as_ref().unwrap().count,
            slots[1].is_none(),
            slots[2].as_ref().unwrap().count
        ),
        (1, true, 1)
    );
}

#[test]
fn changed_part_or_output_rejects_the_whole_pickaxe_plan() {
    let catalog = Catalog::embedded().unwrap();
    let parts = [
        Part {
            kind: PartKind::Head,
            material: "tconstruct:flint".into(),
        },
        Part {
            kind: PartKind::Handle,
            material: "tconstruct:wood".into(),
        },
        Part {
            kind: PartKind::Binding,
            material: "tconstruct:wood".into(),
        },
    ];
    let planned = plan_pickaxe(&catalog, &parts).unwrap();
    assert_eq!(
        planned.materials,
        ["tconstruct:flint", "tconstruct:wood", "tconstruct:wood"]
    );
    let mut slots = vec![
        item("example:pick_head", 1),
        item("example:handle", 1),
        item("example:binding", 1),
        None,
    ];
    let changes = [
        change(0, slots[0].clone(), None),
        change(1, slots[1].clone(), None),
        change(2, slots[2].clone(), None),
        change(3, None, item("example:tool", 1)),
    ];
    slots[2] = None; // another actor took the binding after the preview
    let before = slots.clone();
    let mut revision = 0; // even without a revision bump the complete slot check catches it
    assert_eq!(
        inventory_transaction::apply(&mut slots, &mut revision, 0, &changes, allowed),
        Err(TransactionError::ChangedSlot)
    );
    assert_eq!((revision, slots), (0, before));
}

#[test]
fn repair_plan_does_not_spend_material_if_tool_state_changed() {
    let catalog = Catalog::embedded().unwrap();
    let mut tool = tconstruct_horizon::ToolState::pickaxe(["flint", "wood", "wood"]);
    tool.damage_points(&catalog, 85).unwrap();
    let plan = plan_repair(&catalog, &tool, "flint", 1).unwrap();
    assert_eq!(
        (
            plan.material_items,
            plan.repaired_points,
            plan.output.damage
        ),
        (1, 56, 29)
    );
    let mut tool_stack = ItemStack::new("example:tool", 1);
    tool_stack.data_mut().custom_name = Some("old-state".into());
    let mut slots = vec![Some(tool_stack.clone()), item("minecraft:flint", 1)];
    let mut replacement = tool_stack.clone();
    replacement.data_mut().custom_name = Some("repaired-state".into());
    let changes = [
        change(0, Some(tool_stack), Some(replacement)),
        change(1, slots[1].clone(), None),
    ];
    slots[0].as_mut().unwrap().data_mut().custom_name = Some("changed-state".into());
    let before = slots.clone();
    let mut revision = 1;
    assert_eq!(
        inventory_transaction::apply(&mut slots, &mut revision, 1, &changes, allowed),
        Err(TransactionError::ChangedSlot)
    );
    assert_eq!((revision, slots), (1, before));
}
