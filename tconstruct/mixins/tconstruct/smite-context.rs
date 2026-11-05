fn tconstruct_smite_context_end(game: &mut crate::game::Game<'_>, result: &mut (f32, f32, bool, bool)) {
    let changed = game.world.chunks.__horizon_mixins_tconstruct_smite_context_mut().pop()
        .is_some_and(|(actor, _, bonus, factor)| actor == game.lan_acting
            && (bonus.is_some_and(|bonus| bonus > 0.0) || factor > 1.0));
    result.3 |= changed;
}
