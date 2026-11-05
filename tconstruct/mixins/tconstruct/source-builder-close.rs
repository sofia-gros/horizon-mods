fn builder_close(game: &mut crate::game::Game<'_>) {
    if crate::game::tconstruct_chest_ui::kind(game).is_some() {
        if let Some(pos) = crate::game::remote::tconstruct_source_machine_remote::position(game).or(game.container) {
            game.world.chunks.__horizon_mixins_tconstruct_chest_ui_pages_mut().remove(&pos);
        }
        if let Some(pos)=game.container { game.world.chunks.__horizon_mixins_tconstruct_builder_side_scroll_mut().remove(&pos); }
        return;
    }
    if crate::game::tconstruct_source_smeltery_ui::is_smeltery(game) {
        if let Some(pos) = crate::game::tconstruct_source_smeltery_ui::position(game) {
            game.world.chunks.__horizon_mixins_tconstruct_smeltery_closed_projection_mut().remove(&pos);
            if game.remote.is_none() {
                let offset = game.world.chunks.__horizon_mixins_tconstruct_smeltery_ui_pages().get(&pos).map(|page| page.1);
                let containers = game.world.chunks.__horizon_mixins_tconstruct_smeltery_local_containers().get(&pos)
                    .map(|(slots, _)| slots.clone()).unwrap_or([None, None]);
                if let Some(raw) = crate::game::tconstruct_source_melter_store::read_outer(&game.world, pos) {
                    let cleared = match game.world.block_entities.get_mut(&pos) {
                        Some(horizon_client::containers::BlockEntity::Chest(live)) =>
                            crate::game::tconstruct_source_machine_destroy_guard::close_projection(&raw, live, offset, &containers),
                        _ => false,
                    };
                    if cleared { game.world.chunks.__horizon_mixins_tconstruct_smeltery_closed_projection_mut().insert(pos); }
                }
            }
            if let Some((containers, _)) = game.world.chunks.__horizon_mixins_tconstruct_smeltery_local_containers_mut().remove(&pos) {
                if game.remote.is_none() {
                    for stack in containers.into_iter().flatten() { game.give_or_drop(stack); }
                }
            }
            game.world.chunks.__horizon_mixins_tconstruct_smeltery_ui_pages_mut().remove(&pos);
            game.world.chunks.__horizon_mixins_tconstruct_smeltery_ui_dragging_mut().remove(&pos);
            game.world.chunks.__horizon_mixins_tconstruct_smeltery_ui_menu_state_mut().remove(&pos);
        }
        return;
    }
    if crate::game::tconstruct_ui::kind(game) == Some(true) {
        if let Some(pos) = game.container {
            game.world.chunks.__horizon_mixins_tconstruct_station_name_edit_mut().remove(&pos);
            game.world.chunks.__horizon_mixins_tconstruct_station_info_scroll_mut().remove(&pos);
            game.world.chunks.__horizon_mixins_tconstruct_station_armor_preview_mut().remove(&pos);
            game.world.chunks.__horizon_mixins_tconstruct_builder_info_mut().remove(&pos);
            game.world.chunks.__horizon_mixins_tconstruct_builder_info_recv_mut().remove(&pos);
            game.world.chunks.__horizon_mixins_tconstruct_part_selections_mut().remove(&pos);
            game.world.chunks.__horizon_mixins_tconstruct_builder_wire_recv_mut().remove(&pos);
        }
        return;
    }
    if crate::game::tconstruct_ui::kind(game) != Some(false) { return; }
    if let Some(pos) = game.container {
        game.world.chunks.__horizon_mixins_tconstruct_builder_side_mut().remove(&pos);
        game.world.chunks.__horizon_mixins_tconstruct_builder_side_size_mut().remove(&pos);
        game.world.chunks.__horizon_mixins_tconstruct_builder_side_title_mut().remove(&pos);
        game.world.chunks.__horizon_mixins_tconstruct_builder_side_scroll_mut().remove(&pos);
        game.world.chunks.__horizon_mixins_tconstruct_builder_scroll_mut().remove(&pos);
        game.world.chunks.__horizon_mixins_tconstruct_builder_info_scroll_mut().remove(&pos);
        if game.remote.is_some() {
            // Remote menus may reuse the same virtual container coordinate.
            // The next Station must not interpret this Pattern as its layout.
            game.world.chunks.__horizon_mixins_tconstruct_part_selections_mut().remove(&pos);
            game.world.chunks.__horizon_mixins_tconstruct_builder_choices_mut().remove(&pos);
            game.world.chunks.__horizon_mixins_tconstruct_builder_wire_recv_mut().remove(&pos);
            game.world.chunks.__horizon_mixins_tconstruct_builder_cast_costs_mut().remove(&pos);
            game.world.chunks.__horizon_mixins_tconstruct_builder_cast_wire_recv_mut().remove(&pos);
            game.world.chunks.__horizon_mixins_tconstruct_builder_info_mut().remove(&pos);
            game.world.chunks.__horizon_mixins_tconstruct_builder_info_recv_mut().remove(&pos);
        }
    }
}
