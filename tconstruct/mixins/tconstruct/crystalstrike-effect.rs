fn tconstruct_crystalstrike_effect(game: &crate::game::Game<'_>, effect: &str, ticks: &mut i32) {
    *ticks = crate::game::tconstruct_crystalstrike_world::effect_duration(game, effect, *ticks);
}
