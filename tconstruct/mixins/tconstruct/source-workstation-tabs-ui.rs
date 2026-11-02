use super::*;
pub(super) fn draw(game: &Game<'_>,batch: &mut horizon_client::hud::HudBatch,ox: f32,oy: f32,station: bool) {
    let atlas=&game.gui.atlas;
    if let Some(start) = crate::game::tconstruct_source_workstation_tabs::start(game) {
        let tabs = crate::game::tconstruct_source_workstation_tabs::from_game(game, start);
        if let Some(icon) = atlas.sprites.get("gui/tconstruct_icons").copied() {
            let (du, dv) = ((icon.u1 - icon.u0) / icon.w as f32, (icon.v1 - icon.v0) / icon.h as f32);
            for (index, (pos, name)) in tabs.iter().enumerate() {
                let (x, y) = crate::game::tconstruct_source_workstation_tabs::position(index, station);
                let mut region = icon;
                region.u0 += if *pos == start { 52.0 * du } else { 0.0 };
                region.v0 += 18.0 * dv;
                region.u1 = region.u0 + 26.0 * du; region.v1 = region.v0 + 30.0 * dv;
                batch.rect_uv(ox + x, oy + y, 26.0, 30.0, region, u32::MAX);
                if game.res.items().get(name).is_some() {
                    game.draw_item(batch, &horizon_items::items::ItemStack::new(name, 1), ox + x + 5.0, oy + y + 7.0);
                }
                if !crate::game::tconstruct_source_workstation_tabs::supported(name) {
                    batch.text(atlas, "!", ox + x + 20.0, oy + y + 18.0, horizon_client::hud::rgba(255, 255, 85, 255), true);
                }
            }
        }
    }
}
