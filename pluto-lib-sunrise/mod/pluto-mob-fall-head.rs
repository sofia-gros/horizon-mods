fn pluto_mob_fall_dispatch(mob: &crate::mobs::Mob, contact: Option<(&crate::mobs::MobWorld<'_>, [i32; 3])>, distance: f64, damage: f32) -> std::ops::ControlFlow<f32> {
    match pluto_mob_landing::fall(mob, contact, distance, damage) {
        Some(damage) => std::ops::ControlFlow::Break(damage),
        None => std::ops::ControlFlow::Continue(()),
    }
}
