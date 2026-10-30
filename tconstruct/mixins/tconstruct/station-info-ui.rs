use super::*;
use horizon_world::nbt::Tag;

pub(super) fn lines(game: &Game<'_>) -> Option<(Vec<(String, Option<String>)>, Vec<(String, Option<String>)>)> {
    let atlas = &game.gui.atlas;
    let layout = super::tconstruct_ui::station_layout(game)?;
    let entity = game.container_kind()?;
    let slots = entity.slots();
    let mut tool = if game.remote.is_some() {
        slots.get(24).cloned().flatten().or_else(|| slots.get(25).cloned().flatten())
    } else {
        super::tconstruct_source_station_world::building_plan(&game.world, slots).map(|(_, output)| output)
            .or_else(|| super::tconstruct_source_station_world::operation_plan(&game.world, slots).map(|plan| plan.output))
    };
    if game.remote.is_none() && !super::tconstruct_source_station_world::has_operation_tool(slots) {
        if let Some(output) = tool.as_mut() {
            let name = game.container.and_then(|pos| game.world.chunks.__horizon_mixins_tconstruct_station_name_edit().get(&pos)).map(|edit| edit.value.as_str());
            super::tconstruct_source_station_name::name_result(output, name);
        }
    }
    if tool.is_none() && super::tconstruct_source_station_world::has_operation_tool(slots) { tool = slots.first().cloned().flatten(); }
    let mut top = Vec::new();
    let mut bottom = Vec::new();
    if let Some(tool) = tool {
        let id = tool.item.split_once(':').map(|(namespace, path)| format!("item.{namespace}.{path}"));
        let title = tool.custom_name().map(str::to_owned).or_else(|| id.map(|key| game.pack.names.display_get(&key).unwrap_or(key))).unwrap_or_default();
        top.push((title, None));
        if let Some(data) = tool.data().and_then(|data| data.other("minecraft:custom_data")) {
            for (field, key) in [("tic_max_damage", "tool_stat.tconstruct.durability"), ("tic_attack_damage", "tool_stat.tconstruct.attack_damage"), ("tic_attack_speed", "tool_stat.tconstruct.attack_speed"), ("tic_draw_speed", "tool_stat.tconstruct.draw_speed"), ("tic_velocity", "tool_stat.tconstruct.velocity"), ("tic_accuracy", "tool_stat.tconstruct.accuracy"), ("tic_armor", "tool_stat.tconstruct.armor"), ("tic_armor_toughness", "tool_stat.tconstruct.armor_toughness"), ("tic_knockback_resistance", "tool_stat.tconstruct.knockback_resistance")] {
                if matches!(field, "tic_draw_speed" | "tic_velocity" | "tic_accuracy")
                    && !matches!(tool.item.as_ref(), "tconstruct:longbow" | "tconstruct:crossbow") { continue; }
                let number = match data.get(field) {
                    Some(Tag::Int(value)) if field == "tic_max_damage" => Some(format!("{}/{}", value.saturating_sub(tool.damage as i32), value)),
                    Some(Tag::Int(value)) if *value != 0 => Some(value.to_string()),
                    Some(Tag::Float(bits)) if f32::from_bits(*bits) != 0.0 => Some(format!("{:.2}", f32::from_bits(*bits))),
                    _ => None,
                };
                if let Some(number) = number {
                    let label = game.pack.names.display_get(key).unwrap_or_else(|| key.to_owned());
                    top.push((format!("{label}{number}"), game.pack.names.display_get(&format!("{key}.description"))));
                }
            }
            let traits = if game.remote.is_some() {
                game.container.and_then(|pos| game.world.chunks.__horizon_mixins_tconstruct_builder_info().get(&pos))
                    .and_then(|frame| frame.first()).and_then(|json| serde_json::from_str::<std::collections::BTreeMap<String, i32>>(json).ok())
            } else { super::tconstruct_source_material_traits::levels(&game.world, &tool).map(|traits| traits.levels) };
            if let Some(traits) = traits {
                for (id, level) in traits {
                    if let Some((namespace, name)) = id.split_once(':') {
                        let key = format!("modifier.{namespace}.{name}");
                        let label = game.pack.names.display_get(&key).unwrap_or_else(|| key.clone());
                        let tip = game.pack.names.display_get(&format!("{key}.description"));
                        bottom.push((format!("{label} {level}"), tip));
                    }
                }
            }
            if let Some(Tag::Compound(upgrades)) = data.get("tic_upgrades") {
                for (id, value) in upgrades {
                    if let Tag::Int(amount) = value {
                        let id = id.to_string_lossy();
                        let key = id.split_once(':').map(|(namespace, path)| format!("modifier.{namespace}.{path}"));
                        let label = key.as_ref().map(|key| game.pack.names.display_get(key).unwrap_or_else(|| key.clone())).unwrap_or_else(|| id.to_string());
                        let tip = key.and_then(|key| game.pack.names.display_get(&format!("{key}.description")));
                        let needed = match id.as_ref() { "tconstruct:sharpness" => 36, "tconstruct:swiftstrike" => 72, _ => continue };
                        bottom.push((format!("{label}: {amount}/{needed}"), tip));
                    }
                }
            }
        }
    } else {
        top.push((game.pack.names.display_get(&layout.translation).unwrap_or_else(|| layout.translation.clone()), None));
        let description = format!("{}.description", layout.translation);
        if let Some(description) = game.pack.names.display_get(&description) {
            top.extend(horizon_client::multiplayer::wrap(&description, 112.0, atlas).into_iter().map(|line| (line, None)));
        }
        bottom.push((game.pack.names.display_get("gui.tconstruct.tinker_station.components").unwrap_or_else(|| "Components".into()), None));
        for (index, spec) in layout.input_slots.iter().enumerate() {
            if let Some(key) = spec.translation.as_deref() {
                let slot_index = index + usize::from(layout.tool_slot.is_some());
                let stack = slots.get(slot_index).and_then(Option::as_ref);
                let valid = stack.is_some_and(|stack| spec.accepts_item(stack.item.as_ref()));
                let label = game.pack.names.display_get(key).unwrap_or_else(|| key.to_owned());
                bottom.push((format!("{} {label}", if valid { "✓" } else { "•" }), None));
            }
        }
    }
    Some((top, bottom))
}

pub(super) fn draw(game: &Game<'_>, batch: &mut horizon_client::hud::HudBatch, ox: f32, oy: f32) {
    let Some((top, bottom)) = lines(game) else { return; };
    let atlas = &game.gui.atlas;
    let scroll = game.container.and_then(|pos| game.world.chunks.__horizon_mixins_tconstruct_station_info_scroll().get(&pos)).copied().unwrap_or_default();
    for (offset, lines, scroll) in [(5.0, top, scroll.0), (94.0, bottom, scroll.1)] {
        super::tconstruct_builder_info_ui::panel(game, batch, ox + 182.0, oy + offset, 83.0);
        let scroll = scroll.min(lines.len().saturating_sub(7));
        if lines.iter().any(|(_, tip)| tip.is_some()) { batch.text(atlas, "?", ox + 299.0, oy + offset + 5.0, horizon_client::hud::rgba(95, 95, 95, 255), false); }
        for (index, (line, _)) in lines.iter().skip(scroll).take(7).enumerate() {
            batch.text(atlas, line, ox + 187.0, oy + offset + 5.0 + index as f32 * 10.0, u32::MAX, true);
        }
    }
}
