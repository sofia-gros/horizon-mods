fn after_menu_button(value: (), game: &mut crate::game::Game<'_>, pos: (i32, i32, i32)) {
    game.apply_queued_menu_switch(pos);
    value
}
