fn tconstruct_protection_damage_refresh(value: (), game: &mut crate::game::Game<'_>) {
    let _ = value;
    crate::game::tconstruct_protection_damage_world::refresh(game);
    crate::game::tconstruct_consecrated_world::refresh(game);
}
