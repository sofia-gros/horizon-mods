use horizon_world::nbt::Tag;

pub(super) fn before_hit(game: &mut super::Game<'_>, index: usize, scale: f32) {
    let candidate = (|| {
        if !scale.is_finite() || scale <= 0.9 || !game.mobs.get(index)?.alive()
            || !super::tconstruct_source_pierce::supported_definition(&game.world)
            || horizon_client::brewing::effect("tconstruct:pierce").is_none() { return None; }
        let (stack, traits) = super::tconstruct_smite_world::context(game)?;
        let custom = stack.data()?.other("minecraft:custom_data")?;
        let amount = match custom.get("tic_upgrades") {
            None => 0,
            Some(upgrades @ Tag::Compound(_)) => match upgrades.get("tconstruct:pierce") {
                None => 0, Some(Tag::Int(amount)) => *amount, _ => return None,
            },
            _ => return None,
        };
        let level = super::tconstruct_source_upgrade_rules::pierce_effective_level(
            traits.levels.get("tconstruct:pierce").copied().unwrap_or(0), amount)?;
        super::tconstruct_source_pierce::amplifier(level)
    })();
    let Some(amplifier) = candidate else { return; };
    let Some(mob) = game.mobs.get_mut(index) else { return; };
    let (new, entity) = (!mob.has_effect("tconstruct:pierce"), mob.id as i32);
    mob.add_effect("tconstruct:pierce", 80, amplifier);
    game.mod_effect_added("tconstruct:pierce", Some(entity), amplifier, new);
}
