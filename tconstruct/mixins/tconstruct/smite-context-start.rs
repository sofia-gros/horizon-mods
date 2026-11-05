fn tconstruct_smite_context_start(game: &mut crate::game::Game<'_>, index: usize) {
    let actor = game.lan_acting;
    let kind = format!("minecraft:{}", game.mobs[index].kind.id.trim_start_matches("minecraft:"));
    let bonus = crate::game::tconstruct_smite_world::bonus(game, index);
    let factor = crate::game::tconstruct_conducting_world::factor(game).unwrap_or(1.0);
    game.world.chunks.__horizon_mixins_tconstruct_smite_context_mut().push((actor, kind, bonus, factor));
}
