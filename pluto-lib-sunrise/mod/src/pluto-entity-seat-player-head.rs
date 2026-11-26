// The local player as a vehicle: the game's seat table answers for armor
// stands and kept entities only, so a mob riding the player would be let go
// by the rider synchronisation and never seated. Answer with the player's
// own seat — the position and dimensions the game itself seats a passenger
// on the player with — and let every other id run the original.
fn pluto_entity_seat_player(game: &Game<'_>, id: u32) -> core::ops::ControlFlow<Option<Seat>> {
    let is_local_player = game.remote.as_ref().is_some_and(|remote| remote.entity_id as u32 == id);
    if is_local_player {
        if let Some(player) = &game.player {
            return std::ops::ControlFlow::Break(Some(Seat {
                position: player.position,
                height: f64::from(player.dimensions.height),
                width: player.dimensions.width,
            }));
        }
    }
    std::ops::ControlFlow::Continue(())
}
