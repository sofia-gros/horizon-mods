fn tconstruct_crafting_client_slot(game: &mut crate::game::Game<'_>, container: i32, slot: i32, item: &Option<ItemStack>) -> std::ops::ControlFlow<()> {
    if game.remote_menu_title().map(|title|title.plain()).as_deref()!=Some("TConstruct Crafting Station") || game.remote_menu_id()!=container || !(46..94).contains(&slot) {return std::ops::ControlFlow::Continue(());}
    if let Some((window,_,_,items))=game.world.chunks.__horizon_mixins_tconstruct_crafting_side_client_mut().as_mut() {
        if *window==container {items[(slot-46) as usize]=item.clone();}
    }
    std::ops::ControlFlow::Break(())
}
