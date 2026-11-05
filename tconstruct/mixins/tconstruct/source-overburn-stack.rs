// Stage tank, active fuel, and Overslime together on a detached ItemStack.
use super::tconstruct_source_overburn as rule;
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
const PERSISTENT: &str = "tic_persistent";
const FLUID: &str = "tconstruct:tank_fluid";

fn read_fuel(stack: &ItemStack) -> Option<Option<rule::FuelInfo>> {
    let custom = stack.data()?.other("minecraft:custom_data")?;
    let persistent = match custom.get(PERSISTENT) {
        None => return Some(None),
        Some(Tag::Compound(v)) => v,
        _ => return None,
    };
    let info = match persistent.get(&rule::ID.into()) {
        None => return Some(None),
        Some(Tag::Compound(v)) => v,
        _ => return None,
    };
    if info.len() != 2 {
        return None;
    }
    let (Some(Tag::Long(expiration)), Some(Tag::Int(rate))) =
        (info.get(&"expiration".into()), info.get(&"rate".into()))
    else {
        return None;
    };
    Some(Some(rule::FuelInfo {
        expiration: *expiration,
        rate: u32::try_from(*rate).ok().filter(|n| *n > 0)?,
    }))
}
fn write_fuel(stack: &mut ItemStack, fuel: Option<rule::FuelInfo>) -> Option<()> {
    let mut custom = stack.data()?.other("minecraft:custom_data")?.clone();
    let Tag::Compound(ref mut fields) = custom else {
        return None;
    };
    let mut persistent = match fields.remove(&PERSISTENT.into()) {
        None => Default::default(),
        Some(Tag::Compound(v)) => v,
        _ => return None,
    };
    match fuel {
        Some(fuel) => {
            persistent.insert(
                rule::ID.into(),
                Tag::compound([
                    ("expiration", Tag::Long(fuel.expiration)),
                    ("rate", Tag::Int(i32::try_from(fuel.rate).ok()?)),
                ]),
            );
        }
        None => {
            persistent.remove(&rule::ID.into());
        }
    }
    if !persistent.is_empty() {
        fields.insert(PERSISTENT.into(), Tag::Compound(persistent));
    }
    stack
        .data_mut()
        .set_other("minecraft:custom_data", Some(custom));
    Some(())
}
/// Source removal hook. Other persistent fields, including tank fluid, survive.
pub(super) fn clear_fuel(stack: &mut ItemStack) -> Option<()> {
    write_fuel(stack, None)
}

pub(super) fn plan(
    stack: &ItemStack,
    level: i32,
    overworked: i32,
    now: i64,
    fuel: impl FnOnce(&str) -> Option<rule::Fuel>,
    mut roll: impl FnMut() -> Option<u32>,
) -> Option<ItemStack> {
    if stack.count != 1 || stack.wire_patch().is_some() || overworked < 0 {
        return None;
    }
    let interval = rule::interval(level)?;
    let custom = stack.data()?.other("minecraft:custom_data")?;
    if !matches!(custom.get("tic_overslime_active"), Some(Tag::Byte(1))) {
        return None;
    }
    let (Some(Tag::Int(current)), Some(Tag::Int(capacity))) = (
        custom.get("tic_overslime"),
        custom.get("tic_overslime_capacity"),
    ) else {
        return None;
    };
    if !super::tconstruct_source_overgrowth::eligible(*current, *capacity, level) {
        return None;
    }
    let (current, capacity) = (*current as u32, *capacity as u32);
    let old = read_fuel(stack)?;
    let needs_fuel = old.is_none_or(|info| info.expiration < now);
    let mut output = stack.clone();
    let mut next_fluid = None;
    let mut tank_capacity = 0;
    let refill = if needs_fuel {
        tank_capacity = match custom.get("tic_tank_capacity")? {
            Tag::Int(n) if *n > 0 => *n as u32,
            _ => return None,
        };
        let fluid = super::pluto_stack_fluid::read(stack, PERSISTENT, FLUID)?;
        match fluid {
            Some(fluid) if fluid.amount <= tank_capacity && fluid.tag.is_none() => {
                let recipe = fuel(&fluid.id);
                let available = fluid.amount;
                next_fluid = Some(fluid);
                recipe.map(|f| (available, f))
            }
            Some(_) => return None,
            None => None,
        }
    } else {
        None
    };
    let step = rule::fuel_step(now, interval, old, refill)?;
    if step.consumed > 0 {
        let fluid = next_fluid.as_mut()?;
        fluid.amount = fluid.amount.checked_sub(step.consumed)?;
        output = super::pluto_stack_fluid::write(
            &output,
            PERSISTENT,
            FLUID,
            (fluid.amount > 0).then_some(&*fluid),
            tank_capacity,
        )?;
    }
    let restore = match step.active {
        None => 0,
        Some(info) => rule::restore(
            info.rate,
            overworked,
            if info.rate % 10 > 0 {
                Some(roll()?)
            } else {
                None
            },
        )?,
    };
    write_fuel(&mut output, step.saved)?;
    let mut custom = output.data()?.other("minecraft:custom_data")?.clone();
    let Tag::Compound(ref mut fields) = custom else {
        return None;
    };
    fields.insert(
        "tic_overslime".into(),
        Tag::Int(current.saturating_add(restore).min(capacity) as i32),
    );
    output
        .data_mut()
        .set_other("minecraft:custom_data", Some(custom));
    Some(output)
}
