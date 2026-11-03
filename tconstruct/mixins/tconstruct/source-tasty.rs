// Bounded source Tasty use on a held tool, excluding overlapping food/use traits.
use horizon_items::items::ItemStack;
use horizon_core::random::RandomSource;

fn profile(tasty: i32, scrumptious: i32) -> Option<(i32, f32, u32)> {
    if tasty <= 0 || scrumptious < 0 { return None; }
    let hunger = tasty.checked_add(scrumptious)?;
    let mut damage = u32::try_from(tasty).ok()?.checked_mul(5)?.checked_add(5)?;
    if scrumptious > 0 { damage = damage.checked_add(u32::try_from(scrumptious).ok()?.checked_mul(3)?.checked_add(5)?)?; }
    Some((hunger, if scrumptious > 0 { 0.6 + 0.1 } else { 0.6 }, damage))
}

pub(super) fn levels(game: &super::Game<'_>, stack: &ItemStack) -> Option<(i32, i32)> {
    if !stack.item.starts_with("tconstruct:") || stack.count != 1 || stack.wire_patch().is_some()
        || super::tconstruct_source_melee_shape::intact(stack).is_none() { return None; }
    let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
    if traits.levels.get("tconstruct:savory").copied().unwrap_or(0) > 0 { return None; }
    let upgrades = stack.data().and_then(|data| data.other("minecraft:custom_data")).and_then(|data| data.get("tic_upgrades"));
    if ["tconstruct:scrumptious", "tconstruct:savory"].iter().any(|id| matches!(upgrades.and_then(|data| data.get(id)), Some(horizon_world::nbt::Tag::Int(level)) if *level > 0)) { return None; }
    let tasty = super::tconstruct_source_material_traits::level_with_upgrade(stack, &traits, "tconstruct:tasty")?;
    let scrumptious = traits.levels.get("tconstruct:scrumptious").copied().unwrap_or(0);
    profile(tasty, scrumptious)?;
    Some((tasty, scrumptious))
}

pub(super) fn can_start(game: &super::Game<'_>, stack: &ItemStack) -> bool {
    !super::tconstruct_source_slurping::passive_item(stack) && levels(game, stack).is_some() && game.player.as_ref().is_some_and(|player| !player.health.dead_or_dying() && (game.creative || player.health.food.food < 20))
        && super::tconstruct_source_slurping::plan(game, stack).is_none()
}

pub(super) fn finish(game: &mut super::Game<'_>, stack: &ItemStack, result: &mut Option<ItemStack>) {
    if game.remote.is_some() || !result.as_ref().is_some_and(|held| held.count == stack.count && held.damage == stack.damage && held.same_item_same_components(stack)) { return; }
    if !can_start(game, stack) { return; }
    eat(game, stack, result);
}

fn eat(game: &mut super::Game<'_>, stack: &ItemStack, result: &mut Option<ItemStack>) {
    let Some((tasty, scrumptious)) = levels(game, stack) else { return; };
    let Some((hunger, saturation, damage)) = profile(tasty, scrumptious) else { return; };
    let mut next = stack.clone();
    if !game.creative { let next_damage = next.damage.saturating_add(damage); super::tconstruct_tx::fixed_set_damage(&mut next, next_damage); }
    game.apply_effect_action(horizon_client::effects::EffectAction::Eat(hunger, saturation));
    if scrumptious > 0 {
        let before = game.potions.effects.list.len();
        game.potions.effects.list.retain(|effect| effect.id != "minecraft:poison");
        if game.potions.effects.list.len() != before { game.apply_effect_status(); }
    }
    if let Some(at) = game.player.as_ref().map(|p| [p.position.x, p.position.y, p.position.z]) {
        let pitch = 1.0 + (game.world.random.next_float() - game.world.random.next_float()) * 0.4;
        let burp_pitch = game.world.random.next_float() * 0.1 + 0.9;
        game.audio.play("minecraft:entity.generic.eat", Some(at), 1.0, pitch);
        game.audio.play("minecraft:entity.player.burp", Some(at), 0.5, burp_pitch);
    }

    *result = Some(next);
}

// The source EdibleModule runs once at 16 ticks while another modifier is
// active, without ending that use. Limit this to known supported active uses.
pub(super) fn concurrent_tick(game: &mut super::Game<'_>, stack: &ItemStack) {
    if game.remote.is_some() || !matches!(game.using, Some((_, 16))) { return; }
    let active = matches!(&*stack.item, "tconstruct:longbow" | "tconstruct:crossbow" | "tconstruct:battlesign" | "tconstruct:travelers_shield" | "tconstruct:plate_shield")
        || (!super::tconstruct_source_slurping::passive_item(stack) && super::tconstruct_source_slurping::plan(game, stack).is_some());
    if !active || !game.player.as_ref().is_some_and(|p| !p.health.dead_or_dying() && (game.creative || p.health.food.food < 20)) { return; }
    let Some((tasty, scrumptious)) = levels(game, stack) else { return; };
    let Some((_, _, damage)) = profile(tasty, scrumptious) else { return; };
    let Some(max) = super::tconstruct_source_melee_shape::intact(stack) else { return; };
    // Launcher continuation re-reads the held stack after this callback.
    // Other active uses still carry the earlier stack for this tick.
    let launcher = matches!(&*stack.item, "tconstruct:longbow" | "tconstruct:crossbow");
    if !launcher && !game.creative && stack.damage.saturating_add(damage) >= max { return; }
    if !game.held().is_some_and(|held| held.count == stack.count && held.damage == stack.damage && held.same_item_same_components(stack)) { return; }
    let mut result = Some(stack.clone());
    eat(game, stack, &mut result);
    *game.held_mut() = result;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tasty_profile_matches_source_hunger_saturation_and_direct_wear() {
        assert_eq!(profile(0, 0), None);
        assert_eq!(profile(1, 0), Some((1, 0.6, 10)));
        assert_eq!(profile(2, 0), Some((2, 0.6, 15)));
        let (hunger, saturation, damage) = profile(1, 1).unwrap();
        assert_eq!((hunger, damage), (2, 18));
        assert!((saturation - 0.7).abs() < 0.00001);
        assert_eq!(profile(i32::MAX, 1), None);
    }
}
