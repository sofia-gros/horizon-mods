// Apply source conditional break-speed modules after efficiency and potions.
fn tconstruct_material_mining_context(speed: f32, game: &crate::game::Game<'_>, facts: &&horizon_items::blocks::StateFacts, held: &Option<&ItemStack>) -> f32 {
    let speed = game.armor[1].as_ref().and_then(|stack| {
        let custom = stack.data()?.other("minecraft:custom_data")?;
        Some(speed * crate::game::tconstruct_source_mining_rules::haste_armor_multiplier(custom, stack.damage))
    }).unwrap_or(speed);
    let Some(stack) = held.filter(|stack| stack.item.starts_with("tconstruct:")) else { return speed; };
    let Some(data) = stack.data() else { return speed; };
    let Some(custom) = data.other("minecraft:custom_data") else { return speed; };
    let Some(tool) = stack.data().and_then(|data| data.other("minecraft:tool")) else { return speed; };
    if crate::game::tconstruct_source_mining_rules::effective_speed(tool, &facts.name, |tag, block| game.res.items().block_in_tag(tag, block)).is_none() { return speed; }
    crate::game::tconstruct_source_mining_rules::conditional(custom, stack.damage, speed, game.potions.effects.mining_speed_factor()).unwrap_or(speed)
}
