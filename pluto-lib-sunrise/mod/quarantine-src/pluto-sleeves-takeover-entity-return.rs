// The entity interaction is entered: a RETURN on it cannot plan (the body works through
// closures and `?`), so the seam is a cancellable HEAD that runs the interaction itself
// under the module's guard and reports the outcome the interaction leaves.
fn pluto_sleeves_takeover_entity_return(game: &mut Game<'_>, target: Option<InteractTarget>) -> std::ops::ControlFlow<Result<bool>> {
    crate::game::item_actions::pluto_sleeves_takeover_entity::interact(game, target)
}
