fn tconstruct_sweep_position(game:&crate::game::Game<'_>, scale:f32, crit:bool) -> std::ops::ControlFlow<Option<[f64;3]>> {
    use std::ops::ControlFlow::{Break,Continue};
    let Some(stack)=game.held() else{return Continue(());};
    if !matches!(crate::game::tconstruct_source_melee_shape::attack(stack.item.as_ref()), Some(crate::game::tconstruct_source_melee_shape::Attack::Sweep(_))) {return Continue(());}
    if crate::game::tconstruct_source_melee_shape::intact(stack).is_none() {return Break(None);}
    // Until the separate PvP strike path is implemented, do not enable the
    // vanilla fixed-radius player sweep as a substitute for the source rules.
    if !game.world.chunks.__horizon_mixins_tconstruct_melee_context().contains_key(&game.lan_acting) {return Break(None);}
    let Some(player)=game.player.as_ref() else{return Break(None);};
    if !player.is_sweep_attack(scale,crit,true) {return Break(None);}
    let yaw=f64::from(game.yaw.to_radians());
    Break(Some([player.position.x-yaw.sin(),player.position.y+f64::from(player.dimensions.height)*0.5,player.position.z+yaw.cos()]))
}
