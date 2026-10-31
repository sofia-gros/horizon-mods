fn smeltery_scroll(game: &mut crate::game::Game<'_>, delta: f32) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    if !crate::game::tconstruct_source_smeltery_ui::is_smeltery(game) { return Continue(()); }
    let Some(pos) = crate::game::tconstruct_source_smeltery_ui::position(game) else { return Break(()); };
    let (count, offset) = crate::game::tconstruct_source_smeltery_ui::page(game);
    let max = crate::game::tconstruct_source_smeltery_layout::max_offset(count);
    if max == 0 || delta == 0.0 { return Break(()); }
    let Some(size) = game.renderer.as_ref().map(|r| [r.config.width, r.config.height]) else { return Break(()); };
    let batch = horizon_client::hud::HudBatch::new(size);
    let (ox, oy) = game.inventory_origin(&batch);
    let left = -crate::game::tconstruct_source_smeltery_layout::panel_width(count);
    let height = crate::game::tconstruct_source_smeltery_layout::visible_rows(count) as f32 * 18.0 + 14.0;
    if !(ox + left..ox).contains(&game.cursor.0) || !(oy + 4.0..oy + 4.0 + height).contains(&game.cursor.1) { return Continue(()); }
    if game.remote.is_some() {
        if let Some((window, _)) = game.remote.as_ref().and_then(|remote| remote.menu.as_ref()) {
            let button = if delta > 0.0 { 32000 } else { 32001 };
            game.send(horizon_protocol::packets::play::Serverbound::ContainerButtonClick(horizon_protocol::packets::play::ContainerButtonClick { container: *window, button }));
        }
    } else {
        let next = (offset as i32 - delta.signum() as i32).clamp(0, max as i32) as usize;
        let Some(state) = crate::game::tconstruct_source_melter_store::read(&game.world, pos, "tconstruct:smeltery")
            .and_then(|tag| crate::game::tconstruct_source_smeltery_state::Smeltery::from_nbt(&tag)) else { return Break(()); };
        if let Some(horizon_client::containers::BlockEntity::Chest(slots)) = game.world.block_entities.get_mut(&pos) {
            *slots = crate::game::tconstruct_source_smeltery_page::project(&state.slots, next);
            game.world.chunks.__horizon_mixins_tconstruct_smeltery_ui_pages_mut().insert(pos, (count, next));
        }
    }
    Break(())
}
