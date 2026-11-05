// Source EnchantmentModule.Constant adds one native Depth Strider level per
// modifier level. Project it to the host's water movement attribute without writing a
// second enchantment into the item or persisting a stale modifier level.
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
use super::tconstruct_world::World;

pub(super) fn supported_definition(world: &World) -> bool {
    let Some(raw) = world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot()
        .as_ref().and_then(|raw| raw.as_ref().ok()) else { return false; };
    let Some(bytes) = raw.0.get("tconstruct:tinkering/modifiers/depth_strider")
        .and_then(|layers| layers.last()) else { return false; };
    serde_json::from_slice::<serde_json::Value>(bytes).ok().is_some_and(|value|
        value == serde_json::json!({"level_display":"tconstruct:default", "modules":[{
            "type":"tconstruct:constant_enchantment", "level":1, "name":"minecraft:depth_strider"
        }]}))
}

pub(super) fn level(world: &World, boots: Option<&ItemStack>) -> Option<i32> {
    let stack = boots?;
    if stack.count != 1 || !matches!(stack.item.as_ref(),
        "tconstruct:travelers_boots" | "tconstruct:plate_boots" | "tconstruct:slime_boots")
        || !supported_definition(world) { return None; }
    let custom = stack.data()?.other("minecraft:custom_data")?;
    let max = match custom.get("tic_max_damage") {
        Some(Tag::Int(max)) if *max > 0 => *max as u32,
        _ => return None,
    };
    if stack.damage >= max || !matches!(custom.get("tic_broken"), None | Some(Tag::Byte(0))) { return None; }
    let tags = world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
    if !tags.get("tconstruct:modifiable/armor/boots")?.contains(stack.item.as_ref()) { return None; }
    let traits = super::tconstruct_source_material_traits::levels(world, stack)?;
    super::tconstruct_source_material_traits::level_with_upgrade(stack, &traits, "tconstruct:depth_strider")
        .filter(|level| *level > 0)
}
