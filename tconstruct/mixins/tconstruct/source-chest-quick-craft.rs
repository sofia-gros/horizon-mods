fn tconstruct_chest_quick_craft(game: &mut crate::game::Game<'_>,ops: &[(Option<usize>,i32,horizon_client::menu::Input)]) -> std::ops::ControlFlow<bool> {
    use horizon_client::menu::Input;
    if let Some(kind)=crate::game::tconstruct_ui::side_chest_kind(game) {
        if game.remote.is_some() || ops.is_empty() || ops.iter().any(|(_,_,input)|*input!=Input::QuickCraft) {return std::ops::ControlFlow::Continue(());}
        let Some(table)=game.container else {return std::ops::ControlFlow::Break(true);};
        let Some(side)=game.world.chunks.__horizon_mixins_tconstruct_builder_side().get(&table).copied() else {return std::ops::ControlFlow::Break(true);};
        let Some(BlockEntity::Chest(contents))=game.world.block_entities.get(&side) else {return std::ops::ControlFlow::Break(true);};
        let mut contents=contents.clone();
        let Some(BlockEntity::Chest(inputs))=game.world.block_entities.get(&table) else {return std::ops::ControlFlow::Break(true);};
        let mut combined=inputs.clone();combined.extend(game.inventory.clone());
        let Some(cursor)=game.carried.as_ref() else {return std::ops::ControlFlow::Break(true);};
        let tags=game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|r|r.as_ref().ok());
        let part=tags.and_then(|t|t.get("tconstruct:chest_parts")).is_some_and(|v|v.contains(cursor.item.as_ref()));
        let pattern=tags.and_then(|t|t.get("tconstruct:patterns")).is_some_and(|v|v.contains(cursor.item.as_ref()));
        let max=game.res.items().get(&cursor.item).map_or(64,|info|info.max_stack).max(1);
        let offset=game.world.chunks.__horizon_mixins_tconstruct_builder_side_scroll().get(&table).map_or(0,|s|s.0);
        let mode=((ops[0].1>>2)&3) as u8;
        if ops[0].1&3!=0 || ops.last().is_none_or(|(_,button,_)|button&3!=2) {return std::ops::ControlFlow::Break(true);}
        let selected:Vec<_>=ops.iter().filter_map(|(slot,button,_)| {
            if button&3!=1 || ((button>>2)&3) as u8!=mode {return None;}
            match slot {Some(i @ 0..=35)=>Some((false,27+*i)),Some(300) if pattern=>Some((false,0)),Some(301)=>Some((false,1)),Some(i @ 327..=380)=>Some((true,offset*6+*i-327)),_=>None}
        }).collect();
        if crate::game::tconstruct_chest_rules::quick_craft(kind,&mut contents,&mut combined,&mut game.carried,&selected,mode,max,part,game.creative) {
            if let Some(BlockEntity::Chest(inputs))=game.world.block_entities.get_mut(&table) {*inputs=combined[..27].to_vec();}
            if let Some(BlockEntity::Chest(items))=game.world.block_entities.get_mut(&side) {*items=contents;}
            game.inventory=combined[27..].to_vec();game.world.modified.insert((table.0>>4,table.2>>4));game.world.modified.insert((side.0>>4,side.2>>4));
        }
        return std::ops::ControlFlow::Break(true);
    }
    let Some(kind)=crate::game::tconstruct_chest_ui::kind(game) else {return std::ops::ControlFlow::Continue(());};
    if game.remote.is_some() || ops.is_empty() || ops.iter().any(|(_,_,input)|*input!=Input::QuickCraft) {return std::ops::ControlFlow::Continue(());}
    let Some(key)=game.container else {return std::ops::ControlFlow::Break(true);};
    let mode=((ops[0].1>>2)&3) as u8;
    if ops[0].1&3!=0 || ops.last().is_none_or(|(_,button,_)|button&3!=2) {return std::ops::ControlFlow::Break(true);}
    let selected:Vec<_>=ops.iter().filter_map(|(slot,button,_)| {
        if button&3!=1 || ((button>>2)&3) as u8!=mode {return None;}
        match slot {Some(i @ 0..=35)=>Some((false,*i)),Some(i @ 300..=555)=>Some((true,*i-300)),_=>None}
    }).collect();
    let Some(cursor)=game.carried.as_ref() else {return std::ops::ControlFlow::Break(true);};
    let max=game.res.items().get(&cursor.item).map_or(64,|info|info.max_stack).max(1);
    let tagged=game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|r|r.as_ref().ok()).and_then(|t|t.get("tconstruct:chest_parts")).is_some_and(|v|v.contains(cursor.item.as_ref()));
    if let Some(BlockEntity::Chest(items))=game.world.block_entities.get_mut(&key) {
        if crate::game::tconstruct_chest_rules::quick_craft(kind,items,&mut game.inventory,&mut game.carried,&selected,mode,max,tagged,game.creative) {game.world.modified.insert((key.0>>4,key.2>>4));}
    }
    std::ops::ControlFlow::Break(true)
}
