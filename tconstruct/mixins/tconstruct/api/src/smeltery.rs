//! The smeltery and foundry controller, from SlimeKnights HeatingStructureBlockEntity, SmelteryBlockEntity,
//! FoundryBlockEntity, MeltingModuleInventory, ByproductMeltingModuleInventory, MultitankFuelModule and EntityMeltingModule
//! (MIT; see LICENSE): the structure check every second, the 4-tick heating cycle over a variable number of melting slots, fuel
//! taken from the tanks in the walls, alloying, and hurting the entities inside. `heating.rs` holds the slot and fuel state
//! machines, `multiblock.rs` the detection and `smeltery_tank.rs` the tank; this ties them to the item slots and the world
//! through `Env`, which the host side fills in from PlutoLib reads.
use crate::{
    fluid_ingredient::FluidTags,
    fluid_values::GLASS_PANE,
    heating::{self, Finished, Fuel, MeltingSlot},
    melter::{LiquidFuel, Melt},
    melting_recipes::Alloy,
    multiblock::{Cell, Multiblock, Pos, Shape, StructureData},
    smeltery_tank::{Alloying, Change, Fluid, SmelteryAlloy, SmelteryTank},
};
use pluto_lib::{Direction, Identifier, ItemSlot, Nbt, NbtCompound, NbtCompoundExt};

/// PlutoLib's most item slots for one block entity: the melting inventory is capped here, not at the original's 10916.
pub(crate) const MAX_SLOTS: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind { Smeltery, Foundry }

/// An entity inside the structure as the entity module sees it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Entity {
    pub id: u64,
    pub kind: Identifier,
    /// The stack of an item entity.
    pub item: Option<ItemSlot>,
    /// A living entity and whether it can be hurt by the smeltery's damage.
    pub living: Option<Living>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Living {
    /// Fire immune mobs are hurt by the smeltery's magic damage instead of its heat.
    pub fire_immune: bool,
    /// Invulnerable to that damage, a creative player, or with fire resistance (`canMeltEntity` is false).
    pub immune: bool,
}

/// The entities of the world, what the host can do about them.
pub(crate) trait Entities {
    /// Every living entity and item entity in the box (min corner, max corner).
    fn in_box(&mut self, bounds: [f64; 6]) -> Vec<Entity>;
    /// Removes an item entity that was taken whole.
    fn discard(&mut self, id: u64);
    /// Leaves what did not fit in an item entity.
    fn set_item(&mut self, id: u64, stack: ItemSlot);
    /// Hurts the entity with the smeltery's magic or heat damage; whether it took any.
    fn hurt(&mut self, id: u64, magic: bool, amount: i32) -> bool;
}

/// What the controller asks of its surroundings.
pub(crate) trait Env {
    /// The block at `pos` as the structure of `kind` sees it; `None` where its chunk is not loaded.
    fn cell(&self, kind: Kind, pos: Pos) -> Option<Cell>;
    /// Joins a block to the controller's structure or frees it (`in_structure`).
    fn mark(&mut self, pos: Pos, in_structure: bool);
    /// The controller's own block state, `(in_structure, active)`.
    fn block_state(&self) -> (bool, bool);
    fn set_block_state(&mut self, in_structure: bool, active: bool);
    fn melt(&self, kind: Kind, stack: &ItemSlot) -> Option<Melt>;
    /// What the foundry gets besides the main fluid; empty for no recipe or none.
    fn byproducts(&self, stack: &ItemSlot) -> Vec<(Identifier, i32)>;
    fn liquid_fuel(&self, fluid: &Identifier) -> Option<LiquidFuel>;
    fn alloys(&self) -> &[Alloy];
    fn fluid_tags(&self) -> &FluidTags;
    /// The fluid in the first tank of the block at `pos` and its amount.
    fn tank_fluid(&mut self, pos: Pos) -> Option<(Identifier, i32)>;
    fn tank_drain(&mut self, pos: Pos, fluid: &Identifier, amount: i32) -> i32;
    /// The fluid and amount an entity melts to, and the damage it takes (`EntityMeltingRecipe`).
    fn entity_melting(&self, entity: &Identifier) -> Option<(Identifier, i32, i32)>;
    fn melting_blacklisted(&self, entity: &Identifier) -> bool;
    /// `None` where the host cannot reach entities.
    fn entities(&mut self) -> Option<&mut dyn Entities>;
    /// `Random.nextInt(bound)` for the alloy shuffle.
    fn random(&mut self, bound: usize) -> usize;
}

/// What an item in a slot was, ignoring how many (the slot restarts when it changes).
type Key = (Identifier, u32, NbtCompound);
fn key_of(stack: &ItemSlot) -> Key { (stack.item.clone(), stack.damage, stack.custom_data.clone()) }

#[derive(Clone, Debug)]
pub(crate) struct Controller {
    pub kind: Kind,
    pos: Pos,
    facing: Direction,
    multiblock: Multiblock,
    pub structure: Option<StructureData>,
    pub tank: SmelteryTank,
    alloying: Alloying,
    pub slots: Vec<MeltingSlot>,
    keys: Vec<Option<Key>>,
    pub fuel: Fuel,
    last_tank: Option<Pos>,
    pub fuel_rate: i32,
    pub tick: i32,
    expand_counter: i32,
    update_queued: bool,
    fluid_update_queued: bool,
    pub error_pos: Option<Pos>,
}

const NO_POS: Pos = [0, i16::MIN as i32, 0];

impl Controller {
    pub(crate) fn new(kind: Kind, pos: Pos, facing: Direction) -> Self {
        Self {
            kind, pos, facing,
            multiblock: match kind { Kind::Smeltery => Multiblock::smeltery(pos), Kind::Foundry => Multiblock::foundry(pos) },
            structure: None, tank: SmelteryTank::default(), alloying: Alloying::default(), slots: Vec::new(), keys: Vec::new(),
            fuel: Fuel::default(), last_tank: None, fuel_rate: 1, tick: 0, expand_counter: 0, update_queued: false, fluid_update_queued: false, error_pos: None,
        }
    }

    pub(crate) fn shape(&self) -> Shape { self.multiblock.shape() }
    pub(crate) fn last_result(&self) -> &crate::multiblock::MultiblockResult { &self.multiblock.last_result }

    /// `updateStructure`: a check on the next tick.
    pub(crate) fn update_structure(&mut self) { self.update_queued = true; }

    // --- structure

    /// `setStructure` of the smeltery and the foundry: tank capacity, melting slots and fuel rate from the inner size.
    fn set_structure(&mut self, structure: Option<StructureData>, items: &mut [Option<ItemSlot>]) {
        self.structure = structure;
        let Some(structure) = &self.structure else { return };
        let (dx, dy, dz) = (structure.inner_x, structure.inner_y, structure.inner_z);
        let size = match self.kind { Kind::Smeltery => heating::smeltery(dx, dy, dz), Kind::Foundry => heating::foundry(dx, dy, dz) };
        self.tank.set_capacity(size.capacity);
        self.resize(size.slots.max(0) as usize, items);
        self.fuel_rate = size.fuel_rate;
    }

    /// `MeltingModuleInventory.resize`: items past the new size stay in their slot (the host cannot drop them from a ticker) and wait
    /// there until the structure grows again.
    fn resize(&mut self, size: usize, _items: &mut [Option<ItemSlot>]) {
        let size = size.min(MAX_SLOTS);
        self.slots.resize(size, MeltingSlot::default());
        self.keys.resize(size, None);
    }

    /// `checkStructure`.
    fn check_structure(&mut self, items: &mut [Option<ItemSlot>], env: &mut dyn Env) {
        let kind = self.kind;
        let (was_formed, _) = env.block_state();
        let old = self.structure.clone();
        let new = self.multiblock.detect(&|pos| env.cell(kind, pos), self.facing);
        let formed = new.is_some();
        if formed != was_formed {
            // unformed controllers are marked inactive
            let active = formed && env.block_state().1;
            env.set_block_state(formed, active);
        }
        match new {
            Some(new) => {
                new.assign_master(old.as_ref(), &mut |pos, joined| env.mark(pos, joined));
                self.set_structure(Some(new), items);
            }
            None => {
                if let Some(old) = &old { old.clear_master(&mut |pos, joined| env.mark(pos, joined)); }
                self.structure = None;
            }
        }
        self.error_pos = self.multiblock.last_result.pos;
        self.expand_counter = 0;
    }

    /// `invalidateStructure`: the controller was broken.
    pub(crate) fn invalidate_structure(&mut self, env: &mut dyn Env) {
        if let Some(structure) = self.structure.take() {
            structure.clear_master(&mut |pos, joined| env.mark(pos, joined));
            self.error_pos = None;
        }
    }

    /// `notifyChange`: a block of the structure changed.
    pub(crate) fn notify_change(&mut self, pos: Pos, cell: &Cell) {
        if let Some(structure) = &self.structure {
            if self.multiblock.should_update(structure, pos, cell) { self.update_structure(); }
        }
    }

    /// Whether a fresh look at the structure disagrees with the formed one. Wall blocks have no block entity here to tell the
    /// controller they changed, so once a second the walls are looked at; growing upward is left to the original's slower check.
    fn walls_changed(&mut self, env: &dyn Env) -> bool {
        let Some(old) = &self.structure else { return false };
        let kind = self.kind;
        let mut probe = self.multiblock.clone();
        match probe.detect(&|pos| env.cell(kind, pos), self.facing) {
            None => true,
            Some(now) => (now.min, now.tanks.clone(), [now.max[0], now.max[2]]) != (old.min, old.tanks.clone(), [old.max[0], old.max[2]]) || now.max[1] < old.max[1],
        }
    }

    // --- fuel

    /// `MultitankFuelModule.findFuel`: the tank the last fuel came from first, then the others in structure order.
    pub(crate) fn find_fuel(&mut self, consume: bool, env: &mut dyn Env) -> i32 {
        let tanks = self.structure.as_ref().map(|structure| structure.tanks.clone()).unwrap_or_default();
        let mut order = Vec::with_capacity(tanks.len());
        if let Some(last) = self.last_tank.filter(|last| tanks.contains(last)) { order.push(last); }
        order.extend(tanks.iter().copied().filter(|pos| Some(*pos) != self.last_tank));
        for pos in order {
            let Some((fluid, amount)) = env.tank_fluid(pos) else { continue };
            let Some(recipe) = env.liquid_fuel(&fluid) else { continue };
            let burn = consume && amount >= recipe.amount;
            let temperature = self.fuel.try_liquid(amount, recipe.amount, recipe.duration, recipe.temperature, recipe.rate, burn);
            if temperature > 0 {
                if burn { env.tank_drain(pos, &fluid, recipe.amount); }
                self.last_tank = Some(pos);
                return temperature;
            }
        }
        if consume { self.fuel.none_found(); }
        0
    }

    fn can_melt_entities(&mut self, env: &mut dyn Env) -> bool { self.fuel.has_fuel() || self.find_fuel(false, env) > 0 }

    // --- melting inventory

    /// `MeltingModule.setStack` for every slot whose item changed since the last pass, however it got there.
    fn sync(&mut self, items: &[Option<ItemSlot>], env: &dyn Env) {
        for index in 0..self.slots.len() {
            let stack = items.get(index).and_then(Option::as_ref);
            let key = stack.map(key_of);
            if key == self.keys[index] { continue; }
            self.slots[index].set_stack(stack.map(|stack| (env.melt(self.kind, stack).map(|melt| (melt.time, melt.temperature)), false)));
            self.keys[index] = key;
        }
    }

    pub(crate) fn can_heat(&mut self, temperature: i32) -> bool { self.slots.iter_mut().any(|slot| slot.can_heat(temperature)) }

    /// `tryFillTank` (and the foundry's byproducts): the whole amount goes in or nothing does.
    fn finish(&mut self, stack: Option<&ItemSlot>, env: &dyn Env) -> Finished {
        let Some(melt) = stack.and_then(|stack| env.melt(self.kind, stack)) else { return Finished::NoRecipe };
        if self.tank.fill(&melt.fluid, melt.amount, false) == melt.amount {
            self.tank.fill(&melt.fluid, melt.amount, true);
            if self.kind == Kind::Foundry {
                for (fluid, amount) in env.byproducts(stack.expect("a recipe was found")) { self.tank.fill(&fluid, amount, true); }
            }
            Finished::Filled
        } else {
            Finished::NoSpace
        }
    }

    fn each_slot(&mut self, items: &mut [Option<ItemSlot>], env: &dyn Env, act: &mut dyn FnMut(&mut MeltingSlot, &mut dyn FnMut() -> Finished)) {
        for index in 0..self.slots.len() {
            let stack = items.get(index).and_then(Option::as_ref).cloned();
            let mut slot = self.slots[index];
            let mut emptied = false;
            {
                let mut finish = || {
                    let result = self.finish(stack.as_ref(), env);
                    emptied |= result == Finished::Filled;
                    result
                };
                act(&mut slot, &mut finish);
            }
            self.slots[index] = slot;
            if emptied {
                if let Some(held) = items.get_mut(index) { *held = None; }
                self.keys[index] = None;
            }
        }
    }

    pub(crate) fn heat_items(&mut self, items: &mut [Option<ItemSlot>], env: &dyn Env) {
        let (temperature, rate) = (self.fuel.temperature, self.fuel.rate);
        self.each_slot(items, env, &mut |slot, finish| slot.heat(temperature, rate, finish));
    }

    pub(crate) fn cool_items(&mut self, items: &mut [Option<ItemSlot>], env: &dyn Env) {
        self.each_slot(items, env, &mut |slot, finish| slot.cool(finish));
    }

    /// `ItemHandlerHelper.insertItem` over the melting slots: one item to the first empty slot, in order.
    fn insert_item(&mut self, items: &mut [Option<ItemSlot>], mut stack: ItemSlot) -> Option<ItemSlot> {
        for index in 0..self.slots.len().min(items.len()) {
            if stack.count == 0 { break; }
            if items[index].is_none() {
                let mut one = stack.clone();
                one.count = 1;
                items[index] = Some(one);
                stack.count -= 1;
            }
        }
        (stack.count > 0).then_some(stack)
    }

    // --- entities

    /// `EntityMeltingModule.interactWithEntities`: items inside go into the melting slots, living entities are hurt for their fluid.
    /// True if something was melted, so fuel is needed.
    fn interact_with_entities(&mut self, items: &mut [Option<ItemSlot>], env: &mut dyn Env) -> bool {
        let Some(structure) = &self.structure else { return false };
        let bounds = structure.bounds;
        let Some(entities) = env.entities() else { return false };
        let found = entities.in_box(bounds);
        let mut can_melt: Option<bool> = None;
        let mut melted = false;
        for entity in found {
            if let Some(stack) = entity.item.clone() {
                match self.insert_item(items, stack) {
                    None => { if let Some(entities) = env.entities() { entities.discard(entity.id); } }
                    Some(rest) => { if let Some(entities) = env.entities() { entities.set_item(entity.id, rest); } }
                }
                continue;
            }
            let Some(living) = entity.living else { continue };
            if can_melt == Some(false) || living.immune || env.melting_blacklisted(&entity.kind) { continue; }
            let allowed = *can_melt.get_or_insert_with(|| self.can_melt_entities(env));
            if !allowed { continue; }
            let (fluid, amount, damage) = env.entity_melting(&entity.kind)
                .unwrap_or_else(|| (Identifier::parse("tconstruct:liquid_soul").expect("valid id"), GLASS_PANE / 5, 2));
            let hurt = env.entities().is_some_and(|entities| entities.hurt(entity.id, living.fire_immune, damage));
            if hurt {
                // it is fine if we do not fill it all, leftover fluid is just lost
                self.tank.fill(&fluid, amount, true);
                melted = true;
            }
        }
        melted
    }

    // --- the tick

    fn alloy(&mut self, env: &mut dyn Env, temperature: i32, act: impl FnOnce(&mut Alloying, &[Alloy], &mut SmelteryAlloy<'_>, &FluidTags) -> bool) -> bool {
        let alloys = env.alloys().to_vec();
        let tags = env.fluid_tags().clone();
        let mut view = SmelteryAlloy { tank: &mut self.tank, temperature };
        act(&mut self.alloying, &alloys, &mut view, &tags)
    }

    /// `cool`: while inactive or without a fuel tank, every 4 ticks consume fuel and cool items.
    fn cool(&mut self, items: &mut [Option<ItemSlot>], env: &dyn Env) {
        match self.tick % 4 {
            1 => { if !self.fuel.has_fuel() { self.cool_items(items, env); } }
            3 => { if self.fuel.has_fuel() && self.fuel_rate > 0 { self.fuel.decrease(self.fuel_rate); } }
            _ => {}
        }
    }

    /// `heat`: one game tick of the cycle on a formed structure.
    fn heat(&mut self, items: &mut [Option<ItemSlot>], env: &mut dyn Env) {
        let Some(structure) = &self.structure else { return };
        if !structure.has_tanks() {
            self.cool(items, env);
            return;
        }
        let mut entity_melted = false;
        if self.tick == 12 { entity_melted = self.interact_with_entities(items, env); }
        let smeltery = self.kind == Kind::Smeltery;
        match self.tick % 4 {
            0 => {
                if !self.fuel.has_fuel() {
                    if entity_melted {
                        self.find_fuel(true, env);
                    } else {
                        let possible = self.find_fuel(false, env);
                        let can_alloy = smeltery && self.alloy(env, possible, |module, alloys, tank, tags| module.can_alloy(alloys, tank, tags));
                        if self.can_heat(possible) || can_alloy { self.find_fuel(true, env); }
                    }
                }
            }
            1 => { if self.fuel.has_fuel() { self.heat_items(items, env); } else { self.cool_items(items, env); } }
            2 => {
                if smeltery && self.fuel.has_fuel() {
                    let temperature = self.fuel.temperature;
                    // Collections.shuffle draws from the level's random
                    let alloys = env.alloys().to_vec();
                    let tags = env.fluid_tags().clone();
                    let mut draws = Vec::new();
                    for bound in (2..=alloys.len()).rev() { draws.push(env.random(bound)); }
                    let mut draws = draws.into_iter();
                    let mut view = SmelteryAlloy { tank: &mut self.tank, temperature };
                    self.alloying.do_alloy(&alloys, &mut view, &tags, &mut |_| draws.next().unwrap_or(0));
                }
            }
            _ => {
                // update the active state, consume fuel
                let has_fuel = self.fuel.has_fuel();
                let (in_structure, active) = env.block_state();
                if active != has_fuel { env.set_block_state(in_structure, has_fuel); }
                self.fuel.decrease(self.fuel_rate);
            }
        }
    }

    /// `notifyFluidsChanged`: a fluid change is sent to clients, and a new fluid means alloys that did not match before might now.
    fn notify_fluids(&mut self) {
        for change in self.tank.take_events() {
            if change == Change::Added { self.alloying.clear_cached_recipes(); }
            self.fluid_update_queued = true;
        }
    }

    /// `serverTick`, one game tick. `items` are the item slots of the block entity; the first `slots.len()` are the melting inventory.
    pub(crate) fn step(&mut self, items: &mut [Option<ItemSlot>], env: &mut dyn Env) {
        self.notify_fluids();
        self.sync(items, env);
        if self.update_queued {
            self.check_structure(items, env);
            self.update_queued = false;
        }
        let (in_structure, _) = env.block_state();
        if self.structure.is_some() && in_structure {
            let kind = self.kind;
            if self.tick == 0 {
                // every 15 seconds, check above the structure to try to expand
                self.expand_counter += 1;
                let height = self.structure.as_ref().map_or(0, |structure| structure.inner_y);
                if self.expand_counter >= 10 && height < self.multiblock.max_height {
                    self.expand_counter = 0;
                    let structure = self.structure.clone().expect("checked");
                    if self.multiblock.can_expand(&structure, &|pos| env.cell(kind, pos)) {
                        self.update_structure();
                    } else {
                        self.error_pos = self.multiblock.last_result.pos;
                    }
                }
            } else if self.tick % 4 == 0 {
                // the next inside position must still be a valid inner block
                if let Some(structure) = self.structure.as_mut() {
                    let next = structure.next_inside_check();
                    if !self.multiblock.is_inner_block(&|pos| env.cell(kind, pos), next) { self.update_queued = true; }
                }
            }
            if self.tick == 10 && self.walls_changed(env) { self.update_queued = true; }
            self.heat(items, env);
            self.notify_fluids();
        } else {
            // every second, try to reform the structure
            if self.tick == 0 { self.update_queued = true; }
            self.cool(items, env);
        }
        self.tick = (self.tick + 1) % 20;
    }

    /// The fuel tank the window shows (`MultitankFuelModule.getFuelInfo`): the one fuel came from last, or the first with fluid in it;
    /// its fluid, the amount of that fluid over all tanks and the capacity of those tanks plus the empty ones.
    pub(crate) fn fuel_info(&self, env: &mut dyn Env, capacity_of: &dyn Fn(Pos) -> i32) -> Option<(Identifier, i32, i32)> {
        let tanks = self.structure.as_ref()?.tanks.clone();
        let main = self.last_tank.filter(|last| tanks.contains(last)).and_then(|pos| env.tank_fluid(pos).map(|(fluid, _)| (pos, fluid)))
            .or_else(|| tanks.iter().find_map(|pos| env.tank_fluid(*pos).map(|(fluid, _)| (*pos, fluid))))?;
        let (mut amount, mut capacity) = (0, 0);
        for pos in tanks {
            match env.tank_fluid(pos) {
                Some((fluid, held)) if fluid == main.1 => { amount += held; capacity += capacity_of(pos).max(held); }
                None => capacity += capacity_of(pos),
                _ => {}
            }
        }
        (amount > 0 && capacity > 0).then_some((main.1, amount, capacity))
    }

    /// Whether the fluid change is waiting to be sent to clients (`fluidUpdateQueued`); the host reads and clears it.
    pub(crate) fn take_fluid_update(&mut self) -> bool { std::mem::take(&mut self.fluid_update_queued) }

    // --- saving

    pub(crate) const TAG_STRUCTURE: &'static str = "multiblock";
    const TAG_TANK: &'static str = "tank";
    const TAG_INVENTORY: &'static str = "inventory";

    pub(crate) fn save(&self, data: &mut NbtCompound) {
        match &self.structure {
            Some(structure) => { data.insert(Self::TAG_STRUCTURE.to_owned(), Nbt::Compound(structure.write_to_tag(self.pos))); }
            None => { data.remove(Self::TAG_STRUCTURE); }
        }
        data.insert(Self::TAG_TANK.to_owned(), Nbt::Compound(self.tank.write()));
        // MeltingModuleInventory.writeToTag: the size, and the progress of every slot that holds an item
        let mut inventory = NbtCompound::new();
        inventory.insert("size".to_owned(), Nbt::Int(self.slots.len() as i32));
        let saved = self.slots.iter().enumerate().filter(|(_, slot)| slot.occupied).map(|(index, slot)| {
            let mut tag = NbtCompound::new();
            tag.insert("slot".to_owned(), Nbt::Int(index as i32));
            tag.insert("time".to_owned(), Nbt::Int(slot.current_time));
            tag.insert("required".to_owned(), Nbt::Int(slot.required_time));
            tag.insert("temp".to_owned(), Nbt::Int(slot.required_temp));
            Nbt::Compound(tag)
        }).collect();
        inventory.insert("items".to_owned(), Nbt::List(saved));
        data.insert(Self::TAG_INVENTORY.to_owned(), Nbt::Compound(inventory));
        for (key, value) in [("fuel", self.fuel.fuel), ("temperature", self.fuel.temperature), ("rate", self.fuel.rate)] {
            data.insert(key.to_owned(), Nbt::Int(value));
        }
        if let Some(last) = self.last_tank {
            data.insert("last_fuel".to_owned(), Nbt::IntArray(vec![last[0] - self.pos[0], last[1] - self.pos[1], last[2] - self.pos[2]]));
        }
    }

    /// Reads the saved state. A stack that is in a slot is the stack the saved progress belongs to.
    pub(crate) fn load(kind: Kind, pos: Pos, facing: Direction, data: &NbtCompound, items: &[Option<ItemSlot>]) -> Self {
        let mut this = Self::new(kind, pos, facing);
        if let Some(tag) = data.get_compound(Self::TAG_TANK) { this.tank.read(tag); }
        if let Some(tag) = data.get_compound(Self::TAG_STRUCTURE) {
            this.structure = this.multiblock.read_from_tag(tag, pos);
            if let Some(structure) = &this.structure {
                let size = match kind { Kind::Smeltery => heating::smeltery(structure.inner_x, structure.inner_y, structure.inner_z), Kind::Foundry => heating::foundry(structure.inner_x, structure.inner_y, structure.inner_z) };
                this.fuel_rate = size.fuel_rate;
            }
        }
        if let Some(inventory) = data.get_compound(Self::TAG_INVENTORY) {
            let size = (inventory.get_int("size", 0) as usize).min(MAX_SLOTS);
            this.slots = vec![MeltingSlot::default(); size];
            this.keys = vec![None; size];
            let saved = inventory.get("items").and_then(Nbt::as_list).unwrap_or(&[]);
            for slot in saved.iter().filter_map(Nbt::as_compound) {
                let index = slot.get_int("slot", -1);
                let Some(stack) = usize::try_from(index).ok().filter(|index| *index < size).and_then(|index| items.get(index).and_then(Option::as_ref).map(|stack| (index, stack))) else { continue };
                this.keys[stack.0] = Some(key_of(stack.1));
                this.slots[stack.0] = MeltingSlot {
                    occupied: true,
                    current_time: slot.get_int("time", 0) as i32,
                    required_time: slot.get_int("required", 0) as i32,
                    required_temp: slot.get_int("temp", 0) as i32,
                };
            }
        }
        this.fuel.fuel = data.get_int("fuel", 0) as i32;
        this.fuel.temperature = data.get_int("temperature", 0) as i32;
        this.fuel.rate = data.get_int("rate", 0) as i32;
        if let Some(Nbt::IntArray(last)) = data.get("last_fuel") {
            if let [x, y, z] = last[..] { this.last_tank = Some([x + pos[0], y + pos[1], z + pos[2]]).filter(|last| last[1] != NO_POS[1]); }
        }
        this
    }

    pub(crate) fn fluids(&self) -> &[Fluid] { self.tank.fluids() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::melting_recipes::Alloy as AlloyRecipe;
    use std::collections::{BTreeMap, HashMap};

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }

    #[derive(Default)]
    struct Mobs { list: Vec<Entity>, discarded: Vec<u64>, left: Vec<(u64, ItemSlot)>, hurt: Vec<(u64, bool, i32)> }
    impl Entities for Mobs {
        fn in_box(&mut self, _: [f64; 6]) -> Vec<Entity> { self.list.clone() }
        fn discard(&mut self, id: u64) { self.discarded.push(id); }
        fn set_item(&mut self, id: u64, stack: ItemSlot) { self.left.push((id, stack)); }
        fn hurt(&mut self, id: u64, magic: bool, amount: i32) -> bool { self.hurt.push((id, magic, amount)); true }
    }

    /// A smeltery of 3 x 3 x 3 with a lava tank in the wall and iron that melts at 800.
    struct World {
        cells: HashMap<Pos, Cell>,
        tanks: BTreeMap<Pos, Option<(Identifier, i32)>>,
        state: (bool, bool),
        marked: HashMap<Pos, bool>,
        alloys: Vec<AlloyRecipe>,
        tags: FluidTags,
        mobs: Mobs,
    }

    fn brick() -> Cell { Cell { in_structure: Some(false), any: true, floor: true, wall: true, ..Cell::default() } }

    impl World {
        fn new() -> Self {
            let mut cells = HashMap::new();
            for x in 0..5 { for z in 0..5 { cells.insert([x, 0, z], brick()); } }
            for y in 1..=3 {
                for i in 1..4 {
                    for pos in [[i, y, 0], [i, y, 4], [0, y, i], [4, y, i]] { cells.insert(pos, brick()); }
                }
            }
            // the controller sits at [2, 2, 0]; a tank in the east wall
            cells.insert([4, 2, 2], Cell { in_structure: Some(false), any: true, tank: true, wall: true, ..Cell::default() });
            let mut tanks = BTreeMap::new();
            tanks.insert([4, 2, 2], Some((id("minecraft:lava"), 1000)));
            Self { cells, tanks, state: (false, false), marked: HashMap::new(), alloys: Vec::new(), tags: FluidTags::default(), mobs: Mobs::default() }
        }
    }

    impl Env for World {
        fn cell(&self, _: Kind, pos: Pos) -> Option<Cell> { Some(self.cells.get(&pos).cloned().unwrap_or(Cell { air: true, ..Cell::default() })) }
        fn mark(&mut self, pos: Pos, in_structure: bool) { self.marked.insert(pos, in_structure); }
        fn block_state(&self) -> (bool, bool) { self.state }
        fn set_block_state(&mut self, in_structure: bool, active: bool) { self.state = (in_structure, active); }
        fn melt(&self, _: Kind, stack: &ItemSlot) -> Option<Melt> {
            (stack.item.path() == "iron_ingot").then(|| Melt { time: 60, temperature: 800, fluid: id("tconstruct:molten_iron"), amount: 90 })
        }
        fn byproducts(&self, _: &ItemSlot) -> Vec<(Identifier, i32)> { vec![(id("tconstruct:molten_gold"), 10)] }
        fn liquid_fuel(&self, fluid: &Identifier) -> Option<LiquidFuel> {
            (fluid.path() == "lava").then_some(LiquidFuel { amount: 50, duration: 100, temperature: 1000, rate: 10 })
        }
        fn alloys(&self) -> &[Alloy] { &self.alloys }
        fn fluid_tags(&self) -> &FluidTags { &self.tags }
        fn tank_fluid(&mut self, pos: Pos) -> Option<(Identifier, i32)> { self.tanks.get(&pos)?.clone() }
        fn tank_drain(&mut self, pos: Pos, _: &Identifier, amount: i32) -> i32 {
            let Some(Some((_, held))) = self.tanks.get_mut(&pos) else { return 0 };
            let taken = amount.min(*held);
            *held -= taken;
            if *held == 0 { self.tanks.insert(pos, None); }
            taken
        }
        fn entity_melting(&self, entity: &Identifier) -> Option<(Identifier, i32, i32)> {
            (entity.path() == "zombie").then(|| (id("tconstruct:molten_iron"), 18, 5))
        }
        fn melting_blacklisted(&self, entity: &Identifier) -> bool { entity.path() == "villager" }
        fn entities(&mut self) -> Option<&mut dyn Entities> { Some(&mut self.mobs) }
        fn random(&mut self, _: usize) -> usize { 0 }
    }

    fn run(controller: &mut Controller, items: &mut Vec<Option<ItemSlot>>, world: &mut World, ticks: i32) {
        for _ in 0..ticks { controller.step(items, world); }
    }

    fn started() -> (Controller, Vec<Option<ItemSlot>>, World) {
        let mut world = World::new();
        let mut controller = Controller::new(Kind::Smeltery, [2, 2, 0], Direction::North);
        let mut items = vec![None; 256];
        // tick 0 queues a check, tick 1 forms the structure and the block says so; the helper ends at the start of the next cycle
        run(&mut controller, &mut items, &mut world, 20);
        assert!(controller.structure.is_some(), "{:?}", controller.last_result());
        assert_eq!(world.state, (true, false));
        (controller, items, world)
    }

    #[test]
    fn a_new_controller_forms_its_structure_and_sizes_its_tank_slots_and_fuel_rate() {
        let (controller, _, world) = started();
        // 3 x 3 x 3: 12 ingots a block, 27 slots, 1 + 45 / 15
        assert_eq!((controller.tank.capacity(), controller.slots.len(), controller.fuel_rate), (29160, 27, 4));
        assert_eq!(world.marked.len(), 45);
        assert!(world.marked.values().all(|joined| *joined));
        assert_eq!(controller.structure.as_ref().unwrap().tanks, vec![[4, 2, 2]]);
    }

    #[test]
    fn an_ingot_melts_on_lava_from_a_tank_and_the_smeltery_drains_it() {
        let (mut controller, mut items, mut world) = started();
        items[0] = Some(ItemSlot::new(id("minecraft:iron_ingot"), 1));
        // tick 0 of the next cycle finds fuel (lava at 1000 beats 800), tick 1 heats, tick 3 burns down by the fuel rate
        run(&mut controller, &mut items, &mut world, 4);
        assert_eq!(world.tanks[&[4, 2, 2]], Some((id("minecraft:lava"), 950)), "50 mB of lava per burn");
        assert_eq!((controller.fuel.temperature, controller.fuel.rate), (1000, 10));
        assert!(controller.fuel.fuel < 100 && controller.fuel.fuel > 0);
        assert_eq!(world.state.1, true, "the controller is active while fuel burns");
        assert!(controller.slots[0].current_time > 0);
        // 600 required at 10 a pass: 60 passes of 4 ticks, one more finishes; the fuel lasts 100 / 4 passes so it takes refuelling
        run(&mut controller, &mut items, &mut world, 4 * 70);
        assert!(items[0].is_none(), "melted");
        assert_eq!(controller.tank.fluids().iter().find(|fluid| fluid.fluid == id("tconstruct:molten_iron")).map(|fluid| fluid.amount), Some(90));
        // the tank on the first tick of the cycle takes fluid in the order it arrived
        assert_eq!(controller.tank.drain(40, true).unwrap().fluid, id("tconstruct:molten_iron"));
    }

    #[test]
    fn the_foundry_adds_byproducts_and_does_not_alloy() {
        let mut world = World::new();
        let mut controller = Controller::new(Kind::Foundry, [2, 2, 0], Direction::North);
        let mut items = vec![None; 256];
        // a foundry wants a frame; the plain wall world has none at the corners, so make it foundry shaped
        for y in 1..=3 { for pos in [[0, y, 0], [4, y, 0], [0, y, 4], [4, y, 4]] { world.cells.insert(pos, brick()); } }
        run(&mut controller, &mut items, &mut world, 20);
        assert!(controller.structure.is_some(), "{:?}", controller.last_result());
        // 8 ingots a block of the walls and floor too: 8 * 90 * 5 * 4 * 5
        assert_eq!(controller.tank.capacity(), 72000);
        items[0] = Some(ItemSlot::new(id("minecraft:iron_ingot"), 1));
        run(&mut controller, &mut items, &mut world, 4 * 75);
        assert!(items[0].is_none());
        let fluids: Vec<_> = controller.tank.fluids().iter().map(|fluid| (fluid.fluid.to_string(), fluid.amount)).collect();
        assert_eq!(fluids, vec![("tconstruct:molten_iron".to_owned(), 90), ("tconstruct:molten_gold".to_owned(), 10)]);
    }

    #[test]
    fn without_a_fuel_tank_the_smeltery_only_cools_and_a_full_tank_holds_the_item() {
        let (mut controller, mut items, mut world) = started();
        world.tanks.insert([4, 2, 2], None);
        items[0] = Some(ItemSlot::new(id("minecraft:iron_ingot"), 1));
        run(&mut controller, &mut items, &mut world, 21);
        assert!(!controller.fuel.has_fuel());
        assert_eq!(controller.slots[0].required_time, 600);
        controller.slots[0].current_time = 20;
        run(&mut controller, &mut items, &mut world, 20);
        assert_eq!(controller.slots[0].current_time, 0, "a partly heated item loses 5 a pass, every 4 ticks");
        // a full tank: lava again, the tank has no room, the slot waits with NO_SPACE
        world.tanks.insert([4, 2, 2], Some((id("minecraft:lava"), 1000)));
        controller.tank.fill(&id("a:filler"), 29100, true);
        run(&mut controller, &mut items, &mut world, 20 + 4 * 62);
        assert_eq!(controller.slots[0].current_time, crate::heating::NO_SPACE);
        assert!(items[0].is_some());
    }

    #[test]
    fn a_broken_wall_is_noticed_within_a_second_and_the_structure_reforms_or_drops() {
        let (mut controller, mut items, mut world) = started();
        run(&mut controller, &mut items, &mut world, 19);
        world.cells.remove(&[0, 2, 2]);
        run(&mut controller, &mut items, &mut world, 30);
        assert!(controller.structure.is_none());
        assert_eq!(world.state, (false, false), "the controller is no longer in a structure and not active");
        assert_eq!(controller.error_pos, Some([0, 2, 2]));
        assert!(world.marked.values().filter(|joined| !**joined).count() > 0, "wall blocks are freed");
        world.cells.insert([0, 2, 2], brick());
        run(&mut controller, &mut items, &mut world, 21);
        assert!(controller.structure.is_some(), "it re-forms within a second of the repair");
    }

    #[test]
    fn items_inside_go_into_slots_and_living_things_are_hurt_for_their_fluid_with_fuel() {
        let (mut controller, mut items, mut world) = started();
        world.mobs.list = vec![
            Entity { id: 1, kind: id("minecraft:item"), item: Some(ItemSlot::new(id("minecraft:iron_ingot"), 2)), living: None },
            Entity { id: 2, kind: id("minecraft:zombie"), item: None, living: Some(Living { fire_immune: false, immune: false }) },
            Entity { id: 3, kind: id("minecraft:villager"), item: None, living: Some(Living { fire_immune: false, immune: false }) },
            Entity { id: 4, kind: id("minecraft:blaze"), item: None, living: Some(Living { fire_immune: true, immune: true }) },
            Entity { id: 5, kind: id("minecraft:cow"), item: None, living: Some(Living { fire_immune: false, immune: false }) },
        ];
        // tick 12 of the cycle: the entity pass
        run(&mut controller, &mut items, &mut world, 13);
        assert_eq!(items[0].as_ref().unwrap().count, 1);
        assert_eq!(items[1].as_ref().unwrap().count, 1);
        assert_eq!(world.mobs.left.len(), 0);
        assert_eq!(world.mobs.discarded, vec![1], "the stack fit whole into two slots");
        assert_eq!(world.mobs.hurt, vec![(2, false, 5), (5, false, 2)], "the zombie by recipe, the cow by default; the villager is blacklisted and the blaze immune");
        let iron = controller.tank.fluids().iter().find(|fluid| fluid.fluid == id("tconstruct:molten_iron")).unwrap().amount;
        assert_eq!(iron, 18);
        assert!(controller.tank.fluids().iter().any(|fluid| fluid.fluid == id("tconstruct:liquid_soul") && fluid.amount == 50));
        assert!(controller.fuel.has_fuel(), "melting something needed fuel");
    }

    #[test]
    fn the_controller_saves_its_structure_tank_fuel_and_melting_progress() {
        let (mut controller, mut items, mut world) = started();
        items[3] = Some(ItemSlot::new(id("minecraft:iron_ingot"), 1));
        controller.tank.fill(&id("tconstruct:molten_copper"), 123, true);
        run(&mut controller, &mut items, &mut world, 9);
        let mut data = NbtCompound::new();
        controller.save(&mut data);
        let loaded = Controller::load(Kind::Smeltery, [2, 2, 0], Direction::North, &data, &items);
        assert_eq!(loaded.structure, controller.structure);
        assert_eq!(loaded.tank.fluids(), controller.tank.fluids());
        assert_eq!((loaded.fuel.fuel, loaded.fuel.temperature, loaded.fuel.rate), (controller.fuel.fuel, 1000, 10));
        assert_eq!(loaded.slots.len(), 27);
        assert_eq!(loaded.slots[3], controller.slots[3]);
        assert!(loaded.slots[3].occupied && loaded.slots[3].current_time > 0);
        assert_eq!(loaded.fuel_rate, 4);
        assert_eq!(loaded.last_tank, Some([4, 2, 2]));
        // a saved slot whose stack is gone has no progress to keep
        let without = Controller::load(Kind::Smeltery, [2, 2, 0], Direction::North, &data, &vec![None; 256]);
        assert!(!without.slots[3].occupied);
    }

    #[test]
    fn alloys_run_on_the_third_tick_with_the_fuel_temperature() {
        let (mut controller, mut items, mut world) = started();
        let fields = serde_json::json!({"type": "tconstruct:alloy", "temperature": 900,
            "inputs": [{"amount": 30, "fluid": "tconstruct:molten_copper"}, {"amount": 10, "fluid": "tconstruct:molten_tin"}],
            "result": {"amount": 40, "fluid": "tconstruct:molten_bronze"}});
        world.alloys = vec![AlloyRecipe::decode(&id("tconstruct:bronze"), fields.as_object().unwrap()).unwrap()];
        controller.tank.fill(&id("tconstruct:molten_copper"), 90, true);
        controller.tank.fill(&id("tconstruct:molten_tin"), 30, true);
        // an alloy needs fuel too: tick 0 sees the alloy can run and burns fuel, tick 2 mixes
        run(&mut controller, &mut items, &mut world, 3);
        let amount = |controller: &Controller, name: &str| controller.tank.fluids().iter().find(|fluid| fluid.fluid == id(name)).map(|fluid| fluid.amount);
        assert_eq!(amount(&controller, "tconstruct:molten_bronze"), Some(40));
        assert_eq!(amount(&controller, "tconstruct:molten_copper"), Some(60));
    }
}
