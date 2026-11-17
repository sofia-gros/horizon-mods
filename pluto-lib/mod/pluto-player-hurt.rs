// The game names the attacker of a hit before it lands (the knockback scope): when that is a mob, the mods hear it with the mob's state and may
// put status effects on the mob. The scope also opens for the player's own hits, which no mob id matches, and for hits on mobs by mobs, which the
// mods hear as a hit on the wearer too; the damage type and the amount are not worked out yet. The scope only reads the game, so the effects are
// queued and land at the next player tick (pluto-mob-effect-apply-tick).
fn pluto_player_hurt(game:&crate::game::Game<'_>,attacker:i32) {
    let Some(sink)=crate::entry::pluto_player_hurt_access::sink() else {return};
    if attacker<=0 {return;}
    let Some(mob)=game.mobs.iter().find(|candidate| candidate.id as i32==attacker) else {return};
    let hurt=crate::entry::pluto_player_hurt_access::PlayerHurt {
        attacker,
        attacker_kind:mob.kind.id.to_string(),
        on_ground:mob.on_ground,
        in_water:mob.fluids.in_water,
        fire_ticks:mob.fire_ticks,
        fire_immune:mob.kind.flags.fire_immune,
        effects:mob.extra.effects.iter().map(|effect| effect.0.to_string()).collect(),
        damage_type:String::new(),
        amount:0.0,
    };
    for (effect,ticks,amplifier) in (sink.hurt)(&hurt) {
        crate::entry::pluto_mob_effect_access::push(attacker as u32,&effect,ticks,amplifier);
    }
}
