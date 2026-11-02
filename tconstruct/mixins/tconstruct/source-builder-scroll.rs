fn builder_scroll(game: &mut crate::game::Game<'_>, delta: f32) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    if let Some((_,count,offset,_)) = crate::game::tconstruct_crafting_station::side(game) {
        let Some(pos) = crate::game::tconstruct_crafting_station::position(game) else { return Continue(()); };
        let Some(size) = game.renderer.as_ref().map(|r| [r.config.width,r.config.height]) else { return Continue(()); };
        let batch = horizon_client::hud::HudBatch::new(size); let (ox,oy) = game.inventory_origin(&batch);
        let left = crate::game::tconstruct_source_side_layout::left(count);
        let width = crate::game::tconstruct_source_side_layout::columns(count) as f32 * 18.0 + 24.0;
        let height = crate::game::tconstruct_source_side_layout::visible_rows(count) as f32 * 18.0 + 24.0;
        if (ox+left..ox+left+width).contains(&game.cursor.0) && (oy..oy+height).contains(&game.cursor.1) {
            let max = crate::game::tconstruct_source_side_layout::max_offset(count);
            game.world.chunks.__horizon_mixins_tconstruct_builder_side_scroll_mut().entry(pos).or_default().0 = (offset as i32-delta.signum() as i32).clamp(0,max as i32) as usize;
            return Break(());
        }
    }
    if crate::game::tconstruct_chest_ui::kind(game).is_some() {
        let Some(pos) = game.container else { return Break(()); };
        let Some((count, scroll)) = crate::game::tconstruct_chest_ui::page(game) else { return Break(()); };
        let Some(size) = game.renderer.as_ref().map(|r| [r.config.width,r.config.height]) else { return Break(()); };
        let batch=horizon_client::hud::HudBatch::new(size); let (ox,oy)=game.inventory_origin(&batch);
        let (x,y)=(game.cursor.0-ox,game.cursor.1-oy);
        if !(7.0..169.0).contains(&x) || !(17.0..89.0).contains(&y) { return Continue(()); }
        if let Some(index)=crate::game::tconstruct_chest_rules::hit_slot(count,scroll,x,y) {
            let lookup=if game.remote.is_some() { index-scroll*crate::game::tconstruct_chest_rules::CHEST_COLUMNS } else { index };
            if game.container.and_then(|key|game.world.block_entities.get(&key)).and_then(|e|e.slots().get(lookup)).is_some_and(Option::is_some) { return Continue(()); }
        }
        let max=crate::game::tconstruct_chest_rules::max_scroll(count);
        let next=(scroll as i32-delta.signum() as i32).clamp(0,max as i32) as usize;
        crate::game::tconstruct_chest_ui::set_scroll(game,next);
        return Break(());
    }
    if crate::game::tconstruct_ui::kind(game) == Some(true) {
        let Some(pos) = game.container else { return Continue(()); };
        let Some(size) = game.renderer.as_ref().map(|r| [r.config.width, r.config.height]) else { return Continue(()); };
        let batch = horizon_client::hud::HudBatch::new(size);
        let (ox, oy) = game.inventory_origin(&batch);
        if !(ox + 182.0..ox + 182.0 + crate::game::tconstruct_builder_info_ui::panel_width(game, true)).contains(&game.cursor.0) { return Continue(()); }
        let Some((top, bottom)) = crate::game::tconstruct_station_info_ui::lines(game) else { return Continue(()); };
        let state = game.world.chunks.__horizon_mixins_tconstruct_station_info_scroll_mut().entry(pos).or_default();
        let (offset, count, visible) = if (oy + 5.0..oy + 88.0).contains(&game.cursor.1) { (&mut state.0, top.len(), crate::game::tconstruct_station_info_ui::TOP_VISIBLE) }
            else if (oy + 94.0..oy + 177.0).contains(&game.cursor.1) { (&mut state.1, bottom.len(), crate::game::tconstruct_station_info_ui::BOTTOM_VISIBLE) }
            else { return Continue(()); };
        *offset = (*offset as i32 - delta.signum() as i32).clamp(0, count.saturating_sub(visible) as i32) as usize;
        return Break(());
    }
    if crate::game::tconstruct_ui::kind(game) != Some(false) { return Continue(()); }
    let Some(pos) = game.container else { return Break(()); };
    if let Some(size) = game.renderer.as_ref().map(|renderer| [renderer.config.width, renderer.config.height]) {
        let batch = horizon_client::hud::HudBatch::new(size);
        let (ox, oy) = game.inventory_origin(&batch);
        let count = crate::game::tconstruct_ui::side_count(game);
        let columns = crate::game::tconstruct_source_side_layout::columns(count);
        let max = crate::game::tconstruct_source_side_layout::max_offset(count);
        let left = crate::game::tconstruct_source_side_layout::left(count);
        let width = columns as f32 * 18.0 + 24.0;
        let height = crate::game::tconstruct_source_side_layout::visible_rows(count) as f32 * 18.0 + 24.0;
        if max > 0 && (ox + left..ox + left + width).contains(&game.cursor.0) && (oy..oy + height).contains(&game.cursor.1) {
            let current=game.world.chunks.__horizon_mixins_tconstruct_builder_side_scroll().get(&pos).map_or(0,|s|s.0);
            let next=(current as i32-delta.signum() as i32).clamp(0,max as i32) as usize;
            crate::game::tconstruct_ui::set_side_scroll(game,next);
            return Break(());
        }
        if (ox + 180.0..ox + 180.0 + crate::game::tconstruct_builder_info_ui::panel_width(game, false)).contains(&game.cursor.0) && (oy..oy + 184.0).contains(&game.cursor.1) {
            let count = crate::game::tconstruct_builder_info_ui::lines(game, &game.gui.atlas).len();
            let scroll = game.world.chunks.__horizon_mixins_tconstruct_builder_info_scroll_mut().entry(pos).or_default();
            *scroll = (*scroll as i32 - delta.signum() as i32).clamp(0, count.saturating_sub(crate::game::tconstruct_builder_info_ui::VISIBLE_LINES) as i32) as usize;
            return Break(());
        }
    }
    let count = game.world.chunks.__horizon_mixins_tconstruct_builder_choices().get(&pos)
        .and_then(|(_, _, list)| list.as_ref()).map_or(0, Vec::len);
    let state = game.world.chunks.__horizon_mixins_tconstruct_builder_scroll_mut().entry(pos)
        .or_insert_with(|| (0.0, 0, false, Vec::new()));
    state.0 = crate::game::tconstruct_source_builder_layout::wheel(count, state.0, f64::from(delta));
    state.1 = crate::game::tconstruct_source_builder_layout::offset(count, state.0);
    Break(())
}
