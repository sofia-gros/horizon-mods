fn pluto_mob_move_dispatch(mob: &mut crate::mobs::Mob, world: &crate::mobs::MobWorld<'_>, events: &mut Vec<crate::mobs::MobEvent>) -> std::ops::ControlFlow<()> {
    pluto_mob_landing::do_move(mob, world, events);
    std::ops::ControlFlow::Break(())
}
