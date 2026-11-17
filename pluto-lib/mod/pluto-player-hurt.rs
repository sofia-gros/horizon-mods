// The game notes the mob that is about to hurt the player (the one place that names the attacker before the hit): the mods hear it with the
// mob's state and may put status effects on the mob. The amount of the hit is not worked out yet and is reported as 0.
fn pluto_player_hurt(game:&mut crate::game::Game<'_>,damage_type:&str,mob:u32) {
    let Some(sink)=crate::entry::pluto_player_hurt_access::sink() else {return};
    let Some(index)=game.mobs.iter().position(|candidate| candidate.id==mob) else {return};
    let hurt={
        let attacker=&game.mobs[index];
        crate::entry::pluto_player_hurt_access::PlayerHurt {
            attacker:mob as i32,
            attacker_kind:attacker.kind.id.to_string(),
            on_ground:attacker.on_ground,
            in_water:attacker.in_water,
            fire_ticks:attacker.fire_ticks,
            fire_immune:attacker.kind.flags.fire_immune,
            effects:attacker.extra.effects.iter().map(|effect| effect.0.to_string()).collect(),
            damage_type:damage_type.to_string(),
            amount:0.0,
        }
    };
    for (effect,ticks,amplifier) in (sink.hurt)(&hurt) {
        if ticks<=0 || amplifier<0 {continue;}
        let effect=crate::entry::pluto_mob_effect_access::interned(&effect);
        if let Some(attacker)=game.mobs.iter_mut().find(|candidate| candidate.id==mob) {attacker.add_effect(effect,ticks,amplifier);}
    }
}
