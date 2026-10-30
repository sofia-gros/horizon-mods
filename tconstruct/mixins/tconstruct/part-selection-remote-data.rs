fn part_selection_remote_data(game: &mut crate::game::Game<'_>, id: usize, value: i16) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(station) = crate::game::tconstruct_ui::kind(game) else { return Continue(()); };
    if station {
        let Some(key) = game.container else { return Break(()); };
        if id >= 31000 {
            if id == 31000 && value == -1 {
                game.world.chunks.__horizon_mixins_tconstruct_builder_info_mut().remove(&key);
                game.world.chunks.__horizon_mixins_tconstruct_builder_info_recv_mut().insert(key, Vec::new());
            } else if id == 31000 && value == -2 {
                game.world.chunks.__horizon_mixins_tconstruct_builder_info_mut().remove(&key);
                game.world.chunks.__horizon_mixins_tconstruct_builder_info_recv_mut().remove(&key);
            } else if id == 31000 && value >= 0 {
                let pending = game.world.chunks.__horizon_mixins_tconstruct_builder_info_recv_mut().remove(&key);
                let info = pending.and_then(|words| crate::game::tconstruct_source_builder_wire::decode(value, &words));
                if let Some(info) = info.filter(|info| info.len() == 1) { game.world.chunks.__horizon_mixins_tconstruct_builder_info_mut().insert(key, info); }
            } else if id > 31000 {
                let pending = game.world.chunks.__horizon_mixins_tconstruct_builder_info_recv_mut();
                if let Some(words) = pending.get_mut(&key) {
                    if words.len() == id - 31001 { words.push(value); } else { pending.remove(&key); }
                }
            }
            return Break(());
        }
        if id == 10000 && value == -1 {
            game.world.chunks.__horizon_mixins_tconstruct_builder_wire_recv_mut().insert(key, Vec::new());
        } else if id == 10000 && value == -2 {
            game.world.chunks.__horizon_mixins_tconstruct_builder_wire_recv_mut().remove(&key);
            if let Some(snapshot) = game.world.chunks.__horizon_mixins_tconstruct_material_snapshot_mut().as_mut() {
                snapshot.retain(|id, _| !id.contains(":tinkering/station_layouts/"));
            }
        } else if id == 10000 && value >= 0 {
            let frame = game.world.chunks.__horizon_mixins_tconstruct_builder_wire_recv_mut().remove(&key);
            let pairs = frame.and_then(|words| crate::game::tconstruct_source_builder_wire::decode(value, &words));
            if let Some(pairs) = pairs.filter(|pairs| pairs.len() % 2 == 0 && pairs.chunks_exact(2).all(|pair| pair[0].contains(":tinkering/station_layouts/") && crate::game::tconstruct_source_station_layout::StationLayout::parse(pair[0].clone(), pair[1].as_bytes()).is_some())) {
                let snapshot = game.world.chunks.__horizon_mixins_tconstruct_material_snapshot_mut().get_or_insert_with(Default::default);
                snapshot.retain(|id, _| !id.contains(":tinkering/station_layouts/"));
                for pair in pairs.chunks_exact(2) { snapshot.insert(pair[0].clone(), vec![pair[1].as_bytes().to_vec()]); }
            }
        } else if id > 10000 {
            if let Some(words) = game.world.chunks.__horizon_mixins_tconstruct_builder_wire_recv_mut().get_mut(&key) {
                if words.len() == id - 10001 { words.push(value); } else { game.world.chunks.__horizon_mixins_tconstruct_builder_wire_recv_mut().remove(&key); }
            }
        }
        return Break(());
    }
    if id == 30001 {
        if let Some(key) = game.container { game.world.chunks.__horizon_mixins_tconstruct_builder_side_title_mut().insert(key, value); }
        return Break(());
    }
    if id == 30000 {
        if let Some(key) = game.container {
            if let Ok(size) = usize::try_from(value) { game.world.chunks.__horizon_mixins_tconstruct_builder_side_size_mut().insert(key, size); }
        }
        return Break(());
    }
    if id >= 31000 {
        if let Some(key) = game.container {
            if id == 31000 && value == -1 {
                game.world.chunks.__horizon_mixins_tconstruct_builder_info_mut().remove(&key);
                game.world.chunks.__horizon_mixins_tconstruct_builder_info_recv_mut().insert(key, Vec::new());
            } else if id == 31000 && value == -2 {
                game.world.chunks.__horizon_mixins_tconstruct_builder_info_mut().remove(&key);
                game.world.chunks.__horizon_mixins_tconstruct_builder_info_recv_mut().remove(&key);
            } else if id == 31000 && value >= 0 {
                let pending = game.world.chunks.__horizon_mixins_tconstruct_builder_info_recv_mut().remove(&key);
                let info = pending.and_then(|words| crate::game::tconstruct_source_builder_wire::decode(value, &words));
                if let Some(info) = info.filter(|info| info.len() == 6) {
                    game.world.chunks.__horizon_mixins_tconstruct_builder_info_mut().insert(key, info);
                }
            } else if id > 31000 {
                let pending = game.world.chunks.__horizon_mixins_tconstruct_builder_info_recv_mut();
                if let Some(words) = pending.get_mut(&key) {
                    if words.len() == id - 31001 { words.push(value); } else { pending.remove(&key); }
                }
            }
        }
        return Break(());
    }
    // Reserved ids are above the maximum Pattern-frame data-slot range.
    if id >= 20000 {
        if let Some(key) = game.container {
            if id == 20000 && value == -1 {
                game.world.chunks.__horizon_mixins_tconstruct_builder_cast_costs_mut().remove(&key);
                game.world.chunks.__horizon_mixins_tconstruct_builder_cast_wire_recv_mut().insert(key, Vec::new());
            } else if id == 20000 && value == -2 {
                game.world.chunks.__horizon_mixins_tconstruct_builder_cast_costs_mut().remove(&key);
                game.world.chunks.__horizon_mixins_tconstruct_builder_cast_wire_recv_mut().remove(&key);
            } else if id == 20000 && value >= 0 {
                let pending = game.world.chunks.__horizon_mixins_tconstruct_builder_cast_wire_recv_mut().remove(&key);
                let costs = pending.and_then(|words| crate::game::tconstruct_source_builder_wire::decode(value, &words))
                    .and_then(|values| crate::game::tconstruct_source_cast_info::parse_costs(&values));
                if let Some(costs) = costs { game.world.chunks.__horizon_mixins_tconstruct_builder_cast_costs_mut().insert(key, costs); }
                else { game.world.chunks.__horizon_mixins_tconstruct_builder_cast_costs_mut().remove(&key); }
            } else if id > 20000 {
                let pending = game.world.chunks.__horizon_mixins_tconstruct_builder_cast_wire_recv_mut();
                if let Some(words) = pending.get_mut(&key) {
                    if words.len() == id - 20001 { words.push(value); } else { pending.remove(&key); }
                }
            }
        }
        return Break(());
    }
    if id == 0 {
        if let Some(key) = game.container {
            let pattern = usize::try_from(value).ok().and_then(|index| {
                game.world.chunks.__horizon_mixins_tconstruct_builder_choices().get(&key)
                    .and_then(|(_, _, patterns)| patterns.as_ref()).and_then(|patterns| patterns.get(index)).cloned()
            });
            if let Some(pattern) = pattern {
                game.world.chunks.__horizon_mixins_tconstruct_part_selections_mut().insert(key, pattern);
            } else { game.world.chunks.__horizon_mixins_tconstruct_part_selections_mut().remove(&key); }
        }
    } else if let Some(key) = game.container {
        if id == 1 && value == -1 {
            game.world.chunks.__horizon_mixins_tconstruct_builder_choices_mut().remove(&key);
            game.world.chunks.__horizon_mixins_tconstruct_builder_wire_recv_mut().insert(key, Vec::new());
        } else if id == 1 && value == -2 {
            game.world.chunks.__horizon_mixins_tconstruct_builder_wire_recv_mut().remove(&key);
            game.world.chunks.__horizon_mixins_tconstruct_builder_choices_mut().remove(&key);
            if let Some(scroll) = game.world.chunks.__horizon_mixins_tconstruct_builder_scroll_mut().get_mut(&key) {
                crate::game::tconstruct_source_builder_layout::update_display(0, scroll);
                scroll.3.clear();
            }
        } else if id == 1 && value >= 0 {
            let received = game.world.chunks.__horizon_mixins_tconstruct_builder_wire_recv_mut().remove(&key);
            let patterns = received.and_then(|words| crate::game::tconstruct_source_builder_wire::decode(value, &words));
            if let Some(patterns) = patterns {
                let scroll = game.world.chunks.__horizon_mixins_tconstruct_builder_scroll_mut().entry(key)
                    .or_insert_with(|| (0.0, 0, false, Vec::new()));
                if scroll.3 != patterns {
                    crate::game::tconstruct_source_builder_layout::update_display(patterns.len(), scroll);
                    scroll.3 = patterns.clone();
                }
                game.world.chunks.__horizon_mixins_tconstruct_builder_choices_mut().insert(key, (None, None, Some(patterns)));
            } else { game.world.chunks.__horizon_mixins_tconstruct_builder_choices_mut().remove(&key); }
        } else if id >= 2 {
            let pending = game.world.chunks.__horizon_mixins_tconstruct_builder_wire_recv_mut();
            if let Some(words) = pending.get_mut(&key) {
                if words.len() == id - 2 { words.push(value); } else { pending.remove(&key); }
            }
        }
    }
    Break(())
}
