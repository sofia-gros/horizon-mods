fn tconstruct_chest_drag_accept(game: &mut crate::game::Game<'_>,slot: usize,ty: i32,dragged: usize) -> std::ops::ControlFlow<bool> {
    if crate::game::tconstruct_ui::side_chest_kind(game).is_some() && (slot<36 || (300..302).contains(&slot)) {
        let Some(cursor)=game.carried.clone() else {return std::ops::ControlFlow::Break(false);};
        if !(0..=2).contains(&ty) || (ty==2 && !game.creative) || (ty!=2 && cursor.count as usize<=dragged) {return std::ops::ControlFlow::Break(false);}
        if slot==300 {
            let pattern=game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|r|r.as_ref().ok()).and_then(|t|t.get("tconstruct:patterns")).map_or(game.remote.is_some(),|v|v.contains(cursor.item.as_ref()));
            if !pattern {return std::ops::ControlFlow::Break(false);}
        }
        let current=game.slot_view(slot);let max=game.res.items().get(&cursor.item).map_or(64,|info|info.max_stack).max(1);
        return std::ops::ControlFlow::Break(current.as_ref().is_none_or(|s|s.same_item_same_components(&cursor) && s.count<max));
    }
    if (327..381).contains(&slot) {
        if let Some(kind)=crate::game::tconstruct_ui::side_chest_kind(game) {
            let Some(cursor)=game.carried.clone() else {return std::ops::ControlFlow::Break(false);};
            if !(0..=2).contains(&ty) || (ty==2 && !game.creative) || (ty!=2 && cursor.count as usize<=dragged) {return std::ops::ControlFlow::Break(false);}
            let Some(table)=game.container else {return std::ops::ControlFlow::Break(false);};
            let offset=game.world.chunks.__horizon_mixins_tconstruct_builder_side_scroll().get(&table).map_or(0,|s|s.0);
            let index=offset*6+slot-327;
            if index>=kind.slots() {return std::ops::ControlFlow::Break(false);}
            let current=game.slot_view(slot);let max=game.res.items().get(&cursor.item).map_or(64,|info|info.max_stack).max(1);
            if current.as_ref().is_some_and(|s|!s.same_item_same_components(&cursor) || s.count>=kind.slot_limit(max)) {return std::ops::ControlFlow::Break(false);}
            let tagged=game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|r|r.as_ref().ok()).and_then(|t|t.get("tconstruct:chest_parts")).map_or(game.remote.is_some(),|v|v.contains(cursor.item.as_ref()));
            let first=if game.remote.is_some() {game.world.block_entities.get(&table).and_then(|e|e.slots().get(27..81)).and_then(|slots|slots.iter().position(|s|s.as_ref().is_some_and(|s|s.same_item_same_components(&cursor)))).map(|i|offset*6+i)} else {game.world.chunks.__horizon_mixins_tconstruct_builder_side().get(&table).and_then(|side|game.world.block_entities.get(side)).and_then(|e|e.slots().iter().position(|s|s.as_ref().is_some_and(|s|s.same_item_same_components(&cursor))))};
            return std::ops::ControlFlow::Break(kind.accepts(index,first,tagged));
        }
    }
    if (600..648).contains(&slot) {
        if let Some((side,count,offset,kind))=crate::game::tconstruct_crafting_station::side(game) {
            let Some(cursor)=game.carried.clone() else {return std::ops::ControlFlow::Break(false);};
            if !(0..=2).contains(&ty) || (ty==2 && !game.creative) || (ty!=2 && cursor.count as usize<=dragged) {return std::ops::ControlFlow::Break(false);}
            let index=offset*crate::game::tconstruct_source_side_layout::columns(count)+slot-600;
            if index>=count {return std::ops::ControlFlow::Break(false);}
            let current=game.slot_view(slot);let max=game.res.items().get(&cursor.item).map_or(64,|info|info.max_stack).max(1);
            if current.as_ref().is_some_and(|s|!s.same_item_same_components(&cursor) || s.count>=kind.slot_limit(max)) {return std::ops::ControlFlow::Break(false);}
            let tagged=game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|r|r.as_ref().ok()).and_then(|t|t.get("tconstruct:chest_parts")).map_or(game.remote.is_some(),|v|v.contains(cursor.item.as_ref()));
            let first=if game.remote.is_some() {game.world.chunks.__horizon_mixins_tconstruct_crafting_side_client().as_ref().and_then(|(_,_,_,items)|items.iter().position(|s|s.as_ref().is_some_and(|s|s.same_item_same_components(&cursor)))).map(|i|offset*crate::game::tconstruct_source_side_layout::columns(count)+i)} else {crate::game::tconstruct_crafting_station::side_items(game,side).and_then(|items|items.iter().position(|s|s.as_ref().is_some_and(|s|s.same_item_same_components(&cursor))))};
            return std::ops::ControlFlow::Break(kind.accepts(index,first,tagged));
        }
    }
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
