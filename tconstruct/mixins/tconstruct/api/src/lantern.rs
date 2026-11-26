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
                .fluid_reader(|data, _items| read_lanterns(data))
                .fluid_filler(fill_lantern)
                .fluid_drainer(drain_lantern)
                .on_use(use_lantern),
        );
        FluidItems::register(FluidItemDef::tank(id(name), TAG_TANK, LANTERN_CAPACITY as u32).scaled());
    }
    for (block, speed_factor, jump_factor) in MOVEMENT_FACTORS {
        pluto_lib::movement::register(block, speed_factor, jump_factor).expect("movement factors register once");
    }
}

/// The blocks of this unit whose originals set a non-default movement factor, with the factor the host's
/// movement table must apply. The lanterns themselves set none: `SearedLanternBlock` is built on
/// `searedNonSolidProps(SoundType.LANTERN)` with only a light level, so both lanterns keep the default
/// speed and jump factors of 1.0 and need no registration. The soul glass does carry one: the smeltery
/// pair is built on the same non-solid properties with `.noCollission().speedFactor(0.1f)`, the commons
/// pair on `glassBuilder(...).speedFactor(0.2F)`, and the commons pane keeps its collision. The cheese
/// block is the same reach from the commons unit: `HalfTransparentBlock(...).speedFactor(0.4F).jumpFactor(0.5F)`.
/// The host's block declarations carry no speed-factor key, so these ride in PlutoLib's movement registry
/// and the `pluto_movement` mixin applies them to the movement table at load.
const MOVEMENT_FACTORS: [(&str, f32, f32); 7] = [
    ("tconstruct:seared_soul_glass", 0.1, 1.0),
    ("tconstruct:scorched_soul_glass", 0.1, 1.0),
    ("tconstruct:seared_soul_glass_pane", 0.1, 1.0),
    ("tconstruct:scorched_soul_glass_pane", 0.1, 1.0),
    ("tconstruct:soul_glass", 0.2, 1.0),
    ("tconstruct:soul_glass_pane", 0.2, 1.0),
    ("tconstruct:cheese_block", 0.4, 0.5),
];

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
    fn a_bucket_never_trades_with_a_lantern_but_a_partial_container_does() {
        crate::fluids::register();
        crate::fluid_items::register();
        register();
        // A bucket holds 1000 and trades all-or-nothing (it has no part-filled form),
        // and the lantern holds 50, so no bucket trade is possible in either
        // direction — the original gives the lantern block no bucket trade at all
        // (SearedLanternBlock has no use; only SearedTankBlock answers
        // FluidTransferHelper.interactWithTank), and the click passes to the
        // bucket's own use. The step-2 session's side-face click moving nothing is
        // this rule, not an unwired route.
        let mut data = NbtCompound::new();
        let mut items = Vec::new();
        let (result, held) = {
            let mut used = pluto_lib::BlockEntityUse { pos: [0; 3], data: &mut data, items: &mut items, held: Some(ItemSlot::new(fluid_id("tconstruct:molten_iron_bucket"), 1)), sneaking: false, give: Vec::new() };
            let result = use_lantern(&mut used);
            (result, used.held.as_ref().map(|held| held.item.to_string()))
        };
        assert_eq!(result, ActionResult::Pass, "a full bucket cannot partly fill a 50 lantern");
        assert_eq!(held.as_deref(), Some("tconstruct:molten_iron_bucket"), "the bucket is untouched");
        assert_eq!(Tank::read(&data, LANTERN_CAPACITY).amount(), 0);
        // Nor can an empty bucket draw from a filled lantern: it needs a full 1000.
        Tank { fluid: Some(crate::tank::Fluid { fluid: fluid_id("tconstruct:molten_iron"), amount: 50 }), capacity: LANTERN_CAPACITY }.write(&mut data);
        let (result, held) = {
            let mut used = pluto_lib::BlockEntityUse { pos: [0; 3], data: &mut data, items: &mut items, held: Some(ItemSlot::new(fluid_id("minecraft:bucket"), 1)), sneaking: false, give: Vec::new() };
            let result = use_lantern(&mut used);
            (result, used.held.as_ref().map(|held| held.item.to_string()))
        };
        assert_eq!(result, ActionResult::Pass, "50 in the lantern cannot fill a bucket");
        assert_eq!(held.as_deref(), Some("minecraft:bucket"));
        assert_eq!(Tank::read(&data, LANTERN_CAPACITY).amount(), 50, "the lantern keeps its fluid");
        // A container that trades in part (a tank item) does trade: of its 100,
        // the lantern takes its 50 and the item keeps the rest.
        let mut data = NbtCompound::new();
        let mut carrier = ItemSlot::new(fluid_id("tconstruct:seared_ingot_tank"), 1);
        crate::tank::set_item_tank(&mut carrier, &Tank { fluid: Some(crate::tank::Fluid { fluid: fluid_id("tconstruct:molten_iron"), amount: 100 }), capacity: 4320 });
        let (result, kept) = {
            let mut used = pluto_lib::BlockEntityUse { pos: [0; 3], data: &mut data, items: &mut items, held: Some(carrier), sneaking: false, give: Vec::new() };
            let result = use_lantern(&mut used);
            (result, used.held.as_ref().and_then(|held| crate::tank::item_tank(held)).map(|tank| tank.amount()))
        };
        assert_eq!(result, ActionResult::Success, "the lantern's use route answers a partial container");
        assert_eq!(Tank::read(&data, LANTERN_CAPACITY).amount(), 50);
        assert_eq!(kept, Some(50), "the item keeps what the lantern could not take");
    }

    #[test]
    fn the_soul_glass_and_cheese_carry_their_canon_movement_factors() {
        use pluto_lib::movement::{MovementFactors, get};
        register();
        for (block, speed_factor, jump_factor) in MOVEMENT_FACTORS {
            assert_eq!(get(block), Some(MovementFactors { speed_factor, jump_factor }), "{block} carries its canon factors");
        }
        assert_eq!(get("tconstruct:seared_lantern"), None, "the lanterns set no factor and keep the default 1.0");
        assert_eq!(get("tconstruct:scorched_lantern"), None);
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
