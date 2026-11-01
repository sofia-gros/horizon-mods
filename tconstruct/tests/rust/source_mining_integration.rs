use horizon_items::items::{ItemStack, Items};
use horizon_world::nbt::Tag;
use std::collections::{BTreeMap, HashSet};
#[path = "../../mixins/tconstruct/source-mining-math.rs"] mod tconstruct_source_mining_math;
#[path = "../../mixins/tconstruct/source-station-stats.rs"] mod tconstruct_source_station_stats;
#[path = "../../mixins/tconstruct/source-mining-stack.rs"] mod tconstruct_source_mining_stack;
#[path = "../../mixins/tconstruct/source-mining-rules.rs"] mod tconstruct_source_mining_rules;
include!("../../mixins/tconstruct/source-mining-return.rs");

#[test]
fn per_stack_rules_respect_effective_blocks_broken_and_material_swap() {
    let definition = br#"{"modules":[{"type":"tconstruct:part_stats","parts":["tconstruct:pick_head","tconstruct:tool_handle"]}]}"#;
    let head = serde_json::json!({"tconstruct:head":{"durability":100,"mining_speed":3.5}}).as_object().unwrap().clone();
    let handle = serde_json::json!({"tconstruct:handle":{"durability":0,"mining_speed":0}}).as_object().unwrap().clone();
    let stats = tconstruct_source_station_stats::base_stats(definition, &[head, handle]).unwrap();
    let mut stack = ItemStack::new("tconstruct:pickaxe", 1);
    stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_max_damage", Tag::Int(100))])));
    stack.data_mut().set_other("minecraft:tool", Some(Tag::compound([("rules", Tag::List(vec![
        Tag::compound([("blocks", Tag::String("#minecraft:mineable/pickaxe".into())), ("speed", Tag::Float(3.5f32.to_bits()))])
    ]))])));
    let mut items = Items::tags_only(std::path::Path::new("/tmp")).unwrap();
    items.block_tags.insert("minecraft:mineable/pickaxe".into(), HashSet::from(["minecraft:stone".into()]));
    let levels = BTreeMap::from([("tconstruct:lightweight".to_string(), 1), ("tconstruct:jagged".to_string(), 1)]);
    let order = vec!["tconstruct:lightweight".to_string(), "tconstruct:jagged".to_string()];
    tconstruct_source_mining_stack::apply(&mut stack, &stats, &levels, &order, true).unwrap();
    assert!((tconstruct_material_mining_speed(1.0, Some(&stack), "minecraft:stone", &items) - 3.78).abs() < 0.0001);
    assert_eq!(tconstruct_material_mining_speed(1.0, Some(&stack), "minecraft:dirt", &items), 1.0);
    stack.damage = 25;
    let base = tconstruct_material_mining_speed(1.0, Some(&stack), "minecraft:stone", &items);
    assert!((base - 3.78).abs() < 0.0001);
    let custom = stack.data().unwrap().other("minecraft:custom_data").unwrap();
    // Efficiency is added before the source percentage modifier; Haste scales both.
    assert!((tconstruct_source_mining_rules::conditional(custom, 25, (base + 1.0) * 1.2, 1.2).unwrap() - 5.4492).abs() < 0.0001);
    stack.damage = 100;
    assert_eq!(tconstruct_material_mining_speed(0.3, Some(&stack), "minecraft:stone", &items), 0.3);
    stack.damage = 25;
    tconstruct_source_mining_stack::apply(&mut stack, &stats, &BTreeMap::new(), &[], true).unwrap();
    assert!((tconstruct_material_mining_speed(1.0, Some(&stack), "minecraft:stone", &items) - 3.5).abs() < 0.0001);
}
