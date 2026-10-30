use super::*;
use horizon_world::nbt::Tag;

pub(super) fn draw(game: &Game<'_>, batch: &mut horizon_client::hud::HudBatch, ox: f32, oy: f32) {
    let atlas = &game.gui.atlas;
    let Some(layout) = super::tconstruct_ui::station_layout(game) else { return; };
    let Some(entity) = game.container_kind() else { return; };
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
        top.push(title);
        if let Some(data) = tool.data().and_then(|data| data.other("minecraft:custom_data")) {
            for (field, key) in [("tic_max_damage", "tool_stat.tconstruct.durability"), ("tic_attack_damage", "tool_stat.tconstruct.attack_damage"), ("tic_attack_speed", "tool_stat.tconstruct.attack_speed")] {
                let number = match data.get(field) {
                    Some(Tag::Int(value)) => Some(value.to_string()),
                    Some(Tag::Float(bits)) => Some(format!("{:.2}", f32::from_bits(*bits))),
                    _ => None,
                };
                if let Some(number) = number {
                    let label = game.pack.names.display_get(key).unwrap_or_else(|| key.to_owned());
                    top.push(format!("{label}{number}"));
                }
            }
            if let Some(Tag::Compound(upgrades)) = data.get("tic_upgrades") {
                for (id, value) in upgrades {
                    if let Tag::Int(amount) = value {
                        let id = id.to_string_lossy();
                        let key = id.split_once(':').map(|(namespace, path)| format!("modifier.{namespace}.{path}"));
                        let label = key.map(|key| game.pack.names.display_get(&key).unwrap_or(key)).unwrap_or_else(|| id.to_string());
                        let needed = match id.as_ref() { "tconstruct:sharpness" => 36, "tconstruct:swiftstrike" => 72, _ => continue };
                        bottom.push(format!("{label}: {amount}/{needed}"));
                    }
                }
            }
        }
    } else {
        top.push(game.pack.names.display_get(&layout.translation).unwrap_or_else(|| layout.translation.clone()));
        let description = format!("{}.description", layout.translation);
        if let Some(description) = game.pack.names.display_get(&description) {
            top.extend(horizon_client::multiplayer::wrap(&description, 112.0, atlas));
        }
        bottom.push(game.pack.names.display_get("gui.tconstruct.tinker_station.components").unwrap_or_else(|| "Components".into()));
        for (index, spec) in layout.input_slots.iter().enumerate() {
            if let Some(key) = spec.translation.as_deref() {
                let slot_index = index + usize::from(layout.tool_slot.is_some());
                let stack = slots.get(slot_index).and_then(Option::as_ref);
                let valid = stack.is_some_and(|stack| spec.filter.as_ref().and_then(|filter| filter.get("item")).and_then(serde_json::Value::as_str).is_none_or(|item| item == stack.item.as_ref()));
                let label = game.pack.names.display_get(key).unwrap_or_else(|| key.to_owned());
                bottom.push(format!("{} {label}", if valid { "✓" } else { "•" }));
            }
        }
    }
    for (offset, lines) in [(5.0, top), (94.0, bottom)] {
        super::tconstruct_builder_info_ui::panel(game, batch, ox + 182.0, oy + offset, 83.0);
        for (index, line) in lines.iter().take(7).enumerate() {
            batch.text(atlas, line, ox + 187.0, oy + offset + 5.0 + index as f32 * 10.0, u32::MAX, true);
        }
    }
}
