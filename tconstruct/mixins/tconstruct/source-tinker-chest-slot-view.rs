fn tconstruct_chest_slot_view(game: &mut crate::game::Game<'_>, slot: usize) -> std::ops::ControlFlow<Option<ItemStack>> {
    if let Some((side,count,offset,_)) = crate::game::tconstruct_crafting_station::side(game) {
        if (600..648).contains(&slot) {
            if crate::game::tconstruct_crafting_station::remote_side(game).is_some() {
                let item=game.world.chunks.__horizon_mixins_tconstruct_crafting_side_client().as_ref().and_then(|(_,_,_,items)|items.get(slot-600)).cloned().flatten();
                return std::ops::ControlFlow::Break(item);
            }
            let index = offset * crate::game::tconstruct_source_side_layout::columns(count) + slot - 600;
            let item = game.world.block_entities.get(&side).and_then(|entity| entity.slots().get(index)).cloned().flatten();
            return std::ops::ControlFlow::Break(item);
        }
    }
    if game.remote.is_none() && crate::game::tconstruct_ui::side_chest_kind(game).is_some() && (327..381).contains(&slot) {
        let Some(table)=game.container else { return std::ops::ControlFlow::Break(None); };
        let Some(side)=game.world.chunks.__horizon_mixins_tconstruct_builder_side().get(&table).copied() else { return std::ops::ControlFlow::Break(None); };
        let count=crate::game::tconstruct_ui::side_count(game);
        let offset=game.world.chunks.__horizon_mixins_tconstruct_builder_side_scroll().get(&table).map_or(0,|state|state.0);
        let index=offset*crate::game::tconstruct_source_side_layout::columns(count)+slot-327;
        let item=game.world.block_entities.get(&side).and_then(|entity|entity.slots().get(index)).cloned().flatten();
        return std::ops::ControlFlow::Break(item);
    }
    if game.remote.is_some() || crate::game::tconstruct_chest_ui::kind(game).is_none() || !(300..556).contains(&slot) { return std::ops::ControlFlow::Continue(()); }
    let stack = game.container.and_then(|key| game.world.block_entities.get(&key)).and_then(|entity| entity.slots().get(slot - 300)).cloned().flatten();
    std::ops::ControlFlow::Break(stack)
}
