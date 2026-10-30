fn tconstruct_part_name(
    mut value: Vec<(horizon_client::text::Text, u32)>,
    game: &crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
) -> Vec<(horizon_client::text::Text, u32)> {
    // Remote cost data is only available while the source menu is open; do
    // not display an unsynchronized value in other remote contexts.
    if let Some(index) = crate::game::tconstruct_source_cast_info::cast_index(&stack.item) {
        let hidden = stack.data().and_then(|data| data.other("minecraft:tooltip_display"))
            .and_then(|display| display.get("hide_tooltip"))
            .is_some_and(|tag| matches!(tag, horizon_world::nbt::Tag::Byte(n) if *n != 0));
        let costs = if game.remote.is_some() {
            game.container.and_then(|pos| game.world.chunks.__horizon_mixins_tconstruct_builder_cast_costs().get(&pos))
        } else {
            game.world.chunks.__horizon_mixins_tconstruct_source_cast_costs().as_ref()
        };
        if !hidden {
            if let Some(cost) = costs.and_then(|values| values.get(index)).copied().filter(|n| *n > 0) {
                let key = "item.tconstruct.cast.cost";
                let text = game.pack.names.display_get(key).unwrap_or_else(|| key.to_owned()).replace("%d", "%s");
                let text = horizon_client::text::format_translation(&text, &[horizon_client::text::Text::literal(cost.to_string())]);
                let name_visible = horizon_client::item_tooltip::TooltipDisplay::for_stack(stack).name_visible(game.screen == Screen::Creative);
                let index = usize::from(name_visible).min(value.len());
                value.insert(index, (text, horizon_client::hud::rgba(170, 170, 170, 255)));
            }
        }
        return value;
    }
    // DummyMaterialItem.appendHoverText is shared by all four stone plating
    // items. It is independent of explicit names and material components.
    if ["tconstruct:helmet_plating_dummy", "tconstruct:chestplate_plating_dummy",
        "tconstruct:leggings_plating_dummy", "tconstruct:boots_plating_dummy"].contains(&&*stack.item) {
        let hidden = stack.data().and_then(|data| data.other("minecraft:tooltip_display"))
            .and_then(|display| display.get("hide_tooltip"))
            .is_some_and(|value| matches!(value, horizon_world::nbt::Tag::Byte(n) if *n != 0));
        if !hidden {
            let key = "item.tconstruct.dummy_tool_part.tooltip";
            let text = game.pack.names.display_get(key).unwrap_or_else(|| key.to_owned());
            let name_visible = horizon_client::item_tooltip::TooltipDisplay::for_stack(stack)
                .name_visible(game.screen == Screen::Creative);
            let index = usize::from(name_visible).min(value.len());
            value.insert(index, (horizon_client::text::Text::legacy(&text), horizon_client::hud::rgba(170, 170, 170, 255)));
        }
        return value;
    }
    if stack.custom_name().is_some()
        || !horizon_client::item_tooltip::TooltipDisplay::for_stack(stack).name_visible(game.screen == Screen::Creative) {
        return value;
    }
    // Preserve any explicit 26.3 item-name component, including old prototype outputs.
    if stack.item_name().is_some() { return value; }
    let Some(item) = stack.item.strip_prefix("tconstruct:") else { return value; };
    if !["pick_head", "hammer_head", "small_axe_head", "broad_axe_head", "small_blade", "broad_blade", "adze_head", "large_plate", "bow_limb", "bow_grip", "bowstring", "arrow_head", "arrow_shaft", "fletching", "tool_binding", "tough_binding", "tool_handle", "tough_handle", "helmet_plating", "chestplate_plating", "leggings_plating", "boots_plating", "maille", "shield_core", "ribcage", "shell", "laces", "repair_kit", "fake_ingot", "fake_storage_block"].contains(&item) { return value; }
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
