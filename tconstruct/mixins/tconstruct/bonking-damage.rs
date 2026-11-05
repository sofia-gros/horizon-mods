fn tconstruct_bonking_damage(
    value: (f32, f32, bool, bool),
    game: &mut crate::game::Game<'_>,
) -> (f32, f32, bool, bool) {
    if crate::game::tconstruct_bonking_world::attack_charge(game).is_none() {
        return value;
    }
    let (amount, scale, crit, magic) = value;
    let base = game.attack_damage();
    // Source applies 1.5x before its additive base critical bonus; do not
    // multiply the native critical bonus a second time.
    let bonus = base * 0.5 * (0.2 + scale * scale * 0.8);
    (amount + bonus, scale, crit, magic || bonus > 0.0)
}
