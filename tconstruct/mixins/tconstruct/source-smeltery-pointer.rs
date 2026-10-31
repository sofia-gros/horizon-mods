fn smeltery_pointer(game: &mut crate::game::Game<'_>, right: bool) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    if !crate::game::tconstruct_source_smeltery_ui::is_smeltery(game) { return Continue(()); }
    let Some(pos) = crate::game::tconstruct_source_smeltery_ui::position(game) else { return Continue(()); };
    let Some(size) = game.renderer.as_ref().map(|r| [r.config.width, r.config.height]) else { return Continue(()); };
    let batch = horizon_client::hud::HudBatch::new(size);
    let (ox, oy) = game.inventory_origin(&batch);
    if !right && (ox + 124.0..ox + 142.0).contains(&game.cursor.0) && (oy + 69.0..oy + 87.0).contains(&game.cursor.1) {
        if let Some((window, _)) = game.remote.as_ref().and_then(|remote| remote.menu.as_ref()) {
            game.send(horizon_protocol::packets::play::Serverbound::ContainerButtonClick(horizon_protocol::packets::play::ContainerButtonClick { container: *window, button: 0 }));
        } else if let Some(mut state) = crate::game::tconstruct_source_melter_store::read(&game.world, pos, "tconstruct:smeltery")
            .and_then(|tag| crate::game::tconstruct_source_smeltery_state::Smeltery::from_nbt(&tag)) {
            let (mut slots, mode) = game.world.chunks.__horizon_mixins_tconstruct_smeltery_local_containers().get(&pos).cloned().unwrap_or(([None, None], 0));
            let direction = crate::game::tconstruct_source_fluid_container::Direction::from_index(mode).next();
            let known_fluids = game.world.chunks.__horizon_mixins_tconstruct_tank_fluid_context().as_ref()
                .and_then(|value| value.as_ref().ok()).map(|(fluids, _)| fluids.clone()).unwrap_or_default();
            crate::game::tconstruct_source_fluid_container::transfer(&mut state.tank, &mut slots, direction,
                &|id| game.res.items().get(id).is_some(), &|id| known_fluids.contains(id));
            if crate::game::tconstruct_source_melter_store::write(&mut game.world, pos, "tconstruct:smeltery", state.to_nbt()) {
                game.world.chunks.__horizon_mixins_tconstruct_smeltery_local_containers_mut().insert(pos, (slots.clone(), direction.index()));
                let (_, offset) = crate::game::tconstruct_source_smeltery_ui::page(game);
                if let Some(horizon_client::containers::BlockEntity::Chest(live)) = game.world.block_entities.get_mut(&pos) {
                    *live = crate::game::tconstruct_source_smeltery_page::project_with_containers(&state.slots, offset, &slots);
                }
            }
        }
        return Break(true);
    }
    let (count, _) = crate::game::tconstruct_source_smeltery_ui::page(game);
    let columns = crate::game::tconstruct_source_smeltery_layout::columns(count);
    let max = crate::game::tconstruct_source_smeltery_layout::max_offset(count);
    let left = -crate::game::tconstruct_source_smeltery_layout::panel_width(count);
    let x = ox + left + 7.0 + columns as f32 * 22.0;
    let height = crate::game::tconstruct_source_smeltery_layout::visible_rows(count) as f32 * 18.0;
    if !right && max > 0 && (x..x+10.0).contains(&game.cursor.0) && (oy+11.0..oy+11.0+height).contains(&game.cursor.1) {
        game.world.chunks.__horizon_mixins_tconstruct_smeltery_ui_dragging_mut().insert(pos);
        let next = crate::game::tconstruct_source_smeltery_layout::drag_offset(count, game.cursor.1 - oy);
        crate::game::tconstruct_source_smeltery_ui::set_offset(game, pos, count, next);
        return Break(true);
    }
    if !(ox + 8.0..ox + 114.0).contains(&game.cursor.0) || !(oy + 16.0..oy + 122.0).contains(&game.cursor.1) { return Continue(()); }
    // Filling a held container from a selected fluid has not been connected;
    // do not move or consume the carried item until that transaction exists.
    if game.carried.is_some() { return Break(true); }
    let Some(tank) = crate::game::tconstruct_source_melter_store::read(&game.world, pos, "tconstruct:smeltery")
        .and_then(|tag| tag.get("tank").and_then(crate::game::tconstruct_source_smeltery_tank::SmelteryTank::from_nbt)) else { return Break(true); };
    let amounts = tank.fluids.iter().map(|fluid| fluid.amount).collect::<Vec<_>>();
    let Some(index) = crate::game::tconstruct_source_smeltery_layout::fluid_at(&amounts, tank.capacity, game.cursor.1 - oy) else { return Break(true); };
    if let Some((window, _)) = game.remote.as_ref().and_then(|remote| remote.menu.as_ref()) {
        game.send(horizon_protocol::packets::play::Serverbound::ContainerButtonClick(horizon_protocol::packets::play::ContainerButtonClick { container: *window, button: 4 + index as i32 }));
    } else if let Some(mut state) = crate::game::tconstruct_source_melter_store::read(&game.world, pos, "tconstruct:smeltery")
        .and_then(|tag| crate::game::tconstruct_source_smeltery_state::Smeltery::from_nbt(&tag)) {
        if state.tank.move_to_bottom(index) { let _ = crate::game::tconstruct_source_melter_store::write(&mut game.world, pos, "tconstruct:smeltery", state.to_nbt()); }
    }
    Break(true)
}
