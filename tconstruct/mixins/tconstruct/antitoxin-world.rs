use horizon_world::nbt::Tag;
use horizon_items::items::ItemStack;
use super::tconstruct_source_material_traits::Traits;

fn poison(game: &super::Game<'_>) -> Option<i32> {
    let effect = game.potions.effects.get("poison")?;
    (effect.duration == -1 || effect.duration > 0).then_some(effect.amplifier)
}

fn level(game: &super::Game<'_>, stack: &ItemStack, traits: &Traits) -> Option<i32> {
    if !super::tconstruct_source_antitoxin::supported_definition(&game.world) { return None; }
    let custom = stack.data()?.other("minecraft:custom_data")?;
    if custom.get("tic_upgrades").is_some_and(|upgrades| !matches!(upgrades, Tag::Compound(_))
        || upgrades.get("tconstruct:antitoxin").is_some()) { return None; }
    traits.levels.get("tconstruct:antitoxin").copied().filter(|level| *level > 0)
}

pub(super) fn melee_bonus(game: &super::Game<'_>, stack: &ItemStack, traits: &Traits, multiplier: f32) -> Option<f32> {
    super::tconstruct_source_antitoxin::bonus(poison(game)?, level(game, stack, traits)?, multiplier, false)
}

pub(super) fn draw_bonus(game: &super::Game<'_>, stack: &ItemStack) -> Option<f32> {
    if game.remote.is_some() || stack.count != 1 || stack.wire_patch().is_some()
        || !matches!(stack.item.as_ref(), "tconstruct:longbow" | "tconstruct:crossbow")
        || super::tconstruct_source_melee_shape::intact(stack).is_none() { return None; }
    // release_used_item clears `using` first; its acting hand still identifies
    // the released stack. Crossbow ticks and model queries use the live use slot.
    let actual = match game.using { Some((slot,_)) => game.stack_in(slot), None => game.held() };
    if actual != Some(stack) || game.player.is_none() { return None; }
    let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
    if !tags.get("tconstruct:modifiable/ranged/launcher")?.contains(stack.item.as_ref()) { return None; }
    let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
    let custom = stack.data()?.other("minecraft:custom_data")?;
    if !matches!(custom.get("tic_broken"), None | Some(Tag::Byte(0))) { return None; }
    // This existing rebuild field is the shared conditional draw multiplier:
    // tool multiply_stats plus Vintage, excluding base-only Quick Charge.
    let multiplier = match custom.get("tic_overwield_draw_multiplier") {
        Some(Tag::Float(bits)) => f32::from_bits(*bits), _ => return None,
    };
    super::tconstruct_source_antitoxin::bonus(poison(game)?, level(game, stack, &traits)?, multiplier, true)
}

pub(super) fn draw_ticks(game: &super::Game<'_>, stack: &ItemStack) -> u32 {
    super::tconstruct_ranged_draw::draw_ticks_with_bonus(stack, draw_bonus(game, stack).unwrap_or(0.0))
}
