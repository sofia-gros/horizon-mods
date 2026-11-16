// A mob hurts the player: the mods hear it with the mob's state and may put status effects on the mob.
fn pluto_player_hurt(game:&mut crate::game::Game<'_>,source:horizon_world::damage_source::DamageTypeId,amount:f32,causing:i32) {
    let Some(sink)=crate::entry::pluto_player_hurt_access::sink() else {return};
    if causing<=0 {return;}
    let Some(index)=game.mobs.iter().position(|mob| mob.id as i32==causing) else {return};
    let damage_type=game.health_resources.damage.definition(source).map(|definition| definition.name.clone()).unwrap_or_default();
    let mob=&game.mobs[index];
    let hurt=crate::entry::pluto_player_hurt_access::PlayerHurt {
        attacker:causing,
        attacker_kind:mob.kind.id.to_string(),
        on_ground:mob.on_ground,
        in_water:mob.in_water,
        fire_ticks:mob.fire_ticks,
        fire_immune:mob.kind.flags.fire_immune,
        effects:mob.extra.effects.iter().map(|effect| effect.0.to_string()).collect(),
        damage_type:damage_type.to_string(),
        amount,
    };
    for (effect,ticks,amplifier) in (sink.hurt)(&hurt) {
        if ticks<=0 || amplifier<0 {continue;}
        let effect=crate::entry::pluto_mob_effect_access::interned(&effect);
        if let Some(mob)=game.mobs.iter_mut().find(|mob| mob.id as i32==causing) {mob.add_effect(effect,ticks,amplifier);}
    }
}
