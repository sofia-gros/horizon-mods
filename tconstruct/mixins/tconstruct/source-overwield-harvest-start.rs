// Run after native cancellation handlers and before actual block removal.
// Area recursion spends only on its first allowed call.
fn tconstruct_overwield_harvest_start(game: &mut crate::game::Game<'_>, canceled: bool, state: horizon_world::registry::BlockStateId) {
    let actor = game.lan_acting;
    game.world.chunks.__horizon_mixins_tconstruct_momentum_break_context_mut().remove(&actor);
    if !canceled {
        let facts = game.res.facts().get(state);
        let context = (facts.name.to_string(), facts.requires_tool);
        game.world.chunks.__horizon_mixins_tconstruct_momentum_break_context_mut().insert(actor, context);
    }
    super::tconstruct_native_hook_bodies::tconstruct_overwield_harvest_start(game, canceled);
}
