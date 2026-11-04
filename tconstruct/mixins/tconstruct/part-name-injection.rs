fn tconstruct_part_name(
    mut value: Vec<(horizon_client::text::Text, u32)>,
    game: &crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
) -> Vec<(horizon_client::text::Text, u32)> {
    let hidden_tooltip = stack.data().and_then(|data| data.other("minecraft:tooltip_display"))
        .and_then(|display| display.get("hide_tooltip"))
        .is_some_and(|tag| matches!(tag, horizon_world::nbt::Tag::Byte(n) if *n != 0));
    if !hidden_tooltip {
        let context = game.world.chunks.__horizon_mixins_tconstruct_tank_fluid_context().as_ref()
            .and_then(|value| value.as_ref().ok());
        let can = &*stack.item == "tconstruct:copper_can";
        if can {
            let name_visible = horizon_client::item_tooltip::TooltipDisplay::for_stack(stack).name_visible(game.screen == Screen::Creative);
            let index = usize::from(name_visible).min(value.len());
            if crate::game::tconstruct_source_fluid_container::can_display_empty_potion(stack,
                &|id| context.is_some_and(|(fluids, _)| fluids.contains(id))) {
                let name = crate::game::tconstruct_source_melter_render::fluid_name_tagged(game, "tconstruct:potion", None, true);
                value.insert(index, (horizon_client::text::Text::translate("item.tconstruct.copper_can.contents", &[horizon_client::text::Text::literal(name)]), 0xffaaaaaa));
                if game.settings.file.bool("advancedItemTooltips") {
                    value.insert(index + 1, (horizon_client::text::Text::translate("item.tconstruct.tank.fluid_id", &[horizon_client::text::Text::literal("tconstruct:potion")]), 0xff555555));
                }
            } else if let Some(tag) = crate::game::tconstruct_source_fluid_container::can_display_potion(stack,
                &|id| context.is_some_and(|(fluids, _)| fluids.contains(id))) {
                let name = crate::game::tconstruct_source_melter_render::fluid_name_tagged(game, "tconstruct:potion", Some(&tag), true);
                value.insert(index, (horizon_client::text::Text::translate("item.tconstruct.copper_can.contents", &[horizon_client::text::Text::literal(name)]), 0xffaaaaaa));
                if game.settings.file.bool("advancedItemTooltips") {
                    value.insert(index + 1, (horizon_client::text::Text::translate("item.tconstruct.tank.fluid_id", &[horizon_client::text::Text::literal("tconstruct:potion")]), 0xff555555));
                }
            } else if let Some(fluid) = crate::game::tconstruct_source_fluid_container::can_display_fluid(stack,
                &|id| context.is_some_and(|(fluids, _)| fluids.contains(id))) {
                let name = crate::game::tconstruct_source_melter_render::fluid_name(game, &fluid);
                value.insert(index, (horizon_client::text::Text::translate("item.tconstruct.copper_can.contents", &[horizon_client::text::Text::literal(name)]), 0xffaaaaaa));
                if game.settings.file.bool("advancedItemTooltips") {
                    value.insert(index + 1, (horizon_client::text::Text::translate("item.tconstruct.tank.fluid_id", &[horizon_client::text::Text::literal(fluid)]), 0xff555555));
                }
            } else if crate::game::tconstruct_source_fluid_container::can_display_empty(stack) {
                value.insert(index, (horizon_client::text::Text::translate("item.tconstruct.copper_can.tooltip", &[]), 0xffaaaaaa));
            }
        } else if crate::game::tconstruct_source_fluid_container::tank_display_empty_potion(stack,
            &|id| context.is_some_and(|(fluids, _)| fluids.contains(id))) {
            let name_visible = horizon_client::item_tooltip::TooltipDisplay::for_stack(stack).name_visible(game.screen == Screen::Creative);
            let index = usize::from(name_visible).min(value.len());
            let name = crate::game::tconstruct_source_melter_render::fluid_name_tagged(game, "tconstruct:potion", None, true);
            value.insert(index, (horizon_client::text::Text::literal(name), 0xffaaaaaa));
            if game.settings.file.bool("advancedItemTooltips") {
                value.insert(index + 1, (horizon_client::text::Text::translate("item.tconstruct.tank.fluid_id", &[horizon_client::text::Text::literal("tconstruct:potion")]), 0xff555555));
            }
        } else if let Some(tag) = crate::game::tconstruct_source_fluid_container::tank_display_potion(stack,
            &|id| context.is_some_and(|(fluids, _)| fluids.contains(id))) {
            let name_visible = horizon_client::item_tooltip::TooltipDisplay::for_stack(stack).name_visible(game.screen == Screen::Creative);
            let index = usize::from(name_visible).min(value.len());
            let name = crate::game::tconstruct_source_melter_render::fluid_name_tagged(game, "tconstruct:potion", Some(&tag), true);
            value.insert(index, (horizon_client::text::Text::literal(name), 0xffaaaaaa));
            if game.settings.file.bool("advancedItemTooltips") {
                value.insert(index + 1, (horizon_client::text::Text::translate("item.tconstruct.tank.fluid_id", &[horizon_client::text::Text::literal("tconstruct:potion")]), 0xff555555));
            }
        } else if let Some(fluid) = crate::game::tconstruct_source_fluid_container::tank_display_fluid(stack,
            &|id| context.is_some_and(|(fluids, _)| fluids.contains(id))) {
            let name_visible = horizon_client::item_tooltip::TooltipDisplay::for_stack(stack).name_visible(game.screen == Screen::Creative);
            let index = usize::from(name_visible).min(value.len());
            let name = crate::game::tconstruct_source_melter_render::fluid_name(game, &fluid);
            value.insert(index, (horizon_client::text::Text::literal(name), 0xffaaaaaa));
            if game.settings.file.bool("advancedItemTooltips") {
                value.insert(index + 1, (horizon_client::text::Text::translate("item.tconstruct.tank.fluid_id", &[horizon_client::text::Text::literal(fluid)]), 0xff555555));
            }
        }
    }
    if &*stack.item == "tconstruct:modifier_crystal" && stack.custom_name().is_none() && stack.item_name().is_none()
        && horizon_client::item_tooltip::TooltipDisplay::for_stack(stack).name_visible(game.screen == Screen::Creative) {
        let modifier = stack.data().and_then(|data| data.other("minecraft:custom_data")).and_then(|data| data.get("modifier"));
        if let Some(horizon_world::nbt::Tag::String(id)) = modifier {
            let id = id.to_string_lossy();
            if let Some((namespace, path)) = id.split_once(':') {
                let key = format!("modifier.{namespace}.{path}");
                let label = game.pack.names.display_get(&key).unwrap_or(key);
                let template = game.pack.names.display_get("item.tconstruct.modifier_crystal.format").unwrap_or_else(|| "%s Crystal".into());
                let name = horizon_client::text::format_translation(&template, &[horizon_client::text::Text::literal(label)]);
                if let Some((text, _)) = value.first_mut() { *text = name; }
            }
        }
    }
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
    // Show applied incremental upgrades on the item itself, including partial progress.
    let hidden = stack.data().and_then(|data| data.other("minecraft:tooltip_display"))
        .and_then(|display| display.get("hide_tooltip"))
        .is_some_and(|tag| matches!(tag, horizon_world::nbt::Tag::Byte(n) if *n != 0));
    if !hidden {
        let custom = stack.data().and_then(|data| data.other("minecraft:custom_data"));
        // Source display-only stacks do not expose part details. Malformed
        // flags are unknown and are conservatively treated as display-only.
        let display = match custom.and_then(|data| data.get("tic_display")) {
            None | Some(horizon_world::nbt::Tag::Byte(0)) => false,
            Some(_) => true,
        };
        if let Some(horizon_world::nbt::Tag::String(raw)) = custom.and_then(|data| data.get("Material")) {
                if let Some(material) = crate::game::tconstruct_source_part_material_id::source_part_material_id(&stack.item, &raw.to_string_lossy(), game.options_advanced_tooltips(), hidden, display) {
                    let key = "tooltip.tconstruct.part.material_id";
                    let template = game.pack.names.display_get(key).unwrap_or_else(|| key.into());
                    let text = horizon_client::text::format_translation(&template, &[horizon_client::text::Text::literal(material)]);
                    value.push((text, horizon_client::hud::rgba(85, 85, 85, 255)));
                }
        }
        let upgrades = stack.data().and_then(|data| data.other("minecraft:custom_data")).and_then(|data| data.get("tic_upgrades"));
        let amount = stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|d| d.get("tic_overslime"));
        let capacity = stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|d| d.get("tic_overslime_capacity"));
        if let (Some(horizon_world::nbt::Tag::Int(amount)), Some(horizon_world::nbt::Tag::Int(capacity))) = (amount, capacity) {
            let active = matches!(stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|d| d.get("tic_overslime_active")), Some(horizon_world::nbt::Tag::Byte(v)) if *v != 0);
            if active && *capacity > 0 { value.push((horizon_client::text::Text::literal(format!("Overslime {amount}/{capacity}")), horizon_client::hud::rgba(85, 255, 255, 255))); }
        }
        let necrotic_upgrade = upgrades.and_then(|data| data.get("tconstruct:necrotic"))
            .is_some_and(|tag| matches!(tag, horizon_world::nbt::Tag::Int(level) if *level > 0));
        let necrotic_trait = crate::game::tconstruct_source_material_traits::levels(&game.world, stack)
            .is_some_and(|traits| traits.levels.get("tconstruct:necrotic").copied().unwrap_or(0) > 0);
        if necrotic_upgrade || necrotic_trait {
            value.push((horizon_client::text::Text::literal("Necrotic: held mob melee only; worn, PvP and projectiles unavailable"), horizon_client::hud::rgba(255, 170, 0, 255)));
        }
        let hydraulic_upgrade = upgrades.and_then(|data| data.get("tconstruct:hydraulic"))
            .is_some_and(|tag| matches!(tag, horizon_world::nbt::Tag::Int(amount) if *amount > 0));
        let hydraulic_trait = stack.data().and_then(|d| d.other("minecraft:custom_data"))
            .and_then(|d| d.get("tic_mining_traits")).is_some_and(|tag| match tag {
                horizon_world::nbt::Tag::List(entries) => entries.iter().any(|entry|
                    matches!((entry.get("id"), entry.get("level")),
                        (Some(horizon_world::nbt::Tag::String(id)), Some(horizon_world::nbt::Tag::Int(level)))
                        if id.to_string_lossy() == "tconstruct:hydraulic" && *level > 0)),
                _ => false,
            });
        if hydraulic_upgrade || hydraulic_trait {
            value.push((horizon_client::text::Text::literal("Hydraulic: underwater only; rain bonus unavailable"), horizon_client::hud::rgba(255, 170, 0, 255)));
        }
        for (id, needed) in [("blasting", 20), ("hydraulic", 36), ("sharpness", 36), ("swiftstrike", 72), ("power", 72), ("blindshot", 10), ("speedy", 45), ("step_up", 1), ("haste", 45), ("quick_charge", 5), ("trueshot", 10), ("diamond", 1), ("emerald", 1), ("reinforced", 4), ("overforced", 4), ("netherite", 1), ("unbreakable", 1), ("necrotic", 1), ("tasty", 1), ("writable", 1), ("harmonious", 1), ("recapitated", 1), ("forecast", 1), ("draconic", 1)] {
            let key = format!("tconstruct:{id}");
            let Some(horizon_world::nbt::Tag::Int(amount)) = upgrades.and_then(|data| data.get(&key)).filter(|tag| matches!(tag, horizon_world::nbt::Tag::Int(n) if *n > 0)) else { continue; };
            let level = (amount - 1) / needed + 1;
            let base_key = format!("modifier.tconstruct.{id}");
            let level_key = format!("{base_key}.{level}");
            let text = if id == "blasting" {
                let base = game.pack.names.display_get(&base_key).unwrap_or(base_key);
                let Some(text) = crate::game::tconstruct_station_info_ui::blasting_label(&base, *amount) else { continue; };
                text
            } else {
                let name = game.pack.names.display_get(&level_key).or_else(|| game.pack.names.display_get(&base_key)).unwrap_or(base_key);
                let remainder = amount % needed;
                if remainder == 0 { name } else { format!("{name} {remainder}/{needed}") }
            };
            if game.options_advanced_tooltips() && tconstruct_source_config::client_modifier_ids() {
                let template = game.pack.names.display_get("item.tconstruct.tool.id_format")
                    .unwrap_or_else(|| "%s (%s)".to_owned());
                let text = horizon_client::text::format_translation(&template, &[
                    horizon_client::text::Text::literal(text), horizon_client::text::Text::literal(key),
                ]);
                value.push((text, horizon_client::hud::rgba(85, 85, 85, 255)));
            } else {
                value.push((horizon_client::text::Text::literal(text), horizon_client::hud::rgba(170, 170, 170, 255)));
            }
        }
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
