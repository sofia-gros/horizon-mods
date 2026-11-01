// Original Netherite sets rare rarity; enchantments raise rare to epic.
fn tconstruct_netherite_rarity(color: u32, stack: &ItemStack) -> u32 {
    use horizon_world::nbt::Tag;
    let netherite = matches!(stack.data().and_then(|d| d.other("minecraft:custom_data"))
        .and_then(|d| d.get("tconstruct:netherite")), Some(Tag::Byte(v)) if *v != 0);
    if !netherite { return color; }
    if stack.enchantments().is_empty() { rgba(85, 255, 255, 255) }
    else { rgba(255, 85, 255, 255) }
}
