// Frozen old/new crop blocks with explicit input substitutions, not renderer mocks.
use horizon_client::hud::{HudBatch,Rect};
use std::collections::HashMap;
#[derive(Clone,Copy)]
pub struct Inputs {pub ox:f32,pub oy:f32,pub source:[f32;4],pub destination:[f32;4],pub color:u32,pub selected:bool,pub hovered:bool,pub scrollable:bool,pub progress:f32,pub empty:[bool;2]}
#[allow(unused_variables)]
pub fn old_worktable(batch: &mut HudBatch, regions:&HashMap<String,Rect>, texture:&str, input:Inputs) {
    let Inputs {ox,oy,source:[sx,sy,sw,sh],destination:[x,y,w,h],color,selected,hovered,scrollable,progress,empty}=input;
if let Some(mut region) = regions.get("gui/tconstruct_worktable").copied() {
                    let (du, dv) = ((region.u1 - region.u0) / region.w as f32, (region.v1 - region.v0) / region.h as f32);
                    region.u0 += 176.0 * du; region.v0 += (if selected { 33.0 } else { 15.0 }) * dv;
                    region.u1 = region.u0 + 18.0 * du; region.v1 = region.v0 + 18.0 * dv;
                    batch.rect_uv(x, y, 18.0, 18.0, region, u32::MAX);
                }
}

#[allow(unused_variables)]
pub fn old_part(batch: &mut HudBatch, regions:&HashMap<String,Rect>, texture:&str, input:Inputs) {
    let Inputs {ox,oy,source:[sx,sy,sw,sh],destination:[x,y,w,h],color,selected,hovered,scrollable,progress,empty}=input;
if let Some(mut r) = regions.get(texture).copied() {
            let (du, dv) = ((r.u1 - r.u0) / r.w as f32, (r.v1 - r.v0) / r.h as f32);
            r.u0 += sx * du; r.v0 += sy * dv;
            r.u1 = r.u0 + 18.0 * du; r.v1 = r.v0 + 18.0 * dv;
            batch.rect_uv(ox + x, oy + y, 18.0, 18.0, r, color);
        }
}

#[allow(unused_variables)]
pub fn old_cover(batch: &mut HudBatch, regions:&HashMap<String,Rect>, texture:&str, input:Inputs) {
    let Inputs {ox,oy,source:[sx,sy,sw,sh],destination:[x,y,w,h],color,selected,hovered,scrollable,progress,empty}=input;
if let Some(mut r) = regions.get(texture).copied() {
            let (du, dv) = ((r.u1 - r.u0) / r.w as f32, (r.v1 - r.v0) / r.h as f32);
            r.u0 += 176.0 * du; r.v0 += 18.0 * dv;
            r.u1 = r.u0 + 70.0 * du; r.v1 = r.v0 + 64.0 * dv;
            batch.rect_uv(ox + 7.0, oy + 18.0, 70.0, 64.0, r, horizon_client::hud::rgba(255, 255, 255, 209));
        }
}

#[allow(unused_variables)]
pub fn old_name_field(batch: &mut HudBatch, regions:&HashMap<String,Rect>, texture:&str, input:Inputs) {
    let Inputs {ox,oy,source:[sx,sy,sw,sh],destination:[x,y,w,h],color,selected,hovered,scrollable,progress,empty}=input;
if let Some(mut r) = regions.get(texture).copied() {
                let (du, dv) = ((r.u1 - r.u0) / r.w as f32, (r.v1 - r.v0) / r.h as f32);
                r.v0 += 244.0 * dv; r.u1 = r.u0 + 90.0 * du; r.v1 = r.v0 + 12.0 * dv;
                batch.rect_uv(ox + 79.0, oy + 5.0, 90.0, 12.0, r, u32::MAX);
            }
}

#[allow(unused_variables)]
pub fn old_button(batch: &mut HudBatch, regions:&HashMap<String,Rect>, texture:&str, input:Inputs) {
    let Inputs {ox,oy,source:[sx,sy,sw,sh],destination:[x,y,w,h],color,selected,hovered,scrollable,progress,empty}=input;
if let Some(mut r) = regions.get("gui/tconstruct_icons").copied() {
                let (du, dv) = ((r.u1 - r.u0) / r.w as f32, (r.v1 - r.v0) / r.h as f32);
                r.u0 += (if selected { 144.0 } else if hovered { 216.0 } else { 180.0 }) * du;
                r.v0 += 216.0 * dv;
                r.u1 = r.u0 + 18.0 * du; r.v1 = r.v0 + 18.0 * dv;
                batch.rect_uv(ox + x, oy + y, 18.0, 18.0, r, u32::MAX);
            }
}

#[allow(unused_variables)]
pub fn old_empty_icons(batch: &mut HudBatch, regions:&HashMap<String,Rect>, texture:&str, input:Inputs) {
    let Inputs {ox,oy,source:[sx,sy,sw,sh],destination:[x,y,w,h],color,selected,hovered,scrollable,progress,empty}=input;
if let Some(icon) = regions.get("gui/tconstruct_icons").copied() {
                let (du, dv) = ((icon.u1 - icon.u0) / icon.w as f32, (icon.v1 - icon.v0) / icon.h as f32);
                for (slot, sx, sy, x) in [(0, 0.0, 216.0, 7.0), (1, 54.0, 234.0, 28.0)] {
                    if empty[slot] {
                        let mut r = icon;
                        r.u0 += sx * du; r.v0 += sy * dv; r.u1 = r.u0 + 18.0 * du; r.v1 = r.v0 + 18.0 * dv;
                        batch.rect_uv(ox + x, oy + 42.0, 18.0, 18.0, r, u32::MAX);
                    }
                }
            }
}

#[allow(unused_variables)]
pub fn old_scroll(batch: &mut HudBatch, regions:&HashMap<String,Rect>, texture:&str, input:Inputs) {
    let Inputs {ox,oy,source:[sx,sy,sw,sh],destination:[x,y,w,h],color,selected,hovered,scrollable,progress,empty}=input;
if let Some(mut r) = regions.get(texture).copied() {
                    let (du, dv) = ((r.u1 - r.u0) / r.w as f32, (r.v1 - r.v0) / r.h as f32);
                    r.u0 += if scrollable { 176.0 * du } else { 188.0 * du };
                    r.u1 = r.u0 + 12.0 * du; r.v1 = r.v0 + 15.0 * dv;
                    batch.rect_uv(ox + 126.0, oy + 15.0 + (59.0 * progress.clamp(0.0, 1.0)) as i32 as f32, 12.0, 15.0, r, u32::MAX);
                }
}

#[allow(unused_variables)]
pub fn old_region(batch: &mut HudBatch, regions:&HashMap<String,Rect>, texture:&str, input:Inputs) {
    let Inputs {ox,oy,source:[sx,sy,sw,sh],destination:[x,y,w,h],color,selected,hovered,scrollable,progress,empty}=input;
if let Some(sprite) = regions.get(texture).copied() {
                let mut r = sprite;
                let (du, dv) = ((r.u1 - r.u0) / r.w as f32, (r.v1 - r.v0) / r.h as f32);
                r.u0 += sx * du; r.v0 += sy * dv; r.u1 = r.u0 + sw * du; r.v1 = r.v0 + sh * dv;
                batch.rect_uv(ox + x, oy + y, w, h, r, u32::MAX);
            }
}

#[allow(unused_variables)]
pub fn new_worktable(batch: &mut HudBatch, regions:&HashMap<String,Rect>, texture:&str, input:Inputs) {
    let Inputs {ox,oy,source:[sx,sy,sw,sh],destination:[x,y,w,h],color,selected,hovered,scrollable,progress,empty}=input;
if let Some(region) = regions.get("gui/tconstruct_worktable").map(super::Sprite) {
                    let _ = super::crop(batch, region,
                        [176.0, if selected { 33.0 } else { 15.0 }, 18.0, 18.0], region.logical_size(),
                        [x, y, 18.0, 18.0], u32::MAX);
                }
}

#[allow(unused_variables)]
pub fn new_part(batch: &mut HudBatch, regions:&HashMap<String,Rect>, texture:&str, input:Inputs) {
    let Inputs {ox,oy,source:[sx,sy,sw,sh],destination:[x,y,w,h],color,selected,hovered,scrollable,progress,empty}=input;
if let Some(region) = regions.get(texture).map(super::Sprite) {
            let _ = super::crop(batch, region,
                [sx, sy, 18.0, 18.0], region.logical_size(),
                [ox + x, oy + y, 18.0, 18.0], color);
        }
}

#[allow(unused_variables)]
pub fn new_cover(batch: &mut HudBatch, regions:&HashMap<String,Rect>, texture:&str, input:Inputs) {
    let Inputs {ox,oy,source:[sx,sy,sw,sh],destination:[x,y,w,h],color,selected,hovered,scrollable,progress,empty}=input;
if let Some(region) = regions.get(texture).map(super::Sprite) {
            let _ = super::crop(batch, region,
                [176.0, 18.0, 70.0, 64.0], region.logical_size(),
                [ox + 7.0, oy + 18.0, 70.0, 64.0], horizon_client::hud::rgba(255, 255, 255, 209));
        }
}

#[allow(unused_variables)]
pub fn new_name_field(batch: &mut HudBatch, regions:&HashMap<String,Rect>, texture:&str, input:Inputs) {
    let Inputs {ox,oy,source:[sx,sy,sw,sh],destination:[x,y,w,h],color,selected,hovered,scrollable,progress,empty}=input;
if let Some(region) = regions.get(texture).map(super::Sprite) {
                let _ = super::crop(batch, region,
                    [0.0, 244.0, 90.0, 12.0], region.logical_size(),
                    [ox + 79.0, oy + 5.0, 90.0, 12.0], u32::MAX);
            }
}

#[allow(unused_variables)]
pub fn new_button(batch: &mut HudBatch, regions:&HashMap<String,Rect>, texture:&str, input:Inputs) {
    let Inputs {ox,oy,source:[sx,sy,sw,sh],destination:[x,y,w,h],color,selected,hovered,scrollable,progress,empty}=input;
if let Some(region) = regions.get("gui/tconstruct_icons").map(super::Sprite) {
                let _ = super::crop(batch, region,
                    [if selected { 144.0 } else if hovered { 216.0 } else { 180.0 }, 216.0, 18.0, 18.0], region.logical_size(),
                    [ox + x, oy + y, 18.0, 18.0], u32::MAX);
            }
}

#[allow(unused_variables)]
pub fn new_empty_icons(batch: &mut HudBatch, regions:&HashMap<String,Rect>, texture:&str, input:Inputs) {
    let Inputs {ox,oy,source:[sx,sy,sw,sh],destination:[x,y,w,h],color,selected,hovered,scrollable,progress,empty}=input;
if let Some(icon) = regions.get("gui/tconstruct_icons").map(super::Sprite) {
                for (slot, sx, sy, x) in [(0, 0.0, 216.0, 7.0), (1, 54.0, 234.0, 28.0)] {
                    if empty[slot] {
                        let _ = super::crop(batch, icon,
                            [sx, sy, 18.0, 18.0], icon.logical_size(),
                            [ox + x, oy + 42.0, 18.0, 18.0], u32::MAX);
                    }
                }
            }
}

#[allow(unused_variables)]
pub fn new_scroll(batch: &mut HudBatch, regions:&HashMap<String,Rect>, texture:&str, input:Inputs) {
    let Inputs {ox,oy,source:[sx,sy,sw,sh],destination:[x,y,w,h],color,selected,hovered,scrollable,progress,empty}=input;
if let Some(region) = regions.get(texture).map(super::Sprite) {
                    let _ = super::crop(batch, region,
                        [if scrollable { 176.0 } else { 188.0 }, 0.0, 12.0, 15.0], region.logical_size(),
                        [ox + 126.0, oy + 15.0 + (59.0 * progress.clamp(0.0, 1.0)) as i32 as f32, 12.0, 15.0], u32::MAX);
                }
}

#[allow(unused_variables)]
pub fn new_region(batch: &mut HudBatch, regions:&HashMap<String,Rect>, texture:&str, input:Inputs) {
    let Inputs {ox,oy,source:[sx,sy,sw,sh],destination:[x,y,w,h],color,selected,hovered,scrollable,progress,empty}=input;
if let Some(sprite) = regions.get(texture).map(super::Sprite) {
                let _ = super::crop(batch, sprite,
                    [sx, sy, sw, sh], sprite.logical_size(),
                    [ox + x, oy + y, w, h], u32::MAX);
            }
}
