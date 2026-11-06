// Additional frozen crop bodies from horizon-mods a10de5e0.
// Parameters replace only atlas lookup and caller state. See crop-oracle-sources.json.
use horizon_client::hud::{HudBatch, Rect};

pub fn heat(batch: &mut HudBatch, mut sprite: Rect, x: f32, y: f32, source_x: f32, height: f32) {
    if height <= 0.0 { return; }
    let (du, dv) = ((sprite.u1 - sprite.u0) / sprite.w as f32, (sprite.v1 - sprite.v0) / sprite.h as f32);
    sprite.u0 += source_x * du;
    sprite.v0 += (150.0 + 16.0 - height) * dv;
    sprite.u1 = sprite.u0 + 3.0 * du; sprite.v1 = sprite.v0 + height * dv;
    batch.rect_uv(x - 4.0, y + 16.0 - height, 3.0, height, sprite, u32::MAX);
}

pub fn fire(batch: &mut HudBatch, mut sprite: Rect, x: f32, y: f32, height: f32) {
    if height == 0.0 { return; }
    let (du, dv) = ((sprite.u1 - sprite.u0) / sprite.w as f32, (sprite.v1 - sprite.v0) / sprite.h as f32);
    sprite.u0 += 176.0 * du; sprite.v0 += (136.0 + 14.0 - height) * dv;
    sprite.u1 = sprite.u0 + 14.0 * du; sprite.v1 = sprite.v0 + height * dv;
    batch.rect_uv(x, y + 14.0 - height, 14.0, height, sprite, u32::MAX);
}

pub fn panel(batch: &mut HudBatch, mut sprite: Rect, ox: f32, oy: f32, has_heater: bool) {
    let du = (sprite.u1 - sprite.u0) / sprite.w as f32;
    let dv = (sprite.v1 - sprite.v0) / sprite.h as f32;
    let (x, width, height, sx) = if has_heater { (150.0, 18.0, 36.0, 176.0) } else { (152.0, 14.0, 38.0, 194.0) };
    sprite.u0 += sx * du; sprite.v0 += 52.0 * dv;
    sprite.u1 = sprite.u0 + width * du; sprite.v1 = sprite.v0 + height * dv;
    batch.rect_uv(ox + x, oy + 31.0, width, height, sprite, u32::MAX);
}

pub fn melter_gauge(batch: &mut HudBatch, mut gauge: Rect, ox: f32, oy: f32) {
    let (du, dv) = ((gauge.u1 - gauge.u0) / gauge.w as f32, (gauge.v1 - gauge.v0) / gauge.h as f32);
    gauge.u0 += 176.0 * du;
    gauge.u1 = gauge.u0 + 52.0 * du; gauge.v1 = gauge.v0 + 52.0 * dv;
    batch.rect_uv(ox + 90.0, oy + 16.0, 52.0, 52.0, gauge, u32::MAX);
}

pub fn smeltery_gauge(batch: &mut HudBatch, mut gauge: Rect, ox: f32, oy: f32) {
    let (du, dv) = ((gauge.u1 - gauge.u0) / gauge.w as f32, (gauge.v1 - gauge.v0) / gauge.h as f32);
    gauge.u0 += 176.0 * du;
    gauge.u1 = gauge.u0 + 80.0 * du; gauge.v1 = gauge.v0 + 106.0 * dv;
    batch.rect_uv(ox + 8.0, oy + 16.0, 80.0, 106.0, gauge, u32::MAX);
}
