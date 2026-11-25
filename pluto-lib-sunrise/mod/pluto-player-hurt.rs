// The game names the attacker of a hit before it lands (the knockback scope): when that is a mob close to the player, the mods hear it with the
// mob's state and may put status effects on the mob. The scope also opens for the player's own hits (no mob id matches), for nested scopes of
// one hit and for mobs hitting each other: a mob is heard once for each game tick, and only within a few blocks of the player, which leaves a
// mob fighting another mob beside the player as the one case heard wrongly. The damage type and the amount are not worked out yet and are
// reported as empty and 0. The scope only reads the game, so the effects are queued and land at the next player tick (pluto-mob-effect-apply-tick).
// The same opening names the attacker for the damage events: what the mob holds in its main hand and wears on its chest, its attack attribute
// and its state are noted for the hit that follows, for a victim of any kind and at any distance, which the wearer report above does not cover.
// A player's own opening is noted too, with both hands, so a shot's damage event can name the hand that launched.
fn pluto_player_hurt(game:&crate::game::Game<'_>,attacker:i32) {
    use std::sync::atomic::{AtomicI32,AtomicI64,Ordering};
    static LAST_TICK:AtomicI64=AtomicI64::new(i64::MIN);
    static LAST_ATTACKER:AtomicI32=AtomicI32::new(0);
    if attacker==0 {
        // The player's own hit: the scope opens with the player's entity id, which the host writes as 0.
        // The damage events read only the hands from this note: the shot's weapon says which of them
        // launched. The rest stays neutral (no attack attribute, chest piece or effects a mob's note
        // carries), and the player is never heard as hurt by their own hit.
        if let Some(facts_sink)=crate::entry::pluto_damage_facts_access::sink() {
            if let Some(player)=game.player.as_ref() {
                let facts=crate::entry::pluto_damage_facts_access::AttackerFacts {
                    attacker,
                    kind:"minecraft:player".to_string(),
                    main_hand:game.held().cloned(),
                    off_hand:game.offhand.clone(),
                    chest:None,
                    attack_damage:0.0,
                    on_ground:player.ground.on_ground,
                    in_water:player.swimming,
                    fire_ticks:player.fire_ticks,
                    fire_immune:false,
                    effects:Vec::new(),
                };
                (facts_sink.attacker)(&facts);
            }
        }
        return;
    }
    if attacker<=0 {return;}
    let Some(mob)=game.mobs.iter().find(|candidate| candidate.id as i32==attacker) else {return};
    if let Some(facts_sink)=crate::entry::pluto_damage_facts_access::sink() {
        // The worn chest stack when the mob keeps it (a piece with data of its own), else the plain stack of the
        // chest item it wears.
        let chest=mob.extra.armor[1].clone().or_else(|| mob.extra.chest.as_ref().map(|item| crate::items::ItemStack {item:item.clone(),count:1,damage:0,extra:None}));
        let facts=crate::entry::pluto_damage_facts_access::AttackerFacts {
            attacker,
            kind:mob.kind.id.to_string(),
            main_hand:mob.extra.main_hand_stack().cloned(),
            off_hand:mob.extra.off_hand.as_ref().map(|item| crate::items::ItemStack {item:item.clone(),count:1,damage:0,extra:None}),
            chest,
            attack_damage:mob.attrs.attack_damage,
            on_ground:mob.on_ground,
            in_water:mob.fluids.in_water,
            fire_ticks:mob.fire_ticks,
            fire_immune:mob.kind.flags.fire_immune,
            effects:mob.extra.effects.iter().map(|effect| effect.0.to_string()).collect(),
        };
        (facts_sink.attacker)(&facts);
    }
    let Some(sink)=crate::entry::pluto_player_hurt_access::sink() else {return};
    let Some(player)=game.player.as_ref() else {return};
    let (dx,dy,dz)=(mob.position.x-player.position.x,mob.position.y-player.position.y,mob.position.z-player.position.z);
    if dx*dx+dy*dy+dz*dz>49.0 {return;}
    let tick=game.world.game_time;
    if LAST_TICK.swap(tick,Ordering::Relaxed)==tick && LAST_ATTACKER.swap(attacker,Ordering::Relaxed)==attacker {return;}
    LAST_ATTACKER.store(attacker,Ordering::Relaxed);
    let hurt=crate::entry::pluto_player_hurt_access::PlayerHurt {
        attacker,
        attacker_kind:mob.kind.id.to_string(),
        position:[mob.position.x,mob.position.y,mob.position.z],
        on_ground:mob.on_ground,
        in_water:mob.fluids.in_water,
        eyes_in_water:mob.fluids.eye_in_water,
        // The game's own check (weather, sky light, motion-blocking height, biome precipitation)
        // at the attacker's feet block, the same block vanilla's entity block position gives.
        rain_exposed:game.raining_at(horizon_core::position::BlockPos::new(mob.position.x.floor() as i32, mob.position.y.floor() as i32, mob.position.z.floor() as i32)),
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
