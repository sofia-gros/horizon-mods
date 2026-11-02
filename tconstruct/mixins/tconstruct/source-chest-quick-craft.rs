fn tconstruct_chest_quick_craft(game: &mut crate::game::Game<'_>,ops: &[(Option<usize>,i32,horizon_client::menu::Input)]) -> std::ops::ControlFlow<bool> {
    use horizon_client::menu::Input;
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
