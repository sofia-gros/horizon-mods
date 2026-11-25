fn pluto_mob_inside_dispatch(mob: &mut crate::mobs::Mob, world: &crate::mobs::MobWorld<'_>, random: &mut sunrise_core::random::LegacyRandom, events: &mut Vec<crate::mobs::MobEvent>) -> std::ops::ControlFlow<()> {
    pluto_mob_inside::effects(mob, world, random, events);
    std::ops::ControlFlow::Break(())
}
