//! Run the demo transaction against actual Horizon stacks without a game.
extern crate self as horizon_client;
pub mod items {
    pub use horizon_items::items::*;
}
pub mod pluto_inventory {
    pub use pluto_inventory_api::*;
}
#[path = "../../examples/inventory/plan_insert.rs"]
mod plan_insert;
#[path = "../../examples/inventory/mixins/pluto_inventory_demo/shared.rs"]
mod shared;

#[test]
fn real_stack_components_survive_the_consumer_transaction() {
    use horizon_items::items::ItemStack;
    use horizon_world::nbt::Tag;
    fn stack(count: u32, marker: i32) -> ItemStack {
        let mut stack = ItemStack::new("minecraft:stone", count);
        stack.data_mut().set_other(
            "minecraft:custom_data",
            Some(Tag::compound([("example:marker", Tag::Int(marker))])),
        );
        stack
    }
    let source = stack(10, 1);
    let same = stack(6, 1);
    let different = stack(1, 2);
    let mut slots = vec![
        None,
        Some(source.clone()),
        Some(same),
        None,
        Some(different.clone()),
        None,
    ];
    assert!(shared::run(&mut slots));
    assert!(slots[1].is_none());
    assert_eq!(slots[2].as_ref().unwrap().count, 8);
    assert_eq!(slots[3].as_ref().unwrap().count, 8);
    assert!(slots[2]
        .as_ref()
        .unwrap()
        .same_item_same_components(&source));
    assert!(slots[3]
        .as_ref()
        .unwrap()
        .same_item_same_components(&source));
    assert!(slots[4]
        .as_ref()
        .unwrap()
        .same_item_same_components(&different));
    assert!(slots[5].is_none());
}

#[test]
fn incoming_stack_adapter_preserves_consumer_policy_and_input() {
    use horizon_items::items::ItemStack;
    let incoming = ItemStack::new("example:part", 11);
    let slots = vec![None, Some(ItemStack::new("example:part", 6))];
    let plan = plan_insert::plan_insert(&slots, &incoming, 1, 8, |index, _| index == 1)
        .unwrap()
        .unwrap();
    assert_eq!((plan.from, plan.to, plan.count), (2, 1, 2));
    assert_eq!(incoming.count, 11);
    assert_eq!(slots[1].as_ref().unwrap().count, 6);
    assert!(
        plan_insert::plan_insert(&slots, &incoming, 0, 8, |_, _| false)
            .unwrap()
            .is_none()
    );
    assert!(plan_insert::plan_insert(&slots, &incoming, 2, 8, |_, _| true).is_err());
}
