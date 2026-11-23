//! The modifier HUD elements (SlimeKnights ModifierClientEvents' hotbar overlay, MIT; see
//! LICENSE), drawn through PlutoLib's HUD registry from the worn tools' own data: the shield
//! strap's stored item above the offhand slot, the sleeves' selected item beside it, the item
//! frame's stored items in their anchored grid, and the minimap's map at its configured
//! corner. Each is gated by its client config switch, as the original's are, and none draws
//! for a spectator.
//!
//! The minimap reads the worn helmet's own minimap inventory, as the other elements read
//! theirs: the original keeps the current map in the player's data, filled from the helmet's
//! inventory on equip, but the helmet's selected stored map is the same stack, so the HUD
//! derives it directly and no player-data key is kept. The top-right corner's shift for the
//! player's active effects is applied (52 for a visible non-beneficial effect, 26 for a visible
//! beneficial one), the map's decorations are drawn by the map-face flush (all of them, the
//! player arrow included), and the paper panel behind the face is the original's background
//! texture (the checkerboard when the map has data, the plain sheet when it does not).
//!
//! What does not carry over: the map is not ticked while it is stored (the original swaps it
//! into the offhand each tick so it updates and its data reaches the player; PlutoLib has no
//! map-tick verb). Closing that needs a new PlutoLib reach — a host-side player-tick injection
//! that swaps the helmet's stored map into the offhand, lets the vanilla tick run, swaps it
//! back and sends the update packet, with the mod supplying the stored map through a registered
//! handler (the host cannot parse a Tinkers helmet's NBT layout) — its own design and weave
//! verification, not a fix in this file.

use crate::{config::{self, Orientation2D}, tool_stack};
use pluto_lib::{HudDraw, HudEffect, HudRegistry, ItemSlot, Nbt, NbtCompound, NbtCompoundExt};

/// `slimeknights.tconstruct.library.client.Icons.ICONS`: the slot frames' sheet.
const ICONS: &str = "tconstruct:gui/icons";
/// The frame's size (SLOT_BACKGROUND_SIZE); the item sits inside it at 16.
const FRAME: f32 = 22.0;

/// Whether the worn stack carries the modifier at any level (its upgrades list).
fn has_modifier(stack: &ItemSlot, modifier: &str) -> bool {
    tool_stack::upgrades(&stack.custom_data).entries().iter().any(|entry| entry.id.to_string() == modifier && entry.level > 0)
}

/// The tool's persistent data, where its inventory modules keep their stacks.
fn persistent(stack: &ItemSlot) -> Option<&NbtCompound> {
    stack.custom_data.get(tool_stack::TAG_PERSISTENT).and_then(Nbt::as_compound)
}

/// A modifier inventory's stored stacks, in slot order: the persistent list under the modifier's
/// own id, each entry a saved stack with its `Slot` (the layout `tool_inventory` writes).
fn stored_stacks(stack: &ItemSlot, modifier: &str) -> Vec<ItemSlot> {
    let Some(persistent) = persistent(stack) else { return Vec::new() };
    let Some(Nbt::List(list)) = persistent.get(modifier) else { return Vec::new() };
    let mut stored: Vec<(i64, ItemSlot)> = list.iter().filter_map(Nbt::as_compound).filter_map(|compound| {
        let slot = compound.get("Slot").and_then(Nbt::as_i64).unwrap_or(0);
        crate::tool_inventory::read_stack(compound).map(|stack| (slot, stack))
    }).collect();
    stored.sort_by_key(|(slot, _)| *slot);
    stored.into_iter().map(|(_, stack)| stack).collect()
}

/// One stored stack of a modifier inventory, by its slot.
fn stored_slot(stack: &ItemSlot, modifier: &str, slot: i64) -> Option<ItemSlot> {
    let persistent = persistent(stack)?;
    let list = match persistent.get(modifier) { Some(Nbt::List(list)) => list, _ => return None };
    list.iter().filter_map(Nbt::as_compound)
        .find(|compound| compound.get("Slot").and_then(Nbt::as_i64).unwrap_or(0) == slot)
        .and_then(crate::tool_inventory::read_stack)
}

/// One frame of the icons sheet and the item inside it.
fn framed(draw: &mut HudDraw<'_>, u: f32, at: [f32; 2], item: &ItemSlot) {
    draw.canvas.sprite(ICONS, [u, 0.0, FRAME, FRAME], [at[0], at[1], FRAME, FRAME]);
    draw.canvas.item(item, [at[0] + 3.0, at[1] + 3.0], 16.0);
}

/// Where an anchored grid starts: the orientation's alignment of the space left around it, plus
/// the configured offsets (Orientation2D's align, as the original places the item frame).
fn grid_origin(location: Orientation2D, size: [f32; 2], grid: [f32; 2], offsets: (i32, i32)) -> [f32; 2] {
    let x = match location { Orientation2D::TopLeft | Orientation2D::Left | Orientation2D::BottomLeft => 0.0, Orientation2D::Top | Orientation2D::Bottom => (size[0] - grid[0]) / 2.0, _ => size[0] - grid[0] };
    let y = match location { Orientation2D::TopLeft | Orientation2D::Top | Orientation2D::TopRight => 0.0, Orientation2D::Left | Orientation2D::Right => (size[1] - grid[1]) / 2.0, _ => size[1] - grid[1] };
    [x + offsets.0 as f32, y + offsets.1 as f32]
}

/// ModifierClientEvents.getEffectOffset: the top-right corner's shift for the player's active
/// effects — 52 when any visible effect is not beneficial, 26 when any visible effect is
/// beneficial, 0 otherwise. A visible effect is one that shows its icon (the draw is given the
/// icon flag as the host reports it: not ambient); an effect hidden with showIcon still shifts,
/// which vanilla never does.
fn effect_offset(effects: &[HudEffect]) -> f32 {
    if effects.iter().any(|effect| effect.visible && !effect.beneficial) { 52.0 }
    else if effects.iter().any(|effect| effect.visible && effect.beneficial) { 26.0 }
    else { 0.0 }
}

/// The minimap: the worn helmet's selected stored map, drawn at its configured corner at the
/// configured scale (ModifierClientEvents' map rendering: a 142-square panel, the map's face
/// inset by seven). Returns where it sits and how far its footprint reaches, for the item
/// frame's shift when the two share a location; `None` when it does not draw.
fn minimap(draw: &mut HudDraw<'_>, client: &config::Client) -> Option<(Orientation2D, f32)> {
    if client.map_scale <= 0.0 { return None; }
    let head = draw.equipment.head.as_ref()?;
    if !has_modifier(head, "tconstruct:minimap") { return None; }
    let selected = persistent(head).map_or(0, |data| data.get_int("tconstruct:minimap_selected", 0));
    let stack = stored_slot(head, "tconstruct:minimap", selected)?;
    if stack.item.to_string() != "minecraft:filled_map" { return None; }
    let scale = client.map_scale as f32;
    let size = 142.0 * scale;
    let mut origin = grid_origin(client.map_location, draw.size, [size, size], (client.map_x_offset, client.map_y_offset));
    // The top-right corner's shift for the player's active effects: the effect icons sit above
    // the map there, as the original shifts it below them, and the footprint grows by the shift
    // (the original's mapOffset) so the item frame clears the map's bottom edge.
    let shift = if client.map_location == Orientation2D::TopRight { effect_offset(draw.effects) } else { 0.0 };
    origin[1] += shift;
    draw.canvas.fill([origin[0], origin[1], size, size], 0xFFE8_D8B0);
    draw.canvas.map([origin[0] + 7.0 * scale, origin[1] + 7.0 * scale, 128.0 * scale, 128.0 * scale], &stack);
    Some((client.map_location, size + shift))
}

fn draw(draw: &mut HudDraw<'_>) {
    if draw.spectator { return; }
    let Ok(client) = config::client() else { return };
    // The minimap draws first: the item frame shifts out of its way when they share a location.
    let map = minimap(draw, &client);
    let center = draw.size[0] / 2.0;
    let right_handed = draw.equipment.right_handed;
    let empty_offhand = draw.equipment.offhand.is_none();
    // the shield strap: the legs tool's first stored item, above the offhand slot
    if client.render_shield_slot_item {
        if let Some(legs) = &draw.equipment.legs {
            if has_modifier(legs, "tconstruct:shield_strap") {
                if let Some(item) = stored_slot(legs, "tconstruct:shield_strap", 0) {
                    let at = [center + if right_handed { -120.0 } else { 98.0 }, draw.size[1] - 41.0];
                    framed(draw, if empty_offhand { 211.0 } else { 189.0 }, at, &item);
                }
            }
        }
    }
    // the sleeves: the chest tool's selected stored item, beside the offhand slot
    if client.render_sleeves_item {
        if let Some(chest) = &draw.equipment.chest {
            if has_modifier(chest, "tconstruct:sleeves") {
                let selected = persistent(chest).map_or(0, |data| data.get_int("tconstruct:sleeves_selected", 0));
                if let Some(item) = stored_slot(chest, "tconstruct:sleeves", selected) {
                    let at = [center + if right_handed { -139.0 } else { 117.0 }, draw.size[1] - 22.0];
                    let u = if empty_offhand { 211.0 } else if right_handed { 145.0 } else { 123.0 };
                    framed(draw, u, at, &item);
                }
            }
        }
    }
    // the item frame: the head tool's stored items, in their grid
    if client.render_item_frame && client.items_per_row > 0 {
        if let Some(head) = &draw.equipment.head {
            if has_modifier(head, "tconstruct:item_frame") {
                let items = stored_stacks(head, "tconstruct:item_frame");
                if !items.is_empty() {
                    let columns = (client.items_per_row as usize).min(items.len());
                    let rows = items.len().div_ceil(columns);
                    let mut origin = grid_origin(client.item_frame_location, draw.size, [FRAME * columns as f32, FRAME * rows as f32],
                        (client.item_frame_x_offset, client.item_frame_y_offset));
                    // A minimap at the same location pushes the frame down (or up, at the
                    // bottom) by its footprint, as the original's rendering shifts it.
                    if let Some((location, footprint)) = map {
                        if location == client.item_frame_location {
                            origin[1] += match location {
                                Orientation2D::TopLeft | Orientation2D::Top | Orientation2D::TopRight => footprint,
                                Orientation2D::Left | Orientation2D::Right => (footprint + FRAME * rows as f32) / 2.0,
                                _ => -footprint,
                            };
                        }
                    }
                    // The item frame at the top right shifts for the player's active effects
                    // too, as the original shifts its grid there — unless the minimap is at the
                    // same corner, whose footprint already carries the shift.
                    if client.item_frame_location == Orientation2D::TopRight && map.map_or(true, |(location, _)| location != Orientation2D::TopRight) {
                        origin[1] += effect_offset(draw.effects);
                    }
                    for (index, item) in items.iter().enumerate() {
                        let at = [origin[0] + (index % columns) as f32 * FRAME, origin[1] + (index / columns) as f32 * FRAME];
                        framed(draw, 167.0, at, item);
                    }
                }
            }
        }
    }
}

/// Registers the HUD elements. Called once from the product's hooks.
pub(crate) fn register() { HudRegistry::register(draw); }

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::Identifier;

    fn tool(modifier: &str, stored: &[(&str, i64)]) -> ItemSlot {
        let mut stack = ItemSlot::new(Identifier::parse("tconstruct:plate_leggings").unwrap(), 1);
        let mut entry = NbtCompound::new();
        entry.insert("name".to_owned(), Nbt::String(modifier.to_owned()));
        entry.insert("level".to_owned(), Nbt::Int(1));
        let mut persistent = NbtCompound::new();
        let list: Vec<Nbt> = stored.iter().map(|(item, slot)| {
            let mut compound = NbtCompound::new();
            compound.insert("id".to_owned(), Nbt::String(item.to_string()));
            compound.insert("Count".to_owned(), Nbt::Byte(1));
            compound.insert("Slot".to_owned(), Nbt::Int(*slot as i32));
            Nbt::Compound(compound)
        }).collect();
        persistent.insert(modifier.to_owned(), Nbt::List(list));
        stack.custom_data.insert(tool_stack::TAG_UPGRADES.to_owned(), Nbt::List(vec![Nbt::Compound(entry)]));
        stack.custom_data.insert(tool_stack::TAG_PERSISTENT.to_owned(), Nbt::Compound(persistent));
        stack
    }

    #[test]
    fn stored_stacks_read_the_modifiers_own_list_in_slot_order() {
        let stack = tool("tconstruct:item_frame", &[("minecraft:apple", 2), ("minecraft:torch", 0)]);
        let items = stored_stacks(&stack, "tconstruct:item_frame");
        assert_eq!(items.iter().map(|item| item.item.to_string()).collect::<Vec<_>>(), ["minecraft:torch", "minecraft:apple"]);
        assert_eq!(stored_slot(&stack, "tconstruct:item_frame", 2).map(|item| item.item.to_string()), Some("minecraft:apple".to_owned()));
        assert!(stored_stacks(&stack, "tconstruct:sleeves").is_empty(), "another modifier's list is not this one's");
    }

    #[test]
    fn the_effect_shift_is_52_for_a_visible_harmful_effect_and_26_for_a_beneficial_one() {
        let harmful = HudEffect { beneficial: false, visible: true };
        let beneficial = HudEffect { beneficial: true, visible: true };
        let hidden = HudEffect { beneficial: false, visible: false };
        assert_eq!(effect_offset(&[]), 0.0);
        assert_eq!(effect_offset(&[hidden]), 0.0, "an ambient effect shows no icon, so it shifts nothing");
        assert_eq!(effect_offset(&[beneficial]), 26.0);
        assert_eq!(effect_offset(&[harmful]), 52.0);
        assert_eq!(effect_offset(&[beneficial, harmful]), 52.0, "any non-beneficial visible effect wins");
        assert_eq!(effect_offset(&[beneficial, hidden]), 26.0);
    }
}
