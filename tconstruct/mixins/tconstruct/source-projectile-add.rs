fn tconstruct_projectile_add(game: &mut crate::game::Game<'_>, p: &mut horizon_client::projectiles::Projectile) {
    use horizon_client::projectiles::ProjectileKind;
    if !matches!(p.kind, ProjectileKind::Arrow { .. }) { return; }
    let actor = game.lan_acting;
    let shot = game.world.chunks.__horizon_mixins_tconstruct_material_projectile_context_mut().get_mut(&actor).and_then(|(velocity, spread, shots, bow, damage, crystal, glowing)| {
        if shots.is_empty() { None } else { let (angle, power, primary) = shots.remove(0); Some((*velocity, *spread, angle, power, *bow, *damage, *crystal, *glowing, primary)) }
    });
    let Some((velocity, spread, angle, power, bow, damage, crystal, glowing, primary)) = shot else { return; };
    if let ProjectileKind::Arrow { ref mut base_damage, .. } = p.kind { *base_damage += damage; }
    if glowing {
        if let Some(stack) = game.held().cloned() {
            if let Some(arrow) = p.arrow.as_mut() { if arrow.fired_from_weapon.is_none() { arrow.fired_from_weapon = Some(stack.clone()); } }
            if primary && !game.creative {
                let (loss, shield) = crate::game::tconstruct_source_overslime_host::damage_with_cause(game, &stack, 5, Some("tconstruct:glowing"));
                if let Some(held) = game.held_mut().as_mut().filter(|held| held.item == stack.item) {
                    crate::game::tconstruct_source_overslime::spend(held, shield);
                    crate::game::tconstruct_tx::fixed_damage(held, loss);
                }
            }
        }
    }
    if bow {
        if let ProjectileKind::Arrow { ref mut critical, .. } = p.kind { *critical = power >= 3.0; }
        let (v, on_ground) = game.player.as_ref().map_or((horizon_core::geometry::Vec3::ZERO, true), |pl| (pl.velocity, pl.ground.on_ground));
        p.shoot_from_rotation(&mut game.world.random, game.pitch, game.yaw, 0.0, power * velocity, spread, v, on_ground);
    } else {
        let view = glam::DVec3::from_array(super::view_vector(game.yaw, game.pitch));
        let up = glam::DVec3::from_array(super::view_vector(game.yaw, game.pitch - 90.0));
        let dir = glam::DQuat::from_axis_angle(up.normalize_or_zero(), angle.to_radians()) * view;
        p.shoot(&mut game.world.random, dir.to_array(), power * velocity, spread);
    }
    let (direction, yaw) = crate::game::tconstruct_projectile_stats::restrict_direction([p.velocity.x, p.velocity.y, p.velocity.z], crystal);
    if let Some(yaw) = yaw { p.velocity.x = direction[0]; p.velocity.y = direction[1]; p.velocity.z = direction[2]; p.yaw = yaw; }
    // Stamp after any launch hook that compares the native weapon snapshot.
    crate::game::tconstruct_enderference_arrow_world::launch(game, p);
}
