// Apply source conditional break-speed modules after efficiency and potions.
fn tconstruct_material_mining_context(speed: f32, game: &crate::game::Game<'_>, facts: &&horizon_items::blocks::StateFacts, held: &Option<&ItemStack>) -> f32 {
    let Some(stack) = held.filter(|stack| stack.item.starts_with("tconstruct:")) else { return speed; };
    let Some(data) = stack.data() else { return speed; };
    let Some(custom) = data.other("minecraft:custom_data") else { return speed; };
    let Some(tool) = custom.get("tic_mining_tool") else { return speed; };
    if crate::game::tconstruct_source_mining_rules::effective_speed(tool, &facts.name, |tag, block| game.res.items().block_in_tag(tag, block)).is_none() { return speed; }
    crate::game::tconstruct_source_mining_rules::conditional(custom, stack.damage, speed, game.potions.effects.mining_speed_factor()).unwrap_or(speed)
}
