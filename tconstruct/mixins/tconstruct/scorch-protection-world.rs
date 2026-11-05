use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
const ID: &str = "tconstruct:scorch_protection";

pub(super) fn supported(game: &super::Game<'_>) -> bool {
    game.world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref()
        .and_then(|raw|raw.as_ref().ok())
        .and_then(|raw|raw.0.get("tconstruct:tinkering/modifiers/scorch_protection"))
        .and_then(|layers|layers.last())
        .and_then(|bytes|serde_json::from_slice::<serde_json::Value>(bytes).ok())
        .is_some_and(|value|super::tconstruct_source_scorch_protection::supported_value(&value))
}

/// Enchantment generation has no brokenness filter in the source hook.
pub(super) fn level(game: &super::Game<'_>, stack: &ItemStack) -> Option<i32> {
    let traits = super::tconstruct_source_material_traits::levels(&game.world,stack)?;
    super::tconstruct_source_material_traits::level_with_upgrade(stack,&traits,ID)
}

pub(super) fn piece(game: &super::Game<'_>, stack: &ItemStack, worn: bool) -> Option<(f32,bool,bool)> {
    if stack.count != 1 || stack.wire_patch().is_some() { return None; }
    let custom = stack.data()?.other("minecraft:custom_data")?;
    let maximum = match custom.get("tic_max_damage") { Some(Tag::Int(n)) if *n > 0 => *n as u32, _ => return None };
    let broken = match custom.get("tic_broken") { None | Some(Tag::Byte(0)) => false, Some(Tag::Byte(1)) => true, _ => return None };
    Some((level(game,stack)? as f32,broken || stack.damage >= maximum,worn))
}
