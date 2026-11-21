//! The melter's block entity logic, from SlimeKnights MelterBlockEntity, MeltingModuleInventory, MeltingModule,
//! FuelModule and SolidFuelModule (MIT; see LICENSE): three item slots that heat into one tank, fuel found in the
//! block below, and the 4-tick cycle (tick 0 finds fuel, tick 2 heats or cools). `heating.rs` holds the slot and fuel
//! state machines; this ties them to the item slots, the tank, the recipes and the neighbour below.
//! The block states follow the original: `in_structure` is the block behaviour's (the `fuel_tanks` tag below,
//! in `melter_blocks`), and this cycle drives `active` — its own and the fuel tank below's — at its third tick
//! while formed, through the ticking world the host opens around a ticker (`Surroundings::set_active`).
use crate::{
    fluid_values::OreRate,
    heating::{Finished, Fuel, MeltingSlot, MELTER_CAPACITY, MELTER_SLOTS},
    smeltery_data::Data,
    tool_materials::part_material,
    vanilla_fuels,
};
use pluto_lib::{BlockEntityTick, BlockPos, Direction, FluidRegistry, FluidStack, Identifier, ItemSlot, Nbt, NbtCompound, NbtCompoundExt, Neighbours, World};

pub(crate) const SLOTS: usize = MELTER_SLOTS as usize;
pub(crate) const MELTING_TAG: &str = "melting";
/// The block tag a fuel source below the melter must be in (`TinkerTags.Blocks.FUEL_TANKS`).
const FUEL_TANKS: &str = "tconstruct:fuel_tanks";

/// What an item in a slot melts into, from the recipe that matches it (IMeltingRecipe: time, temperature, getOutput).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Melt { pub time: i32, pub temperature: i32, pub fluid: Identifier, pub amount: i32 }

/// A liquid fuel recipe as one burn uses it (MeltingFuel).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LiquidFuel { pub amount: i32, pub duration: i32, pub temperature: i32, pub rate: i32 }

/// The shipped solid fuel recipe: the temperature and rate every burned item gets (MeltingFuelLookup.getSolid).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SolidFuel { pub temperature: i32, pub rate: i32 }

/// The container or tank the melter takes fuel from (SolidFuelModule's `fuelPos`).
pub(crate) trait FuelSource {
    /// The fluid in its first tank, if it holds any (`getFluidInTank(0)`).
    fn fluid(&mut self) -> Option<FluidStack>;
    /// Drains up to `amount`; returns how much came out.
    fn drain(&mut self, amount: i32) -> i32;
    /// The slots whose stack can be taken, with the stack.
    fn items(&mut self) -> Vec<(usize, ItemSlot)>;
    /// Takes one item out of `slot`.
    fn take(&mut self, slot: usize) -> Option<ItemSlot>;
    /// Puts a container back where the item came from; what did not fit comes back.
    fn give_back(&mut self, stack: ItemSlot) -> Option<ItemSlot>;
}

/// Everything around the melter that the cycle asks about.
pub(crate) trait Surroundings {
    fn melt(&self, stack: &ItemSlot) -> Option<Melt>;
    fn liquid_fuel(&self, fluid: &Identifier) -> Option<LiquidFuel>;
    fn solid_fuel(&self) -> Option<SolidFuel>;
    /// The vanilla burn time of an item in ticks, 0 for none (ForgeHooks.getBurnTime).
    fn burn_ticks(&self, item: &Identifier) -> i32;
    /// The item a burned fuel item leaves behind (a lava bucket's bucket).
    fn remainder(&self, item: &Identifier) -> Option<Identifier>;
    /// The container or tank the fuel comes from; none where the block below holds neither (the fuel module
    /// then finds no handlers, whatever the structure state).
    fn source(&mut self) -> Option<&mut dyn FuelSource>;
    /// Whether the melter is in its tiny structure: the block below is in the `fuel_tanks` tag, which the
    /// block's `in_structure` state already says (`MelterBlockEntity.isFormed`).
    fn formed(&self) -> bool;
    /// `MelterBlockEntity.tick`'s block update: the melter's `active` state takes `active`, and so does the
    /// block below while it is a fuel tank with an `active` state of its own (a heater lights with its melter).
    fn set_active(&mut self, active: bool);
}

/// The output tank (FluidTankAnimated over the melter's capacity).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Tank { pub fluid: Option<FluidStack>, pub capacity: i32 }

impl Tank {
    pub(crate) fn read(tick: &BlockEntityTick<'_>) -> Self { Tank { fluid: tick.fluid(0), capacity: MELTER_CAPACITY } }
    pub(crate) fn write(&self, tick: &mut BlockEntityTick<'_>) { tick.set_fluid(0, self.fluid.clone()); }

    /// MeltingModuleInventory.tryFillTank: the whole amount goes in, or nothing does.
    pub(crate) fn try_fill(&mut self, fluid: &Identifier, amount: i32) -> bool {
        if amount <= 0 { return true; }
        match &mut self.fluid {
            None => {
                if amount > self.capacity { return false; }
                self.fluid = Some(FluidStack { fluid: fluid.clone(), amount: amount as u32, tag: None });
                true
            }
            Some(held) if &held.fluid == fluid => {
                if held.amount as i64 + amount as i64 > self.capacity as i64 { return false; }
                held.amount += amount as u32;
                true
            }
            Some(_) => false,
        }
    }
}

/// What a slot's item was, ignoring how many: a changed item restarts the slot (ItemHandlerHelper.canItemStacksStack).
type Key = (Identifier, u32, NbtCompound);
fn key_of(stack: &ItemSlot) -> Key { (stack.item.clone(), stack.damage, stack.custom_data.clone()) }

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct State {
    pub slots: [MeltingSlot; SLOTS],
    keys: [Option<Key>; SLOTS],
    pub fuel: Fuel,
    pub tick: i32,
}

impl State {
    /// Reads the saved data. A stack that is in a slot is the stack the saved progress belongs to.
    pub(crate) fn load(data: &NbtCompound, items: &[Option<ItemSlot>]) -> Self {
        let mut state = State::default();
        let saved = data.get(MELTING_TAG).and_then(Nbt::as_list).unwrap_or(&[]);
        for index in 0..SLOTS {
            let Some(stack) = items.get(index).and_then(Option::as_ref) else { continue };
            let saved_slot = saved.get(index).and_then(Nbt::as_compound);
            // The progress belongs to the stack that was in the slot when it was saved: a different one (put in by a click, a command or
            // a hopper since the last tick) is left unsynchronised, so the first pass sets it up (MeltingModule.setStack).
            let saved_key = saved_slot.and_then(|slot| Some((Identifier::parse(slot.get("key_item")?.as_str()?).ok()?, slot.get_int("key_damage", 0) as u32, slot.get("key_data")?.as_compound()?.clone())));
            if saved_key.as_ref() != Some(&key_of(stack)) { continue; }
            state.keys[index] = Some(key_of(stack));
            state.slots[index].occupied = true;
            if let Some(slot) = saved_slot {
                state.slots[index].current_time = slot.get_int("time", 0) as i32;
                state.slots[index].required_time = slot.get_int("required", 0) as i32;
                state.slots[index].required_temp = slot.get_int("temp", 0) as i32;
            }
        }
        state.fuel = Fuel {
            fuel: data.get_int("fuel", 0) as i32,
            quality: data.get_int("quality", 0) as i32,
            temperature: data.get_int("temperature", 0) as i32,
            rate: data.get_int("rate", 0) as i32,
        };
        state.tick = (data.get_int("tick", 0) as i32).rem_euclid(4);
        state
    }

    pub(crate) fn save(&self, data: &mut NbtCompound) {
        let slots = self.slots.iter().enumerate().map(|(index, slot)| {
            let mut tag = NbtCompound::new();
            tag.insert("time".to_owned(), Nbt::Int(slot.current_time));
            tag.insert("required".to_owned(), Nbt::Int(slot.required_time));
            tag.insert("temp".to_owned(), Nbt::Int(slot.required_temp));
            if let Some((item, damage, custom)) = &self.keys[index] {
                tag.insert("key_item".to_owned(), Nbt::String(item.to_string()));
                tag.insert("key_damage".to_owned(), Nbt::Int(*damage as i32));
                tag.insert("key_data".to_owned(), Nbt::Compound(custom.clone()));
            }
            Nbt::Compound(tag)
        }).collect();
        data.insert(MELTING_TAG.to_owned(), Nbt::List(slots));
        for (key, value) in [("fuel", self.fuel.fuel), ("quality", self.fuel.quality), ("temperature", self.fuel.temperature), ("rate", self.fuel.rate), ("tick", self.tick)] {
            data.insert(key.to_owned(), Nbt::Int(value));
        }
    }

    /// MeltingModule.setStack for every slot whose item changed since the last pass, however it got there (a click, a hopper).
    fn sync(&mut self, items: &[Option<ItemSlot>], env: &dyn Surroundings) {
        for index in 0..SLOTS {
            let stack = items.get(index).and_then(Option::as_ref);
            let key = stack.map(key_of);
            if key == self.keys[index] { continue; }
            self.slots[index].set_stack(stack.map(|stack| (env.melt(stack).map(|melt| (melt.time, melt.temperature)), false)));
            self.keys[index] = key;
        }
    }

    /// MeltingModuleInventory.canHeat.
    fn can_heat(&mut self, temperature: i32) -> bool { self.slots.iter_mut().any(|slot| slot.can_heat(temperature)) }

    /// MeltingModule.onItemFinishedHeating for slot `index`: the recipe's fluid into the tank, or no room.
    fn finish(stack: Option<&ItemSlot>, tank: &mut Tank, env: &dyn Surroundings) -> Finished {
        let Some(melt) = stack.and_then(|stack| env.melt(stack)) else { return Finished::NoRecipe };
        if tank.try_fill(&melt.fluid, melt.amount) { Finished::Filled } else { Finished::NoSpace }
    }

    fn heat(&mut self, items: &mut [Option<ItemSlot>], tank: &mut Tank, env: &dyn Surroundings) {
        let (temperature, rate) = (self.fuel.temperature, self.fuel.rate);
        for index in 0..SLOTS {
            let stack = items.get(index).and_then(Option::as_ref).cloned();
            let mut emptied = false;
            self.slots[index].heat(temperature, rate, &mut || {
                let result = Self::finish(stack.as_ref(), tank, env);
                emptied |= result == Finished::Filled;
                result
            });
            if emptied { self.consume(items, index, env); }
        }
    }

    fn cool(&mut self, items: &mut [Option<ItemSlot>], tank: &mut Tank, env: &dyn Surroundings) {
        for index in 0..SLOTS {
            let stack = items.get(index).and_then(Option::as_ref).cloned();
            let mut emptied = false;
            self.slots[index].cool(&mut || {
                let result = Self::finish(stack.as_ref(), tank, env);
                emptied |= result == Finished::Filled;
                result
            });
            if emptied { self.consume(items, index, env); }
        }
    }

    /// One item of the slot has melted. The original's slot limit is 1 (MeltingModuleInventory.getSlotLimit),
    /// so its setStack(EMPTY) removes exactly that one item; a stack the host let in melts one item at a
    /// time and the rest stay, starting over as a fresh insert would (setStack re-runs its setup).
    fn consume(&mut self, items: &mut [Option<ItemSlot>], index: usize, env: &dyn Surroundings) {
        let Some(slot) = items.get_mut(index) else { return };
        let Some(stack) = slot.as_mut() else { self.keys[index] = None; return; };
        stack.count = stack.count.saturating_sub(1);
        if stack.count == 0 {
            *slot = None;
            self.keys[index] = None;
            return;
        }
        let next = stack.clone();
        self.slots[index].set_stack(Some((env.melt(&next).map(|melt| (melt.time, melt.temperature)), false)));
        self.keys[index] = Some(key_of(&next));
    }

    /// FuelModule.findFuel(consume): liquid fuel first (it usually goes hotter), then solid. Returns the temperature found, 0 for none.
    pub(crate) fn find_fuel(&mut self, consume: bool, env: &mut dyn Surroundings) -> i32 {
        let mut temperature = 0;
        if let Some(stack) = env.source().and_then(|source| source.fluid()) {
            if let Some(recipe) = env.liquid_fuel(&stack.fluid) {
                let available = stack.amount.min(i32::MAX as u32) as i32;
                let burn = consume && available >= recipe.amount;
                temperature = self.fuel.try_liquid(available, recipe.amount, recipe.duration, recipe.temperature, recipe.rate, burn);
                if burn && temperature > 0 {
                    if let Some(source) = env.source() { source.drain(recipe.amount); }
                }
            }
        }
        if temperature == 0 { temperature = self.try_solid(consume, env); }
        if temperature == 0 && consume { self.fuel.none_found(); }
        temperature
    }

    /// SolidFuelModule.trySolidFuel: the first slot of the source holding an item that burns.
    fn try_solid(&mut self, consume: bool, env: &mut dyn Surroundings) -> i32 {
        let Some(solid) = env.solid_fuel() else { return 0 };
        let candidates = env.source().map(|source| source.items()).unwrap_or_default();
        for (slot, stack) in candidates {
            let burn = env.burn_ticks(&stack.item);
            if burn / 4 <= 0 { continue; }
            if !consume { return self.fuel.try_solid(burn, solid.temperature, solid.rate, false); }
            let taken = env.source().and_then(|source| source.take(slot));
            if taken.as_ref().is_some_and(|taken| taken.item == stack.item) {
                self.fuel.try_solid(burn, solid.temperature, solid.rate, true);
                // return the container (a lava bucket's bucket); what the source cannot take back is lost, the host cannot drop it from here
                if let Some(container) = env.remainder(&stack.item) {
                    if let Some(source) = env.source() { source.give_back(ItemSlot::new(container, 1)); }
                }
            } else {
                eprintln!("Tinkers: invalid item removed from the melter's fuel source");
            }
            return solid.temperature;
        }
        0
    }

    /// MelterBlockEntity.tick, one game tick: the cycle only runs in a structure (the fuel tank below, by
    /// tag); outside one, the third tick still burns stored fuel down or cools the slots, but nothing is
    /// heated and no new fuel is found.
    pub(crate) fn step(&mut self, items: &mut [Option<ItemSlot>], tank: &mut Tank, env: &mut dyn Surroundings) {
        self.sync(items, env);
        let formed = env.formed();
        match self.tick {
            0 if formed => {
                if !self.fuel.has_fuel() {
                    let found = self.find_fuel(false, env);
                    if self.can_heat(found) { self.find_fuel(true, env); }
                }
            }
            2 => {
                let has_fuel = self.fuel.has_fuel();
                if formed {
                    env.set_active(has_fuel);
                    if has_fuel {
                        self.heat(items, tank, env);
                        self.fuel.decrease(1);
                    } else {
                        self.cool(items, tank, env);
                    }
                } else if has_fuel {
                    self.fuel.decrease(1);
                } else {
                    self.cool(items, tank, env);
                }
            }
            _ => {}
        }
        self.tick = (self.tick + 1) % 4;
    }
}

/// The synchronized menu values: 0 fuel, 1 fuel quality, 2 temperature, 3 rate, then four per slot (current time, required time,
/// required temperature, and 1 when an item is in it), then the tank's amount and which defined fluid it holds (one more than its
/// position in `fluid_order`, 0 for none). The original syncs the three times per slot; the occupied flag is what lets the window
/// leave an empty slot without a heat bar.
pub(crate) const fn slot_data(slot: usize) -> usize { 4 + 4 * slot }
pub(crate) const DATA_TANK_AMOUNT: usize = slot_data(SLOTS);
pub(crate) const DATA_TANK_FLUID: usize = DATA_TANK_AMOUNT + 1;
pub(crate) const DATA_SLOTS: usize = DATA_TANK_FLUID + 1;

/// Every fluid a mod defined through PlutoLib, in id order: both halves of a game hold the same list, so a position names a fluid.
pub(crate) fn fluid_order() -> Vec<Identifier> {
    let mut ids: Vec<Identifier> = FluidRegistry::registered().into_iter().map(|def| def.id).collect();
    ids.sort();
    ids
}

impl State {
    pub(crate) fn publish(&self, tick: &mut BlockEntityTick<'_>, tank: &Tank) {
        let values = [self.fuel.fuel, self.fuel.quality, self.fuel.temperature, self.fuel.rate];
        for (index, value) in values.into_iter().enumerate() { tick.set_menu_data(index, value); }
        for (index, slot) in self.slots.iter().enumerate() {
            let base = slot_data(index);
            tick.set_menu_data(base, slot.current_time);
            tick.set_menu_data(base + 1, slot.required_time);
            tick.set_menu_data(base + 2, slot.required_temp);
            tick.set_menu_data(base + 3, i32::from(slot.occupied));
        }
        tick.set_menu_data(DATA_TANK_AMOUNT, tank.fluid.as_ref().map_or(0, |fluid| fluid.amount as i32));
        let which = tank.fluid.as_ref().and_then(|fluid| fluid_order().iter().position(|id| *id == fluid.fluid)).map_or(0, |position| position as i32 + 1);
        tick.set_menu_data(DATA_TANK_FLUID, which);
    }
}

/// The vanilla burn time of an item in ticks (0 for none): the item must burn for more than 3 ticks to be fuel, as the quarter is what counts.
pub(crate) fn burn_time(item: &Identifier) -> i32 {
    let name = item.to_string();
    vanilla_fuels::FUELS.binary_search_by(|(own, _)| (*own).cmp(name.as_str())).map_or(0, |position| vanilla_fuels::FUELS[position].1)
}

/// HeaterItemHandler.isItemValid: only items whose fuel quarter is positive.
pub(crate) fn heater_accepts(item: &str) -> bool {
    Identifier::parse(item).is_ok_and(|item| burn_time(&item) > 3)
}

/// The vanilla container a burned fuel item leaves (`Item.getCraftingRemainingItem`): only the lava bucket is a vanilla fuel with one.
pub(crate) fn fuel_remainder(item: &Identifier) -> Option<Identifier> {
    (item.to_string() == "minecraft:lava_bucket").then(|| Identifier::parse("minecraft:bucket").expect("valid id"))
}

/// The neighbour below, reached through PlutoLib's neighbour queries on its top face.
pub(crate) struct Below { pos: BlockPos, face: Direction }

impl Below {
    /// None where neither a container nor a tank answers.
    pub(crate) fn of(pos: BlockPos) -> Option<Below> {
        let (below, face) = Neighbours::beside(pos, Direction::Down);
        (Neighbours::item_slots(below, face).is_some() || Neighbours::fluid_tanks(below, face).is_some()).then_some(Below { pos: below, face })
    }
}

impl FuelSource for Below {
    fn fluid(&mut self) -> Option<FluidStack> { Neighbours::fluid_tanks(self.pos, self.face)?.into_iter().next()?.fluid }
    fn drain(&mut self, amount: i32) -> i32 { Neighbours::drain_fluid(self.pos, self.face, amount.max(0) as u32).map_or(0, |drained| drained.amount as i32) }
    fn items(&mut self) -> Vec<(usize, ItemSlot)> {
        Neighbours::item_slots(self.pos, self.face).unwrap_or_default().into_iter()
            .filter_map(|slot| if slot.can_take { slot.stack.map(|stack| (slot.slot, stack)) } else { None }).collect()
    }
    fn take(&mut self, slot: usize) -> Option<ItemSlot> { Neighbours::extract_item(self.pos, self.face, slot, 1) }
    fn give_back(&mut self, stack: ItemSlot) -> Option<ItemSlot> { Neighbours::insert_item(self.pos, self.face, stack) }
}

/// The live surroundings of a ticking melter: the loaded recipes and tags, the item tags, and what is below.
pub(crate) struct Live<'a> {
    pub data: &'a Data,
    pub item_tags: Option<&'a pluto_lib::RegistryLookup>,
    pub pos: BlockPos,
    below: Option<Option<Below>>,
}

impl<'a> Live<'a> {
    pub(crate) fn new(data: &'a Data, item_tags: Option<&'a pluto_lib::RegistryLookup>, pos: BlockPos) -> Self { Live { data, item_tags, pos, below: None } }
}

/// What the item melts into in a container with the given ore rate, and the byproducts a foundry also gets (`byproduct_rate`).
pub(crate) fn melt_stack(data: &Data, item_tags: Option<&pluto_lib::RegistryLookup>, stack: &ItemSlot, ore_rate: &OreRate, byproduct_rate: &OreRate) -> Option<(Melt, Vec<(Identifier, i32)>)> {
    let recipes = &data.recipes;
    let in_tag = |item: &Identifier, tag: &Identifier| item_tags.and_then(|tags| tags.get_tag(tag)).is_some_and(|members| members.contains(item));
    if let Some(recipe) = recipes.find_melting(&stack.item, &in_tag, false) {
        // A damageable item melts by what is left of it; with no maximum to scale by, only an undamaged one melts.
        let damage = match (stack.damage, stack.max_damage) {
            (_, Some(max)) => Some((stack.damage as i32, max)),
            (0, None) => None,
            (_, None) if matches!(recipe.kind, crate::melting_recipes::MeltingKind::Damageable { .. }) => return None,
            _ => None,
        };
        let amount = recipe.output_amount(ore_rate, damage);
        let byproducts = recipe.byproducts.iter().zip(recipe.byproduct_amounts(byproduct_rate, damage))
            .filter_map(|(output, amount)| Some((output.resolve(&data.tags)?, amount))).collect();
        return Some((Melt { time: recipe.time, temperature: recipe.temperature, fluid: recipe.output.resolve(&data.tags)?, amount }, byproducts));
    }
    let material = part_material(&stack.custom_data);
    let (recipe, cost) = recipes.find_material_melting(&stack.item, &material)?;
    let byproducts = recipe.byproducts.iter().zip(recipe.byproduct_amounts(cost)).filter_map(|(output, amount)| Some((output.resolve(&data.tags)?, amount))).collect();
    Some((Melt { time: recipe.time(cost), temperature: recipe.temperature, fluid: recipe.result.resolve(&data.tags)?, amount: recipe.output_amount(cost) }, byproducts))
}

impl Surroundings for Live<'_> {
    fn melt(&self, stack: &ItemSlot) -> Option<Melt> {
        // Config.Common.melterOreRate, read per melt as MelterBlockEntity reads .get() per use.
        let rates = crate::config::ore_rates();
        melt_stack(self.data, self.item_tags, stack, &rates.melter, &rates.foundry_byproduct).map(|(melt, _)| melt)
    }

    fn liquid_fuel(&self, fluid: &Identifier) -> Option<LiquidFuel> {
        let in_tag = |fluid: &Identifier, tag: &Identifier| self.data.tags.contains(fluid, tag);
        let fuel = self.data.recipes.find_fuel(fluid, &in_tag)?;
        Some(LiquidFuel { amount: fuel.amount(fluid, &in_tag), duration: fuel.duration, temperature: fuel.temperature, rate: fuel.rate })
    }

    fn solid_fuel(&self) -> Option<SolidFuel> { self.data.recipes.solid_fuel.as_ref().map(|fuel| SolidFuel { temperature: fuel.temperature, rate: fuel.rate }) }
    fn burn_ticks(&self, item: &Identifier) -> i32 { burn_time(item) }
    fn remainder(&self, item: &Identifier) -> Option<Identifier> { fuel_remainder(item) }

    fn source(&mut self) -> Option<&mut dyn FuelSource> {
        let pos = self.pos;
        self.below.get_or_insert_with(|| Below::of(pos)).as_mut().map(|below| below as &mut dyn FuelSource)
    }

    fn formed(&self) -> bool {
        // an explicit `in_structure` decides; where no state can be read — the pure fixtures, which drive
        // this env with no ticking world at all — the melter is the formed one over its fuel source. In a
        // live ticker the host always opens the world scope, so the state is always there to read.
        World::ticking().and_then(|world| world.get_block_state(self.pos))
            .and_then(|state| state.property("in_structure").map(|value| value == "true"))
            .unwrap_or(true)
    }

    fn set_active(&mut self, active: bool) {
        let flag = if active { "true" } else { "false" };
        let Some(world) = World::ticking() else { return };
        if let Some(state) = world.get_block_state(self.pos) {
            if state.property("active") != Some(flag) { world.set_block_state(self.pos, &state.with("active", flag)); }
        }
        // the block below shows the same flame state while it is a fuel tank (MelterBlockEntity.tick)
        let (below, _) = Neighbours::beside(self.pos, Direction::Down);
        if let Some(state) = world.get_block_state(below) {
            if state.is_in_tag(FUEL_TANKS) && state.property("active").is_some_and(|value| value != flag) {
                world.set_block_state(below, &state.with("active", flag));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }

    struct Source { fluid: Option<FluidStack>, items: Vec<Option<ItemSlot>> }
    impl FuelSource for Source {
        fn fluid(&mut self) -> Option<FluidStack> { self.fluid.clone() }
        fn drain(&mut self, amount: i32) -> i32 {
            let Some(held) = self.fluid.as_mut() else { return 0 };
            let taken = (amount as u32).min(held.amount);
            held.amount -= taken;
            if held.amount == 0 { self.fluid = None; }
            taken as i32
        }
        fn items(&mut self) -> Vec<(usize, ItemSlot)> { self.items.iter().enumerate().filter_map(|(slot, stack)| stack.clone().map(|stack| (slot, stack))).collect() }
        fn take(&mut self, slot: usize) -> Option<ItemSlot> {
            let held = self.items.get_mut(slot)?.as_mut()?;
            let mut one = held.clone();
            one.count = 1;
            held.count -= 1;
            if held.count == 0 { self.items[slot] = None; }
            Some(one)
        }
        fn give_back(&mut self, stack: ItemSlot) -> Option<ItemSlot> {
            match self.items.iter_mut().find(|slot| slot.is_none()) { Some(slot) => { *slot = Some(stack); None } None => Some(stack) }
        }
    }

    /// Iron ingots melt (60, 800 C, 90 mB of molten iron); lava and the solid fuel burn as the shipped recipes say.
    /// `formed` is the structure state the block reports; `active` records every block update the cycle asks for.
    struct World { source: Option<Source>, formed: bool, active: Vec<bool> }
    impl Surroundings for World {
        fn melt(&self, stack: &ItemSlot) -> Option<Melt> {
            (stack.item.path() == "iron_ingot").then(|| Melt { time: 60, temperature: 800, fluid: id("tconstruct:molten_iron"), amount: 90 })
        }
        fn liquid_fuel(&self, fluid: &Identifier) -> Option<LiquidFuel> {
            (fluid.path() == "lava").then_some(LiquidFuel { amount: 50, duration: 100, temperature: 1000, rate: 10 })
        }
        fn solid_fuel(&self) -> Option<SolidFuel> { Some(SolidFuel { temperature: 800, rate: 8 }) }
        fn burn_ticks(&self, item: &Identifier) -> i32 { burn_time(item) }
        fn remainder(&self, item: &Identifier) -> Option<Identifier> { fuel_remainder(item) }
        fn source(&mut self) -> Option<&mut dyn FuelSource> { self.source.as_mut().map(|source| source as &mut dyn FuelSource) }
        fn formed(&self) -> bool { self.formed }
        fn set_active(&mut self, active: bool) { self.active.push(active); }
    }

    fn ingot() -> Option<ItemSlot> { Some(ItemSlot::new(id("minecraft:iron_ingot"), 1)) }
    fn run(state: &mut State, items: &mut Vec<Option<ItemSlot>>, tank: &mut Tank, world: &mut World, ticks: i32) {
        for _ in 0..ticks { state.step(items, tank, world); }
    }

    #[test]
    fn solid_fuel_in_the_block_below_melts_an_ingot_in_the_cycle_the_original_runs() {
        let mut world = World { source: Some(Source { fluid: None, items: vec![Some(ItemSlot::new(id("minecraft:coal"), 2))] }), formed: true, active: Vec::new() };
        let mut items = vec![ingot(), None, None];
        let (mut state, mut tank) = (State::default(), Tank { fluid: None, capacity: MELTER_CAPACITY });
        // tick 0 finds the fuel (an ingot can heat at the solid fuel's 800 C), tick 2 heats once
        run(&mut state, &mut items, &mut tank, &mut world, 3);
        assert_eq!((state.fuel.fuel, state.fuel.temperature, state.fuel.rate), (400 - 1, 800, 8), "one coal is a quarter of 1600 burn ticks, minus the pass that just ran");
        assert_eq!(world.source.as_ref().unwrap().items[0].as_ref().unwrap().count, 1, "the coal was taken from the block below");
        assert_eq!(state.slots[0].current_time, 8, "the first heating pass adds the fuel's rate");
        // 600 required at rate 8: 75 passes reach it and one more finishes, 4 ticks each, the first already done
        run(&mut state, &mut items, &mut tank, &mut world, 4 * 75 - 1 + 1);
        assert!(items[0].is_none(), "the finished ingot leaves the slot");
        assert_eq!(tank.fluid, Some(FluidStack { fluid: id("tconstruct:molten_iron"), amount: 90, tag: None }));
    }

    #[test]
    fn a_stack_in_a_slot_melts_one_item_at_a_time_and_keeps_the_rest() {
        // The original's slot limit is 1, so a stack only exists when the host put it there directly;
        // it melts one item at a time (the recipe's output is a fixed amount, never scaled by count).
        let mut world = World { source: Some(Source { fluid: None, items: vec![Some(ItemSlot::new(id("minecraft:coal"), 2))] }), formed: true, active: Vec::new() };
        let mut items = vec![Some(ItemSlot::new(id("minecraft:iron_ingot"), 4)), None, None];
        let (mut state, mut tank) = (State::default(), Tank { fluid: None, capacity: MELTER_CAPACITY });
        // one ingot is 76 heating passes of 4 ticks (75 to reach 600 at rate 8, one to finish); run past the first
        run(&mut state, &mut items, &mut tank, &mut world, 4 * 80);
        assert_eq!(tank.fluid, Some(FluidStack { fluid: id("tconstruct:molten_iron"), amount: 90, tag: None }), "one ingot's worth, not the stack's");
        assert_eq!(items[0].as_ref().unwrap().count, 3, "the rest of the stack stays in the slot");
        // the rest follow one at a time: three more ingots at 76 passes each
        run(&mut state, &mut items, &mut tank, &mut world, 4 * 80 * 3);
        assert_eq!(tank.fluid, Some(FluidStack { fluid: id("tconstruct:molten_iron"), amount: 360, tag: None }));
        assert!(items[0].is_none(), "the last ingot leaves the slot empty");
    }

    #[test]
    fn liquid_fuel_burns_before_solid_fuel_and_a_full_tank_holds_the_item_until_there_is_room() {
        let lava = FluidStack { fluid: id("minecraft:lava"), amount: 100, tag: None };
        let mut world = World { source: Some(Source { fluid: Some(lava), items: vec![Some(ItemSlot::new(id("minecraft:coal"), 1))] }), formed: true, active: Vec::new() };
        let mut items = vec![ingot(), None, None];
        let mut tank = Tank { fluid: Some(FluidStack { fluid: id("tconstruct:molten_iron"), amount: MELTER_CAPACITY as u32 - 80, tag: None }), capacity: MELTER_CAPACITY };
        let mut state = State::default();
        run(&mut state, &mut items, &mut tank, &mut world, 3);
        assert_eq!((state.fuel.temperature, state.fuel.rate), (1000, 10), "lava is tried first");
        assert_eq!(world.source.as_ref().unwrap().fluid.as_ref().unwrap().amount, 50);
        assert_eq!(world.source.as_ref().unwrap().items[0].as_ref().unwrap().count, 1, "the coal stays");
        // 60 passes of rate 10 reach the time, the next finishes: the 90 mB do not fit beside the iron already in the tank
        run(&mut state, &mut items, &mut tank, &mut world, 4 * 61);
        assert_eq!(state.slots[0].current_time, crate::heating::NO_SPACE);
        assert!(items[0].is_some());
        // room appears (somebody drained the tank): cooling or the next heating pass retries and fills it
        tank.fluid = None;
        run(&mut state, &mut items, &mut tank, &mut world, 4);
        assert!(items[0].is_none());
        assert_eq!(tank.fluid.as_ref().unwrap().amount, 90);
    }

    #[test]
    fn without_a_fuel_block_the_melter_only_cools_and_a_replaced_item_restarts() {
        let mut world = World { source: None, formed: false, active: Vec::new() };
        let mut items = vec![ingot(), None, None];
        let (mut state, mut tank) = (State::default(), Tank { fluid: None, capacity: MELTER_CAPACITY });
        run(&mut state, &mut items, &mut tank, &mut world, 1);
        assert_eq!((state.slots[0].required_time, state.slots[0].required_temp), (600, 800), "the slot learned its recipe");
        state.slots[0].current_time = 20;
        run(&mut state, &mut items, &mut tank, &mut world, 4);
        assert_eq!(state.slots[0].current_time, 15, "no fuel: a partly heated item loses 5 a pass");
        assert!(!state.fuel.has_fuel());
        // another kind of item in the slot starts over; an unmeltable one has no recipe
        items[0] = Some(ItemSlot::new(id("minecraft:stick"), 1));
        run(&mut state, &mut items, &mut tank, &mut world, 1);
        assert_eq!((state.slots[0].current_time, state.slots[0].required_time), (0, 0));
        items[0] = None;
        run(&mut state, &mut items, &mut tank, &mut world, 1);
        assert!(!state.slots[0].occupied);
    }

    #[test]
    fn state_survives_a_save_and_the_menu_publishes_what_the_screen_draws() {
        let mut world = World { source: Some(Source { fluid: None, items: vec![Some(ItemSlot::new(id("minecraft:coal"), 1))] }), formed: true, active: Vec::new() };
        let mut items = vec![ingot(), None, None];
        let (mut state, mut tank) = (State::default(), Tank { fluid: Some(FluidStack { fluid: id("tconstruct:molten_iron"), amount: 180, tag: None }), capacity: MELTER_CAPACITY });
        run(&mut state, &mut items, &mut tank, &mut world, 7);
        let mut data = NbtCompound::new();
        let mut copy = items.clone();
        let mut tick = BlockEntityTick { pos: [0, 64, 0], data: &mut data, items: &mut copy };
        state.save(tick.data);
        tank.write(&mut tick);
        state.publish(&mut tick, &tank);
        let loaded = State::load(tick.data, tick.items);
        assert_eq!(loaded, state);
        assert_eq!(Tank::read(&tick), tank);
        let menu = match tick.data.get("pluto:menu_data") { Some(Nbt::IntArray(values)) => values.clone(), _ => panic!("no menu data") };
        assert_eq!(menu.len(), DATA_SLOTS);
        assert_eq!((menu[0], menu[2], menu[3]), (state.fuel.fuel, 800, 8));
        assert_eq!((menu[4], menu[5], menu[6], menu[7]), (state.slots[0].current_time, 600, 800, 1));
        assert_eq!(menu[DATA_TANK_AMOUNT], 180);
    }

    #[test]
    fn an_unformed_melter_finds_no_fuel_and_does_not_heat_even_over_a_container() {
        // the block below holds fuel but is no fuel tank (a chest, say): the tiny structure is not formed, so
        // the cycle never runs — the original gates everything but cooling and fuel burn-down on isFormed
        let mut world = World { source: Some(Source { fluid: None, items: vec![Some(ItemSlot::new(id("minecraft:coal"), 2))] }), formed: false, active: Vec::new() };
        let mut items = vec![ingot(), None, None];
        let (mut state, mut tank) = (State::default(), Tank { fluid: None, capacity: MELTER_CAPACITY });
        run(&mut state, &mut items, &mut tank, &mut world, 8);
        assert!(!state.fuel.has_fuel(), "no fuel is found outside a structure");
        assert_eq!(world.source.as_ref().unwrap().items[0].as_ref().unwrap().count, 2, "the coal below is untouched");
        assert_eq!(state.slots[0].current_time, 0, "nothing heats");
        assert!(world.active.is_empty(), "the active state is the formed cycle's to drive");
        // fuel already burning still burns down at the third tick, without heating
        state.fuel.fuel = 40;
        state.fuel.temperature = 800;
        state.fuel.rate = 8;
        run(&mut state, &mut items, &mut tank, &mut world, 4);
        assert_eq!(state.fuel.fuel, 39, "one burn-down at tick 2 of the cycle");
        assert_eq!(state.slots[0].current_time, 0, "still nothing heats");
    }

    #[test]
    fn a_formed_melter_reports_its_active_state_at_the_third_tick_of_the_cycle() {
        let mut world = World { source: None, formed: true, active: Vec::new() };
        let mut items = vec![ingot(), None, None];
        let (mut state, mut tank) = (State::default(), Tank { fluid: None, capacity: MELTER_CAPACITY });
        // one tick of fuel: the third tick lights the block and burns it, the next cycle's third tick puts it out
        state.fuel.fuel = 1;
        state.fuel.temperature = 800;
        state.fuel.rate = 8;
        run(&mut state, &mut items, &mut tank, &mut world, 3);
        assert_eq!(world.active, vec![true], "lit while fuel burns");
        assert_eq!(state.slots[0].current_time, 8, "the same pass heated the ingot");
        run(&mut state, &mut items, &mut tank, &mut world, 4);
        assert_eq!(world.active, vec![true, false], "out once the fuel is gone");
        assert_eq!(state.slots[0].current_time, 3, "and the ingot cools again (8 - 5)");
    }

    #[test]
    fn the_heater_takes_what_burns_and_vanilla_burn_times_come_from_the_pinned_export() {
        assert_eq!(burn_time(&id("minecraft:coal")), 1600);
        assert_eq!(burn_time(&id("minecraft:lava_bucket")), 20000);
        assert_eq!(burn_time(&id("minecraft:stone")), 0);
        assert!(heater_accepts("minecraft:coal") && !heater_accepts("minecraft:stone") && !heater_accepts("not an id"));
        assert!(heater_accepts("minecraft:oak_button"), "100 ticks of burn time is a positive quarter");
        assert_eq!(fuel_remainder(&id("minecraft:lava_bucket")), Some(id("minecraft:bucket")));
    }
}
