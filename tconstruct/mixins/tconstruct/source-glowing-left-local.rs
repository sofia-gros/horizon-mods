fn tconstruct_glow_left_local(game: &mut crate::game::Game<'_>) -> std::ops::ControlFlow<Result<()>> {
    use std::ops::ControlFlow::{Break, Continue};
    if !game.left {
        *game.world.chunks.__horizon_mixins_tconstruct_glow_left_held_mut() = false;
        return Continue(());
    }
    if *game.world.chunks.__horizon_mixins_tconstruct_glow_left_held() {
        game.attack_clicks = 0;
        return Break(Ok(()));
    }
    if game.attack_clicks == 0 || game.remote.is_some() || game.screen != Screen::Playing || game.title.is_some() || game.sleeping.is_some() || game.entity_targeted() || !game.may_build() { return Continue(()); }
    let Some(hit) = game.pick() else { return Continue(()); };
    let Some(stack) = game.inventory.get(game.selected).and_then(Option::as_ref).cloned() else { return Continue(()); };
    if !crate::game::tconstruct_glowing_world::eligible(game,&stack,true) { return Continue(()); }
    let outcome = game.with_hand(Hand::Main, |g| crate::game::tconstruct_glowing_world::activate_for(g,&stack,hit.pos,hit.face,true,false));
    let Some(result) = outcome else { return Continue(()); };
    *game.world.chunks.__horizon_mixins_tconstruct_glow_left_held_mut() = true;
    game.attack_clicks = 0;
    game.stop_mining();
    game.swing_arm();
    Break(result.map(|_| ()))
}
