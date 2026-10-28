fn tconstruct_part_name(
    mut value: Vec<(horizon_client::text::Text, u32)>,
    game: &crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
) -> Vec<(horizon_client::text::Text, u32)> {
    if stack.custom_name().is_some()
        || !horizon_client::item_tooltip::TooltipDisplay::for_stack(stack).name_visible(game.screen == Screen::Creative) {
        return value;
    }
    // Preserve any explicit 26.3 item-name component, including old prototype outputs.
    if stack.item_name().is_some() { return value; }
    let Some(item) = stack.item.strip_prefix("tconstruct:") else { return value; };
    if !["pick_head", "hammer_head", "small_axe_head", "broad_axe_head", "small_blade", "broad_blade", "adze_head", "large_plate", "bow_limb", "bow_grip", "bowstring", "arrow_head", "arrow_shaft", "fletching", "tool_binding", "tough_binding", "tool_handle", "tough_handle", "helmet_plating", "chestplate_plating", "leggings_plating", "boots_plating", "maille", "shield_core", "ribcage", "shell", "laces", "repair_kit", "fake_ingot"].contains(&item) { return value; }
    let material = stack.data().and_then(|data| data.other("minecraft:custom_data")).and_then(|data| data.get("Material")).and_then(|value| match value {
        horizon_world::nbt::Tag::String(id) => Some(id.to_string_lossy()),
        _ => None,
    }).unwrap_or_else(|| "tconstruct:unknown".into());
    let format = |template: &str, args: &[String]| {
        let args: Vec<_> = args.iter().map(|arg| horizon_client::text::Text::literal(arg.clone())).collect();
        horizon_client::text::format_translation(template, &args).plain()
    };
    let translate = |key: &str| game.pack.names.display_get(key);
    if let Some(name) = crate::game::tconstruct_source_part_name::source_part_name(&stack.item, &material, &translate, &format) {
        if let Some((text, _)) = value.first_mut() { *text = horizon_client::text::Text::legacy(&name); }
    }
    value
}
