fn tconstruct_crafting_menu(game: &mut crate::game::Game<'_>, ops: &[(Option<usize>, i32, horizon_client::menu::Input)]) -> std::ops::ControlFlow<bool> {
    use horizon_client::menu::Input;
    // menu_session commits the transaction and recipe advancements, but on this
    // Horizon revision it does not forward Menu::item_crafted to native hooks or
    // increment crafted-item statistics. Mirror the normal local result path once.
    fn notify_crafted(game: &mut crate::game::Game<'_>, crafted: Vec<(String, horizon_items::items::ItemStack)>) {
        if crafted.is_empty() { return; }
        let player = game.native_acting_id();
        for (recipe, stack) in crafted {
            game.native_item_crafted(player, &recipe, &stack);
            game.stats.add("crafted", &stack.item, stack.count as i32);
        }
    }
    if crate::game::tconstruct_crafting_station::position(game).is_none() { return std::ops::ControlFlow::Continue(()); }
    crate::game::tconstruct_crafting_station::refresh(game);
    if !ops.is_empty() && ops.iter().all(|(_,_,input)|*input==Input::QuickCraft) {
        if let Some((pos,count,offset,kind))=crate::game::tconstruct_crafting_station::side(game) {
            let Some(mut side)=crate::game::tconstruct_crafting_station::side_items(game,pos) else {return std::ops::ControlFlow::Break(true);};
            let Some(cursor)=game.carried.as_ref() else {return std::ops::ControlFlow::Break(true);};
            let max=game.res.items().get(&cursor.item).map_or(64,|info|info.max_stack).max(1);
            let tagged=game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|r|r.as_ref().ok()).and_then(|t|t.get("tconstruct:chest_parts")).is_some_and(|v|v.contains(cursor.item.as_ref()));
            let mode=((ops[0].1>>2)&3) as u8;
            if ops[0].1&3!=0 || ops.last().is_none_or(|(_,button,_)|button&3!=2) {return std::ops::ControlFlow::Break(true);}
            let selected:Vec<_>=ops.iter().filter_map(|(slot,button,_)| {
                if button&3!=1 || ((button>>2)&3) as u8!=mode {return None;}
                match slot {Some(i @ 0..=35)=>Some((false,9+*i)),Some(i @ 100..=108)=>Some((false,*i-100)),Some(i @ 600..=647)=>Some((true,offset*crate::game::tconstruct_source_side_layout::columns(count)+*i-600)),_=>None}
            }).collect();
            let mut combined=game.craft_grid.clone();combined.extend(game.inventory.clone());let mut carried=game.carried.clone();
            if crate::game::tconstruct_chest_rules::quick_craft(kind,&mut side,&mut combined,&mut carried,&selected,mode,max,tagged,game.creative) {
                if crate::game::tconstruct_crafting_station::write_side(game,pos,side) {game.craft_grid=combined[..9].to_vec();game.inventory=combined[9..].to_vec();game.carried=carried;crate::game::tconstruct_crafting_station::sync(game);}
            }
            return std::ops::ControlFlow::Break(true);
        }
    }
    if matches!(ops, [(Some(200), _, Input::QuickMove)]) {
        if let Some((pos,_,_,kind))=crate::game::tconstruct_crafting_station::side(game) {
            let Some(mut side)=crate::game::tconstruct_crafting_station::side_items(game,pos) else {return std::ops::ControlFlow::Break(true);};
            let mut commit_side=false;
            let mut item_crafted = Vec::new();
            let tags=game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v|v.as_ref().ok()).and_then(|v|v.get("tconstruct:chest_parts")).cloned().unwrap_or_default();
            let handled=game.menu_session(|menu,env,_| {
                if crate::game::tconstruct_crafting_output::quick(menu,env,&mut side,kind,&tags).is_err() {return Some(false);}
                item_crafted = std::mem::take(&mut menu.item_crafted);
                commit_side=true;
                Some(true)
            });
            if commit_side {
                let _=crate::game::tconstruct_crafting_station::write_side(game,pos,side);
                notify_crafted(game, item_crafted);
            }
            crate::game::tconstruct_crafting_station::sync(game);
            return std::ops::ControlFlow::Break(handled);
        }
    }
    for &(slot, _, input) in ops {
        if let (Some(slot), Input::QuickMove | Input::Swap) = (slot, input) { game.unselect_bundle(slot); }
    }
    let mut item_crafted = Vec::new();
    let handled = game.menu_session(|menu, env, to_menu| {
        let mut clicks = Vec::new();
        for &(slot, button, input) in ops {
            let slot = match slot {
                Some(slot) => to_menu(slot)? as i32,
                None => -999,
            };
            clicks.push((slot, button, input));
        }
        for (slot, button, input) in clicks {
            if menu.click(env, slot, button, input).is_err() { return Some(false); }
        }
        item_crafted = std::mem::take(&mut menu.item_crafted);
        Some(true)
    });
    if handled { notify_crafted(game, item_crafted); }
    crate::game::tconstruct_crafting_station::sync(game);
    std::ops::ControlFlow::Break(handled)
}
