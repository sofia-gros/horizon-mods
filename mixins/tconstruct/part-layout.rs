fn tconstruct_layout(game: &crate::game::Game<'_>, slot: usize) -> std::ops::ControlFlow<(f32, f32)> {
    let Some(station) = crate::game::tconstruct_ui::kind(game) else { return std::ops::ControlFlow::Continue(()); };
    let position = match slot {
        0..=8 => (8.0 + slot as f32 * 18.0, 160.0),
        9..=35 => (8.0 + ((slot - 9) % 9) as f32 * 18.0, 102.0 + ((slot - 9) / 9) as f32 * 18.0),
        300 => if station { (53.0, 22.0) } else { (8.0, 43.0) },
        301 => if station { (15.0, 60.0) } else { (29.0, 43.0) },
        302 if station => (33.0, 42.0),
        322 if !station => (148.0, 42.0),
        324 => if station { (114.0, 38.0) } else { (52.0, 16.0) },
        325 => if station { (134.0, 38.0) } else { (70.0, 16.0) },
        326 if !station => (88.0, 16.0),
        302..=326 => (-10000.0, -10000.0),
        _ => return std::ops::ControlFlow::Continue(()),
    };
    std::ops::ControlFlow::Break(position)
}
