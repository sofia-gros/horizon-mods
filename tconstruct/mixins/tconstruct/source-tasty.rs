// Bounded source Tasty use on a held tool, excluding overlapping food/use traits.
use horizon_items::items::ItemStack;

fn profile(level: i32) -> Option<(i32, f32, u32)> {
    if level <= 0 { return None; }
    let damage = u32::try_from(level).ok()?.checked_mul(5)?.checked_add(5)?;
    Some((level, 0.6, damage))
}

pub(super) fn level(game: &super::Game<'_>, stack: &ItemStack) -> Option<i32> {
    if !stack.item.starts_with("tconstruct:") || stack.count != 1 || stack.wire_patch().is_some()
        || super::tconstruct_source_melee_shape::intact(stack).is_none()
        || super::tconstruct_source_slurping::passive_item(stack) { return None; }
    let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
    if ["tconstruct:scrumptious", "tconstruct:savory"].iter().any(|id| traits.levels.get(*id).copied().unwrap_or(0) > 0) { return None; }
    let upgrades = stack.data().and_then(|data| data.other("minecraft:custom_data")).and_then(|data| data.get("tic_upgrades"));
    if ["tconstruct:scrumptious", "tconstruct:savory"].iter().any(|id| matches!(upgrades.and_then(|data| data.get(id)), Some(horizon_world::nbt::Tag::Int(level)) if *level > 0)) { return None; }
    let level = traits.levels.get("tconstruct:tasty").copied().unwrap_or(0);
    (level > 0).then_some(level)
}

pub(super) fn can_start(game: &super::Game<'_>, stack: &ItemStack) -> bool {
    level(game, stack).is_some() && game.player.as_ref().is_some_and(|player| !player.health.dead_or_dying() && (game.creative || player.health.food.food < 20))
        && super::tconstruct_source_slurping::plan(game, stack).is_none()
}

pub(super) fn finish(game: &mut super::Game<'_>, stack: &ItemStack, result: &mut Option<ItemStack>) {
    if game.remote.is_some() || !result.as_ref().is_some_and(|held| held.count == stack.count && held.damage == stack.damage && held.same_item_same_components(stack)) { return; }
    if !can_start(game, stack) { return; }
    let Some(level) = level(game, stack) else { return; };
    let Some((hunger, saturation, damage)) = profile(level) else { return; };
    let mut next = stack.clone();
    if !game.creative { let next_damage = next.damage.saturating_add(damage); super::tconstruct_tx::fixed_set_damage(&mut next, next_damage); }
    game.apply_effect_action(horizon_client::effects::EffectAction::Eat(hunger, saturation));
    *result = Some(next);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tasty_profile_matches_source_hunger_saturation_and_direct_wear() {
        assert_eq!(profile(0), None);
        assert_eq!(profile(1), Some((1, 0.6, 10)));
        assert_eq!(profile(2), Some((2, 0.6, 15)));
        assert_eq!(profile(i32::MAX), None);
    }
}
