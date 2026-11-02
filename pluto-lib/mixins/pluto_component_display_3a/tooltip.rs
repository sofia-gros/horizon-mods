fn component_durability_tooltip(stack: &ItemStack, lines: &mut Vec<(Text, u32)>) {
    let Some(horizon_world::nbt::Tag::Int(max)) = stack.data().and_then(|d| d.other("minecraft:max_damage")) else { return };
    let Ok(max) = u32::try_from(*max) else { return };
    if max == 0 { return; }
    for (text, _) in lines.iter_mut() {
        if text.translation_key() == Some("item.durability") {
            *text = Text::translate("item.durability", &[
                Text::literal(max.saturating_sub(stack.damage).to_string()),
                Text::literal(max.to_string()),
            ]);
        }
    }
}
