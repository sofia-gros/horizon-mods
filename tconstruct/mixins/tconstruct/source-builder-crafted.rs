fn tconstruct_builder_crafted(game: &mut crate::game::Game<'_>) {
    use horizon_core::random::RandomSource;
    for (conn, item, count, at) in std::mem::take(game.world.chunks.__horizon_mixins_tconstruct_builder_crafted_mut()) {
        if let Some(play) = game.lan.as_mut().and_then(|host| host.play.get_mut(&conn)) {
            play.stats.add("crafted", &item, count as i32);
        }
        let pitch = 0.8 + 0.4 * game.world.random.next_float();
        game.audio.play("tconstruct:little_saw", Some(at), 0.8, pitch);
    }
}
