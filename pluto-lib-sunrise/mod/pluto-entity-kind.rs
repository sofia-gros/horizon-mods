// Native mob_kind unconditionally prefixes minecraft:, corrupting a derived
// namespaced type. Preserve that type's actual identity for every API consumer.
fn pluto_entity_kind(mob:&sunrise_client::mobs::Mob)->std::ops::ControlFlow<String> {
    let id=&mob.kind.id;
    std::ops::ControlFlow::Break(if id.contains(':') {id.clone()} else {format!("minecraft:{id}")})
}
