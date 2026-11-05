fn tconstruct_smite_context_end(game: &mut crate::game::Game<'_>, result: &mut (f32, f32, bool, bool)) {
    let bonus = game.world.chunks.__horizon_mixins_tconstruct_smite_context_mut().pop()
        .and_then(|(actor,_,bonus)| (actor == game.lan_acting).then_some(bonus)).flatten();
    result.3 |= bonus.is_some_and(|bonus| bonus > 0.0);
}
