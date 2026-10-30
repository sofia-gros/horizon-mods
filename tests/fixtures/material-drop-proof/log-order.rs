fn inspect_drop_order(pos: [i32; 3], stack: &horizon_client::items::ItemStack) {
    if pos == [0, 71, 2] && (&*stack.item == "minecraft:diamond" || &*stack.item == "tconstruct:fake_storage_block") {
        eprintln!("TCONSTRUCT_FIXTURE_DROP_ORDER {}", stack.item);
    }
}
