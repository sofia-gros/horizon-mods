use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;

// One shared jump-count projection: Dreambound gives one per intact piece,
// Double Jump gives one per logical level. Gravity remains independent.
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
    let definition = |id: &str| {
        raw.0
            .get(id)
            .and_then(|layers| layers.last())
            .map(Vec::as_slice)
    };
    let dreambound_known = super::tconstruct_dreambound::known(definition(
        "tconstruct:tinkering/modifiers/dreambound",
    ));
    let double_jump_known = super::tconstruct_dreambound::known_double_jump(definition(
        "tconstruct:tinkering/modifiers/double_jump",
    ));
    let tags = world
        .chunks
        .__horizon_mixins_tconstruct_resolved_item_tags()
        .as_ref()?
        .as_ref()
        .ok()?;
    let worn = tags.get("tconstruct:modifiable/armor/worn")?;
    let modifiable = tags.get("tconstruct:modifiable")?;
    let mut levels = [0; 4];
    let mut double_jump = [0; 4];
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
        double_jump[index] = super::tconstruct_source_material_traits::level_with_upgrade(
            stack,
            &traits,
            super::tconstruct_dreambound::DOUBLE_JUMP_ID,
        )?;
        // An unrelated missing definition must not disable the other modifier,
        // but a positive unsupported contribution must never grant permission.
        if (levels[index] > 0 && !dreambound_known)
            || (double_jump[index] > 0 && !double_jump_known)
        {
            return None;
        }
    }
    super::tconstruct_dreambound::combined_jumps(&levels, &double_jump)
}
