fn tconstruct_entity_damage_files(value: (), game: &crate::game::Game<'_>, files: &mut Vec<(String, Vec<u8>)>) {
    let _ = value;
    tconstruct_source_entity_world::contribute(game, files);
}
