pub(super) fn before_hit(game: &mut super::Game<'_>, index: usize, scale: f32) {
    let eligible = (|| {
        if !scale.is_finite() || scale <= 0.9 || !game.mobs.get(index)?.alive()
            || !super::tconstruct_source_enderference::supported_definition(&game.world)
            || horizon_client::brewing::effect("tconstruct:enderference").is_none() { return None; }
        let (stack, traits) = super::tconstruct_smite_world::context(game)?;
        // Chorus's material trait is supported. There is no ordinary upgrade
        // recipe in this slice, so an opaque saved purchased entry is rejected.
        if stack.data()?.other("minecraft:custom_data")?.get("tic_upgrades")
            .is_some_and(|upgrades| !matches!(upgrades, horizon_world::nbt::Tag::Compound(_))
                || upgrades.get("tconstruct:enderference").is_some()) { return None; }
        (traits.levels.get("tconstruct:enderference").copied().unwrap_or(0) > 0).then_some(())
    })().is_some();
    if !eligible { return; }
    let Some(mob) = game.mobs.get_mut(index) else { return; };
    let (new, entity) = (!mob.has_effect("tconstruct:enderference"), mob.id as i32);
    // Source time and amplifier are flat, independent of the material level.
    mob.add_effect("tconstruct:enderference", 100, 0);
    game.mod_effect_added("tconstruct:enderference", Some(entity), 0, new);
}
