fn tconstruct_part_output(game: &mut crate::game::Game<'_>, slot: Option<usize>, button: i32, input: horizon_client::menu::Input) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(key) = game.container else { return Continue(()); };
    if game.remote.is_some() { return Continue(()); }
    let name = &game.res.facts().get(game.world.block(key.0, key.1, key.2)).name;
    let station = match &**name {
        "tconstruct:part_builder" => false,
        "tconstruct:tinker_station" => true,
        _ => return Continue(()),
    };
    if input != horizon_client::menu::Input::Pickup { return Break(true); }
    if slot.is_some_and(|s| (302..=323).contains(&s) && (!station || s != 302)) { return Break(true); }
    let Some(index) = slot.filter(|s| (324..=326).contains(s)) else { return Continue(()); };
    if button != 0 { return Break(true); }
    let Some(BlockEntity::Chest(stacks)) = game.world.block_entities.get_mut(&key) else { return Break(true); };
    if crate::game::tconstruct_tx::take(stacks, &mut game.carried, station, (index - 300) as i16) {
        game.world.modified.insert((key.0 >> 4, key.2 >> 4));
    }
    Break(true)
}
