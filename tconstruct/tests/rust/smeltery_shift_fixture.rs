use horizon_items::items::ItemStack;
// __PRODUCTION_TRANSFER__
#[test] fn quick_move_splits_one_per_empty_slot_and_keeps_remainder() {
    let mut slots=vec![Some(ItemStack::new("minecraft:iron_ore",1)),None,None];
    let rest=source_give_inventory(&mut slots,0,ItemStack::new("minecraft:iron_ore",5),1,false).unwrap();
    assert_eq!(slots.iter().map(|s|s.as_ref().unwrap().count).collect::<Vec<_>>(),vec![1,1,1]);
    assert_eq!(rest.count,3);
}
#[test] fn existing_oversized_slots_are_not_rewritten_or_merged() {
    let mut slots=vec![Some(ItemStack::new("minecraft:iron_ore",4)),None];
    let rest=source_give_inventory(&mut slots,0,ItemStack::new("minecraft:iron_ore",3),1,false).unwrap();
    assert_eq!(slots[0].as_ref().unwrap().count,4); assert_eq!(slots[1].as_ref().unwrap().count,1); assert_eq!(rest.count,2);
}
#[test] fn full_input_returns_the_whole_stack() {
    let mut slots=vec![Some(ItemStack::new("minecraft:stone",1))];
    assert_eq!(source_give_inventory(&mut slots,0,ItemStack::new("minecraft:copper_ore",8),1,false).unwrap().count,8);
}
