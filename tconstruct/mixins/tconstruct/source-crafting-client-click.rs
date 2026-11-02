fn tconstruct_crafting_client_click(game: &mut crate::game::Game<'_>, ops: &[(Option<usize>,i32,horizon_client::menu::Input)]) -> std::ops::ControlFlow<bool> {
    if crate::game::tconstruct_crafting_station::remote_side(game).is_none() || !ops.iter().any(|(slot,_,_)|slot.is_some_and(|slot|(600..648).contains(&slot))) {return std::ops::ControlFlow::Continue(());}
    let mut clicks=Vec::with_capacity(ops.len());
    for &(slot,button,input) in ops {
        let index=match slot {None=>-999,Some(slot @ 600..=647)=>(slot-600+46) as i16,Some(slot @ 100..=108)=>(slot-100+1) as i16,Some(slot @ 9..=35)=>(slot-9+10) as i16,Some(slot @ 0..=8)=>(slot+37) as i16,_=>return std::ops::ControlFlow::Break(false)};
        clicks.push((index,button,input,Vec::new(),game.carried.clone()));
    }
    game.send_clicks(game.remote_menu_id(),clicks);
    std::ops::ControlFlow::Break(true)
}
