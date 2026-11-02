fn tconstruct_chest_drag_accept(game: &mut crate::game::Game<'_>,slot: usize,ty: i32,dragged: usize) -> std::ops::ControlFlow<bool> {
    let Some(kind)=crate::game::tconstruct_chest_ui::kind(game) else {return std::ops::ControlFlow::Continue(());};
    let Some(cursor)=game.carried.clone() else {return std::ops::ControlFlow::Break(false);};
    if !(0..=2).contains(&ty) || (ty==2 && !game.creative) || (ty!=2 && cursor.count as usize<=dragged) {return std::ops::ControlFlow::Break(false);}
    if !(slot<36 || (300..556).contains(&slot)) {return std::ops::ControlFlow::Break(false);}
    let current=game.slot_view(slot);
    if current.as_ref().is_some_and(|s|!s.same_item_same_components(&cursor)) {return std::ops::ControlFlow::Break(false);}
    let max=game.res.items().get(&cursor.item).map_or(64,|info|info.max_stack).max(1);
    let limit=if slot<36 {max} else {kind.slot_limit(max)};
    if current.as_ref().is_some_and(|s|s.count>=limit) {return std::ops::ControlFlow::Break(false);}
    if slot>=300 {
        let tagged=game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|r|r.as_ref().ok()).and_then(|t|t.get("tconstruct:chest_parts")).map_or(game.remote.is_some(),|v|v.contains(cursor.item.as_ref()));
        let first=game.container.and_then(|key|game.world.block_entities.get(&key)).and_then(|e|e.slots().iter().position(|s|s.as_ref().is_some_and(|s|s.same_item_same_components(&cursor))));
        if !kind.accepts(slot-300,first,tagged) {return std::ops::ControlFlow::Break(false);}
    }
    std::ops::ControlFlow::Break(true)
}
