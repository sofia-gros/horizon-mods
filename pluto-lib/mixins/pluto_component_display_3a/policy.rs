use horizon_client::{items::ItemStack, text::Text};

pub(crate) fn component_durability_lines(stack: &ItemStack, lines: &mut Vec<(Text, u32)>, advanced: bool) {
    let Some(horizon_world::nbt::Tag::Int(max)) = stack.data().and_then(|d| d.other("minecraft:max_damage")) else { return };
    let Ok(max) = u32::try_from(*max) else { return };
    if max == 0 { return; }
    let replacement = Text::translate("item.durability", &[
        Text::literal(max.saturating_sub(stack.damage).to_string()),
        Text::literal(max.to_string()),
    ]);
    let mut found = false;
    for (text, _) in lines.iter_mut() {
        if text.translation_key() == Some("item.durability") {
            *text = replacement.clone();
            found = true;
        }
    }
    if !found && advanced && stack.damage > 0
        && horizon_client::item_tooltip::TooltipDisplay::for_stack(stack).shows("minecraft:damage")
    {
        // The host puts durability immediately before the advanced item ID.
        let id = Text::literal(stack.item.to_string());
        let index = lines.iter().rposition(|(text, _)| *text == id).unwrap_or(lines.len());
        lines.insert(index, (replacement, u32::MAX));
    }
}
