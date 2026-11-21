//! The seared and scorched lanterns, from SlimeKnights SearedLanternBlock, LanternBlockEntity and TankItem (MIT; see LICENSE):
//! the vanilla lantern holding one fluid in a tank of 50 (`FluidValues.LANTERN_CAPACITY`), saved under `tank` as the tanks
//! save theirs, so the loot table's copy keeps the fluid in the dropped item. The lantern item is a tank item without the
//! tanks' stack limit (`TankItem`'s `limitStackSize` is false here): a filled stack still stacks, and a stack of any size
//! takes fluid, scaled by the stack as every tank item is. The block's `light` state follows the fluid, as a tank's does.
//! The fluid drawn in the model (the original's model fluid, `LanternBlockEntity.isFluidInModel`), the placed item's fluid
//! and the comparator output need the host, as they do for the tanks.
use crate::{fluid_values::LANTERN_CAPACITY, tank::{Tank, TAG_TANK}};
use pluto_lib::{
    ActionResult, BlockEntityDrain, BlockEntityFill, BlockEntityTick, BlockEntityType, BlockEntityTypeRegistry, FluidItemDef, FluidItems, FluidStack,
    Identifier, NeighbourTank, NbtCompound, TransferDirection, World,
};

pub(crate) const LANTERNS: [&str; 2] = ["seared_lantern", "scorched_lantern"];

fn id(name: &str) -> Identifier { Identifier::new("tconstruct", name).expect("valid lantern id") }

fn lantern_of(data: &NbtCompound) -> Tank { Tank::read(data, LANTERN_CAPACITY) }

/// `TankBlockEntity.updateLight`: the block's `light` state follows the fluid.
fn tick_lantern(tick: &mut BlockEntityTick<'_>) {
    let wanted = lantern_of(tick.data).light();
    let Some(world) = World::ticking() else { return };
    let Some(state) = world.get_block_state(tick.pos) else { return };
    let text = wanted.to_string();
    if state.property("light").is_some_and(|light| light != text) { world.set_block_state(tick.pos, &state.with("light", &text)); }
}

pub(crate) fn read_lanterns(data: &NbtCompound) -> Vec<NeighbourTank> {
    let tank = lantern_of(data);
    vec![NeighbourTank { fluid: tank.fluid.map(|fluid| FluidStack { fluid: fluid.fluid, amount: fluid.amount as u32, tag: None }), capacity: tank.capacity as u32 }]
}

pub(crate) fn fill_lantern(fill: &mut BlockEntityFill<'_>) -> u32 {
    let mut tank = lantern_of(fill.data);
    let filled = tank.fill(&fill.fluid.fluid, fill.fluid.amount.min(i32::MAX as u32) as i32, true);
    if filled > 0 { tank.write(fill.data); }
    filled as u32
}

pub(crate) fn drain_lantern(drain: &mut BlockEntityDrain<'_>) -> Option<FluidStack> {
    let mut tank = lantern_of(drain.data);
    let out = tank.drain(drain.wanted.as_ref(), drain.amount.min(i32::MAX as u32) as i32, true)?;
    tank.write(drain.data);
    Some(FluidStack { fluid: out.fluid, amount: out.amount as u32, tag: None })
}

/// `FluidTransferHelper.interactWithTank`, the trade a tank block entity answers: the held container empties into the
/// lantern or fills from it; `Pass` when nothing moved.
fn use_lantern(used: &mut pluto_lib::BlockEntityUse<'_>) -> ActionResult {
    struct Block(Tank);
    impl pluto_lib::FluidTarget for Block {
        fn fill(&mut self, fluid: &FluidStack, execute: bool) -> u32 { self.0.fill(&fluid.fluid, fluid.amount.min(i32::MAX as u32) as i32, execute).max(0) as u32 }
        fn drain(&mut self, wanted: Option<&Identifier>, amount: u32, execute: bool) -> Option<FluidStack> {
            self.0.drain(wanted, amount.min(i32::MAX as u32) as i32, execute).map(|out| FluidStack { fluid: out.fluid, amount: out.amount as u32, tag: None })
        }
    }
    let mut target = Block(lantern_of(used.data));
    let Some(done) = FluidItems::interact(used.held.as_ref(), &mut target, TransferDirection::Auto) else { return ActionResult::Pass };
    target.0.write(used.data);
    used.held = done.held;
    used.give.extend(done.give);
    ActionResult::Success
}

pub(crate) fn register() {
    for name in LANTERNS {
        BlockEntityTypeRegistry::register(
            BlockEntityType::new(id(name)).sync_data()
                .ticker(tick_lantern)
                .fluid_reader(read_lanterns)
                .fluid_filler(fill_lantern)
                .fluid_drainer(drain_lantern)
                .on_use(use_lantern),
        );
        FluidItems::register(FluidItemDef::tank(id(name), TAG_TANK, LANTERN_CAPACITY as u32).scaled());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::ItemSlot;

    fn fluid_id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }

    #[test]
    fn a_lantern_holds_fifty_of_one_fluid() {
        let mut tank = Tank::empty(LANTERN_CAPACITY);
        assert_eq!(tank.fill(&fluid_id("minecraft:lava"), 1000, true), 50, "the lantern capacity cuts a bucket to 50");
        assert_eq!(tank.fill(&fluid_id("minecraft:lava"), 1000, true), 0, "full");
        assert_eq!(tank.drain(None, 20, true).unwrap().amount, 20);
        assert_eq!(tank.fill(&fluid_id("minecraft:water"), 10, true), 0, "a second fluid is refused while one is held");
        assert_eq!(tank.comparator(), 1 + 14 * 30 / 50, "the comparator reads the fill, as a tank's does");
        assert_eq!(Tank::empty(LANTERN_CAPACITY).comparator(), 0);
    }

    #[test]
    fn the_hooks_answer_a_neighbours_reads_fills_and_drains() {
        let mut data = NbtCompound::new();
        let mut items = Vec::new();
        let mut fill = BlockEntityFill { pos: [0; 3], data: &mut data, items: &mut items, fluid: FluidStack { fluid: fluid_id("minecraft:lava"), amount: 5000, tag: None }, face: None, execute: true };
        assert_eq!(fill_lantern(&mut fill), 50);
        assert_eq!(read_lanterns(&data)[0].fluid.as_ref().unwrap().amount, 50);
        assert_eq!(read_lanterns(&data)[0].capacity, 50);
        let mut drain = BlockEntityDrain { pos: [0; 3], data: &mut data, items: &mut items, wanted: None, amount: 90, execute: true };
        assert_eq!(drain_lantern(&mut drain).unwrap().amount, 50, "a drain of more than the lantern holds takes what there is");
        assert_eq!(Tank::read(&data, LANTERN_CAPACITY).amount(), 0);
        register();
        assert!(BlockEntityTypeRegistry::get("tconstruct:seared_lantern").is_some());
        assert!(BlockEntityTypeRegistry::get("tconstruct:scorched_lantern").is_some());
    }

    #[test]
    fn a_lantern_item_is_a_scaled_tank_without_the_tanks_stack_limit() {
        crate::fluids::register();
        register();
        let one = ItemSlot::new(fluid_id("tconstruct:seared_lantern"), 1);
        let filled = FluidItems::fill(&one, &FluidStack { fluid: fluid_id("minecraft:lava"), amount: 1000, tag: None });
        assert_eq!(filled.amount, 50, "one lantern takes its 50");
        // the tank items take no fluid above a stack of 16; the lantern item has no such limit and scales by the stack
        let many = ItemSlot::new(fluid_id("tconstruct:scorched_lantern"), 20);
        let filled = FluidItems::fill(&many, &FluidStack { fluid: fluid_id("minecraft:lava"), amount: 1000, tag: None });
        assert_eq!(filled.amount, 1000, "twenty lanterns of fifty");
        let mut single = filled.container.clone();
        single.count = 1;
        assert_eq!(Tank::read(&single.custom_data, LANTERN_CAPACITY).amount(), 50, "kept per lantern under `tank`, as TankItem reads it");
    }
}
