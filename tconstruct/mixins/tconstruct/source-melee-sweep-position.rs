fn tconstruct_sweep_position(game:&crate::game::Game<'_>, scale:f32, crit:bool) -> std::ops::ControlFlow<Option<[f64;3]>> {
    use std::ops::ControlFlow::{Break,Continue};
    let Some(stack)=game.held() else{return Continue(());};
    if !matches!(crate::game::tconstruct_source_melee_shape::attack(stack.item.as_ref()), Some(crate::game::tconstruct_source_melee_shape::Attack::Sweep(_))) {return Continue(());}
    if crate::game::tconstruct_source_melee_shape::intact(stack).is_none() {return Break(None);}
    // Until the separate PvP strike path is implemented, do not enable the
    // vanilla fixed-radius player sweep as a substitute for the source rules.
    let Some((_,_,speed))=game.world.chunks.__horizon_mixins_tconstruct_melee_context().get(&game.lan_acting).copied() else {return Break(None);};
    let Some(player)=game.player.as_ref() else{return Break(None);};
    // The server-side guest motion's cached speed can be zero while idle.
    // Use its actual movement attribute captured at the attack boundary,
    // retaining Horizon's existing velocity comparison and source conditions.
    let limit=f64::from(speed)*2.5;
    if scale<=0.9 || crit || player.sprinting || !player.ground.on_ground
        || player.velocity.x*player.velocity.x+player.velocity.z*player.velocity.z>=limit*limit {return Break(None);}
    let yaw=f64::from(game.yaw.to_radians());
    Break(Some([player.position.x-yaw.sin(),player.position.y+f64::from(player.dimensions.height)*0.5,player.position.z+yaw.cos()]))
}
