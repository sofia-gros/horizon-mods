fn tconstruct_armor_wear_call(
    game: &mut crate::game::Game<'_>, source: horizon_world::damage_source::DamageTypeId,
    damage: f32,
    original: impl FnOnce(&mut crate::game::Game<'_>, horizon_world::damage_source::DamageTypeId, f32),
) {
    crate::game::tconstruct_armor_wear_world::wrap(game, source, damage, original);
}
