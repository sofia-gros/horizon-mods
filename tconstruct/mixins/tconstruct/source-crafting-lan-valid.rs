fn tconstruct_crafting_lan_valid(game: &crate::game::Game<'_>, guest: &Guest, pos: [i32; 3]) -> std::ops::ControlFlow<bool> {
    if &*game.res.facts().get(game.world.block(pos[0], pos[1], pos[2])).name != "tconstruct:crafting_station" { return std::ops::ControlFlow::Continue(()); }
    std::ops::ControlFlow::Break(guest.within_block_range(pos, 4.0))
}
