// Source crafting-table refill: one existing shield-bearing tool and matching bottles.
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;

pub(super) fn plan(grid: &[Option<ItemStack>]) -> Option<(ItemStack, Vec<Option<ItemStack>>)> {
    let mut tool = None;
    let mut kind = None;
    let mut bottles = Vec::new();
    for (i, slot) in grid.iter().enumerate() {
        let Some(stack) = slot else { continue; };
        let restore = match stack.item.as_ref() {
            "tconstruct:earth_slime_bottle" => Some(20),
            "tconstruct:sky_slime_bottle" => Some(50),
            "tconstruct:ichor_slime_bottle" => Some(60),
            "tconstruct:ender_slime_bottle" => Some(80),
            _ => None,
        };
        if let Some(restore) = restore {
            if kind.is_some_and(|(name, _)| name != stack.item.as_ref()) { return None; }
            kind = Some((stack.item.as_ref(), restore));
            bottles.push(i);
        } else if tool.replace(stack).is_some() { return None; }
    }
    let tool = tool?;
    if tool.count != 1 { return None; }
    let (_, restore) = kind?;
    let cap = super::tconstruct_source_overslime::amount(tool, "tic_overslime_capacity") as u32;
    let current = super::tconstruct_source_overslime::amount(tool, "tic_overslime") as u32;
    let active = matches!(tool.data()?.other("minecraft:custom_data")?.get("tic_overslime_active"), Some(Tag::Byte(n)) if *n != 0);
    if !active && current >= cap { return None; }
    let overworked = super::tconstruct_source_overslime::amount(tool, "tic_overslime_overworked") as u32;
    let (after, count) = if current >= cap { (current, 0) } else {
        super::tconstruct_source_overslime::refill(current, cap, restore, bottles.len() as u32, overworked)?
    };
    let mut output = tool.clone();
    let mut custom = output.data()?.other("minecraft:custom_data")?.clone();
    let Tag::Compound(ref mut fields) = custom else { return None; };
    fields.insert("tic_overslime".into(), Tag::Int(after as i32));
    output.data_mut().set_other("minecraft:custom_data", Some(custom));
    let mut remainder = vec![None; grid.len()];
    for (index, slot) in bottles.into_iter().enumerate() {
        remainder[slot] = Some(ItemStack::new(if index < count as usize { "minecraft:glass_bottle" } else { kind?.0 }, 1));
    }
    Some((output, remainder))
}
