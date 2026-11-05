fn tconstruct_advancement_tag_predicate(view: &View<'_, '_>, registry: &str, tag: &str, id: &str) -> std::ops::ControlFlow<bool> {
    let game = view.0;
    if registry == "item" && self::tconstruct_source_advancement_tag::managed(tag) {
        return std::ops::ControlFlow::Break(self::tconstruct_source_advancement_tag::members(game, tag).contains(&id));
    }
    std::ops::ControlFlow::Continue(())
}
