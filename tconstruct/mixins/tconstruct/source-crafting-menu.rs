fn tconstruct_crafting_menu(game: &mut crate::game::Game<'_>, ops: &[(Option<usize>, i32, horizon_client::menu::Input)]) -> std::ops::ControlFlow<bool> {
    use horizon_client::menu::Input;
    if crate::game::tconstruct_crafting_station::position(game).is_none() { return std::ops::ControlFlow::Continue(()); }
    crate::game::tconstruct_crafting_station::refresh(game);
    if matches!(ops, [(Some(200), _, Input::QuickMove)]) {
        if let Some((pos,_,_,kind))=crate::game::tconstruct_crafting_station::side(game) {
            let Some(BlockEntity::Chest(items))=game.world.block_entities.get(&pos) else {return std::ops::ControlFlow::Break(true);};
            let mut side=items.clone();
            let mut commit_side=false;
            let tags=game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v|v.as_ref().ok()).and_then(|v|v.get("tconstruct:chest_parts")).cloned().unwrap_or_default();
            let handled=game.menu_session(|menu,env,_| {
                if crate::game::tconstruct_crafting_output::quick(menu,env,&mut side,kind,&tags).is_err() {return Some(false);}
                commit_side=true;
                Some(true)
            });
            if commit_side {
                if let Some(BlockEntity::Chest(items))=game.world.block_entities.get_mut(&pos) {*items=side;}
                game.world.modified.insert((pos.0>>4,pos.2>>4));
            }
            crate::game::tconstruct_crafting_station::sync(game);
            return std::ops::ControlFlow::Break(handled);
        }
    }
    for &(slot, _, input) in ops {
        if let (Some(slot), Input::QuickMove | Input::Swap) = (slot, input) { game.unselect_bundle(slot); }
    }
    let ops = ops.to_vec();
    let handled = game.menu_session(move |menu, env, to_menu| {
        let mut clicks = Vec::new();
        for (slot, button, input) in ops {
            let slot = match slot {
                Some(200) if input != Input::QuickMove => return None,
                Some(slot) => to_menu(slot)? as i32,
                None => -999,
            };
            clicks.push((slot, button, input));
        }
        for (slot, button, input) in clicks {
            if menu.click(env, slot, button, input).is_err() { return Some(false); }
        }
        Some(true)
    });
    crate::game::tconstruct_crafting_station::sync(game);
    std::ops::ControlFlow::Break(handled)
}
