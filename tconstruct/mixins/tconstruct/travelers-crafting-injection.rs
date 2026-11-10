fn tconstruct_travelers_crafting_files(
    value: (),
    game: &crate::game::Game<'_>,
    files: &mut Vec<(String, Vec<u8>)>,
) {
    let _ = value;
    tconstruct_source_travelers_world::contribute(game, files);
}
