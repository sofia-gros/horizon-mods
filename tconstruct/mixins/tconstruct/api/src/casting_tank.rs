//! The casting tank, from SlimeKnights CastingTankBlock and CastingTankBlockEntity (MIT; see LICENSE): a tank of
//! four buckets with two single-item slots in front of it. An item put in the input slot trades its fluid with
//! the tank (a filled container empties into it, an empty one fills from it) and the result lands in the output
//! slot; using the block trades fluid with the held container, or swaps items around when no fluid moves. The
//! block's `light` state follows the fluid and the comparator reads the fill, as for the tanks (`tank.rs`). The
//! tank is saved under `tank` as Forge's `FluidName` and `Amount`, the item keeps it the same way (the block's
//! item is a `TankItem` of the same capacity), and the saved `redstone` flag is the last signal the block saw.
//!
//! Not ported, each named: the click split of the original's `use` (a click below 0.6875 of the block's height
//! trades fluid, above it swaps items) — the use hook carries no hit position, so a fluid trade is tried first
//! and the item swap second, as the fluid cannons do; the redstone trigger itself — no PlutoLib hook tells a
//! block entity that a neighbour changed or lets it read the signal, so `handle_redstone` (a rising edge swaps
//! the slots and plays the click) waits for that wiring, as the casting tables' redstone swapping does;
//! automation's sided slot rules (in through the input while the output is empty, out of the output only, and
//! only fluid containers in) — the host's slots take no rules, the processing below simply leaves a stack it
//! cannot trade with where it lies; the fluid and the items drawn in the block (the render data is already
//! generated; the renderer keys tank drawing off `tank::capacity_of`, which does not know this block) and the
//! placed item's fluid seeding the block entity (the tanks' known gap: the placement stack is not reachable).

use crate::{
    fluid_items::TankTarget,
    fluid_values::BUCKET,
    melter_blocks::facing_toward,
    tank::{Fluid, Tank},
};
use pluto_lib::{
    ActionResult, BlockBehavior, BlockBehaviors, BlockEntityDrain, BlockEntityFill, BlockEntityTick, BlockEntityType, BlockEntityTypeRegistry, BlockEntityUse,
    FluidItemDef, FluidItems, FluidStack, Identifier, ItemSlot, Nbt, NbtCompound, NbtCompoundExt, NeighbourTank, Placement, TransferDirection, World,
};

pub(crate) const BLOCK: &str = "seared_casting_tank";
/// `CastingTankBlockEntity.DEFAULT_CAPACITY`: four buckets.
pub(crate) const CAPACITY: i32 = BUCKET * 4;
/// The slot an item goes into to be filled or emptied.
pub(crate) const INPUT: usize = 0;
/// The slot the processed item lands in.
pub(crate) const OUTPUT: usize = 1;
/// `CastingTankBlockEntity.TAG_REDSTONE`.
const TAG_REDSTONE: &str = "redstone";

fn tank_of(data: &NbtCompound) -> Tank { Tank::read(data, CAPACITY) }

fn slot(items: &[Option<ItemSlot>], index: usize) -> Option<&ItemSlot> { items.get(index).and_then(Option::as_ref) }

fn take_slot(items: &mut [Option<ItemSlot>], index: usize) -> Option<ItemSlot> { items.get_mut(index)?.take() }

fn put_slot(items: &mut [Option<ItemSlot>], index: usize, stack: Option<ItemSlot>) {
    if let Some(cell) = items.get_mut(index) { *cell = stack; }
}

/// The tank as a trade partner for `FluidItems::interact`, and back.
fn target_of(tank: &Tank) -> TankTarget {
    TankTarget { fluid: tank.fluid.as_ref().map(|fluid| FluidStack { fluid: fluid.fluid.clone(), amount: fluid.amount as u32 }), capacity: CAPACITY as u32 }
}

fn apply_target(tank: &mut Tank, target: &TankTarget) {
    tank.fluid = target.fluid.as_ref().map(|fluid| Fluid { fluid: fluid.fluid.clone(), amount: fluid.amount as i32 });
}

/// `ITankBlockEntity.comparatorStrength` over the saved tank.
pub(crate) fn comparator(data: &NbtCompound) -> i32 { tank_of(data).comparator() }

/// `CastingTankBlockEntity.canPlaceItem` for the input slot: both slots empty, and the stack may hold fluid —
/// in the original, a stack a Mantle container transfer may use or with a fluid handler; here, a registered
/// fluid container.
pub(crate) fn can_place_input(items: &[Option<ItemSlot>], stack: &ItemSlot) -> bool {
    slot(items, INPUT).is_none() && slot(items, OUTPUT).is_none() && FluidItems::is_container(stack)
}

/// `CastingTankBlockEntity.tryToProcessItem`: the input item trades its fluid with the tank and the result
/// goes to the output slot. The original runs it when the input is set and when the tank changes; here the
/// ticker runs it every tick, which reaches the same end state, as the melter's cycle does for its slots.
fn process(data: &mut NbtCompound, items: &mut [Option<ItemSlot>]) {
    if slot(items, OUTPUT).is_some() { return; }
    let Some(input) = slot(items, INPUT).cloned() else { return };
    let mut tank = tank_of(data);
    let mut target = target_of(&tank);
    let Some(done) = FluidItems::interact(Some(&input), &mut target, TransferDirection::Auto) else { return };
    apply_target(&mut tank, &target);
    tank.write(data);
    // a trade over several containers hands the processed one back separately and keeps the rest in the hand;
    // a single item (the slot's limit) simply becomes the result
    let (result, rest) = match done.give.into_iter().next() {
        Some(result) => (Some(result), done.held),
        None => (done.held, None),
    };
    put_slot(items, OUTPUT, result);
    put_slot(items, INPUT, rest);
}

/// `CastingTankBlockEntity.swap`: the two slots change places.
pub(crate) fn swap_slots(items: &mut [Option<ItemSlot>]) {
    if items.len() > OUTPUT { items.swap(INPUT, OUTPUT); }
}

/// `CastingTankBlockEntity.handleRedstone`: a rising signal swaps the slots (the original schedules the swap
/// two ticks later) and plays `Sounds.CASTING_CLICKS`; the flag is saved either way. Nothing calls this yet:
/// the block has no neighbour hook to learn the signal from (see the module notes).
pub(crate) fn handle_redstone(data: &mut NbtCompound, items: &mut [Option<ItemSlot>], has_signal: bool, pos: [i32; 3]) {
    let last = data.get_int(TAG_REDSTONE, 0) != 0;
    if has_signal && !last {
        swap_slots(items);
        if let Some(world) = World::ticking() {
            world.play_sound("tconstruct:casting_clicks", [f64::from(pos[0]) + 0.5, f64::from(pos[1]) + 0.5, f64::from(pos[2]) + 0.5], 1.0, 1.0);
        }
    }
    data.insert(TAG_REDSTONE.to_owned(), Nbt::Int(i32::from(has_signal)));
}

fn tick_casting_tank(tick: &mut BlockEntityTick<'_>) {
    // TankBlockEntity.updateLight, which the original runs on every tank change: the `light` state follows the fluid
    let wanted = tank_of(tick.data).light();
    if let Some(world) = World::ticking() {
        if let Some(state) = world.get_block_state(tick.pos) {
            let text = wanted.to_string();
            if state.property("light").is_some_and(|light| light != text) { world.set_block_state(tick.pos, &state.with("light", &text)); }
        }
    }
    process(tick.data, tick.items);
}

/// `CastingTankBlockEntity.interact` without the click split (see the module notes): the held container trades
/// with the tank first; then the output is taken, then the input, then a held container is placed.
fn use_casting_tank(used: &mut BlockEntityUse<'_>) -> ActionResult {
    let mut tank = tank_of(used.data);
    let mut target = target_of(&tank);
    if let Some(done) = FluidItems::interact(used.held.as_ref(), &mut target, TransferDirection::Auto) {
        apply_target(&mut tank, &target);
        tank.write(used.data);
        used.held = done.held;
        used.give.extend(done.give);
        return ActionResult::Success;
    }
    if let Some(output) = take_slot(used.items, OUTPUT) {
        used.give.push(output);
    } else if let Some(input) = take_slot(used.items, INPUT) {
        used.give.push(input);
    } else if let Some(held) = used.held.clone() {
        if can_place_input(used.items, &held) {
            let mut one = held.clone();
            one.count = 1;
            put_slot(used.items, INPUT, Some(one));
            used.held = if held.count > 1 { let mut rest = held; rest.count -= 1; Some(rest) } else { None };
        }
    }
    ActionResult::Success
}

pub(crate) fn read_tank(data: &NbtCompound) -> Vec<NeighbourTank> {
    let tank = tank_of(data);
    vec![NeighbourTank { fluid: tank.fluid.map(|fluid| FluidStack { fluid: fluid.fluid, amount: fluid.amount as u32 }), capacity: tank.capacity as u32 }]
}

pub(crate) fn fill_tank(fill: &mut BlockEntityFill<'_>) -> u32 {
    let mut tank = tank_of(fill.data);
    let filled = tank.fill(&fill.fluid.fluid, fill.fluid.amount.min(i32::MAX as u32) as i32, true);
    if filled > 0 { tank.write(fill.data); }
    filled.max(0) as u32
}

pub(crate) fn drain_tank(drain: &mut BlockEntityDrain<'_>) -> Option<FluidStack> {
    let mut tank = tank_of(drain.data);
    let out = tank.drain(drain.wanted.as_ref(), drain.amount.min(i32::MAX as u32) as i32, true)?;
    tank.write(drain.data);
    Some(FluidStack { fluid: out.fluid, amount: out.amount as u32 })
}

pub(crate) fn register() {
    let block = Identifier::new("tconstruct", BLOCK).expect("valid casting tank id");
    BlockEntityTypeRegistry::register(
        BlockEntityType::new(block.clone()).item_slots(2).sync_data()
            .ticker(tick_casting_tank)
            .fluid_reader(|data| read_tank(data))
            .fluid_filler(fill_tank)
            .fluid_drainer(drain_tank)
            .on_use(use_casting_tank),
    );
    // CastingTankBlock.getStateForPlacement: facing the placer; the light it also sets from the placed item's
    // fluid follows on the first tick instead (the placement context names the item, not its data)
    BlockBehaviors::register(BlockBehavior::new(block.clone()).state_for_placement(|c| Placement::State(c.state.clone().with("facing", facing_toward(c.yaw)))));
    // the item is a TankItem of the block's capacity (limitStackSize: a stack above 16 takes no fluid)
    FluidItems::register(FluidItemDef::tank(block, crate::tank::TAG_TANK, CAPACITY as u32).scaled().fill_stack_limit(16));
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::BlockEntityUse;

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
    fn stack(text: &str, count: u32) -> ItemSlot { ItemSlot::new(id(text), count) }
    fn fluid(text: &str, amount: u32) -> FluidStack { FluidStack { fluid: id(text), amount } }

    fn registered() {
        crate::fluids::register();
        crate::fluid_items::register();
        register();
    }

    #[test]
    fn the_block_entity_is_registered_with_two_slots_and_the_item_is_a_tank_of_four_buckets() {
        registered();
        assert!(BlockEntityTypeRegistry::get("tconstruct:seared_casting_tank").is_some());
        let item = stack("tconstruct:seared_casting_tank", 1);
        assert_eq!(FluidItems::capacity(&item), 4000);
        // a scaled tank like the eight tank items: two of them hold two tanks' worth, a stack above 16 nothing
        let filled = FluidItems::fill(&stack("tconstruct:seared_casting_tank", 2), &fluid("minecraft:lava", 20_000));
        assert_eq!(filled.amount, 8000);
        assert_eq!(FluidItems::fill(&stack("tconstruct:seared_casting_tank", 17), &fluid("minecraft:lava", 68_000)).amount, 0);
    }

    #[test]
    fn the_hooks_fill_and_drain_the_saved_tank_and_the_comparator_reads_it() {
        registered();
        let mut data = NbtCompound::new();
        let mut items = Vec::new();
        let mut fill = BlockEntityFill { pos: [0; 3], data: &mut data, items: &mut items, fluid: fluid("minecraft:lava", 5000), face: None, execute: true };
        assert_eq!(fill_tank(&mut fill), 4000, "four buckets and no more");
        assert_eq!(read_tank(&data)[0].fluid, Some(fluid("minecraft:lava", 4000)));
        assert_eq!(data.get_compound("tank").unwrap().get_string("FluidName"), Some("minecraft:lava"), "saved as the original saves it");
        assert_eq!(comparator(&data), 15);
        let mut drain = BlockEntityDrain { pos: [0; 3], data: &mut data, items: &mut items, wanted: None, amount: 2000, execute: true };
        assert_eq!(drain_tank(&mut drain), Some(fluid("minecraft:lava", 2000)));
        assert_eq!(comparator(&data), 1 + 14 * 2000 / 4000);
        assert_eq!(fill_tank(&mut BlockEntityFill { pos: [0; 3], data: &mut data, items: &mut items, fluid: fluid("minecraft:water", 10), face: None, execute: true }), 0, "a different fluid is refused");
    }

    #[test]
    fn an_item_in_the_input_trades_with_the_tank_and_lands_in_the_output() {
        registered();
        let mut data = NbtCompound::new();
        // a filled bucket empties into the empty tank
        let mut items = vec![Some(stack("tconstruct:molten_iron_bucket", 1)), None];
        process(&mut data, &mut items);
        assert_eq!(tank_of(&data).amount(), 1000);
        assert_eq!(items[INPUT], None);
        assert_eq!(items[OUTPUT].as_ref().map(|stack| stack.item.to_string()), Some("minecraft:bucket".to_owned()));
        // an empty bucket fills back out of the tank
        let mut items = vec![Some(stack("minecraft:bucket", 1)), None];
        process(&mut data, &mut items);
        assert_eq!(tank_of(&data).amount(), 0);
        assert_eq!(items[OUTPUT].as_ref().map(|stack| stack.item.to_string()), Some("tconstruct:molten_iron_bucket".to_owned()));
        // a stick has no trade and stays; a full output blocks the next item
        let mut tank = tank_of(&data);
        tank.fill(&id("minecraft:lava"), 1000, true);
        let mut tank_data = NbtCompound::new();
        tank.write(&mut tank_data);
        let mut items = vec![Some(stack("minecraft:stick", 1)), None];
        process(&mut tank_data, &mut items);
        assert_eq!(items[INPUT], Some(stack("minecraft:stick", 1)));
        let mut items = vec![Some(stack("minecraft:bucket", 1)), Some(stack("minecraft:stick", 1))];
        process(&mut tank_data, &mut items);
        assert_eq!(items[INPUT], Some(stack("minecraft:bucket", 1)), "the bucket waits for the output to clear");
        assert_eq!(tank_of(&tank_data).amount(), 1000);
    }

    #[test]
    fn using_the_block_trades_fluid_first_then_swaps_the_items_around() {
        registered();
        let mut data = NbtCompound::new();
        let mut items = vec![None, None];
        let mut used = BlockEntityUse { pos: [0; 3], data: &mut data, items: &mut items, held: Some(stack("tconstruct:molten_iron_bucket", 1)), sneaking: false, give: Vec::new() };
        assert_eq!(use_casting_tank(&mut used), ActionResult::Success);
        assert_eq!(tank_of(&used.data).amount(), 1000, "the held bucket emptied into the tank");
        assert_eq!(used.held.as_ref().map(|held| held.item.to_string()), Some("minecraft:bucket".to_owned()));
        // a held container with no trade to make (this tank is empty) goes into the input, one at a time
        let mut data = NbtCompound::new();
        let mut items = vec![None, None];
        let mut used = BlockEntityUse { pos: [0; 3], data: &mut data, items: &mut items, held: Some(stack("tconstruct:seared_fuel_tank", 2)), sneaking: false, give: Vec::new() };
        assert_eq!(use_casting_tank(&mut used), ActionResult::Success);
        assert_eq!(used.items[INPUT], Some(stack("tconstruct:seared_fuel_tank", 1)));
        assert_eq!(used.held, Some(stack("tconstruct:seared_fuel_tank", 1)));
        // the input comes back before anything new is placed; a plain item is never placed
        used.held = None;
        assert_eq!(use_casting_tank(&mut used), ActionResult::Success);
        assert_eq!(used.give, vec![stack("tconstruct:seared_fuel_tank", 1)]);
        assert_eq!(used.items[INPUT], None);
        used.held = Some(stack("minecraft:stick", 1));
        used.give.clear();
        assert_eq!(use_casting_tank(&mut used), ActionResult::Success);
        assert_eq!(used.items[INPUT], None, "only fluid containers go in");
        assert!(used.give.is_empty());
    }

    #[test]
    fn a_redstone_rising_edge_swaps_the_slots_once_and_the_flag_is_saved() {
        registered();
        let mut data = NbtCompound::new();
        let mut items = vec![Some(stack("minecraft:bucket", 1)), Some(stack("tconstruct:molten_iron_bucket", 1))];
        handle_redstone(&mut data, &mut items, false, [0; 3]);
        assert_eq!(items[INPUT], Some(stack("minecraft:bucket", 1)), "no signal, no swap");
        handle_redstone(&mut data, &mut items, true, [0; 3]);
        assert_eq!((items[INPUT].clone(), items[OUTPUT].clone()), (Some(stack("tconstruct:molten_iron_bucket", 1)), Some(stack("minecraft:bucket", 1))), "the rising edge swapped them");
        handle_redstone(&mut data, &mut items, true, [0; 3]);
        assert_eq!(items[INPUT], Some(stack("tconstruct:molten_iron_bucket", 1)), "a held signal does not swap again");
        handle_redstone(&mut data, &mut items, false, [0; 3]);
        handle_redstone(&mut data, &mut items, true, [0; 3]);
        assert_eq!(items[INPUT], Some(stack("minecraft:bucket", 1)), "the next pulse swaps back");
        assert_eq!(data.get_int(TAG_REDSTONE, 0), 1);
    }
}
