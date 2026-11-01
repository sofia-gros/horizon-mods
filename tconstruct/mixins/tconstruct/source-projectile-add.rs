fn tconstruct_projectile_add(game: &mut crate::game::Game<'_>, p: &mut horizon_client::projectiles::Projectile) {
    use horizon_client::projectiles::ProjectileKind;
    if !matches!(p.kind, ProjectileKind::Arrow { .. }) { return; }
    let actor = game.lan_acting;
    let shot = game.world.chunks.__horizon_mixins_tconstruct_material_projectile_context_mut().get_mut(&actor).and_then(|(velocity, spread, shots, bow)| {
        if shots.is_empty() { None } else { let (angle, power) = shots.remove(0); Some((*velocity, *spread, angle, power, *bow)) }
    });
    let Some((velocity, spread, angle, power, bow)) = shot else { return; };
    if bow {
        let (v, on_ground) = game.player.as_ref().map_or((horizon_core::geometry::Vec3::ZERO, true), |pl| (pl.velocity, pl.ground.on_ground));
        p.shoot_from_rotation(&mut game.world.random, game.pitch, game.yaw, 0.0, power * velocity, spread, v, on_ground);
    } else {
        let view = glam::DVec3::from_array(super::view_vector(game.yaw, game.pitch));
        let up = glam::DVec3::from_array(super::view_vector(game.yaw, game.pitch - 90.0));
        let dir = glam::DQuat::from_axis_angle(up.normalize_or_zero(), angle.to_radians()) * view;
        p.shoot(&mut game.world.random, dir.to_array(), power * velocity, spread);
    }
}
