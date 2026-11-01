// Read the per-stack source mining stat for matching effective-block rules.
fn tconstruct_material_mining_speed(speed: f32, held: Option<&ItemStack>, block: &str, items: &Items) -> f32 {
    use horizon_world::nbt::Tag;
    let Some(stack) = held.filter(|stack| stack.item.starts_with("tconstruct:")) else { return speed; };
    let Some(custom) = stack.data().and_then(|data| data.other("minecraft:custom_data")) else { return speed; };
    let Some(Tag::Int(max)) = custom.get("tic_max_damage") else { return speed; };
    if *max <= 0 || stack.damage >= *max as u32 || custom.get("tic_mining_traits").is_none() { return speed; }
    let Some(tool) = custom.get("tic_mining_tool") else { return speed; };
    self::tconstruct_source_mining_rules::effective_speed(tool, block, |tag, block| items.block_in_tag(tag, block)).unwrap_or(speed)
}
