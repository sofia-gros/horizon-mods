use super::*;
pub(super) fn draw(game: &Game<'_>, batch: &mut horizon_client::hud::HudBatch, ox: f32, oy: f32, count: usize, offset: usize, title: &str) {
    let atlas = &game.gui.atlas;
    let columns = crate::game::tconstruct_source_side_layout::columns(count);
    let left = crate::game::tconstruct_source_side_layout::left(count);
    let scrollable = crate::game::tconstruct_source_side_layout::max_offset(count) > 0;
    let width = columns as f32 * 18.0 + if scrollable { 10.0 } else { 0.0 };
    let slots_height = crate::game::tconstruct_source_side_layout::visible_rows(count) as f32 * 18.0;
    let height = slots_height + 10.0;
    if let Some(sprite) = atlas.sprites.get("gui/tconstruct_generic").copied() {
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
        batch.text(atlas, title, ox + left + 7.0, oy + 6.0, horizon_client::hud::rgba(64, 64, 64, 255), false);
        let first = offset.min(crate::game::tconstruct_source_side_layout::max_offset(count)) * columns;
        for i in 0..count.saturating_sub(first).min(crate::game::tconstruct_source_side_layout::visible_rows(count) * columns) {
            region(batch, 7.0, 7.0, 18.0, 18.0,
                7.0 + (i % columns) as f32 * 18.0, 17.0 + (i / columns) as f32 * 18.0, 18.0, 18.0);
        }
        let shown = count.saturating_sub(first).min(crate::game::tconstruct_source_side_layout::visible_rows(count) * columns);
        if shown % columns != 0 {
            for i in shown..shown.div_ceil(columns) * columns {
                region(batch, 25.0, 7.0, 18.0, 18.0,
                    7.0 + (i % columns) as f32 * 18.0, 17.0 + (i / columns) as f32 * 18.0, 18.0, 18.0);
            }
        }
        if scrollable {
            let x = 7.0 + columns as f32 * 18.0;
            region(batch, 43.0, 8.0, 12.0, 30.0, x, 17.0, 10.0, slots_height);
            let progress = offset.min(crate::game::tconstruct_source_side_layout::max_offset(count)) as f32
                / crate::game::tconstruct_source_side_layout::max_offset(count) as f32;
            region(batch, 7.0, 25.0, 10.0, 15.0, x, 17.0 + progress * (slots_height - 15.0), 10.0, 15.0);
        }
    }
}
