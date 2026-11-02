//! Exercise the public planner with the fixed Horizon host's actual ItemStack.
use horizon_items::items::ItemStack;
use pluto_inventory_api::{plan_move, StackSnapshot};

#[test]
fn host_stacks_use_complete_identity_without_copying_into_the_api() {
    let source = ItemStack::new("minecraft:stone", 11);
    let destination = ItemStack::new("minecraft:stone", 6);
    let mut other = ItemStack::new("minecraft:stone", 1);
    other.data_mut().set_other(
        "minecraft:custom_data",
        Some(horizon_world::nbt::Tag::compound([(
            "example:value",
            horizon_world::nbt::Tag::Int(1),
        )])),
    );
    let slots = [
        Some(StackSnapshot {
            item: &source,
            count: source.count,
        }),
        Some(StackSnapshot {
            item: &destination,
            count: destination.count,
        }),
        Some(StackSnapshot {
            item: &other,
            count: other.count,
        }),
    ];
    let plan = plan_move(
        &slots,
        0,
        1,
        11,
        |_, _| 8,
        |_, _| true,
        |a, b| a.same_item_same_components(b),
    )
    .unwrap()
    .unwrap();
    assert_eq!(plan.count, 2);
    assert_eq!(source.count, 11);
    assert_eq!(destination.count, 6);
    assert!(plan_move(
        &slots,
        0,
        2,
        11,
        |_, _| 64,
        |_, _| true,
        |a, b| a.same_item_same_components(b)
    )
    .unwrap()
    .is_none());
}
