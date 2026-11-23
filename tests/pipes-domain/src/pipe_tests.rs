//! The pipes mod's transport rules, exercised end to end over a small simulated world.
//! `SimOps` stands in for the host's neighbour adapter: plain containers and tanks behave the
//! way the memory world of PlutoLib's own tests behaves, and a pipe position is answered from
//! the pipe's saved data and inbox slots — inserts into a pipe land in the slot of the face
//! they came through and nothing can be taken out of a pipe, exactly what the registered
//! block entity types declare (and what the registry test below pins down).
use crate::common::face_index;
use crate::connections::{self, Family};
use crate::fluid_pipe::{self, FlowState};
use crate::item_pipe::{self, PipeState, Transit};
use pluto_lib::{
    with_neighbours, BlockEntitySignal, BlockEntityTick, BlockEntityTypeRegistry, BlockPos, BlockState, Direction,
    FluidStack, Identifier, ItemSlot, MemoryBlockView, NeighbourOps, NeighbourSlot, NeighbourTank, NbtCompound,
    TransferMode,
};
use std::collections::{BTreeMap, BTreeSet};

fn id(text: &str) -> Identifier {
    Identifier::parse(text).unwrap()
}
fn stack(text: &str, count: u32) -> ItemSlot {
    ItemSlot::new(id(text), count)
}
fn fluid(text: &str, amount: u32) -> FluidStack {
    FluidStack { fluid: id(text), amount, tag: None }
}

#[derive(Default)]
struct SimOps {
    states: BTreeMap<BlockPos, String>,
    chests: BTreeMap<BlockPos, Vec<Option<ItemSlot>>>,
    tanks: BTreeMap<BlockPos, Vec<(Option<FluidStack>, u32)>>,
    pipe_items: BTreeMap<BlockPos, Vec<Option<ItemSlot>>>,
    pipe_data: BTreeMap<BlockPos, NbtCompound>,
    /// Positions whose slots report `can_take` false (a casting table mid-cast).
    take_blocked: BTreeSet<BlockPos>,
}

impl SimOps {
    fn add_chest(&mut self, pos: BlockPos, state: &str, slots: Vec<Option<ItemSlot>>) {
        self.states.insert(pos, state.to_owned());
        self.chests.insert(pos, slots);
    }
    fn add_tank(&mut self, pos: BlockPos, state: &str, held: Option<FluidStack>, capacity: u32) {
        self.states.insert(pos, state.to_owned());
        self.tanks.insert(pos, vec![(held, capacity)]);
    }
    fn add_item_pipe(&mut self, pos: BlockPos, state: &str) {
        self.states.insert(pos, state.to_owned());
        self.pipe_data.insert(pos, NbtCompound::new());
        self.pipe_items.insert(pos, vec![None; 6]);
    }
    fn add_fluid_pipe(&mut self, pos: BlockPos, state: &str) {
        self.states.insert(pos, state.to_owned());
        self.pipe_data.insert(pos, NbtCompound::new());
    }
    fn tick_item(&mut self, pos: BlockPos, kind: item_pipe::Kind) {
        let mut data = self.pipe_data.remove(&pos).expect("item pipe data");
        let mut items = self.pipe_items.remove(&pos).expect("item pipe items");
        with_neighbours(self, || {
            let mut tick = BlockEntityTick { pos, data: &mut data, items: &mut items };
            item_pipe::tick_pipe(&mut tick, kind);
        });
        self.pipe_data.insert(pos, data);
        self.pipe_items.insert(pos, items);
    }
    fn tick_fluid(&mut self, pos: BlockPos, kind: fluid_pipe::Kind) {
        let mut data = self.pipe_data.remove(&pos).expect("fluid pipe data");
        let mut items: Vec<Option<ItemSlot>> = Vec::new();
        with_neighbours(self, || {
            let mut tick = BlockEntityTick { pos, data: &mut data, items: &mut items };
            fluid_pipe::tick_pipe(&mut tick, kind);
        });
        self.pipe_data.insert(pos, data);
    }
    fn power_item(&mut self, pos: BlockPos, powered: bool) {
        let mut data = self.pipe_data.remove(&pos).expect("item pipe data");
        let mut items = self.pipe_items.remove(&pos).expect("item pipe items");
        let mut signal = BlockEntitySignal { pos, data: &mut data, items: &mut items, strength: if powered { 15 } else { 0 }, powered };
        item_pipe::signal_pipe(&mut signal);
        self.pipe_data.insert(pos, data);
        self.pipe_items.insert(pos, items);
    }
    fn power_fluid(&mut self, pos: BlockPos, powered: bool) {
        let mut data = self.pipe_data.remove(&pos).expect("fluid pipe data");
        let mut items: Vec<Option<ItemSlot>> = Vec::new();
        let mut signal = BlockEntitySignal { pos, data: &mut data, items: &mut items, strength: if powered { 15 } else { 0 }, powered };
        fluid_pipe::signal_pipe(&mut signal);
        self.pipe_data.insert(pos, data);
    }
    fn pipe_fluid(&mut self, pos: &BlockPos) -> Option<Option<FluidStack>> {
        let data = self.pipe_data.get_mut(pos)?;
        let mut items: Vec<Option<ItemSlot>> = Vec::new();
        let tick = BlockEntityTick { pos: *pos, data, items: &mut items };
        Some(tick.fluid(0))
    }
    fn set_pipe_fluid(&mut self, pos: &BlockPos, held: Option<FluidStack>) {
        if let Some(data) = self.pipe_data.get_mut(pos) {
            let mut items: Vec<Option<ItemSlot>> = Vec::new();
            let mut tick = BlockEntityTick { pos: *pos, data, items: &mut items };
            tick.set_fluid(0, held);
        }
    }
    fn fluid_amount(&mut self, pos: BlockPos) -> u32 {
        if self.pipe_data.contains_key(&pos) {
            return self.pipe_fluid(&pos).flatten().map_or(0, |stack| stack.amount);
        }
        self.tanks.get(&pos).and_then(|tanks| tanks.first()).and_then(|(held, _)| held.as_ref()).map_or(0, |stack| stack.amount)
    }
    fn item_total(&self, pos: BlockPos, item: &str) -> u32 {
        self.chests.get(&pos).into_iter().flatten().flatten().filter(|stack| stack.item == id(item)).map(|stack| stack.count).sum()
    }
}

impl NeighbourOps for SimOps {
    fn item_slots(&mut self, pos: BlockPos, _: Direction) -> Option<Vec<NeighbourSlot>> {
        if let Some(items) = self.pipe_items.get(&pos) {
            return Some(items.iter().enumerate().map(|(slot, stack)| NeighbourSlot { slot, stack: stack.clone(), can_take: false }).collect());
        }
        let blocked = self.take_blocked.contains(&pos);
        self.chests.get(&pos).map(|slots| {
            slots.iter().enumerate().map(|(slot, stack)| NeighbourSlot { slot, stack: stack.clone(), can_take: stack.is_some() && !blocked }).collect()
        })
    }

    fn extract_item(&mut self, pos: BlockPos, _: Direction, slot: usize, count: u32) -> Option<ItemSlot> {
        if self.pipe_items.contains_key(&pos) {
            return None;
        }
        let held = self.chests.get_mut(&pos)?.get_mut(slot)?;
        let stack = held.as_mut()?;
        let taken = count.min(stack.count);
        if taken == 0 {
            return None;
        }
        let mut out = stack.clone();
        out.count = taken;
        stack.count -= taken;
        if stack.count == 0 {
            *held = None;
        }
        Some(out)
    }

    fn insert_item(&mut self, pos: BlockPos, face: Direction, mut stack: ItemSlot) -> Option<ItemSlot> {
        if let Some(items) = self.pipe_items.get_mut(&pos) {
            let held = &mut items[face_index(face)];
            match held {
                Some(existing) if existing.item == stack.item && existing.custom_data == stack.custom_data && existing.damage == stack.damage => {
                    let room = 64u32.saturating_sub(existing.count).min(stack.count);
                    existing.count += room;
                    stack.count -= room;
                }
                None => {
                    *held = Some(stack.clone());
                    stack.count = 0;
                }
                _ => {}
            }
            return (stack.count > 0).then_some(stack);
        }
        let Some(slots) = self.chests.get_mut(&pos) else { return Some(stack) };
        for held in slots.iter_mut() {
            match held {
                Some(existing) if existing.item == stack.item && existing.custom_data == stack.custom_data && existing.damage == stack.damage => {
                    let room = 64u32.saturating_sub(existing.count).min(stack.count);
                    existing.count += room;
                    stack.count -= room;
                }
                None if stack.count > 0 => {
                    *held = Some(stack.clone());
                    stack.count = 0;
                }
                _ => {}
            }
            if stack.count == 0 {
                return None;
            }
        }
        Some(stack)
    }

    fn fluid_tanks(&mut self, pos: BlockPos, _: Direction) -> Option<Vec<NeighbourTank>> {
        if self.pipe_data.contains_key(&pos) {
            let held = self.pipe_fluid(&pos)?;
            return Some(vec![NeighbourTank { fluid: held, capacity: fluid_pipe::CAPACITY }]);
        }
        self.tanks.get(&pos).map(|tanks| tanks.iter().map(|(held, capacity)| NeighbourTank { fluid: held.clone(), capacity: *capacity }).collect())
    }

    fn drain_fluid(&mut self, pos: BlockPos, _: Direction, amount: u32) -> Option<FluidStack> {
        if self.pipe_data.contains_key(&pos) {
            let held = self.pipe_fluid(&pos)??;
            let taken = amount.min(held.amount);
            if taken == 0 {
                return None;
            }
            let left = held.amount - taken;
            self.set_pipe_fluid(&pos, (left > 0).then(|| fluid(&held.fluid.to_string(), left)));
            return Some(fluid(&held.fluid.to_string(), taken));
        }
        let tanks = self.tanks.get_mut(&pos)?;
        let (held, _) = tanks.iter_mut().find(|(held, _)| held.is_some())?;
        let stack = held.as_mut()?;
        let taken = amount.min(stack.amount);
        if taken == 0 {
            return None;
        }
        let out = fluid(&stack.fluid.to_string(), taken);
        stack.amount -= taken;
        if stack.amount == 0 {
            *held = None;
        }
        Some(out)
    }

    fn fill_fluid(&mut self, pos: BlockPos, _: Direction, stack: &FluidStack, execute: bool) -> u32 {
        if self.pipe_data.contains_key(&pos) {
            let held = self.pipe_fluid(&pos).flatten();
            let room = match &held {
                Some(own) if !own.same_fluid(stack) => 0,
                Some(own) => fluid_pipe::CAPACITY.saturating_sub(own.amount),
                None => fluid_pipe::CAPACITY,
            };
            let taken = room.min(stack.amount);
            if taken > 0 && execute {
                let merged = match held {
                    Some(mut own) => {
                        own.amount += taken;
                        own
                    }
                    None => fluid(&stack.fluid.to_string(), taken),
                };
                self.set_pipe_fluid(&pos, Some(merged));
            }
            return taken;
        }
        let Some(tanks) = self.tanks.get_mut(&pos) else { return 0 };
        for (held, capacity) in tanks.iter_mut() {
            let room = match held {
                Some(own) if !own.same_fluid(stack) => continue,
                Some(own) => capacity.saturating_sub(own.amount),
                None => *capacity,
            };
            let taken = room.min(stack.amount);
            if taken == 0 {
                continue;
            }
            if execute {
                match held {
                    Some(own) => own.amount += taken,
                    None => *held = Some(fluid(&stack.fluid.to_string(), taken)),
                }
            }
            return taken;
        }
        0
    }

    fn block_state(&mut self, pos: BlockPos) -> Option<String> {
        self.states.get(&pos).cloned()
    }
}

#[test]
fn ore_rides_from_a_chest_through_an_extraction_pipe_and_a_pipe_into_the_melter() {
    let (chest, extraction, pipe, melter) = ([0, 64, 0], [1, 64, 0], [2, 64, 0], [3, 64, 0]);
    let mut sim = SimOps::default();
    sim.add_chest(chest, "minecraft:chest", vec![Some(stack("minecraft:iron_ore", 3))]);
    sim.add_item_pipe(extraction, "pipes:extraction_pipe[facing=west]");
    sim.add_item_pipe(pipe, "pipes:item_pipe");
    sim.add_chest(melter, "tconstruct:seared_melter", vec![None, None]);
    sim.power_item(extraction, true);
    for _ in 0..60 {
        sim.tick_item(extraction, item_pipe::Kind::Extraction);
        sim.tick_item(pipe, item_pipe::Kind::Transport);
    }
    assert_eq!(sim.item_total(chest, "minecraft:iron_ore"), 0, "the chest is empty");
    assert_eq!(sim.item_total(melter, "minecraft:iron_ore"), 3, "all three ores reached the melter, where its own ticker melts them");
}

#[test]
fn a_casting_tables_product_is_taken_only_once_it_can_be_taken() {
    let (table, extraction, chest) = ([0, 64, 0], [1, 64, 0], [2, 64, 0]);
    let mut sim = SimOps::default();
    // The casting table: slot 0 is the cast (empty here), slot 1 the finished product.
    sim.add_chest(table, "tconstruct:seared_table", vec![None, Some(stack("tconstruct:iron_ingot", 1))]);
    sim.add_item_pipe(extraction, "pipes:extraction_pipe[facing=west]");
    sim.add_chest(chest, "minecraft:chest", vec![None]);
    sim.power_item(extraction, true);
    sim.take_blocked.insert(table);
    for _ in 0..30 {
        sim.tick_item(extraction, item_pipe::Kind::Extraction);
    }
    assert_eq!(sim.item_total(table, "tconstruct:iron_ingot"), 1, "mid-cast (the side may not take): the product stays");
    assert_eq!(sim.item_total(chest, "tconstruct:iron_ingot"), 0);
    sim.take_blocked.remove(&table);
    for _ in 0..30 {
        sim.tick_item(extraction, item_pipe::Kind::Extraction);
    }
    assert_eq!(sim.item_total(table, "tconstruct:iron_ingot"), 0);
    assert_eq!(sim.item_total(chest, "tconstruct:iron_ingot"), 1, "once the cast is done and the slot can be taken, the product rides to the chest");
}

#[test]
fn fluid_rides_from_the_smeltery_through_an_extraction_pipe_and_a_pipe_into_a_tank() {
    let (smeltery, extraction, pipe, tank) = ([0, 64, 0], [1, 64, 0], [2, 64, 0], [3, 64, 0]);
    let mut sim = SimOps::default();
    sim.add_tank(smeltery, "tconstruct:smeltery_controller", Some(fluid("tconstruct:molten_iron", 900)), 4000);
    sim.add_fluid_pipe(extraction, "pipes:fluid_extraction_pipe[facing=west]");
    sim.add_fluid_pipe(pipe, "pipes:fluid_pipe");
    sim.add_tank(tank, "tconstruct:seared_ingot_tank", None, 4000);
    sim.power_fluid(extraction, true);
    for _ in 0..60 {
        sim.tick_fluid(extraction, fluid_pipe::Kind::Extraction);
        sim.tick_fluid(pipe, fluid_pipe::Kind::Transport);
    }
    let total = sim.fluid_amount(smeltery) + sim.fluid_amount(extraction) + sim.fluid_amount(pipe) + sim.fluid_amount(tank);
    assert_eq!(total, 900, "fluid is only ever moved, never created or lost");
    assert_eq!(sim.fluid_amount(smeltery), 0, "the smeltery is drained");
    assert!(sim.fluid_amount(tank) >= 600, "most of it reached the tank, the rest is still in the pipes: {}", sim.fluid_amount(tank));
}

#[test]
fn an_unpowered_extraction_pipe_pulls_nothing() {
    let (chest, extraction, dest) = ([0, 64, 0], [1, 64, 0], [2, 64, 0]);
    let mut sim = SimOps::default();
    sim.add_chest(chest, "minecraft:chest", vec![Some(stack("minecraft:iron_ore", 2))]);
    sim.add_item_pipe(extraction, "pipes:extraction_pipe[facing=west]");
    sim.add_chest(dest, "minecraft:chest", vec![None]);
    for _ in 0..24 {
        sim.tick_item(extraction, item_pipe::Kind::Extraction);
    }
    assert_eq!(sim.item_total(chest, "minecraft:iron_ore"), 2);
    assert_eq!(sim.item_total(dest, "minecraft:iron_ore"), 0);

    let (source, fluid_extraction, sink) = ([0, 66, 0], [1, 66, 0], [2, 66, 0]);
    sim.add_tank(source, "tconstruct:smeltery_controller", Some(fluid("tconstruct:molten_iron", 500)), 4000);
    sim.add_fluid_pipe(fluid_extraction, "pipes:fluid_extraction_pipe[facing=west]");
    sim.add_tank(sink, "tconstruct:seared_ingot_tank", None, 4000);
    for _ in 0..24 {
        sim.tick_fluid(fluid_extraction, fluid_pipe::Kind::Extraction);
    }
    assert_eq!(sim.fluid_amount(source), 500);
    assert_eq!(sim.fluid_amount(sink), 0);
}

#[test]
fn a_junction_alternates_its_exits_and_never_sends_a_stack_back() {
    let (centre, north, south) = ([1, 64, 0], [1, 64, -1], [1, 64, 1]);
    let mut sim = SimOps::default();
    sim.add_item_pipe(centre, "pipes:item_pipe");
    sim.add_chest(north, "minecraft:chest", vec![None]);
    sim.add_chest(south, "minecraft:chest", vec![None]);
    for _ in 0..2 {
        sim.pipe_items.get_mut(&centre).unwrap()[face_index(Direction::West)] = Some(stack("minecraft:cobblestone", 1));
        for _ in 0..8 {
            sim.tick_item(centre, item_pipe::Kind::Transport);
        }
    }
    assert_eq!(sim.item_total(north, "minecraft:cobblestone"), 1, "the first stack takes the first exit");
    assert_eq!(sim.item_total(south, "minecraft:cobblestone"), 1, "the second stack takes the other exit (round robin)");
}

#[test]
fn a_full_destination_holds_the_stack_until_it_has_room() {
    let (pipe, dest) = ([1, 64, 0], [2, 64, 0]);
    let mut sim = SimOps::default();
    sim.add_item_pipe(pipe, "pipes:item_pipe");
    sim.add_chest(dest, "minecraft:chest", vec![Some(stack("minecraft:stone", 64))]);
    sim.pipe_items.get_mut(&pipe).unwrap()[face_index(Direction::West)] = Some(stack("minecraft:cobblestone", 5));
    for _ in 0..16 {
        sim.tick_item(pipe, item_pipe::Kind::Transport);
    }
    assert_eq!(sim.item_total(dest, "minecraft:cobblestone"), 0);
    let held = PipeState::load(sim.pipe_data.get(&pipe).unwrap());
    assert_eq!(held.transit.len(), 1, "the stack waits inside the pipe; it is not dropped or voided");
    assert_eq!(held.transit[0].stack.count, 5);
    sim.chests.get_mut(&dest).unwrap()[0] = None;
    for _ in 0..16 {
        sim.tick_item(pipe, item_pipe::Kind::Transport);
    }
    assert_eq!(sim.item_total(dest, "minecraft:cobblestone"), 5, "with room again, the delivery completes");
}

#[test]
fn fluids_do_not_mix() {
    let (water, extraction) = ([0, 64, 0], [1, 64, 0]);
    let mut sim = SimOps::default();
    sim.add_tank(water, "tconstruct:seared_ingot_tank", Some(fluid("minecraft:water", 500)), 4000);
    sim.add_fluid_pipe(extraction, "pipes:fluid_extraction_pipe[facing=west]");
    sim.set_pipe_fluid(&extraction, Some(fluid("minecraft:lava", 100)));
    sim.power_fluid(extraction, true);
    for _ in 0..10 {
        sim.tick_fluid(extraction, fluid_pipe::Kind::Extraction);
    }
    assert_eq!(sim.fluid_amount(extraction), 100, "the lava stays undiluted");
    assert_eq!(sim.fluid_amount(water), 500, "no water was drawn into a lava pipe");

    let (west, east) = ([3, 64, 0], [4, 64, 0]);
    sim.add_fluid_pipe(west, "pipes:fluid_pipe");
    sim.add_fluid_pipe(east, "pipes:fluid_pipe");
    sim.set_pipe_fluid(&west, Some(fluid("minecraft:lava", 100)));
    sim.set_pipe_fluid(&east, Some(fluid("minecraft:water", 100)));
    for _ in 0..10 {
        sim.tick_fluid(west, fluid_pipe::Kind::Transport);
    }
    assert_eq!(sim.fluid_amount(west), 100);
    assert_eq!(sim.fluid_amount(east), 100, "neighbouring pipes of different fluids exchange nothing");
}

#[test]
fn saved_state_round_trips_through_data() {
    let mut custom = NbtCompound::new();
    custom.insert("Material".to_owned(), pluto_lib::Nbt::String("tconstruct:iron".to_owned()));
    let mut stack = stack("tconstruct:pick_head", 1);
    stack.damage = 7;
    stack.max_damage = Some(250);
    stack.custom_data = custom;
    let state = PipeState {
        transit: vec![Transit { stack, from: Direction::West, to: Direction::East, ticks: 3, retries: 1 }],
        cursor: 2,
        powered: true,
        clock: 5,
    };
    let mut data = NbtCompound::new();
    state.save(&mut data);
    assert_eq!(PipeState::load(&data), state);
    assert_eq!(PipeState::load(&NbtCompound::new()), PipeState::default());

    let flow = FlowState { powered: true, cursor: 4 };
    let mut data = NbtCompound::new();
    flow.save(&mut data);
    assert_eq!(FlowState::load(&data), flow);
}

#[test]
fn connections_follow_the_neighbour_and_the_family() {
    let mut view = MemoryBlockView::default();
    view.blocks.insert([1, 64, 0], BlockState::parse("minecraft:chest").unwrap());
    let base = BlockState::parse("pipes:extraction_pipe").unwrap();
    // The chest at x=1 is clicked on its west face, so the pipe lands at x=0 facing back east.
    let placed = connections::placement(base, [0, 64, 0], Direction::West, &view, Family::Items, true);
    assert_eq!(placed.property("east"), Some("true"), "an arm toward the chest");
    assert_eq!(placed.property("facing"), Some("east"), "it pulls out of the block it was placed against");
    assert_eq!(placed.property("north"), Some("false"));
    let air = BlockState::parse("minecraft:air").unwrap();
    let cleared = connections::shape_update(&placed, Direction::East, Some(&air), Family::Items).unwrap();
    assert_eq!(cleared.property("east"), Some("false"), "the arm goes when the neighbour does");
    assert!(!connections::connectable(Family::Fluids, &BlockState::parse("minecraft:chest").unwrap()));
    assert!(connections::connectable(Family::Fluids, &BlockState::parse("tconstruct:seared_ingot_tank").unwrap()));
    assert!(connections::connectable(Family::Items, &BlockState::parse("tconstruct:seared_table").unwrap()));
    assert!(connections::connectable(Family::Items, &BlockState::parse("tconstruct:seared_melter").unwrap()));
}

#[test]
fn the_registered_types_open_one_inbox_slot_per_face_and_refuse_takes() {
    item_pipe::register();
    fluid_pipe::register();
    for face in Direction::ALL {
        let access = BlockEntityTypeRegistry::sided_access("pipes:item_pipe", face).expect("item pipe sided access");
        assert_eq!(access.insert, vec![face_index(face)], "face {face:?} inserts into its own slot");
        assert!(access.take.is_empty(), "nothing may be taken out of a pipe");
    }
    let data = NbtCompound::new();
    let items: Vec<Option<ItemSlot>> = vec![None; 6];
    let ore = stack("minecraft:iron_ore", 1);
    let allowed = |slot: usize, face: Option<Direction>, mode: TransferMode| {
        BlockEntityTypeRegistry::item_transfer_allowed("pipes:item_pipe", [0, 0, 0], &data, &items, slot, face, mode, ore.clone())
    };
    assert!(allowed(face_index(Direction::East), Some(Direction::East), TransferMode::Insert));
    assert!(!allowed(face_index(Direction::West), Some(Direction::East), TransferMode::Insert), "a stack may only enter through the slot of the face it arrives on");
    assert!(!allowed(face_index(Direction::East), Some(Direction::East), TransferMode::Take), "takes are refused");
    let tanks = BlockEntityTypeRegistry::read_tanks("pipes:fluid_pipe", &NbtCompound::new()).expect("fluid pipe tanks");
    assert_eq!(tanks.len(), 1);
    assert_eq!(tanks[0].capacity, fluid_pipe::CAPACITY);
}
