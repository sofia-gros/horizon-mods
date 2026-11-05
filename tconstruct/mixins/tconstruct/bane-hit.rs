fn tconstruct_bane_primary_hit(
    game: &mut crate::game::Game<'_>, index: usize,
    original: impl FnOnce(&mut crate::game::Game<'_>, usize),
) {
    // attack_mob reaches weapon_hit only after a successful primary hit.
    // Read the still-intact weapon before the native call spends its wear.
    crate::game::tconstruct_smite_world::bane_hit(game, index);
    crate::game::tconstruct_lacerating_world::after_hit(game, index);
    original(game, index);
}
