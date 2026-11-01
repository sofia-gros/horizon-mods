fn tconstruct_melee_attack_return(value:(),game:&mut crate::game::Game<'_>) -> () {
    let actor=game.lan_acting;
    if !game.world.chunks.__horizon_mixins_tconstruct_melee_active().contains(&actor) {
        game.world.chunks.__horizon_mixins_tconstruct_melee_context_mut().remove(&actor);
    }
    value
}
