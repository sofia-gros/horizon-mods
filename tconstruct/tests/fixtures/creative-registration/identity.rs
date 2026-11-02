//! Compiled by the focused runner with the selected Horizon `horizon-items` crate.
use horizon_items::item_data::{canon_stack, parse_stack};
use horizon_items::items::ItemStack;

fn stack(raw: &str) -> ItemStack { parse_stack(raw).unwrap().unwrap() }
fn key(mut value: ItemStack) -> String { value.count = 1; canon_stack(Some(&value)) }

#[test]
fn representative_identity_matches_host_component_equality() {
    let mut samples = vec![
        stack(r#"{id:"minecraft:stick",count:1}"#),
        stack(r#"{count:7,id:"minecraft:stick"}"#),
        stack(r#"{id:"minecraft:stick",count:1,components:{"minecraft:custom_data":{level:1}}}"#),
        stack(r#"{id:"minecraft:stick",count:4,components:{"minecraft:custom_data":{level:2}}}"#),
        stack(r#"{id:"minecraft:stone",count:1}"#),
        stack(include_str!("finalized-stack.snbt")),
    ];
    let mut other_count = samples.last().unwrap().clone();
    other_count.count = 17;
    samples.push(other_count);
    for a in &samples {
        for b in &samples {
            assert_eq!(key(a.clone()) == key(b.clone()), a.same_item_same_components(b));
        }
    }
}
