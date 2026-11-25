// The first damage declaration has one LOAD: the value passed to mod_block_fall.
// Pinned Scopes ignores reads of Frog/Goat's compound-write places. Select that
// read after class adjustments, on the uncancelled/nonimmune branch only. The
// later shadowed damage is a different local. Actual native plan remains pending.
fn pluto_living_entity_fall(value:f64,mob:&Mob,distance:f64,multiplier:f32)->f64 {
    let Some(sink)=super::pluto_living_entity_access::sink() else {return value};
    if !(sink.has_fall)(&mob.kind.id) {return value;}
    (sink.fall)(mob.id,&mob.kind.id,distance,multiplier,value).map_or(value,f64::from)
}
