use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;

// Source's default attribute-style module grants a flat one jump per intact
// equipped piece, at every positive level. The separate gravity module has an
// exact-level condition and remains active on broken armor.
pub(super) fn capacity(
    world: &super::tconstruct_world::World,
    armor: &[Option<ItemStack>],
) -> Option<u8> {
    if armor.len() != 4 {
        return None;
    }
    let raw = world
        .chunks
        .__horizon_mixins_tconstruct_modifier_resource_snapshot()
        .as_ref()?
        .as_ref()
        .ok()?;
    if !super::tconstruct_dreambound::known(
        raw.0
            .get("tconstruct:tinkering/modifiers/dreambound")
            .and_then(|layers| layers.last())
            .map(Vec::as_slice),
    ) {
        return None;
    }
    let tags = world
        .chunks
        .__horizon_mixins_tconstruct_resolved_item_tags()
        .as_ref()?
        .as_ref()
        .ok()?;
    let worn = tags.get("tconstruct:modifiable/armor/worn")?;
    let modifiable = tags.get("tconstruct:modifiable")?;
    let mut levels = [0; 4];
    for (index, stack) in armor
        .iter()
        .enumerate()
        .filter_map(|(i, stack)| stack.as_ref().map(|stack| (i, stack)))
    {
        if super::tconstruct_dreambound::armor_slot(&stack.item) != Some(index)
            || !worn.contains(stack.item.as_ref())
            || !modifiable.contains(stack.item.as_ref())
        {
            continue;
        }
        if stack.count != 1 || stack.wire_patch().is_some() {
            return None;
        }
        let custom = stack.data()?.other("minecraft:custom_data")?;
        let max = match custom.get("tic_max_damage")? {
            Tag::Int(n) if *n > 0 => *n as u32,
            _ => return None,
        };
        let broken = match custom.get("tic_broken") {
            None => false,
            Some(Tag::Byte(value)) => *value != 0,
            _ => return None,
        };
        if broken || stack.damage >= max {
            continue;
        }
        let traits = super::tconstruct_source_material_traits::levels(world, stack)?;
        levels[index] = super::tconstruct_source_material_traits::level_with_upgrade(
            stack,
            &traits,
            super::tconstruct_dreambound::ID,
        )?;
    }
    super::tconstruct_dreambound::extra_jumps(&levels)
}
