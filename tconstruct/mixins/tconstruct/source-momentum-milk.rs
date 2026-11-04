// Momentum's source effect is not curable by a milk bucket. Other clear-all
// actions (death, commands, other consumables) retain their host behavior.
fn tconstruct_momentum_milk(game: &mut crate::game::Game<'_>, effect: &horizon_client::items::ConsumeEffect) -> std::ops::ControlFlow<()> {
    if !matches!(effect, horizon_client::items::ConsumeEffect::ClearAllEffects)
        || !game.held().is_some_and(|stack| stack.item.as_ref() == "minecraft:milk_bucket") {
        return std::ops::ControlFlow::Continue(());
    }
    game.potions.effects.list.retain(|entry| matches!(entry.id, "tconstruct:momentum_harvest" | "tconstruct:momentum_ranged"));
    game.apply_effect_status();
    std::ops::ControlFlow::Break(())
}
