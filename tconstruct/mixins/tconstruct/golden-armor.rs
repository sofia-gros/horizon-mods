fn tconstruct_golden_armor(value: bool, game: &crate::game::Game<'_>) -> bool {
    use horizon_world::nbt::Tag;
    if value { return true; }
    let Some(tags) = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()
        .and_then(|tags| tags.as_ref().ok()) else { return false; };
    let Some(worn) = tags.get("tconstruct:modifiable/armor/worn") else { return false; };
    game.armor.iter().flatten().any(|stack| {
        if !stack.item.starts_with("tconstruct:") || !worn.contains(stack.item.as_ref()) { return false; }
        let upgrade = matches!(stack.data().and_then(|data| data.other("minecraft:custom_data"))
            .and_then(|data| data.get("tic_upgrades")).and_then(|upgrades| upgrades.get("tconstruct:golden")),
            Some(Tag::Int(level)) if *level > 0);
        upgrade || crate::game::tconstruct_source_material_traits::levels(&game.world, stack)
            .is_some_and(|traits| traits.levels.get("tconstruct:golden").copied().unwrap_or(0) > 0)
    })
}
