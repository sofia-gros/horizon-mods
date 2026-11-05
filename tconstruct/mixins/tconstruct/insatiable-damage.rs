fn tconstruct_insatiable_damage(
    value: (f32, f32, bool, bool),
    game: &crate::game::Game<'_>,
    living: bool,
) -> (f32, f32, bool, bool) {
    let (amount, scale, crit, magic) = value;
    if !living
        || game
            .player
            .as_ref()
            .is_some_and(|player| player.auto_spin_attack_dmg > 0.0)
    {
        return value;
    }
    let bonus = crate::game::tconstruct_insatiable_world::damage(game, scale);
    (amount + bonus, scale, crit, magic || bonus > 0.0)
}
