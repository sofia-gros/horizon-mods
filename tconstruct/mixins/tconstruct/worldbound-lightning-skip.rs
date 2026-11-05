fn tconstruct_worldbound_lightning_skip(
    game: &crate::game::Game<'_>,
    handle: crate::game::entity::EntityHandle,
    stage: crate::game::entity::ThunderHit,
) -> std::ops::ControlFlow<bool> {
    if crate::game::lightning::tconstruct_worldbound_lightning::skip_item(game, handle, stage) {
        std::ops::ControlFlow::Break(false)
    } else {
        std::ops::ControlFlow::Continue(())
    }
}
