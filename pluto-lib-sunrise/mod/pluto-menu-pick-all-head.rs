fn pluto_menu_pick_all_head(menu: &Menu, slot: usize) -> std::ops::ControlFlow<bool> {
    if pluto_menu_actions::excluded_from_pick_all(menu, slot) {
        std::ops::ControlFlow::Break(false)
    } else {
        std::ops::ControlFlow::Continue(())
    }
}
