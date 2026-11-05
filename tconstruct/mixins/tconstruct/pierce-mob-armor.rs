fn tconstruct_pierced_mob_armor(
    mob: &mut crate::mobs::Mob,
    original: impl FnOnce(&mut crate::mobs::Mob) -> (f32, f32),
) -> (f32, f32) {
    let (armor, toughness) = original(mob);
    // Only the damage calculation sees this delta. Equipment and MobKind stay
    // untouched; the native formula adds innate and covered-shell armor next.
    let penalty = mob.extra.effects.iter().find(|effect| effect.0 == "tconstruct:pierce")
        .and_then(|effect| crate::tconstruct_source_pierce::armor_penalty(effect.1, effect.2)).unwrap_or(0.0);
    (armor - penalty, toughness)
}
