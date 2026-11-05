use horizon_world::nbt::Tag;

pub(super) fn after_hit(game: &mut super::Game<'_>, index: usize) {
    let candidate = (|| {
        let mob = game.mobs.get(index)?;
        let record = game.player.as_ref()?.attack_log.last()?;
        if !mob.alive() || record.id != mob.id || !record.landed
            || !record.scale.is_finite() || record.scale <= 0.9
            || !super::tconstruct_source_lacerating::supported_definition(&game.world)
            || horizon_client::brewing::effect("tconstruct:bleeding").is_none() { return None; }
        let (stack, traits) = super::tconstruct_smite_world::context(game)?;
        let custom = stack.data()?.other("minecraft:custom_data")?;
        if custom.get("tic_upgrades").is_some_and(|upgrades|
            !matches!(upgrades, Tag::Compound(_)) || upgrades.get("tconstruct:lacerating").is_some()) { return None; }
        let level = traits.levels.get("tconstruct:lacerating").copied()?;
        let existing = mob.extra.effects.iter().find(|effect| effect.0 == "tconstruct:bleeding")
            .map(|effect| (effect.1, effect.2));
        // The baseline source starts at Looting 0. Weapon/armor Looting hooks
        // are not implemented by this slice; amplifier never scales damage.
        super::tconstruct_source_lacerating::duration(level, 0, existing)
    })();
    let Some(ticks) = candidate else { return; };
    let Some(mob) = game.mobs.get_mut(index) else { return; };
    let (new, entity) = (!mob.has_effect("tconstruct:bleeding"), mob.id as i32);
    *mob.__horizon_mixins_tconstruct_lacerating_attacker_mut() = Some(horizon_client::mobs::Who::Player);
    mob.add_effect("tconstruct:bleeding", ticks, 0);
    game.mod_effect_added("tconstruct:bleeding", Some(entity), 0, new);
}
