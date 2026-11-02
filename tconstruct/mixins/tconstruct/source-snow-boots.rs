// Source snow_boots is a volatile tool flag; breaking the tool does not clear it.
pub(super) fn refresh(game: &mut super::Game<'_>) {
    use horizon_world::nbt::Tag;
    let boots = game.armor[3].as_ref();
    let vanilla = boots.is_some_and(|stack| stack.item.as_ref() == "minecraft:leather_boots");
    let tconstruct = boots.is_some_and(|stack| {
        if !stack.item.starts_with("tconstruct:") { return false; }
        let is_boots = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()
            .and_then(|tags| tags.as_ref().ok())
            .and_then(|tags| tags.get("forge:armors/boots"))
            .is_some_and(|items| items.contains(stack.item.as_ref()));
        if !is_boots { return false; }
        let upgrade = matches!(stack.data().and_then(|data| data.other("minecraft:custom_data"))
            .and_then(|data| data.get("tic_upgrades")).and_then(|upgrades| upgrades.get("tconstruct:snow_boots")),
            Some(Tag::Int(level)) if *level > 0);
        upgrade || super::tconstruct_source_material_traits::levels(&game.world, stack)
            .is_some_and(|traits| traits.levels.get("tconstruct:snow_boots").copied().unwrap_or(0) > 0)
    });
    if let Some(player) = game.player.as_mut() { player.leather_boots = vanilla || tconstruct; }
}
