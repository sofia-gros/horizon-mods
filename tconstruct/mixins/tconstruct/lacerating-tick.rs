fn tconstruct_lacerating_tick(mob: &mut crate::mobs::Mob) {
    use crate::mobs::{Damage, DamageKind, MobEvent};
    let Some((_, remaining, _)) = mob.extra.effects.iter()
        .find(|effect| effect.0 == "tconstruct:bleeding").copied() else {
        *mob.__horizon_mixins_tconstruct_lacerating_attacker_mut() = None;
        return;
    };
    if !mob.alive() || !crate::tconstruct_source_lacerating::damage_tick(remaining) { return; }
    let source = Damage {
        // Generic has the required bypass-armor / no-knockback behavior without
        // treating bleeding as magic (e.g. a witch's magic resistance).
        kind: DamageKind::Generic,
        attacker: *mob.__horizon_mixins_tconstruct_lacerating_attacker(),
        from: None,
    };
    let old_invulnerable = mob.invulnerable;
    let old_last_hurt = mob.last_hurt;
    mob.invulnerable = 0;
    mob.damage(1.0, source);
    mob.invulnerable = old_invulnerable;
    // ToolAttackUtil.hurtNoInvulnerableTime accumulates the previous hit's
    // damage, preserving the native hurt-frame comparison for the next hit.
    mob.last_hurt += old_last_hurt;
    mob.outbox.push(MobEvent::LevelParticles {
        kind: "minecraft:damage_indicator",
        at: [mob.position.x, mob.center()[1], mob.position.z],
        spread: [0.1, 0.0, 0.1], speed: 0.2, count: 1,
    });
}
