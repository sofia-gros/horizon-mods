fn tconstruct_melee_secondary_damage(value:(f32,f32,bool,bool),game:&mut crate::game::Game<'_>) -> (f32,f32,bool,bool) {
    let (amount,scale,crit,magic)=value;
    if !crit || !game.world.chunks.__horizon_mixins_tconstruct_melee_active().contains(&game.lan_acting) {return value;}
    // Source withAOETarget keeps the primary cooldown but explicitly clears the
    // critical multiplier. Circle tools do not have the native mace smash.
    let hand=1.0+game.attack_item_info().map_or(0.0,|item|item.attack_damage);
    let spin=game.player.as_ref().map_or(0.0,|player|player.auto_spin_attack_dmg);
    let base=if spin>0.0 {spin} else {hand+game.effect_attack_bonus()};
    (amount-0.5*base*(0.2+scale*scale*0.8),scale,false,magic)
}
