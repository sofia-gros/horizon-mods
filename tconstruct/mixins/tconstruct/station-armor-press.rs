fn station_armor_press(game: &mut crate::game::Game<'_>, right: bool) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    if right || crate::game::tconstruct_ui::kind(game) != Some(true)
        || crate::game::tconstruct_ui::station_layouts(game).len() > 36 { return Continue(()); }
    let Some(pos) = game.container else { return Continue(()); };
    let Some(size) = game.renderer.as_ref().map(|r| [r.config.width, r.config.height]) else { return Continue(()); };
    let batch = horizon_client::hud::HudBatch::new(size);
    let (ox, oy) = game.inventory_origin(&batch);
    let (cx, cy) = (ox - 55.0, oy + 195.0);
    if (cx - 32.5..cx + 32.5).contains(&game.cursor.0) && (cy - 65.0..cy + 5.0).contains(&game.cursor.1) {
        let preview = game.world.chunks.__horizon_mixins_tconstruct_station_armor_preview_mut().entry(pos).or_insert((0.0, None));
        preview.1 = Some(game.cursor.0);
        return Break(());
    }
    Continue(())
}
