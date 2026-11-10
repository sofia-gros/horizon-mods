fn tconstruct_smite_context(
    game: &mut crate::game::Game<'_>, kind: &str, living: bool, index: usize,
    original: impl FnOnce(&mut crate::game::Game<'_>, &str, bool) -> (f32,f32,bool,bool),
) -> (f32,f32,bool,bool) {
    let actor = game.lan_acting;
    let bonus = if living { crate::game::tconstruct_smite_world::bonus(game, index) } else { None };
    // Include a sentinel for unsupported nested attacks, so they cannot borrow
    // the outer target's bonus. The scope is only this primary damage call.
    game.world.chunks.__horizon_mixins_tconstruct_smite_context_mut().push((actor, kind.to_owned(), bonus));
    let mut result = original(game, kind, living);
    game.world.chunks.__horizon_mixins_tconstruct_smite_context_mut().pop();
    result.3 |= bonus.is_some_and(|bonus| bonus > 0.0);
    result
}
