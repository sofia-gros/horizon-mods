//! The seared and scorched tanks, from SlimeKnights TankBlockEntity, ITankBlockEntity, SearedTankBlock and TankItem (MIT; see LICENSE):
//! one fluid in one tank whose capacity the block fixes (4 buckets for the fuel tank and gauge, 48 ingots for the ingot ones), saved
//! under `tank` as Forge's `FluidName` and `Amount`, with the fluid also kept in the dropped item's custom data under the same name so
//! that a broken tank keeps its fluid (the loot table copies it). The block's `light` state follows the fluid's light level and the
//! comparator reads the fill. Putting fluid in and out with a held bucket, the placed item's fluid, the comparator update and the fluid
//! drawn in the model need the host (items with fluid, the placement stack, block updates, a renderer).
use crate::fluid_values::{BUCKET, INGOT};
use pluto_lib::{
    BlockEntityDrain, BlockEntityFill, BlockEntityTick, BlockEntityType, BlockEntityTypeRegistry, FluidRegistry, FluidStack, Identifier, ItemSlot, Nbt, NbtCompound,
    NbtCompoundExt, NeighbourTank, World,
};

pub(crate) const KINDS: [&str; 2] = ["seared", "scorched"];
/// `SearedTankBlock.TankType` and its capacity.
pub(crate) const TYPES: [(&str, i32); 4] = [("fuel_tank", BUCKET * 4), ("fuel_gauge", BUCKET * 4), ("ingot_tank", INGOT * 48), ("ingot_gauge", INGOT * 48)];
pub(crate) const TAG_TANK: &str = "tank";

/// Every tank block and its capacity.
pub(crate) fn blocks() -> Vec<(String, i32)> {
    KINDS.iter().flat_map(|kind| TYPES.iter().map(move |(name, capacity)| (format!("{kind}_{name}"), *capacity))).collect()
}

/// The capacity of the tank block or tank item called `name` (`TankBlockEntity.getCapacity(Item)`: a block item asks its block).
pub(crate) fn capacity_of(name: &str) -> Option<i32> {
    blocks().into_iter().find(|(block, _)| block == name).map(|(_, capacity)| capacity)
}

/// What a tank holds: a fluid and an amount in millibuckets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Fluid { pub fluid: Identifier, pub amount: i32 }

/// A `FluidTank` (Forge): a single fluid up to the capacity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Tank { pub fluid: Option<Fluid>, pub capacity: i32 }

fn read_fluid(tag: &NbtCompound) -> Option<Fluid> {
    let name = tag.get_string("FluidName")?;
    let amount = tag.get_int("Amount", 0) as i32;
    if amount <= 0 || name == "minecraft:empty" { return None; }
    Some(Fluid { fluid: Identifier::parse(name).ok()?, amount })
}

fn write_fluid(fluid: &Fluid) -> NbtCompound {
    let mut tag = NbtCompound::new();
    tag.insert("FluidName".to_owned(), Nbt::String(fluid.fluid.to_string()));
    tag.insert("Amount".to_owned(), Nbt::Int(fluid.amount));
    tag
}

impl Tank {
    pub(crate) fn empty(capacity: i32) -> Self { Tank { fluid: None, capacity } }

    /// The tank saved in a block entity's data (or an item's custom data) under `tank`.
    pub(crate) fn read(data: &NbtCompound, capacity: i32) -> Self {
        Tank { fluid: data.get_compound(TAG_TANK).and_then(read_fluid), capacity }
    }

    /// Saves the fluid under `tank`; an empty tank removes the key (`saveSynced` only writes a filled tank).
    pub(crate) fn write(&self, data: &mut NbtCompound) {
        match &self.fluid {
            Some(fluid) => { data.insert(TAG_TANK.to_owned(), Nbt::Compound(write_fluid(fluid))); }
            None => { data.remove(TAG_TANK); }
        }
    }

    pub(crate) fn amount(&self) -> i32 { self.fluid.as_ref().map_or(0, |fluid| fluid.amount) }

    /// `FluidTank.fill`: a different fluid is refused while there is one; with `execute` false it only asks.
    pub(crate) fn fill(&mut self, fluid: &Identifier, amount: i32, execute: bool) -> i32 {
        if amount <= 0 { return 0; }
        match &mut self.fluid {
            None => {
                let filled = self.capacity.min(amount);
                if execute && filled > 0 { self.fluid = Some(Fluid { fluid: fluid.clone(), amount: filled }); }
                filled.max(0)
            }
            Some(held) if &held.fluid != fluid => 0,
            Some(held) => {
                let filled = (self.capacity - held.amount).min(amount).max(0);
                if execute { held.amount += filled; }
                filled
            }
        }
    }

    /// `FluidTank.drain(FluidStack)` and `drain(int)`: `wanted` must match the held fluid when it is named.
    pub(crate) fn drain(&mut self, wanted: Option<&Identifier>, amount: i32, execute: bool) -> Option<Fluid> {
        let held = self.fluid.as_mut()?;
        if amount <= 0 || wanted.is_some_and(|wanted| wanted != &held.fluid) { return None; }
        let drained = held.amount.min(amount);
        let out = Fluid { fluid: held.fluid.clone(), amount: drained };
        if execute {
            held.amount -= drained;
            if held.amount <= 0 { self.fluid = None; }
        }
        Some(out)
    }

    /// `ITankBlockEntity.comparatorStrength`.
    pub(crate) fn comparator(&self) -> i32 {
        if self.fluid.is_none() { 0 } else { 1 + 14 * self.amount() / self.capacity.max(1) }
    }

    /// `FluidType.getLightLevel` of the held fluid, the value of the block's `light` state.
    pub(crate) fn light(&self) -> u8 {
        self.fluid.as_ref().and_then(|fluid| FluidRegistry::get(&fluid.fluid.to_string())).map_or(0, |def| def.attributes.luminance.min(15))
    }
}

// --- the item

/// `TankItem.getTank`: the fluid a tank item holds, kept in the stack's custom data under `tank` and cut to the capacity of its block.
pub(crate) fn item_tank(stack: &ItemSlot) -> Option<Tank> {
    let capacity = capacity_of(stack.item.path())?;
    let mut tank = Tank::read(&stack.custom_data, capacity);
    if let Some(fluid) = &mut tank.fluid { fluid.amount = fluid.amount.min(capacity); }
    Some(tank)
}

/// `TankItem.setTank`: writes the tank into the stack (nothing for an empty tank, which also removes an old one).
pub(crate) fn set_item_tank(stack: &mut ItemSlot, tank: &Tank) { tank.write(&mut stack.custom_data); }

/// `SearedTankBlock.setPlacedBy`: the block entity of a placed tank starts with the fluid of its item.
pub(crate) fn data_from_item(stack: &ItemSlot) -> NbtCompound {
    let mut data = NbtCompound::new();
    if let Some(tank) = item_tank(stack) { tank.write(&mut data); }
    data
}

// --- the block entity

fn tank_of(name: &str, data: &NbtCompound) -> Tank { Tank::read(data, capacity_of(name).unwrap_or(BUCKET * 4)) }

/// `TankBlockEntity.updateLight`: the block's `light` state follows the fluid.
fn tick_tank(name: &str, tick: &mut BlockEntityTick<'_>) {
    let wanted = tank_of(name, tick.data).light();
    let Some(world) = World::ticking() else { return };
    let Some(state) = world.get_block_state(tick.pos) else { return };
    let text = wanted.to_string();
    if state.property("light").is_some_and(|light| light != text) { world.set_block_state(tick.pos, &state.with("light", &text)); }
}

pub(crate) fn read_tanks(name: &str, data: &NbtCompound) -> Vec<NeighbourTank> {
    let tank = tank_of(name, data);
    vec![NeighbourTank { fluid: tank.fluid.map(|fluid| FluidStack { fluid: fluid.fluid, amount: fluid.amount as u32 }), capacity: tank.capacity as u32 }]
}

pub(crate) fn fill_tank(name: &str, fill: &mut BlockEntityFill<'_>) -> u32 {
    let mut tank = tank_of(name, fill.data);
    let filled = tank.fill(&fill.fluid.fluid, fill.fluid.amount.min(i32::MAX as u32) as i32, true);
    if filled > 0 { tank.write(fill.data); }
    filled as u32
}

pub(crate) fn drain_tank(name: &str, drain: &mut BlockEntityDrain<'_>) -> Option<FluidStack> {
    let mut tank = tank_of(name, drain.data);
    let out = tank.drain(drain.wanted.as_ref(), drain.amount.min(i32::MAX as u32) as i32, true)?;
    tank.write(drain.data);
    Some(FluidStack { fluid: out.fluid, amount: out.amount as u32 })
}

/// `FluidTransferHelper.interactWithTank` (SearedTankBlock.use): the held container empties into the tank or fills from it; `Pass` when
/// nothing moved. A stack of several containers trades one at a time, a stack of tanks as a whole (TankItem's scaled handler).
fn use_tank(name: &str, used: &mut pluto_lib::BlockEntityUse<'_>) -> pluto_lib::ActionResult {
    struct Block(Tank);
    impl pluto_lib::FluidTarget for Block {
        fn fill(&mut self, fluid: &FluidStack, execute: bool) -> u32 { self.0.fill(&fluid.fluid, fluid.amount.min(i32::MAX as u32) as i32, execute).max(0) as u32 }
        fn drain(&mut self, wanted: Option<&Identifier>, amount: u32, execute: bool) -> Option<FluidStack> {
            self.0.drain(wanted, amount.min(i32::MAX as u32) as i32, execute).map(|out| FluidStack { fluid: out.fluid, amount: out.amount as u32 })
        }
    }
    let mut target = Block(tank_of(name, used.data));
    let Some(done) = pluto_lib::FluidItems::interact(used.held.as_ref(), &mut target, pluto_lib::TransferDirection::Auto) else { return pluto_lib::ActionResult::Pass };
    target.0.write(used.data);
    used.held = done.held;
    used.give.extend(done.give);
    pluto_lib::ActionResult::Success
}

pub(crate) fn register() {
    for (name, _) in blocks() {
        let block = Identifier::new("tconstruct", &name).expect("valid tank id");
        let (for_tick, for_read, for_fill, for_drain, for_use) = (name.clone(), name.clone(), name.clone(), name.clone(), name);
        BlockEntityTypeRegistry::register(
            BlockEntityType::new(block).sync_data()
                .ticker(move |tick| tick_tank(&for_tick, tick))
                .fluid_reader(move |data| read_tanks(&for_read, data))
                .fluid_filler(move |fill| fill_tank(&for_fill, fill))
                .fluid_drainer(move |drain| drain_tank(&for_drain, drain))
                .on_use(move |used| use_tank(&for_use, used)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }

    #[test]
    fn a_tank_holds_one_fluid_up_to_its_block_capacity_and_the_capacities_are_the_originals() {
        assert_eq!((capacity_of("seared_fuel_tank"), capacity_of("scorched_ingot_gauge"), capacity_of("seared_stone")), (Some(4000), Some(4320), None));
        assert_eq!(blocks().len(), 8);
        let mut tank = Tank::empty(4000);
        assert_eq!(tank.fill(&id("minecraft:lava"), 3000, false), 3000);
        assert_eq!(tank.fluid, None, "asking changes nothing");
        assert_eq!((tank.fill(&id("minecraft:lava"), 3000, true), tank.fill(&id("minecraft:lava"), 3000, true)), (3000, 1000));
        assert_eq!(tank.fill(&id("minecraft:water"), 10, true), 0, "a different fluid is refused while there is one");
        assert_eq!(tank.drain(Some(&id("minecraft:water")), 10, true), None);
        assert_eq!(tank.drain(None, 1500, true), Some(Fluid { fluid: id("minecraft:lava"), amount: 1500 }));
        assert_eq!(tank.amount(), 2500);
        tank.drain(None, 9999, true);
        assert_eq!(tank.fluid, None, "an emptied tank holds no fluid at all");
        assert_eq!(tank.fill(&id("minecraft:water"), 10, true), 10, "and takes any fluid again");
    }

    #[test]
    fn the_fluid_survives_the_block_entity_and_the_item_and_the_comparator_reads_the_fill() {
        let mut tank = Tank::empty(4320);
        tank.fill(&id("tconstruct:molten_iron"), 2160, true);
        let mut data = NbtCompound::new();
        tank.write(&mut data);
        assert_eq!(Tank::read(&data, 4320), tank, "saved as FluidName and Amount");
        assert_eq!(data.get_compound("tank").unwrap().get_string("FluidName"), Some("tconstruct:molten_iron"));
        let mut stack = ItemSlot::new(id("tconstruct:seared_ingot_tank"), 1);
        set_item_tank(&mut stack, &tank);
        assert_eq!(item_tank(&stack), Some(tank.clone()));
        assert_eq!(Tank::read(&data_from_item(&stack), 4320), tank, "a placed tank starts with its item's fluid");
        assert_eq!(tank.comparator(), 1 + 14 * 2160 / 4320);
        assert_eq!(Tank::empty(4000).comparator(), 0);
        let plain = ItemSlot::new(id("minecraft:stick"), 1);
        assert_eq!(item_tank(&plain), None);
        // an item with more than the block holds is cut to the capacity
        stack.custom_data.insert("tank".to_owned(), Nbt::Compound(write_fluid(&Fluid { fluid: id("minecraft:lava"), amount: 99_999 })));
        assert_eq!(item_tank(&stack).unwrap().amount(), 4320);
        set_item_tank(&mut stack, &Tank::empty(4320));
        assert!(stack.custom_data.is_empty(), "an empty tank leaves no tag on the item");
    }

    #[test]
    fn the_hooks_answer_a_neighbours_reads_fills_and_drains() {
        let mut data = NbtCompound::new();
        let mut items = Vec::new();
        let mut fill = BlockEntityFill { pos: [0; 3], data: &mut data, items: &mut items, fluid: FluidStack { fluid: id("minecraft:lava"), amount: 5000 }, face: None, execute: true };
        assert_eq!(fill_tank("seared_fuel_tank", &mut fill), 4000);
        assert_eq!(read_tanks("seared_fuel_tank", &data)[0].fluid.as_ref().unwrap().amount, 4000);
        let mut drain = BlockEntityDrain { pos: [0; 3], data: &mut data, items: &mut items, wanted: None, amount: 90, execute: true };
        assert_eq!(drain_tank("seared_fuel_tank", &mut drain).unwrap().amount, 90);
        assert_eq!(Tank::read(&data, 4000).amount(), 3910);
        register();
        assert!(BlockEntityTypeRegistry::get("tconstruct:scorched_ingot_tank").is_some());
    }
}
