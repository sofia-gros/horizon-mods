// Use the synchronized source predicate and material tier for real drops.
fn tconstruct_material_correct_for_drops(correct: bool, held: Option<&ItemStack>, block: &str, items: &Items) -> bool {
    use horizon_world::nbt::Tag;
    let Some(stack) = held.filter(|stack| stack.item.starts_with("tconstruct:")) else { return correct; };
    let Some(custom) = stack.data().and_then(|data| data.other("minecraft:custom_data")) else { return correct; };
    let Some(Tag::Int(max)) = custom.get("tic_max_damage") else { return correct; };
    if *max <= 0 { return correct; }
    if stack.damage >= *max as u32 { return false; }
    let Some(tool) = stack.data().and_then(|data| data.other("minecraft:tool")) else { return correct; };
    self::tconstruct_source_mining_rules::harvestable(tool, block, |tag, block| items.block_in_tag(tag, block)).unwrap_or(correct)
}
