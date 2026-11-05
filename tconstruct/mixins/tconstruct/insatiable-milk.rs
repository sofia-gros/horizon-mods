// Wrap the consume call so the existing Momentum milk HEAD cannot remove
// Insatiable first. Preserve the supported NoMilkEffect instances in place,
// including hidden upgrades; commands and non-milk clears stay native.
fn tconstruct_insatiable_milk(
    game: &mut crate::game::Game<'_>,
    effect: &horizon_client::items::ConsumeEffect,
    stack: &horizon_client::items::ItemStack,
    original: impl FnOnce(&mut crate::game::Game<'_>, &horizon_client::items::ConsumeEffect),
) {
    if stack.item.as_ref() == "minecraft:milk_bucket"
        && matches!(
            effect,
            horizon_client::items::ConsumeEffect::ClearAllEffects
        )
    {
        game.potions.effects.list.retain(|entry| {
            matches!(
                entry.id,
                "tconstruct:insatiable_melee"
                    | "tconstruct:insatiable_ranged"
                    | "tconstruct:momentum_harvest"
                    | "tconstruct:momentum_ranged"
            )
        });
        game.apply_effect_status();
    } else {
        original(game, effect);
    }
}
