fn tconstruct_melee_attack_head(game:&mut crate::game::Game<'_>, index:usize) {
    let actor=game.lan_acting;
    if game.world.chunks.__horizon_mixins_tconstruct_melee_active().contains(&actor) {return;}
    let enabled=game.held().is_some_and(|stack| {
        matches!(crate::game::tconstruct_source_melee_shape::attack(stack.item.as_ref()),Some(crate::game::tconstruct_source_melee_shape::Attack::Circle(_)))
            && crate::game::tconstruct_source_transform_logic::maximum(stack).is_some_and(|max| stack.damage<max)
    });
    if enabled {
        if let Some(target)=game.mobs.get(index) {
            game.world.chunks.__horizon_mixins_tconstruct_melee_context_mut().insert(actor,(target.id,game.attack_ticks));
        }
    } else { game.world.chunks.__horizon_mixins_tconstruct_melee_context_mut().remove(&actor); }
}
