fn tconstruct_bottle_cauldron_remote(game: &crate::game::Game<'_>, hit: &super::Hit, held: Option<&horizon_items::items::ItemStack>) -> std::ops::ControlFlow<(InteractionResult, DryUse)> {
    use std::ops::ControlFlow::{Break, Continue};
    if game.keys.contains(&KeyCode::ShiftLeft) && (game.stack_in(game.selected).is_some() || game.offhand.is_some()) { return Continue(()); }
    let Some(item) = held else { return Continue(()); };
    let Some((state, _)) = game.cauldron_at(hit.pos) else { return Continue(()); };
    let level = if state.name == "minecraft:water_cauldron" { state.properties.get("level").and_then(|level| level.parse::<u8>().ok()).unwrap_or(0) }
        else if state.name == "minecraft:cauldron" { 0 } else { return Continue(()); };
    let fills = level > 0 && matches!(item.item.as_ref(), "tconstruct:splash_bottle" | "tconstruct:lingering_bottle");
    let empties = level < 3 && matches!(item.item.as_ref(), "minecraft:splash_potion" | "minecraft:lingering_potion") && item.potion() == Some("minecraft:water");
    if fills || empties { Break((InteractionResult::Success, DryUse::default())) } else { Continue(()) }
}
