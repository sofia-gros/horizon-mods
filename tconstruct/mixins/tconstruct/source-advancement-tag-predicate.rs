fn tconstruct_advancement_tag_predicate(view: &View<'_, '_>, registry: &str, tag: &str, id: &str) -> std::ops::ControlFlow<bool> {
    let game = view.0;
    if registry == "item" && tag == "tconstruct:modifiable/multipart" {
        return std::ops::ControlFlow::Break(self::tconstruct_source_advancement_tag::members(game).contains(&id));
    }
    std::ops::ControlFlow::Continue(())
}
