// Read the per-stack source mining stat for matching effective-block rules.
fn tconstruct_material_mining_speed(speed: f32, held: Option<&ItemStack>, block: &str, items: &Items) -> f32 {
    use horizon_world::nbt::Tag;
    let Some(stack) = held.filter(|stack| stack.item.starts_with("tconstruct:")) else { return speed; };
    let Some(custom) = stack.data().and_then(|data| data.other("minecraft:custom_data")) else { return speed; };
    // Do not restore material Tool speed after flag-only broken cancellation.
    if matches!(custom.get("tic_broken"), Some(Tag::Byte(value)) if *value != 0) { return 0.3; }
    let Some(Tag::Int(max)) = custom.get("tic_max_damage") else { return speed; };
    if *max <= 0 || stack.damage >= *max as u32 {
        // Ask the host for its ordinary item-definition result on a temporary
        // stack without Tool. Both return hooks then leave that result alone.
        if stack.data().and_then(|data| data.other("minecraft:tool")).is_some() {
            let mut fallback = stack.clone();
            fallback.data_mut().set_other("minecraft:tool", None);
            return items.mining_speed(Some(&fallback), block);
        }
        return speed;
    }
    if custom.get("tic_mining_traits").is_none() { return speed; }
    let Some(tool) = stack.data().and_then(|data| data.other("minecraft:tool")) else { return speed; };
    match self::tconstruct_source_mining_rules::evaluated_speed(tool, block, |tag, block| items.block_in_tag(tag, block)) {
        Some(value) => value.unwrap_or(1.0).max(1.0),
        None => speed,
    }
}
