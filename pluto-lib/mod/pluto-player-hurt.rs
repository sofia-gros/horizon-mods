// The game names the attacker of a hit before it lands (the knockback scope): when that is a mob and the player is the one hurt, the mods hear
// it with the mob's state and may put status effects on the mob. The scope also opens for the player's own hits, which no mob id matches, and for
// hits on mobs by mobs, which the mods may hear as a hit on the wearer too; the damage type and the amount are not worked out yet.
fn pluto_player_hurt(game:&mut crate::game::Game<'_>,attacker:i32) {
    let Some(sink)=crate::entry::pluto_player_hurt_access::sink() else {return};
    if attacker<=0 {return;}
    let Some(index)=game.mobs.iter().position(|candidate| candidate.id as i32==attacker) else {return};
    let hurt={
        let mob=&game.mobs[index];
        crate::entry::pluto_player_hurt_access::PlayerHurt {
            attacker,
            attacker_kind:mob.kind.id.to_string(),
            on_ground:mob.on_ground,
            in_water:mob.in_water,
            fire_ticks:mob.fire_ticks,
            fire_immune:mob.kind.flags.fire_immune,
            effects:mob.extra.effects.iter().map(|effect| effect.0.to_string()).collect(),
            damage_type:String::new(),
            amount:0.0,
        }
    };
    for (effect,ticks,amplifier) in (sink.hurt)(&hurt) {
        if ticks<=0 || amplifier<0 {continue;}
        let effect=crate::entry::pluto_mob_effect_access::interned(&effect);
        if let Some(mob)=game.mobs.iter_mut().find(|candidate| candidate.id as i32==attacker) {mob.add_effect(effect,ticks,amplifier);}
    }
}
