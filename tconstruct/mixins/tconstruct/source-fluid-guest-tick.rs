fn fluid_guest_tick(game: &mut crate::game::Game<'_>, host: &mut crate::game::lan::LanHost) {
    let indices: Vec<_> = host.hub.guests.iter().enumerate().filter_map(|(index, guest)| {
        let (pos, _, _) = game.world.chunks.__horizon_mixins_tconstruct_smeltery_guest_containers().get(&guest.conn)?;
        (!guest.menu.as_ref().is_some_and(|menu| menu.pos == Some(*pos))).then_some(index)
    }).collect();
    for index in indices { crate::game::tconstruct_source_fluid_return::live(game, host, index); }
}
