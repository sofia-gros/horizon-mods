fn part_selection_remote_data(game: &mut crate::game::Game<'_>, id: usize, value: i16) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    if crate::game::tconstruct_ui::kind(game) != Some(false) { return Continue(()); }
    if id == 0 {
        if let Some(key) = game.container {
            if value < 0 { game.world.chunks.__horizon_mixins_tconstruct_part_selections_mut().remove(&key); }
            else { game.world.chunks.__horizon_mixins_tconstruct_part_selections_mut().insert(key, value); }
        }
    }
    Break(())
}
