fn open_tconstruct_storage(game: &mut crate::game::Game<'_>, pos: [i32; 3], name: &str) -> std::ops::ControlFlow<bool> {
    if name != "tconstruct:part_builder" && name != "tconstruct:tinker_station" {
        return std::ops::ControlFlow::Continue(());
    }
    if name == "tconstruct:part_builder" && game.remote.is_none() { game.ensure_datapacks(); }
    let key = (pos[0], pos[1], pos[2]);
    game.world.block_entities.get_or_insert_with(key, || BlockEntity::Chest(vec![None; 27]));
    game.world.modified.insert((pos[0] >> 4, pos[2] >> 4));
    game.container = Some(key);
    std::ops::ControlFlow::Break(true)
}
