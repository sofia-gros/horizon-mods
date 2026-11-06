// Test-only Overslime policy. Material selection and formulas stay in TConstruct.
use pluto_inventory_api::{
    advancements::predicates::{ComponentValue, ItemSnapshot},
    crafting::products::{CompoundPatch, MissingCompound, Preparer, Product, Proposal, Source},
};
fn amount(item: ItemSnapshot<'_>, key: &str) -> u32 {
    match item
        .components
        .and_then(|c| c.get("minecraft:custom_data"))
        .and_then(|c| c.get(key))
    {
        Some(ComponentValue::Int(v)) => (*v).max(0) as u32,
        _ => 0,
    }
}
pub fn propose(capability: &impl Preparer) -> Option<Proposal> {
    let mut tool = None;
    let mut kind = None;
    let mut bottles = Vec::new();
    let cells = capability.grid().cells().ok()?;
    for index in 0..cells {
        let Some(stack) = capability.cell(index).ok()? else {
            continue;
        };
        let restore = match stack.id {
            "tconstruct:earth_slime_bottle" => Some(20),
            "tconstruct:sky_slime_bottle" => Some(50),
            "tconstruct:ichor_slime_bottle" => Some(60),
            "tconstruct:ender_slime_bottle" => Some(80),
            _ => None,
        };
        if let Some(restore) = restore {
            if kind.is_some_and(|(id, _)| id != stack.id) {
                return None;
            }
            kind = Some((stack.id, restore));
            bottles.push(index);
        } else if tool.replace((index, stack)).is_some() {
            return None;
        }
    }
    let (index, tool) = tool?;
    if tool.count != 1 {
        return None;
    }
    let (_, restore) = kind?;
    let cap = amount(tool, "tic_overslime_capacity");
    let current = amount(tool, "tic_overslime");
    let custom = tool.components?.get("minecraft:custom_data")?;
    let active =
        matches!(custom.get("tic_overslime_active"),Some(ComponentValue::Byte(v)) if *v!=0);
    if !active && current >= cap {
        return None;
    }
    let overworked = amount(tool, "tic_overslime_overworked");
    let (after, count) = if current >= cap {
        (current, 0)
    } else {
        super::tconstruct_source_overslime::refill(
            current,
            cap,
            restore,
            bottles.len() as u32,
            overworked,
        )?
    };
    if !matches!(custom, ComponentValue::Compound(_)) {
        return None;
    }
    let output = Product {
        source: Source::Input(index),
        count: 1,
        patches: vec![CompoundPatch {
            component: "minecraft:custom_data".into(),
            missing: MissingCompound::Reject,
            fields: vec![(
                "tic_overslime".into(),
                Some(ComponentValue::Int(after as i32)),
            )],
        }],
    };
    let mut remainders = vec![None; cells];
    for (order, index) in bottles.into_iter().enumerate() {
        remainders[index] = Some(Product {
            source: if order < count as usize {
                Source::Fresh("minecraft:glass_bottle".into())
            } else {
                Source::Input(index)
            },
            count: 1,
            patches: vec![],
        });
    }
    Some(Proposal { output, remainders })
}
