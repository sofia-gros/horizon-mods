fn tconstruct_layout(game: &crate::game::Game<'_>, slot: usize) -> std::ops::ControlFlow<(f32, f32)> {
    let Some(station) = crate::game::tconstruct_ui::kind(game) else { return std::ops::ControlFlow::Continue(()); };
    let position = match slot {
        0..=8 => (8.0 + slot as f32 * 18.0, 160.0),
        9..=35 => (8.0 + ((slot - 9) % 9) as f32 * 18.0, 102.0 + ((slot - 9) / 9) as f32 * 18.0),
        300 => if station { (53.0, 22.0) } else { (8.0, 43.0) },
        301 => if station { (15.0, 60.0) } else { (29.0, 43.0) },
        302 if station => (33.0, 42.0),
        322 if !station => (148.0, 42.0),
        324 if station => (114.0, 38.0),
        325 if station => (134.0, 38.0),
        302..=326 => (-10000.0, -10000.0),
        327..=353 if !station => {
            let count = if game.remote.is_some() {
                game.container.and_then(|key| crate::game::tconstruct_ui::side_inventory(game, key))
                    .and_then(|side| game.world.block_entities.get(&side)).map_or(game.container_size().saturating_sub(27), |entity| entity.slots().len())
            } else { game.container_size().saturating_sub(27) };
            if slot - 327 >= count { (-10000.0, -10000.0) }
            else { (-114.0 + ((slot - 327) % 3) as f32 * 18.0, 8.0 + ((slot - 327) / 3) as f32 * 18.0) }
        },
        _ => return std::ops::ControlFlow::Continue(()),
    };
    std::ops::ControlFlow::Break(position)
}
