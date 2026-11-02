fn component_damage_bar(
    game: &crate::game::Game<'_>,
    stack: &ItemStack,
    bar: &mut Option<(i32, u32)>,
) {
    let data = stack.data();
    let max = match data.and_then(|d| d.other("minecraft:max_damage")) {
        Some(horizon_world::nbt::Tag::Int(max)) => Some(*max),
        _ => None,
    };
    let unbreakable = data.is_some_and(|d| d.other("minecraft:unbreakable").is_some());
    let bundle = horizon_client::bundle::Contents::of(game.items(), stack).is_some();
    if let Some((damage, max)) =
        crate::game::pluto_component_bar::bar_input(max, stack.damage, unbreakable, bundle)
    {
        *bar = horizon_client::hud_overlays::damage_bar(damage, max);
    }
}
