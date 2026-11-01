fn tconstruct_glow_left_continue(game: &mut crate::game::Game<'_>, down: bool) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break,Continue};
    if !down { *game.world.chunks.__horizon_mixins_tconstruct_glow_left_held_mut() = false; return Continue(()); }
    if *game.world.chunks.__horizon_mixins_tconstruct_glow_left_held() { Break(()) } else { Continue(()) }
}
