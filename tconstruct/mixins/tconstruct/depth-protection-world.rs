use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
const ID: &str = "tconstruct:depth_protection";

fn supported(game: &super::Game<'_>) -> bool {
    game.world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref()
        .and_then(|raw|raw.as_ref().ok())
        .and_then(|raw|raw.0.get("tconstruct:tinkering/modifiers/depth_protection"))
        .and_then(|layers|layers.last())
        .and_then(|bytes|serde_json::from_slice::<serde_json::Value>(bytes).ok())
        .is_some_and(|value|super::tconstruct_depth_protection::supported_value(&value))
}

fn level(game: &super::Game<'_>, stack: &ItemStack) -> Option<i32> {
    if stack.count != 1 || stack.wire_patch().is_some() { return None; }
    let custom = stack.data()?.other("minecraft:custom_data")?;
    let maximum = match custom.get("tic_max_damage") { Some(Tag::Int(n)) if *n > 0 => *n as u32, _ => return None };
    let broken = match custom.get("tic_broken") { None | Some(Tag::Byte(0)) => false, Some(Tag::Byte(1)) => true, _ => return None };
    if broken || stack.damage >= maximum { return Some(0); }
    let traits = super::tconstruct_source_material_traits::levels(&game.world,stack)?;
    super::tconstruct_source_material_traits::level_with_upgrade(stack,&traits,ID)
}

fn wanted(game: &super::Game<'_>) -> Option<Vec<i32>> {
    if !supported(game) { return None; }
    let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
    let belongs = |stack:&ItemStack, tag:&str|tags.get(tag).is_some_and(|types|types.contains(stack.item.as_ref()));
    let mut levels = Vec::new();
    // Match source EquipmentSlot order, including valid held tools.
    for stack in [game.inventory.get(game.selected).and_then(Option::as_ref),game.offhand.as_ref()].into_iter().flatten() {
        if matches!(stack.item.as_ref(),"tconstruct:travelers_shield" | "tconstruct:plate_shield" | "tconstruct:battlesign")
            && belongs(stack,"tconstruct:modifiable/held") { levels.push(level(game,stack)?); }
    }
    for (slot,stack) in game.armor.iter().enumerate().rev().filter_map(|(slot,stack)|stack.as_ref().map(|stack|(slot,stack))) {
        if super::tconstruct_fire_duration::armor_slot(&stack.item) == Some(slot)
            && belongs(stack,"tconstruct:modifiable/armor/worn") { levels.push(level(game,stack)?); }
    }
    Some(levels)
}

pub(super) fn refresh(game: &mut super::Game<'_>) {
    let levels = if game.remote.is_none() { wanted(game).unwrap_or_default() } else { Vec::new() };
    if let Some(player) = game.player.as_mut() {
        let points = super::tconstruct_depth_protection::points(player.position.y,&levels).unwrap_or(0.0);
        *player.status.__horizon_mixins_tconstruct_depth_protection_levels_mut() = levels;
        *player.status.__horizon_mixins_tconstruct_depth_protection_points_mut() = points;
    }
}
