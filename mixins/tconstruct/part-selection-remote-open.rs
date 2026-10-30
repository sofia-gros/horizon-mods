fn part_selection_remote_open(game: &crate::game::Game<'_>, container: i32) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    if crate::game::tconstruct_ui::kind(game).is_none() { return Continue(()); }
    Break(game.remote.as_ref().and_then(|r| r.menu.as_ref()).is_some_and(|(id, _)| *id == container))
}
