fn tconstruct_crafting_client_data(game: &mut crate::game::Game<'_>, id: usize, value: i16) -> std::ops::ControlFlow<()> {
    if game.remote_menu_title().map(|title|title.plain()).as_deref()!=Some("TConstruct Crafting Station") || !matches!(id,30110|30111) {return std::ops::ControlFlow::Continue(());}
    let window=game.remote_menu_id();
    let Ok(value)=usize::try_from(value) else {return std::ops::ControlFlow::Break(());};
    if id==30110 && matches!(value,64|256) {
        *game.world.chunks.__horizon_mixins_tconstruct_crafting_side_client_mut()=Some((window,value,0,vec![None;48]));
    } else if id==30111 {
        if let Some((stored,count,offset,_))=game.world.chunks.__horizon_mixins_tconstruct_crafting_side_client_mut().as_mut() {
            if *stored==window {*offset=value.min(crate::game::tconstruct_source_side_layout::max_offset(*count));}
        }
    }
    std::ops::ControlFlow::Break(())
}
