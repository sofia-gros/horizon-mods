use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
const ID: &str = "tconstruct:turtle_shell";

fn supported(game: &super::Game<'_>) -> bool {
    game.world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref()
        .and_then(|raw|raw.as_ref().ok())
        .and_then(|raw|raw.0.get("tconstruct:tinkering/modifiers/turtle_shell"))
        .and_then(|layers|layers.last())
        .and_then(|bytes|serde_json::from_slice::<serde_json::Value>(bytes).ok())
        .is_some_and(|value|super::tconstruct_turtle_shell::supported_value(&value))
}

fn level(game: &super::Game<'_>, stack: &ItemStack) -> Option<f32> {
    if stack.count != 1 || stack.wire_patch().is_some() { return None; }
    let custom = stack.data()?.other("minecraft:custom_data")?;
    let maximum = match custom.get("tic_max_damage") { Some(Tag::Int(n)) if *n > 0 => *n as u32, _ => return None };
    let broken = match custom.get("tic_broken") { None | Some(Tag::Byte(0)) => false, Some(Tag::Byte(1)) => true, _ => return None };
    if broken || stack.damage >= maximum { return Some(0.0); }
    let raw = match custom.get("tic_upgrades") {
        None => 0,
        Some(upgrades @ Tag::Compound(_)) => match upgrades.get(ID) { None => 0, Some(Tag::Int(raw)) => *raw, _ => return None },
        _ => return None,
    };
    let traits = super::tconstruct_source_material_traits::levels(&game.world,stack)?;
    super::tconstruct_source_protection::effective_level(traits.levels.get(ID).copied().unwrap_or(0),raw)
}

fn wanted(game: &super::Game<'_>) -> Option<Vec<(f32,bool,bool)>> {
    if !supported(game) { return None; }
    let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
    let belongs = |stack:&ItemStack, tag:&str|tags.get(tag).is_some_and(|types|types.contains(stack.item.as_ref()));
    let mut equipment = Vec::new();
    for stack in [game.inventory.get(game.selected).and_then(Option::as_ref),game.offhand.as_ref()].into_iter().flatten() {
        if matches!(stack.item.as_ref(),"tconstruct:travelers_shield" | "tconstruct:plate_shield" | "tconstruct:battlesign")
            && belongs(stack,"tconstruct:modifiable/held") { equipment.push(stack); }
    }
    for (slot,stack) in game.armor.iter().enumerate().rev().filter_map(|(slot,stack)|stack.as_ref().map(|stack|(slot,stack))) {
        if super::tconstruct_fire_duration::armor_slot(&stack.item) == Some(slot)
            && belongs(stack,"tconstruct:modifiable/armor/worn") { equipment.push(stack); }
    }
    equipment.into_iter().map(|stack|Some((level(game,stack)?,
        belongs(stack,"tconstruct:modifiable/armor/helmets") || belongs(stack,"tconstruct:modifiable/armor/chestplate"),
        belongs(stack,"tconstruct:modifiable/armor/leggings") || belongs(stack,"tconstruct:modifiable/armor/boots"),
    ))).collect()
}

pub(super) fn refresh(game: &mut super::Game<'_>) {
    let levels = if game.remote.is_none() { wanted(game).unwrap_or_default() } else { Vec::new() };
    if let Some(player) = game.player.as_mut() {
        let points = super::tconstruct_turtle_shell::points(&levels,
            player.fluids.was_eye_in_water,player.fluids.was_touching_water).unwrap_or(0.0);
        *player.status.__horizon_mixins_tconstruct_turtle_shell_levels_mut() = levels;
        *player.status.__horizon_mixins_tconstruct_turtle_shell_points_mut() = points;
    }
}
