// Exact old/new draw prefixes with Game/atlas input substitutions recorded in panel-oracle-sources.json.
use horizon_client::hud::{HudBatch, Rect};
use std::collections::HashMap;
mod layout {
// HeatingStructureScreen uses up to four 22-pixel columns, each chosen for
// seven source slots. The side panel is capped by the 220-pixel parent screen.
// The two bucket slots precede melting slots in the source container.
pub(super) fn columns(count: usize) -> usize { count.div_ceil(7).min(4) }
pub(super) fn rows(count: usize) -> usize {
    let cols = columns(count);
    if cols == 0 { 0 } else { count.div_ceil(cols) }
}
pub(super) fn visible_rows(count: usize) -> usize { rows(count).min(10) }
pub(super) fn max_offset(count: usize) -> usize { rows(count).saturating_sub(visible_rows(count)) }
pub(super) fn panel_width(count: usize) -> f32 {
    (columns(count) * 22 + 14 + if max_offset(count) > 0 { 10 } else { 0 }) as f32
}
/// Connected source side panels overlap their parent by the six-pixel border.
pub(super) fn panel_left(count: usize) -> f32 { 6.0 - panel_width(count) }
/// Return the source tank layer under a relative screen y coordinate.
pub(super) fn fluid_heights(amounts: &[u32], capacity: u32) -> Vec<u32> {
    if amounts.is_empty() { return Vec::new(); }
    // GuiSmelteryTank normalizes against max(contained, capacity), including
    // tanks retained after a structure shrink. Keep the total wide so malformed
    // or unusually large saved amounts cannot wrap the display denominator.
    let total = amounts.iter().map(|amount| u64::from(*amount)).sum::<u64>();
    let capacity = u64::from(capacity).max(total);
    if capacity == 0 { return vec![0; amounts.len()]; }
    let mut heights: Vec<u32> = amounts.iter().map(|amount|
        (((*amount as f32 / capacity as f32) * 106.0).ceil() as u32).max(3)).collect();
    let available = if total < capacity { 103 } else { 106 };
    if heights.iter().map(|height| u64::from(*height)).sum::<u64>() > u64::from(available) {
        // The source removes pixels from the tallest layer, choosing the first
        // on ties. Find the equivalent cutoff without looping once per pixel
        // when a saved tank exceeds a newly reduced capacity.
        let (mut low, mut high) = (0, *heights.iter().max().unwrap_or(&0));
        while low < high {
            let mid = low + (high - low).div_ceil(2);
            let sum = heights.iter().map(|height| u64::from((*height).min(mid))).sum::<u64>();
            if sum <= u64::from(available) { low = mid; } else { high = mid - 1; }
        }
        let mut spare = available - heights.iter().map(|height| (*height).min(low)).sum::<u32>();
        for height in heights.iter_mut().rev() {
            let original = *height;
            *height = original.min(low);
            if spare > 0 && original > low { *height += 1; spare -= 1; }
        }
    }
    heights
}

pub(super) fn fluid_at(amounts: &[u32], capacity: u32, y: f32) -> Option<usize> {
    if !(16.0..122.0).contains(&y) { return None; }
    let from_bottom = 122.0 - y;
    let mut bottom = 0.0f32;
    for (index, height) in fluid_heights(amounts, capacity).into_iter().enumerate() {
        let height = height as f32;
        if from_bottom <= bottom + height && from_bottom > bottom { return Some(index); }
        bottom += height;
    }
    None
}
pub(super) fn drag_offset(count: usize, y: f32) -> usize {
    let max = max_offset(count);
    if max == 0 { return 0; }
    let travel = (visible_rows(count) as f32 * 18.0 - 15.0).max(1.0);
    (((y - 11.0 - 7.5) / travel).clamp(0.0, 1.0) * max as f32).round() as usize
}
/// Resolve a visible container slot to the real melting slot. No virtual slot
/// is ever a destination; callers must reject a None before mutating items.
pub(super) fn melting_index(count: usize, offset: usize, menu_slot: usize) -> Option<usize> {
    let visible = menu_slot.checked_sub(2)?;
    let cols = columns(count);
    if cols == 0 || visible >= visible_rows(count) * cols { return None; }
    let index = offset.min(max_offset(count)).checked_mul(cols)?.checked_add(visible)?;
    (index < count).then_some(index)
}
pub(super) fn position(count: usize, offset: usize, menu_slot: usize) -> Option<(f32, f32)> {
    match menu_slot {
        0 => Some((125.0, 46.0)), 1 => Some((125.0, 104.0)),
        _ => {
            let index = melting_index(count, offset, menu_slot)?;
            let cols = columns(count);
            let visible = index - offset.min(max_offset(count)) * cols;
            Some((-panel_width(count) + 18.0 + (visible % cols) as f32 * 22.0,
                  12.0 + (visible / cols) as f32 * 18.0))
        }
    }
}

}
pub fn old_panel(batch: &mut HudBatch, regions: &HashMap<String,Rect>, ox:f32, oy:f32, mode:u8, count:usize, offset:usize, empty_input:bool, hovered:bool) {
    if let Some(mut sprite) = regions.get("gui/tconstruct_heating_structure").copied() {
        let (du, dv) = ((sprite.u1 - sprite.u0) / sprite.w as f32, (sprite.v1 - sprite.v0) / sprite.h as f32);
        let original = sprite;
        if empty_input {
            let mut icon = original;
            icon.u0 += 224.0 * du; icon.v0 += 186.0 * dv;
            icon.u1 = icon.u0 + 16.0 * du; icon.v1 = icon.v0 + 16.0 * dv;
            batch.rect_uv(ox + 125.0, oy + 46.0, 16.0, 16.0, icon, u32::MAX);
        }
        if hovered {
            let mut hover = original;
            hover.u0 += 176.0 * du; hover.v0 += 202.0 * dv;
            hover.u1 = hover.u0 + 18.0 * du; hover.v1 = hover.v0 + 18.0 * dv;
            batch.rect_uv(ox + 124.0, oy + 69.0, 18.0, 18.0, hover, u32::MAX);
        }
        sprite.u0 += (176.0 + f32::from(mode) * 16.0) * du; sprite.v0 += 186.0 * dv;
        sprite.u1 = sprite.u0 + 16.0 * du; sprite.v1 = sprite.v0 + 16.0 * dv;
        batch.rect_uv(ox + 125.0, oy + 70.0, 16.0, 16.0, sprite, u32::MAX);
    }
    let cols = layout::columns(count);
    let rows = layout::visible_rows(count);
    let max = layout::max_offset(count);
    let width = layout::panel_width(count);
    if cols > 0 {
        let left = layout::panel_left(count);
        let inner_width = width - 14.0;
        let inner_height = rows as f32 * 18.0;
        if let Some(sprite) = regions.get("gui/tconstruct_generic").copied() {
            let region = |batch: &mut HudBatch, sx: f32, sy: f32, sw: f32, sh: f32, x: f32, y: f32, w: f32, h: f32| {
                let mut r = sprite;
                let (du, dv) = ((r.u1-r.u0)/r.w as f32, (r.v1-r.v0)/r.h as f32);
                r.u0 += sx*du; r.v0 += sy*dv; r.u1 = r.u0+sw*du; r.v1 = r.v0+sh*dv;
                batch.rect_uv(ox+left+x, oy+4.0+y, w, h, r, u32::MAX);
            };
            for (sx, sy, x, y) in [(0.0,0.0,0.0,0.0),(57.0,0.0,inner_width+7.0,0.0),
                (0.0,57.0,0.0,inner_height+7.0),(57.0,57.0,inner_width+7.0,inner_height+7.0)] {
                region(batch,sx,sy,7.0,7.0,x,y,7.0,7.0);
            }
            region(batch,7.0,0.0,50.0,7.0,7.0,0.0,inner_width,7.0);
            region(batch,7.0,57.0,50.0,7.0,7.0,inner_height+7.0,inner_width,7.0);
            region(batch,0.0,7.0,7.0,50.0,0.0,7.0,7.0,inner_height);
            region(batch,57.0,7.0,7.0,50.0,inner_width+7.0,7.0,7.0,inner_height);
            if max > 0 {
                let x = 7.0 + cols as f32 * 22.0;
                region(batch,43.0,8.0,12.0,30.0,x,7.0,10.0,inner_height);
                let progress = offset.min(max) as f32 / max as f32;
                region(batch,7.0,25.0,10.0,15.0,x,7.0+progress*(inner_height-15.0).max(0.0),10.0,15.0);
            }
        }
        if let Some(sprite) = regions.get("gui/tconstruct_heating_structure").copied() {
            let du = (sprite.u1-sprite.u0)/sprite.w as f32; let dv = (sprite.v1-sprite.v0)/sprite.h as f32;
            let first = offset.min(max) * cols;
            for i in 0..rows*cols {
                let mut r = sprite; r.u0 += if first+i < count { 0.0 } else { 22.0*du }; r.v0 += 238.0*dv;
                r.u1 = r.u0+22.0*du; r.v1 = r.v0+18.0*dv;
                batch.rect_uv(ox+left+7.0+(i%cols) as f32*22.0, oy+11.0+(i/cols) as f32*18.0, 22.0,18.0,r,u32::MAX);
            }
        }
    }
}

pub fn new_panel(batch: &mut HudBatch, regions: &HashMap<String,Rect>, ox:f32, oy:f32, mode:u8, count:usize, offset:usize, empty_input:bool, hovered:bool) {
    if let Some(sprite) = regions.get("gui/tconstruct_heating_structure").map(super::Sprite) {
        if empty_input {
            let _ = super::crop(batch, sprite,
                [224.0, 186.0, 16.0, 16.0], sprite.logical_size(),
                [ox + 125.0, oy + 46.0, 16.0, 16.0], u32::MAX);
        }
        if hovered {
            let _ = super::crop(batch, sprite,
                [176.0, 202.0, 18.0, 18.0], sprite.logical_size(),
                [ox + 124.0, oy + 69.0, 18.0, 18.0], u32::MAX);
        }
        let _ = super::crop(batch, sprite,
            [176.0 + f32::from(mode) * 16.0, 186.0, 16.0, 16.0], sprite.logical_size(),
            [ox + 125.0, oy + 70.0, 16.0, 16.0], u32::MAX);
    }
    let cols = layout::columns(count);
    let rows = layout::visible_rows(count);
    let max = layout::max_offset(count);
    let width = layout::panel_width(count);
    if cols > 0 {
        let left = layout::panel_left(count);
        let inner_width = width - 14.0;
        let inner_height = rows as f32 * 18.0;
        if let Some(sprite) = regions.get("gui/tconstruct_generic").map(super::Sprite) {
            let region = |batch: &mut HudBatch, sx: f32, sy: f32, sw: f32, sh: f32, x: f32, y: f32, w: f32, h: f32| {
                let _ = super::crop(batch, sprite,
                    [sx, sy, sw, sh], sprite.logical_size(),
                    [ox+left+x, oy+4.0+y, w, h], u32::MAX);
            };
            for (sx, sy, x, y) in [(0.0,0.0,0.0,0.0),(57.0,0.0,inner_width+7.0,0.0),
                (0.0,57.0,0.0,inner_height+7.0),(57.0,57.0,inner_width+7.0,inner_height+7.0)] {
                region(batch,sx,sy,7.0,7.0,x,y,7.0,7.0);
            }
            region(batch,7.0,0.0,50.0,7.0,7.0,0.0,inner_width,7.0);
            region(batch,7.0,57.0,50.0,7.0,7.0,inner_height+7.0,inner_width,7.0);
            region(batch,0.0,7.0,7.0,50.0,0.0,7.0,7.0,inner_height);
            region(batch,57.0,7.0,7.0,50.0,inner_width+7.0,7.0,7.0,inner_height);
            if max > 0 {
                let x = 7.0 + cols as f32 * 22.0;
                region(batch,43.0,8.0,12.0,30.0,x,7.0,10.0,inner_height);
                let progress = offset.min(max) as f32 / max as f32;
                region(batch,7.0,25.0,10.0,15.0,x,7.0+progress*(inner_height-15.0).max(0.0),10.0,15.0);
            }
        }
        if let Some(sprite) = regions.get("gui/tconstruct_heating_structure").map(super::Sprite) {
            let first = offset.min(max) * cols;
            for i in 0..rows*cols {
                let _ = super::crop(batch, sprite,
                    [if first+i < count { 0.0 } else { 22.0 }, 238.0, 22.0, 18.0], sprite.logical_size(),
                    [ox+left+7.0+(i%cols) as f32*22.0, oy+11.0+(i/cols) as f32*18.0, 22.0, 18.0], u32::MAX);
            }
        }
    }
}
