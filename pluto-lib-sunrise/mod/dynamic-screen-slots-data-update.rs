fn after_native_screen_data(value: bool, game: &mut crate::game::Game<'_>) -> bool {
    if value {
        game.refresh_dynamic_screen_slots();
    }
    value
}
