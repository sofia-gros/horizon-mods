fn after_set_menu_slot(value: (), game: &mut crate::game::Game<'_>) {
    game.refresh_dynamic_screen_slots();
    value
}
