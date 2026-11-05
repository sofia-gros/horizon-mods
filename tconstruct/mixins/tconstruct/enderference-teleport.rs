fn tconstruct_enderference_teleport(mob: &crate::mobs::Mob) -> Option<bool> {
    mob.extra.effects.iter().any(|effect| effect.0 == "tconstruct:enderference"
        && crate::tconstruct_source_enderference::active(effect.1)).then_some(false)
}
