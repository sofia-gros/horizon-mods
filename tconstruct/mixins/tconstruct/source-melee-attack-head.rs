fn tconstruct_melee_attack_head(game:&mut crate::game::Game<'_>, index:usize) {
    if game.remote.is_some() {return;}
    let actor=game.lan_acting;
    if game.world.chunks.__horizon_mixins_tconstruct_melee_active().contains(&actor) {return;}
    let enabled=game.held().is_some_and(|stack| {
        crate::game::tconstruct_source_melee_shape::attack(stack.item.as_ref()).is_some()
            && crate::game::tconstruct_source_melee_shape::intact(stack).is_some()
    });
    if enabled {
        let speed=game.player.as_mut().and_then(|player|player.attribute("minecraft:movement_speed").ok()).unwrap_or(0.0) as f32;
        if let Some(target)=game.mobs.get(index) {
            game.world.chunks.__horizon_mixins_tconstruct_melee_context_mut().insert(actor,(target.id,game.attack_ticks,speed));
        }
    } else { game.world.chunks.__horizon_mixins_tconstruct_melee_context_mut().remove(&actor); }
}
