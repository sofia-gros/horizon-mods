fn tconstruct_advancement_tag_index(game: &mut crate::game::Game<'_>) -> std::ops::ControlFlow<Arc<adv::InventoryIndex>> {
    let tree = game.adv.tree.clone();
    if let Some((built, index)) = &game.adv.inventory_index {
        if std::sync::Arc::ptr_eq(built, &tree) { return std::ops::ControlFlow::Break(index.clone()); }
    }
    let items = game.items();
    let members = self::tconstruct_source_advancement_tag::members(game);
    let index = std::sync::Arc::new(horizon_client::advancements::InventoryIndex::build(&tree, |tag| {
        if tag == "tconstruct:modifiable/multipart" { members.clone() }
        else { items.item_tags.get(tag).map(|values| values.iter().map(String::as_str).collect()).unwrap_or_default() }
    }));
    game.adv.inventory_index = Some((tree, index.clone()));
    std::ops::ControlFlow::Break(index)
}
