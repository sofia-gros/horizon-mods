fn pluto_living_entity_jump(mob:&Mob,world:&MobWorld<'_>)->std::ops::ControlFlow<f32> {
    let Some(sink)=super::pluto_living_entity_access::sink() else {return std::ops::ControlFlow::Continue(())};
    if !(sink.has_jump)(&mob.kind.id) {return std::ops::ControlFlow::Continue(());}
    let size=matches!(mob.species(),super::Species::Slime|super::Species::MagmaCube|super::Species::SulfurCube)
        .then_some(mob.extra.size);
    match (sink.jump)(mob.id,&mob.kind.id,size,mob.block_jump_factor(world)) {
        Some(power)=>std::ops::ControlFlow::Break(power),
        None=>std::ops::ControlFlow::Continue(()),
    }
}
