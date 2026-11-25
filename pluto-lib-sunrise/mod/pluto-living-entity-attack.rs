fn pluto_living_entity_attack(mob:&Mob)->std::ops::ControlFlow<f32> {
    let Some(sink)=super::pluto_living_entity_access::sink() else {return std::ops::ControlFlow::Continue(())};
    if !(sink.has_attack)(&mob.kind.id) {return std::ops::ControlFlow::Continue(());}
    let attribute=super::finalize::value(mob,super::finalize::Attr::AttackDamage);
    match (sink.attack)(mob.id,&mob.kind.id,attribute) {
        Some(damage)=>std::ops::ControlFlow::Break(damage),
        None=>std::ops::ControlFlow::Continue(()),
    }
}
