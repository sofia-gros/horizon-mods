fn tconstruct_overslime_bar(value: (), game: &crate::game::Game<'_>, batch: &mut horizon_client::hud::HudBatch, stack: &horizon_client::items::ItemStack, x: f32, y: f32) -> () {
    let amount = crate::game::tconstruct_source_overslime::amount(stack,"tic_overslime") as u32;
    let cap = crate::game::tconstruct_source_overslime::amount(stack,"tic_overslime_capacity") as u32;
    let active = matches!(stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|d| d.get("tic_overslime_active")), Some(horizon_world::nbt::Tag::Byte(v)) if *v != 0);
    if !active || stack.count != 1 || cap == 0 || (amount == cap && stack.damage == 0) { return value; }
    if crate::game::tconstruct_source_ranged_wear::is_broken(stack) { return value; }
    let max = crate::game::tconstruct_source_overslime::amount(stack,"tic_max_damage") as u32;
    let remaining = if amount > 0 { amount } else { max.saturating_sub(stack.damage) };
    let denominator = if amount > 0 { cap } else { max };
    if denominator == 0 { return value; }
    let width = if remaining == 0 { 0 } else if remaining >= denominator { 13 } else { 1 + (13 * (remaining - 1) / denominator) };
    let color = if amount > 0 { horizon_client::hud::rgba(0,160,255,255) }
        else { horizon_client::hud_overlays::damage_bar(stack.damage as i32, max as i32).map(|(_,c)| c | 0xff000000).unwrap_or(horizon_client::hud::rgba(0,255,0,255)) };
    batch.fill(&game.gui.atlas, x+2.0, y+13.0, 13.0, 2.0, horizon_client::hud::rgba(0,0,0,255));
    batch.fill(&game.gui.atlas, x+2.0, y+13.0, width as f32, 1.0, color);
    value
}
