// Keep the selected item name's rarity in step with the workstation tooltip.
fn tconstruct_netherite_held_name(game: &crate::game::Game<'_>) -> std::ops::ControlFlow<Option<HeldName>> {
    use std::ops::ControlFlow::{Break, Continue};
    use horizon_world::nbt::Tag;
    let Some(stack) = game.inventory.get(game.selected).and_then(Option::as_ref) else { return Continue(()); };
    if !matches!(stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|d| d.get("tconstruct:netherite")), Some(Tag::Byte(v)) if *v != 0) {
        return Continue(());
    }
    Break(Some(HeldName {
        item: stack.item.to_string(),
        hover_name: Text::literal(item_tooltip::name_line(stack, &|key| game.pack.names.display_get(key))),
        rarity_color: if stack.enchantments().is_empty() { text::AQUA } else { text::LIGHT_PURPLE },
        custom_name: stack.custom_name().is_some(),
    }))
}
