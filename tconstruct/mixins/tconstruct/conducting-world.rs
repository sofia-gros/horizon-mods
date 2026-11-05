use horizon_world::nbt::Tag;

pub(super) fn factor(game: &super::Game<'_>) -> Option<f32> {
    if !super::tconstruct_source_conducting::supported_definition(&game.world) { return None; }
    let (stack, traits) = super::tconstruct_smite_world::context(game)?;
    let custom = stack.data()?.other("minecraft:custom_data")?;
    if custom.get("tic_upgrades").is_some_and(|upgrades|
        !matches!(upgrades, Tag::Compound(_)) || upgrades.get("tconstruct:conducting").is_some()) { return None; }
    let level = traits.levels.get("tconstruct:conducting").copied()?;
    let resistance = game.potions.effects.get("fire_resistance")
        .is_some_and(|effect| effect.duration > 0 || effect.duration == -1);
    super::tconstruct_source_conducting::factor(game.player.as_ref()?.fire_ticks, resistance, level)
}

/// Raging is currently added on melee_damage's return. Use the same primary
/// snapshot so its flat contribution receives Conducting exactly once too.
pub(super) fn primary_factor(game: &super::Game<'_>) -> f32 {
    game.world.chunks.__horizon_mixins_tconstruct_smite_context().last()
        .filter(|(actor, _, _, _)| *actor == game.lan_acting)
        .map_or(1.0, |(_, _, _, factor)| *factor)
}
