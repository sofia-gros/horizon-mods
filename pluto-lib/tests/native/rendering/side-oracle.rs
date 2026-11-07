// Frozen actual side/info drawing bodies with Game inputs and font call excluded.
use horizon_client::hud::{HudBatch,Rect};
use std::collections::HashMap;
mod layout {
// The source side inventory caps its height to the parent screen.
pub(super) fn columns(count: usize) -> usize { if count > 27 { 6 } else { 3 } }
pub(super) fn rows(count: usize) -> usize { count.div_ceil(columns(count)) }
pub(super) fn visible_rows(count: usize) -> usize { rows(count).min(8) }
pub(super) fn max_offset(count: usize) -> usize { rows(count).saturating_sub(visible_rows(count)) }
pub(super) fn title_code(name: &str) -> i16 {
    match name {
        "minecraft:chest" | "minecraft:trapped_chest" => 1,
        "minecraft:barrel" => 2,
        "minecraft:dispenser" => 3,
        "minecraft:dropper" => 4,
        "minecraft:hopper" => 5,
        "tconstruct:part_chest" => 7,
        "tconstruct:tinkers_chest" => 8,
        name if name.ends_with("shulker_box") => 6,
        _ => 0,
    }
}
pub(super) fn title_key(code: i16) -> &'static str {
    match code {
        1 => "container.chest", 2 => "container.barrel", 3 => "container.dispenser",
        4 => "container.dropper", 5 => "container.hopper", 6 => "container.shulkerBox",
        7 => "gui.tconstruct.part_chest", 8 => "gui.tconstruct.tinkers_chest",
        _ => "container.inventory",
    }
}
pub(super) fn left(count: usize) -> f32 {
    -(columns(count) as f32 * 18.0 + 14.0 + if max_offset(count) > 0 { 10.0 } else { 0.0 })
}
pub(super) fn slot_position(count: usize, offset: usize, index: usize) -> Option<(f32, f32)> {
    let columns = columns(count);
    let first = offset.min(max_offset(count)) * columns;
    if index >= count || index < first || index >= first + visible_rows(count) * columns { return None; }
    let visible = index - first;
    Some((left(count) + 8.0 + (visible % columns) as f32 * 18.0, 18.0 + (visible / columns) as f32 * 18.0))
}
pub(super) fn drag_offset(count: usize, y: f32) -> usize {
    let max = max_offset(count);
    if max == 0 { return 0; }
    let travel = (visible_rows(count) as f32 * 18.0 - 15.0).max(1.0);
    (((y - 17.0 - 7.5) / travel).clamp(0.0, 1.0) * max as f32).round() as usize
}

/// Keep both modules as visible as possible when their combined width exceeds
/// the centered table's available margins.
pub(super) fn adjusted_origin(width: f32, base: f32, left: f32, right: f32) -> f32 {
    if base + left >= 0.0 && base + right <= width { base }
    else { ((width - (right - left)) / 2.0 - left).round() }
}

}
pub fn old_side(batch:&mut HudBatch,regions:&HashMap<String,Rect>,ox:f32,oy:f32,count:usize,offset:usize) {
    let columns = layout::columns(count);
    let left = layout::left(count);
    let scrollable = layout::max_offset(count) > 0;
    let width = columns as f32 * 18.0 + if scrollable { 10.0 } else { 0.0 };
    let slots_height = layout::visible_rows(count) as f32 * 18.0;
    let height = slots_height + 10.0;
    if let Some(sprite) = regions.get("gui/tconstruct_generic").copied() {
        let region = |batch: &mut horizon_client::hud::HudBatch, sx: f32, sy: f32, sw: f32, sh: f32, x: f32, y: f32, w: f32, h: f32| {
            let mut r = sprite;
            let (du, dv) = ((r.u1 - r.u0) / r.w as f32, (r.v1 - r.v0) / r.h as f32);
            r.u0 += sx * du; r.v0 += sy * dv;
            r.u1 = r.u0 + sw * du; r.v1 = r.v0 + sh * dv;
            batch.rect_uv(ox + left + x, oy + y, w, h, r, u32::MAX);
        };
        for (sx, sy, x, y) in [(0.0, 0.0, 0.0, 0.0), (57.0, 0.0, width + 7.0, 0.0),
            (0.0, 57.0, 0.0, height + 7.0), (57.0, 57.0, width + 7.0, height + 7.0)] {
            region(batch, sx, sy, 7.0, 7.0, x, y, 7.0, 7.0);
        }
        region(batch, 7.0, 0.0, 50.0, 7.0, 7.0, 0.0, width, 7.0);
        region(batch, 7.0, 57.0, 50.0, 7.0, 7.0, height + 7.0, width, 7.0);
        region(batch, 0.0, 7.0, 7.0, 50.0, 0.0, 7.0, 7.0, height);
        region(batch, 57.0, 7.0, 7.0, 50.0, width + 7.0, 7.0, 7.0, height);
        region(batch, 25.0, 7.0, 18.0, 10.0, 7.0, 7.0, width, 10.0);
        let first = offset.min(layout::max_offset(count)) * columns;
        for i in 0..count.saturating_sub(first).min(layout::visible_rows(count) * columns) {
            region(batch, 7.0, 7.0, 18.0, 18.0,
                7.0 + (i % columns) as f32 * 18.0, 17.0 + (i / columns) as f32 * 18.0, 18.0, 18.0);
        }
        let shown = count.saturating_sub(first).min(layout::visible_rows(count) * columns);
        if shown % columns != 0 {
            for i in shown..shown.div_ceil(columns) * columns {
                region(batch, 25.0, 7.0, 18.0, 18.0,
                    7.0 + (i % columns) as f32 * 18.0, 17.0 + (i / columns) as f32 * 18.0, 18.0, 18.0);
            }
        }
        if scrollable {
            let x = 7.0 + columns as f32 * 18.0;
            region(batch, 43.0, 8.0, 12.0, 30.0, x, 17.0, 10.0, slots_height);
            let progress = offset.min(layout::max_offset(count)) as f32
                / layout::max_offset(count) as f32;
            region(batch, 7.0, 25.0, 10.0, 15.0, x, 17.0 + progress * (slots_height - 15.0), 10.0, 15.0);
        }
    }
}
pub fn new_side(batch:&mut HudBatch,regions:&HashMap<String,Rect>,ox:f32,oy:f32,count:usize,offset:usize) {
    let columns = layout::columns(count);
    let left = layout::left(count);
    let scrollable = layout::max_offset(count) > 0;
    let width = columns as f32 * 18.0 + if scrollable { 10.0 } else { 0.0 };
    let slots_height = layout::visible_rows(count) as f32 * 18.0;
    let height = slots_height + 10.0;
    if let Some(sprite) = regions.get("gui/tconstruct_generic").map(super::Sprite) {
        let region = |batch: &mut horizon_client::hud::HudBatch, sx: f32, sy: f32, sw: f32, sh: f32, x: f32, y: f32, w: f32, h: f32| {
            let _ = super::crop(batch, sprite,
                [sx, sy, sw, sh], sprite.logical_size(),
                [ox + left + x, oy + y, w, h], u32::MAX);
        };
        for (sx, sy, x, y) in [(0.0, 0.0, 0.0, 0.0), (57.0, 0.0, width + 7.0, 0.0),
            (0.0, 57.0, 0.0, height + 7.0), (57.0, 57.0, width + 7.0, height + 7.0)] {
            region(batch, sx, sy, 7.0, 7.0, x, y, 7.0, 7.0);
        }
        region(batch, 7.0, 0.0, 50.0, 7.0, 7.0, 0.0, width, 7.0);
        region(batch, 7.0, 57.0, 50.0, 7.0, 7.0, height + 7.0, width, 7.0);
        region(batch, 0.0, 7.0, 7.0, 50.0, 0.0, 7.0, 7.0, height);
        region(batch, 57.0, 7.0, 7.0, 50.0, width + 7.0, 7.0, 7.0, height);
        region(batch, 25.0, 7.0, 18.0, 10.0, 7.0, 7.0, width, 10.0);
        let first = offset.min(layout::max_offset(count)) * columns;
        for i in 0..count.saturating_sub(first).min(layout::visible_rows(count) * columns) {
            region(batch, 7.0, 7.0, 18.0, 18.0,
                7.0 + (i % columns) as f32 * 18.0, 17.0 + (i / columns) as f32 * 18.0, 18.0, 18.0);
        }
        let shown = count.saturating_sub(first).min(layout::visible_rows(count) * columns);
        if shown % columns != 0 {
            for i in shown..shown.div_ceil(columns) * columns {
                region(batch, 25.0, 7.0, 18.0, 18.0,
                    7.0 + (i % columns) as f32 * 18.0, 17.0 + (i / columns) as f32 * 18.0, 18.0, 18.0);
            }
        }
        if scrollable {
            let x = 7.0 + columns as f32 * 18.0;
            region(batch, 43.0, 8.0, 12.0, 30.0, x, 17.0, 10.0, slots_height);
            let progress = offset.min(layout::max_offset(count)) as f32
                / layout::max_offset(count) as f32;
            region(batch, 7.0, 25.0, 10.0, 15.0, x, 17.0 + progress * (slots_height - 15.0), 10.0, 15.0);
        }
    }
}
pub fn old_info(batch:&mut HudBatch,regions:&HashMap<String,Rect>,ox:f32,oy:f32,width:f32,height:f32) {

    let Some(sprite) = regions.get("gui/tconstruct_panel").copied() else { return; };
    let region = |batch: &mut horizon_client::hud::HudBatch, sx: f32, sy: f32, sw: f32, sh: f32, x: f32, y: f32, w: f32, h: f32| {
        let mut r = sprite;
        let (du, dv) = ((r.u1 - r.u0) / r.w as f32, (r.v1 - r.v0) / r.h as f32);
        r.u0 += sx * du; r.v0 += sy * dv; r.u1 = r.u0 + sw * du; r.v1 = r.v0 + sh * dv;
        batch.rect_uv(ox + x, oy + y, w, h, r, u32::MAX);
    };
    let inner = height - 8.0;
    region(batch, 4.0, 4.0, 118.0, 75.0, 4.0, 4.0, width - 8.0, inner);
    for (sx, sy, x, y) in [(0.0,0.0,0.0,0.0),(122.0,0.0,width - 4.0,0.0),(0.0,79.0,0.0,height - 4.0),(122.0,79.0,width - 4.0,height - 4.0)] {
        region(batch, sx, sy, 4.0, 4.0, x, y, 4.0, 4.0);
    }
    region(batch, 4.0, 0.0, 118.0, 4.0, 4.0, 0.0, width - 8.0, 4.0);
    region(batch, 4.0, 79.0, 118.0, 4.0, 4.0, height - 4.0, width - 8.0, 4.0);
    region(batch, 0.0, 4.0, 4.0, 75.0, 0.0, 4.0, 4.0, inner);
    region(batch, 122.0, 4.0, 4.0, 75.0, width - 4.0, 4.0, 4.0, inner);

}
pub fn new_info(batch:&mut HudBatch,regions:&HashMap<String,Rect>,ox:f32,oy:f32,width:f32,height:f32) {

    let Some(sprite) = regions.get("gui/tconstruct_panel").map(super::Sprite) else { return; };
    let region = |batch: &mut horizon_client::hud::HudBatch, sx: f32, sy: f32, sw: f32, sh: f32, x: f32, y: f32, w: f32, h: f32| {
        let _ = super::crop(batch, sprite,
            [sx, sy, sw, sh], sprite.logical_size(),
            [ox + x, oy + y, w, h], u32::MAX);
    };
    let inner = height - 8.0;
    region(batch, 4.0, 4.0, 118.0, 75.0, 4.0, 4.0, width - 8.0, inner);
    for (sx, sy, x, y) in [(0.0,0.0,0.0,0.0),(122.0,0.0,width - 4.0,0.0),(0.0,79.0,0.0,height - 4.0),(122.0,79.0,width - 4.0,height - 4.0)] {
        region(batch, sx, sy, 4.0, 4.0, x, y, 4.0, 4.0);
    }
    region(batch, 4.0, 0.0, 118.0, 4.0, 4.0, 0.0, width - 8.0, 4.0);
    region(batch, 4.0, 79.0, 118.0, 4.0, 4.0, height - 4.0, width - 8.0, 4.0);
    region(batch, 0.0, 4.0, 4.0, 75.0, 0.0, 4.0, 4.0, inner);
    region(batch, 122.0, 4.0, 4.0, 75.0, width - 4.0, 4.0, 4.0, inner);

}
