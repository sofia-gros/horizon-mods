// As the ender dragon's damage is worked out (its actuallyHurt), before the damage hooks run, the mods are
// told the health the dragon has, so the damage event of the hit can clamp by it (a dragon scale drop counts
// by the damage the dragon can still take). The dragon is no mob, so the mob victim reach cannot name it;
// its attack attribute is not named (nothing here reads it).
fn pluto_damage_dragon(dragon:&Dragon) {
    let Some(sink)=crate::pluto_damage_facts_access::sink() else {return};
    (sink.victim)(&crate::pluto_damage_facts_access::VictimFacts {entity:dragon.id as i32,health:dragon.health,attack_damage:None});
}
