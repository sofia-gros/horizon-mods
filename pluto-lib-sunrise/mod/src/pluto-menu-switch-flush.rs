fn after_menu_button(value: (), game: &mut crate::game::Game<'_>, backing: Option<(i32, i32, i32)>) {
    if let Some(pos) = backing {
        game.apply_queued_menu_switch(pos);
    }
    value
}
