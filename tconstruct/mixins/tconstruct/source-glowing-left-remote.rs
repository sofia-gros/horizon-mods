fn tconstruct_glow_left_remote(game: &mut crate::game::Game<'_>) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    if game.clicks.miss_time > 0 || game.spectator() || game.entity_targeted() || !game.may_build() { return Continue(()); }
    let Some(stack) = game.held().cloned() else { return Continue(()); };
    if !crate::game::tconstruct_glowing_world::eligible(game,&stack,true) { return Continue(()); }
    let Some(hit) = game.pick() else { return Continue(()); };
    if game.res.facts().is_air(game.world.block(hit.pos[0],hit.pos[1],hit.pos[2])) { return Continue(()); }
    game.stop_remote_mining();
    let sequence=game.next_sequence();
    game.send(S::PlayerAction(play::PlayerAction { action:0,pos:hit.pos,direction:face_direction(hit.face),sequence }));
    *game.world.chunks.__horizon_mixins_tconstruct_glow_left_held_mut() = true;
    game.swing_arm();
    game.send(S::Punch(Empty));
    Break(true)
}
