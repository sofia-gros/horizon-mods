fn tconstruct_crafting_client_data(game: &mut crate::game::Game<'_>, packet: &horizon_protocol::packets::play::Clientbound) {
    let horizon_protocol::packets::play::Clientbound::ContainerSetData(data)=packet else {return;};
    if game.remote_menu_title().map(|title|title.plain()).as_deref()!=Some("TConstruct Crafting Station") || data.container!=game.remote_menu_id() || !matches!(data.id,30110|30111|30112) {return;}
    let window=game.remote_menu_id();
    let Ok(value)=usize::try_from(data.value) else {return;};
    if data.id==30110 && matches!(value,5|9|27|54|64|256) {
        *game.world.chunks.__horizon_mixins_tconstruct_crafting_side_client_mut()=Some((window,value,0,vec![None;48]));
    } else if data.id==30112 {
        game.world.chunks.__horizon_mixins_tconstruct_builder_side_title_mut().insert((i32::MIN,window,0),data.value);
    } else if data.id==30111 {
        if let Some((stored,count,offset,_))=game.world.chunks.__horizon_mixins_tconstruct_crafting_side_client_mut().as_mut() {
            if *stored==window {*offset=value.min(crate::game::tconstruct_source_side_layout::max_offset(*count));}
        }
    }
}
