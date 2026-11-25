// As a hit is opened on a living entity, before the hit's own hooks run, the mods are told the health a mob
// victim has (and its attack attribute), so the damage events of the hit can read them and clamp by them.
// Players, the ender dragon and armor stands carry no mob here: the dragon is named by its own reach
// (pluto-damage-dragon), and a player's health is not named at this stage.
fn pluto_damage_victim(hurting:&sunrise_client::health::Hurting<'_>) {
    let Some(sink)=crate::entry::pluto_damage_facts_access::sink() else {return};
    if let sunrise_client::health::Hurting::Mob(mob,_)=hurting {
        (sink.victim)(&crate::entry::pluto_damage_facts_access::VictimFacts {entity:mob.id as i32,health:mob.health,attack_damage:Some(mob.attrs.attack_damage)});
    }
}
