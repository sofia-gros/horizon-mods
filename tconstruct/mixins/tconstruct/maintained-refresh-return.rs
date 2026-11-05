fn tconstruct_maintained_refresh_return(value: (), game: &mut crate::game::Game<'_>) {
    let _ = value;
    crate::game::tconstruct_maintained_world::refresh(game);
}
