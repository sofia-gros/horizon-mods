//! The casting table and the casting basin, from SlimeKnights CastingBlockEntity, CastingFluidHandler, CastingContainerWrapper,
//! MoldingContainerWrapper, AbstractCastingBlock and the recipe classes they run (MIT; see LICENSE): two item slots (the cast or
//! mold on the table, the finished item), a tank whose capacity is whatever the matching recipe wants of the fluid poured in, the
//! cooling timer, molding with the player's hand (sand and clay casts, picking a cast up) and the comparator strength.
//! `State` is the whole block entity as plain data (saved in the block entity's own data); `Env` is what it asks of the recipes.
//! A held fluid container empties into the tank or fills from it first (`State::transfer`, over PlutoLib's fluid items); the item swap
//! only runs for a table without fluid. Not ported here, each named where it would go: the `has_item` block state, redstone swapping,
//! sounds and particles (a ticker has no world scope), the hopper face rules, potion recipes, retextured casting's texture, and tool
//! casting (unit 7).
use crate::{
    casting_recipes::{Casting, CastingKind, MaterialCasting, Station},
    fluid_ingredient::{ItemOutput, Source},
    material_recipes::Ingredient,
    smeltery_data::Data,
    tool_materials::{part_material, set_part_material, MaterialVariant},
};
use pluto_lib::{
    BlockEntityFill, BlockEntityTick, BlockEntityType, BlockEntityTypeRegistry, BlockEntityUse, ActionResult, FluidItems, FluidStack, FluidTarget, Identifier, ItemSlot,
    Nbt, NbtCompound, NbtCompoundExt, RegistryLookup, TransferDirection,
};

pub(crate) const INPUT: usize = 0;
pub(crate) const OUTPUT: usize = 1;
pub(crate) const SLOTS: usize = 2;
/// The tank the host sees. The fluid filler takes what the recipe wants, so this is only the ceiling of the largest recipe.
pub(crate) const HOST_CAPACITY: u32 = 10_000;

/// The four blocks, by station and whether they need a cast in them to pour (the scorched ones do).
pub(crate) const BLOCKS: [(&str, Station, bool); 4] = [
    ("seared_table", Station::Table, false), ("scorched_table", Station::Table, true),
    ("seared_basin", Station::Basin, false), ("scorched_basin", Station::Basin, true),
];

/// What a casting recipe asks for the contents of a station and the fluid, and makes of them.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Found { pub id: Identifier, pub fluid_amount: i32, pub cooling_time: i32, pub consumed: bool, pub switch_slots: bool }

/// A molding recipe's outcome: what the material becomes and whether the pattern in the hand is used up.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Molded { pub result: ItemSlot, pub pattern_consumed: bool }

/// Everything the table asks of the recipes and tags.
pub(crate) trait Env {
    /// Whether the item counts as an empty station (`casts/empty/table`, `casts/empty/basin`).
    fn is_empty_cast(&self, stack: &ItemSlot) -> bool;
    /// The first casting recipe of the station that takes the cast (None: an empty station) and the fluid.
    fn find(&self, cast: Option<&ItemSlot>, fluid: &Identifier) -> Option<Found>;
    /// The recipe of that id, if it still takes the cast and the fluid.
    fn get(&self, id: &Identifier, cast: Option<&ItemSlot>, fluid: &Identifier) -> Option<Found>;
    /// What the recipe makes (`assemble`).
    fn assemble(&self, id: &Identifier, cast: Option<&ItemSlot>, fluid: &Identifier) -> Option<ItemSlot>;
    /// The molding recipe for the item on the table and the pattern held, if any.
    fn find_molding(&self, material: &ItemSlot, pattern: Option<&ItemSlot>) -> Option<Molded>;
}

fn slots(items: &mut Vec<Option<ItemSlot>>) { if items.len() < SLOTS { items.resize(SLOTS, None); } }

/// The casting table or basin block entity.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct State {
    pub fluid: Option<FluidStack>,
    /// The tank's capacity: the amount the current recipe wants, 0 while no recipe started.
    pub capacity: i32,
    /// The only fluid the tank takes until it is emptied.
    pub filter: Option<Identifier>,
    pub recipe: Option<Identifier>,
    pub timer: i32,
    /// Ticks the recipe needs once the tank is full, -1 until then.
    pub cooling: i32,
    pub redstone: bool,
}

impl Default for State {
    fn default() -> Self { State { fluid: None, capacity: 0, filter: None, recipe: None, timer: 0, cooling: -1, redstone: false } }
}

impl State {
    /// Reads the saved data (saveSynced and saveAdditional); the tank's fluid is the block entity's tank 0.
    pub(crate) fn load(data: &NbtCompound, fluid: Option<FluidStack>) -> Self {
        State {
            fluid,
            capacity: data.get_int("capacity", 0) as i32,
            filter: data.get("filter").and_then(Nbt::as_str).and_then(|text| Identifier::parse(text).ok()),
            recipe: data.get("recipe").and_then(Nbt::as_str).and_then(|text| Identifier::parse(text).ok()),
            timer: data.get_int("timer", 0) as i32,
            cooling: if data.contains_key("cooling") { data.get_int("cooling", -1) as i32 } else { -1 },
            redstone: data.get_int("redstone", 0) != 0,
        }
    }

    pub(crate) fn save(&self, data: &mut NbtCompound) {
        data.insert("capacity".to_owned(), Nbt::Int(self.capacity));
        data.insert("timer".to_owned(), Nbt::Int(self.timer));
        data.insert("cooling".to_owned(), Nbt::Int(self.cooling));
        data.insert("redstone".to_owned(), Nbt::Int(i32::from(self.redstone)));
        for (key, value) in [("filter", &self.filter), ("recipe", &self.recipe)] {
            match value {
                Some(id) => { data.insert(key.to_owned(), Nbt::String(id.to_string())); }
                None => { data.remove(key); }
            }
        }
    }

    /// CastingContainerWrapper.getStack: the stack in a slot, or none if it counts as an empty cast.
    fn cast(&self, items: &[Option<ItemSlot>], slot: usize, env: &dyn Env) -> Option<ItemSlot> {
        items.get(slot).and_then(Option::as_ref).filter(|stack| !env.is_empty_cast(stack)).cloned()
    }

    /// CastingBlockEntity.reset.
    pub(crate) fn reset(&mut self, env: &dyn Env, items: &[Option<ItemSlot>]) {
        self.timer = 0;
        self.recipe = None;
        self.capacity = 0;
        self.fluid = None;
        self.filter = None;
        self.contents_changed(env, items);
    }

    /// CastingBlockEntity.onContentsChanged: the timer starts once the tank holds the recipe's whole amount.
    pub(crate) fn contents_changed(&mut self, env: &dyn Env, items: &[Option<ItemSlot>]) {
        self.cooling = match (&self.fluid, &self.recipe) {
            (Some(fluid), Some(recipe)) if fluid.amount as i64 >= self.capacity as i64 => {
                let cast = self.cast(items, INPUT, env);
                env.get(recipe, cast.as_ref(), &fluid.fluid).map_or(0, |found| found.cooling_time.max(0))
            }
            _ => -1,
        };
    }

    /// CastingBlockEntity.initNewCasting: the amount the recipe for this fluid wants (0: nothing can start). Starting a recipe
    /// sets `recipe`, and may move the output into the input slot (a second pour onto a finished item).
    pub(crate) fn init_new_casting(&mut self, env: &dyn Env, items: &mut Vec<Option<ItemSlot>>, fluid: &Identifier, require_cast: bool) -> i32 {
        slots(items);
        if self.recipe.is_some() { return 0; }
        let (has_input, has_output) = (items[INPUT].is_some(), items[OUTPUT].is_some());
        if has_input && has_output { return 0; }
        if !has_output {
            // scorched basins and tables need a cast in them
            if !has_input && require_cast { return 0; }
            let cast = self.cast(items, INPUT, env);
            let Some(found) = env.find(cast.as_ref(), fluid) else { return 0 };
            self.recipe = Some(found.id);
            found.fluid_amount
        } else {
            // an output and no input: the output is the input of a further step
            let cast = self.cast(items, OUTPUT, env);
            let Some(found) = env.find(cast.as_ref(), fluid) else { return 0 };
            self.recipe = Some(found.id);
            items[INPUT] = items[OUTPUT].take();
            found.fluid_amount
        }
    }

    /// CastingFluidHandler.fill: how much of the fluid the tank takes. The recipe is chosen by the first fluid poured and fixes the
    /// capacity; the tank then takes only that fluid, up to the amount the recipe wants.
    pub(crate) fn fill(&mut self, env: &dyn Env, items: &mut Vec<Option<ItemSlot>>, resource: &FluidStack, require_cast: bool) -> u32 {
        if resource.amount == 0 { return 0; }
        if self.filter.as_ref().is_some_and(|filter| *filter != resource.fluid) { return 0; }
        let mut capacity = self.capacity;
        if capacity == 0 {
            capacity = self.init_new_casting(env, items, &resource.fluid, require_cast);
            if capacity <= 0 { return 0; }
            self.capacity = capacity;
            self.filter = Some(resource.fluid.clone());
        }
        let taken = match &mut self.fluid {
            None => {
                let amount = (capacity as u32).min(resource.amount);
                self.fluid = Some(FluidStack { fluid: resource.fluid.clone(), amount });
                amount
            }
            Some(held) if held.fluid != resource.fluid => return 0,
            Some(held) => {
                let space = capacity - held.amount as i32;
                if space <= 0 { return 0; }
                let amount = resource.amount.min(space as u32);
                held.amount += amount;
                amount
            }
        };
        self.contents_changed(env, items);
        taken
    }

    /// CastingBlockEntity.serverTick.
    pub(crate) fn tick(&mut self, env: &dyn Env, items: &mut Vec<Option<ItemSlot>>) {
        slots(items);
        // the tank was drained to nothing (CastingFluidHandler.drain resets the block entity)
        if self.fluid.is_none() && (self.recipe.is_some() || self.capacity != 0) {
            self.reset(env, items);
            return;
        }
        let Some(recipe) = self.recipe.clone() else { return };
        if self.cooling < 0 { return; }
        self.timer += 1;
        if self.timer < self.cooling { return; }
        let Some(fluid) = self.fluid.clone() else { return };
        let cast = self.cast(items, INPUT, env);
        let mut found = env.get(&recipe, cast.as_ref(), &fluid.fluid);
        if found.is_none() {
            // lost the recipe, or it wants more fluid than there is: look again, and come around later for the proper amount
            found = env.find(cast.as_ref(), &fluid.fluid);
            self.recipe = found.as_ref().map(|found| found.id.clone());
            if found.as_ref().is_none_or(|found| i64::from(found.fluid_amount) > i64::from(fluid.amount)) {
                self.timer = 0;
                return;
            }
        }
        let found = found.expect("checked above");
        let Some(output) = env.assemble(&found.id, cast.as_ref(), &fluid.fluid) else { self.timer = 0; return };
        if found.switch_slots != self.redstone {
            if !found.consumed { items[OUTPUT] = items[INPUT].take(); }
            items[INPUT] = Some(output);
        } else {
            if found.consumed { items[INPUT] = None; }
            items[OUTPUT] = Some(output);
        }
        self.reset(env, items);
    }

    /// `FluidTransferHelper.interactWithContainer` on the table's tank: a held fluid container empties into the tank (as far as the recipe
    /// takes it) or fills from what the tank holds. Skipped while the recipe cools. The outcome is the hand and what to give, `None` when
    /// nothing moved.
    pub(crate) fn transfer(&mut self, env: &dyn Env, items: &mut Vec<Option<ItemSlot>>, held: Option<&ItemSlot>, require_cast: bool) -> Option<pluto_lib::Interaction> {
        slots(items);
        if self.cooling >= 0 && self.timer > 0 { return None; }
        let mut target = CastingTank { state: self, env, items, require_cast };
        FluidItems::interact(held, &mut target, TransferDirection::Auto)
    }

    /// CastingBlockEntity.interact: a player used the table with `held` (None for an empty hand). Returns nothing; the hand and the
    /// stacks to give are the outcome. Does nothing while the recipe cools or the tank holds fluid (using a fluid container on the
    /// table is not ported).
    pub(crate) fn interact(&mut self, env: &dyn Env, items: &mut Vec<Option<ItemSlot>>, held: &mut Option<ItemSlot>, give: &mut Vec<ItemSlot>) {
        slots(items);
        if self.cooling >= 0 && self.timer > 0 { return; }
        if self.fluid.is_some() { return; }
        let (input, output) = (items[INPUT].clone(), items[OUTPUT].clone());
        // all molding recipes need something on the table and nothing in the output slot
        if let (Some(input), None) = (&input, &output) {
            if let Some(molded) = env.find_molding(input, held.as_ref()) {
                match held.take() {
                    // an empty hand picks the result up (only recipes with no pattern match an empty hand)
                    None => { items[INPUT] = None; *held = Some(molded.result); }
                    // the pattern in the hand goes on the table unless the recipe used it up
                    Some(mut pattern) => {
                        items[INPUT] = Some(molded.result);
                        if !molded.pattern_consumed {
                            let mut one = pattern.clone();
                            one.count = 1;
                            items[OUTPUT] = Some(one);
                        }
                        pattern.count = pattern.count.saturating_sub(1);
                        *held = Some(pattern).filter(|stack| stack.count > 0);
                    }
                }
                return;
            }
            // no recipe with the held item: a pattern-less recipe still happens "on pickup"
            if held.is_some() {
                if let Some(molded) = env.find_molding(input, None) {
                    items[INPUT] = None;
                    give.push(molded.result);
                    return;
                }
            }
        }
        // normal pickup
        if input.is_none() && output.is_none() {
            // the table holds one item at a time
            if let Some(mut stack) = held.take() {
                let mut one = stack.clone();
                one.count = 1;
                stack.count -= 1;
                *held = Some(stack).filter(|stack| stack.count > 0);
                items[INPUT] = Some(one);
            }
        } else {
            // prefer the output, as the input is often a cast to use again; the whole stack comes out
            let slot = if output.is_none() { INPUT } else { OUTPUT };
            if let Some(stack) = items[slot].take() { give.push(stack); }
        }
    }

    /// CastingBlockEntity.swap: a redstone pulse swaps the slots while nothing is being cast.
    pub(crate) fn swap(&self, items: &mut Vec<Option<ItemSlot>>) {
        slots(items);
        if self.recipe.is_none() { items.swap(INPUT, OUTPUT); }
    }

    /// CastingBlockEntity.getAnalogSignal.
    pub(crate) fn analog_signal(&self, items: &[Option<ItemSlot>]) -> i32 {
        let has = |slot: usize| items.get(slot).is_some_and(Option::is_some);
        if has(OUTPUT) { return 15; }
        // 11 - 14 are cooling time from 0 to 99%
        if self.cooling > 0 { return 11 + self.timer * 4 / self.cooling; }
        // 2 - 9 are fluid between 0 and 99%
        let capacity = self.fluid.as_ref().map_or(self.capacity, |fluid| if self.capacity == 0 { fluid.amount as i32 } else { self.capacity });
        if capacity > 0 { return 2 + self.fluid.as_ref().map_or(0, |fluid| fluid.amount as i32) * 9 / capacity; }
        i32::from(has(INPUT))
    }
}

/// The recipes and tags of the loaded data for one station.
pub(crate) struct Live<'a> {
    pub data: &'a Data,
    pub item_tags: Option<&'a RegistryLookup>,
    pub station: Station,
    /// The material registry, for which materials a casting part takes (None while it is not loaded).
    pub materials: Option<&'a crate::materials::Definitions>,
}

impl<'a> Live<'a> {
    fn in_item_tag(&self, item: &Identifier, tag: &Identifier) -> bool { self.item_tags.and_then(|tags| tags.get_tag(tag)).is_some_and(|members| members.contains(item)) }
    fn in_fluid_tag(&self, fluid: &Identifier, tag: &Identifier) -> bool { self.data.tags.contains(fluid, tag) }

    /// ItemOutput.get: a fixed item, or the preferred member of the tag (the first one; there is no pack priority here).
    fn output(&self, output: &ItemOutput) -> Option<ItemSlot> {
        let item = match &output.source {
            Source::Id(item) => item.clone(),
            Source::Tag(tag) => self.item_tags?.get_tag(tag)?.iter().next()?.clone(),
        };
        Some(ItemSlot::new(item, output.count.max(1) as u32))
    }

    fn can_use(&self, part: &Identifier, material: &MaterialVariant) -> bool {
        self.materials.is_some_and(|defs| crate::tool_parts::material_items().get(part).is_some_and(|part| part.can_use(material, defs)))
    }

    /// The recipe's numbers for this cast and fluid, if it takes them.
    fn candidate(&self, recipe: &Casting, cast: Option<&ItemSlot>, fluid: &Identifier) -> Option<Found> {
        let in_fluid = |fluid: &Identifier, tag: &Identifier| self.in_fluid_tag(fluid, tag);
        let in_item = |item: &Identifier, tag: &Identifier| self.in_item_tag(item, tag);
        let item = cast.map(|stack| &stack.item);
        let base = |fluid_amount, cooling_time| Found { id: recipe.id.clone(), fluid_amount, cooling_time, consumed: recipe.cast_consumed, switch_slots: recipe.switch_slots };
        match &recipe.kind {
            CastingKind::Item { .. } | CastingKind::Retextured { .. } | CastingKind::Duplication => {
                // `matches` asks one membership test for both the cast (an item tag) and the fluid (a fluid tag); the two tag sets are told apart by id
                let either = |member: &Identifier, tag: &Identifier| in_fluid(member, tag) || in_item(member, tag);
                recipe.matches(item, fluid, &either, false).then(|| base(recipe.fluid_amount(fluid, &in_fluid), recipe.cooling_time))
            }
            CastingKind::Material { result, item_cost } | CastingKind::Composite { result, item_cost, .. } => {
                if !recipe.cast_matches(item, &in_item, false) { return None; }
                let materials = &self.data.recipes.material_casting;
                let stack_material = cast.map(|stack| part_material(&stack.custom_data));
                let can_cast_with_stat = |stat: &Identifier, material: &Identifier| self.materials.is_some_and(|defs| defs.get_stats(material, stat).is_some());
                let fluid_recipe = materials.fluid_recipe(&recipe.kind, fluid, stack_material.as_ref(), &in_fluid, &can_cast_with_stat)?;
                if !self.can_use(result, &fluid_recipe.output) { return None; }
                Some(base(MaterialCasting::fluid_amount(fluid_recipe, *item_cost, fluid, &in_fluid), MaterialCasting::cooling_time(fluid_recipe, *item_cost, fluid, &in_fluid)))
            }
            // filling a held container and potions wait for fluid stacks inside items
            CastingKind::Potion { .. } | CastingKind::Filling { .. } => None,
        }
    }
}

impl Env for Live<'_> {
    fn is_empty_cast(&self, stack: &ItemSlot) -> bool {
        let tag = match self.station { Station::Table => "tconstruct:casts/empty/table", Station::Basin => "tconstruct:casts/empty/basin" };
        self.in_item_tag(&stack.item, &Identifier::parse(tag).expect("valid tag id"))
    }

    fn find(&self, cast: Option<&ItemSlot>, fluid: &Identifier) -> Option<Found> {
        self.data.recipes.casting.iter().filter(|recipe| recipe.station == self.station).find_map(|recipe| self.candidate(recipe, cast, fluid))
    }

    fn get(&self, id: &Identifier, cast: Option<&ItemSlot>, fluid: &Identifier) -> Option<Found> {
        self.data.recipes.casting.iter().find(|recipe| recipe.station == self.station && recipe.id == *id).and_then(|recipe| self.candidate(recipe, cast, fluid))
    }

    fn assemble(&self, id: &Identifier, cast: Option<&ItemSlot>, fluid: &Identifier) -> Option<ItemSlot> {
        let recipe = self.data.recipes.casting.iter().find(|recipe| recipe.station == self.station && recipe.id == *id)?;
        let in_fluid = |fluid: &Identifier, tag: &Identifier| self.in_fluid_tag(fluid, tag);
        match &recipe.kind {
            CastingKind::Item { result } | CastingKind::Retextured { result } => self.output(result),
            // the cast comes out again, a copy of itself
            CastingKind::Duplication => cast.cloned(),
            CastingKind::Material { result, .. } | CastingKind::Composite { result, .. } => {
                let stack_material = cast.map(|stack| part_material(&stack.custom_data));
                let fluid_recipe = self.data.recipes.material_casting.fluid_recipe(&recipe.kind, fluid, stack_material.as_ref(), &in_fluid, &|stat, material| {
                    self.materials.is_some_and(|defs| defs.get_stats(material, stat).is_some())
                })?;
                let mut part = ItemSlot::new(result.clone(), 1);
                set_part_material(&mut part, &fluid_recipe.output);
                Some(part)
            }
            CastingKind::Potion { .. } | CastingKind::Filling { .. } => None,
        }
    }

    fn find_molding(&self, material: &ItemSlot, pattern: Option<&ItemSlot>) -> Option<Molded> {
        let in_item = |item: &Identifier, tag: &Identifier| self.in_item_tag(item, tag);
        let station_is_table = self.station == Station::Table;
        // molding recipes name their station by type (`molding_table` / `molding_basin`)
        self.data.recipes.molding.iter().filter(|recipe| recipe.table == station_is_table)
            .find(|recipe| recipe.matches(&material.item, pattern.map(|stack| &stack.item), &in_item))
            .and_then(|recipe| Some(Molded { result: self.output(&recipe.result)?, pattern_consumed: recipe.pattern_consumed }))
    }
}

/// The station and the cast requirement of a block id.
pub(crate) fn block_rules(block: &str) -> Option<(Station, bool)> {
    BLOCKS.iter().find(|(name, ..)| format!("tconstruct:{name}") == block).map(|(_, station, require)| (*station, *require))
}

/// Runs `f` with the live recipes for the station; None while the data or the tags are not loaded.
fn with_live<R>(station: Station, f: impl FnOnce(&Live<'_>) -> R) -> Option<R> {
    let data = crate::common::smeltery_data()?.read().ok()?;
    if !data.loaded() { return None; }
    let item_tags = crate::common::item_snapshot();
    let definitions = crate::common::register().read().ok()?;
    let live = Live { data: &data, item_tags: item_tags.as_deref(), station, materials: definitions.server_loaded.then_some(&*definitions) };
    Some(f(&live))
}

/// What the recipe in progress will make, for the casting renderer (`CastingBlockEntity.getRecipeOutput`, through
/// the same live recipes the tick uses).
pub(crate) fn recipe_output(station: Station, recipe: &Identifier, cast: Option<&ItemSlot>, fluid: &Identifier) -> Option<ItemSlot> {
    with_live(station, |env| env.assemble(recipe, cast, fluid)).flatten()
}

fn tick_casting(tick: &mut BlockEntityTick<'_>, station: Station) {
    with_live(station, |env| {
        let mut state = State::load(tick.data, tick.fluid(0));
        state.tick(env, tick.items);
        state.save(tick.data);
        tick.set_fluid(0, state.fluid.clone());
    });
}

fn fill_casting(fill: &mut BlockEntityFill<'_>, station: Station, require_cast: bool) -> u32 {
    with_live(station, |env| {
        let mut state = State::load(fill.data, fill.fluid_in(0));
        let taken = state.fill(env, fill.items, &fill.fluid.clone(), require_cast);
        state.save(fill.data);
        fill.set_fluid(0, state.fluid.clone());
        taken
    }).unwrap_or(0)
}

/// The casting table's tank as a trade partner (`CastingFluidHandler`): it fills by the recipe's rule (`State::fill`) and drains whatever it
/// holds, which resets the table once empty.
struct CastingTank<'a> { state: &'a mut State, env: &'a dyn Env, items: &'a mut Vec<Option<ItemSlot>>, require_cast: bool }

impl FluidTarget for CastingTank<'_> {
    fn fill(&mut self, fluid: &FluidStack, execute: bool) -> u32 {
        if execute { return self.state.fill(self.env, self.items, fluid, self.require_cast); }
        let (mut state, mut items) = (self.state.clone(), self.items.clone());
        state.fill(self.env, &mut items, fluid, self.require_cast)
    }
    fn drain(&mut self, wanted: Option<&Identifier>, amount: u32, execute: bool) -> Option<FluidStack> {
        let held = self.state.fluid.as_ref()?;
        if wanted.is_some_and(|wanted| wanted != &held.fluid) { return None; }
        let taken = held.amount.min(amount);
        if taken == 0 { return None; }
        let out = FluidStack { fluid: held.fluid.clone(), amount: taken };
        if execute {
            let left = held.amount - taken;
            if left == 0 { self.state.reset(self.env, self.items); } else {
                self.state.fluid = Some(FluidStack { fluid: out.fluid.clone(), amount: left });
                self.state.contents_changed(self.env, self.items);
            }
        }
        Some(out)
    }
}

fn use_casting(used: &mut BlockEntityUse<'_>, station: Station, require_cast: bool) -> ActionResult {
    // sneaking uses the block's neighbour instead
    if used.sneaking { return ActionResult::Pass; }
    with_live(station, |env| {
        let mut state = State::load(used.data, used.fluid(0));
        // first the table as a tank; the item swap only runs for a table without fluid
        if let Some(done) = state.transfer(env, used.items, used.held.as_ref(), require_cast) {
            used.held = done.held;
            used.give.extend(done.give);
            state.save(used.data);
            return ActionResult::Success;
        }
        state.interact(env, used.items, &mut used.held, &mut used.give);
        state.save(used.data);
        ActionResult::Success
    }).unwrap_or(ActionResult::Success)
}

pub(crate) fn register() {
    for (name, station, require_cast) in BLOCKS {
        let block = Identifier::new("tconstruct", name).expect("valid casting block id");
        BlockEntityTypeRegistry::register(
            BlockEntityType::new(block).item_slots(SLOTS as u32).fluid_tanks([HOST_CAPACITY]).sync_data()
                .ticker(move |tick| tick_casting(tick, station))
                .fluid_filler(move |fill| fill_casting(fill, station, require_cast))
                .on_use(move |used| use_casting(used, station, require_cast)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
    fn stack(text: &str) -> ItemSlot { ItemSlot::new(id(text), 1) }
    fn fluid(text: &str, amount: u32) -> FluidStack { FluidStack { fluid: id(text), amount } }

    /// One ingot recipe (90 of iron over a cast, 60 ticks), one duplication, one molding recipe and the empty-cast tag.
    struct Recipes;
    impl Env for Recipes {
        fn is_empty_cast(&self, stack: &ItemSlot) -> bool { stack.item.path() == "gold_platform" }
        fn find(&self, cast: Option<&ItemSlot>, fluid: &Identifier) -> Option<Found> {
            match (cast.map(|stack| stack.item.path()), fluid.path()) {
                (Some("ingot_cast"), "molten_iron") => Some(Found { id: id("t:ingot"), fluid_amount: 90, cooling_time: 60, consumed: false, switch_slots: false }),
                (Some("blank_cast"), "molten_gold") => Some(Found { id: id("t:cast"), fluid_amount: 90, cooling_time: 20, consumed: true, switch_slots: true }),
                (None, "molten_iron") => Some(Found { id: id("t:block"), fluid_amount: 810, cooling_time: 180, consumed: false, switch_slots: false }),
                _ => None,
            }
        }
        fn get(&self, recipe: &Identifier, cast: Option<&ItemSlot>, fluid: &Identifier) -> Option<Found> { self.find(cast, fluid).filter(|found| found.id == *recipe) }
        fn assemble(&self, recipe: &Identifier, _: Option<&ItemSlot>, _: &Identifier) -> Option<ItemSlot> {
            Some(stack(match recipe.path() { "ingot" => "t:iron_ingot", "block" => "t:iron_block", _ => "t:gold_cast" }))
        }
        fn find_molding(&self, material: &ItemSlot, pattern: Option<&ItemSlot>) -> Option<Molded> {
            match (material.item.path(), pattern.map(|stack| stack.item.path())) {
                ("sand_cast", None) => Some(Molded { result: stack("t:blank_sand_cast"), pattern_consumed: false }),
                ("clay", Some("ingot")) => Some(Molded { result: stack("t:ingot_clay_cast"), pattern_consumed: false }),
                ("sand", Some("ingot")) => Some(Molded { result: stack("t:ingot_sand_cast"), pattern_consumed: true }),
                _ => None,
            }
        }
    }

    fn pour(state: &mut State, items: &mut Vec<Option<ItemSlot>>, fluid_id: &str, amount: u32, require_cast: bool) -> u32 {
        state.fill(&Recipes, items, &fluid(fluid_id, amount), require_cast)
    }

    #[test]
    fn the_first_pour_picks_the_recipe_and_fixes_the_capacity_and_the_fluid() {
        let mut state = State::default();
        let mut items = vec![Some(stack("t:ingot_cast")), None];
        assert_eq!(pour(&mut state, &mut items, "t:molten_iron", 30, false), 30);
        assert_eq!((state.capacity, state.recipe.clone(), state.cooling), (90, Some(id("t:ingot")), -1), "not full yet");
        assert_eq!(pour(&mut state, &mut items, "t:molten_gold", 30, false), 0, "only iron now");
        assert_eq!(pour(&mut state, &mut items, "t:molten_iron", 100, false), 60, "the tank takes up to the recipe's 90");
        assert_eq!((state.fluid.clone().unwrap().amount, state.cooling), (90, 60), "full: the timer is set");
        assert_eq!(pour(&mut state, &mut items, "t:molten_iron", 10, false), 0, "full");
        let mut other = State::default();
        assert_eq!(pour(&mut other, &mut vec![None, None], "t:molten_water", 10, false), 0, "no recipe for water");
    }

    #[test]
    fn a_full_table_cools_for_the_recipes_time_and_the_ingot_replaces_nothing_but_the_output() {
        let mut state = State::default();
        let mut items = vec![Some(stack("t:ingot_cast")), None];
        pour(&mut state, &mut items, "t:molten_iron", 90, false);
        for _ in 0..59 { state.tick(&Recipes, &mut items); }
        assert!(items[OUTPUT].is_none() && state.recipe.is_some());
        assert_eq!(state.analog_signal(&items), 11 + 59 * 4 / 60);
        state.tick(&Recipes, &mut items);
        assert_eq!(items[OUTPUT].as_ref().unwrap().item, id("t:iron_ingot"));
        assert_eq!(items[INPUT].as_ref().unwrap().item, id("t:ingot_cast"), "the cast is kept");
        assert_eq!((state.fluid.clone(), state.recipe.clone(), state.capacity, state.timer, state.cooling), (None, None, 0, 0, -1), "reset");
        assert_eq!(state.analog_signal(&items), 15);
    }

    #[test]
    fn a_consumed_cast_with_switched_slots_leaves_the_item_in_the_input_and_redstone_flips_it() {
        let run = |redstone: bool| {
            let mut state = State { redstone, ..State::default() };
            let mut items = vec![Some(stack("t:blank_cast")), None];
            pour(&mut state, &mut items, "t:molten_gold", 90, false);
            for _ in 0..20 { state.tick(&Recipes, &mut items); }
            items.iter().map(|slot| slot.as_ref().map(|stack| stack.item.to_string())).collect::<Vec<_>>()
        };
        assert_eq!(run(false), vec![Some("t:gold_cast".to_owned()), None], "switch_slots puts the result where the cast was");
        assert_eq!(run(true), vec![None, Some("t:gold_cast".to_owned())], "a powered table puts it in the output instead");
    }

    #[test]
    fn a_basin_without_a_cast_casts_a_block_but_a_scorched_one_wants_a_cast_and_a_finished_item_can_be_poured_again() {
        let mut state = State::default();
        let mut items = vec![None, None];
        assert_eq!(pour(&mut state, &mut items, "t:molten_iron", 810, true), 0, "scorched: needs a cast");
        assert_eq!(pour(&mut state, &mut items, "t:molten_iron", 810, false), 810);
        for _ in 0..180 { state.tick(&Recipes, &mut items); }
        assert_eq!(items[OUTPUT].as_ref().unwrap().item, id("t:iron_block"));
        // an item in the output and nothing in the input: the output becomes the input of the next step
        let mut next = State::default();
        let mut stage = vec![None, Some(stack("t:ingot_cast"))];
        assert_eq!(pour(&mut next, &mut stage, "t:molten_iron", 90, false), 90);
        assert!(stage[OUTPUT].is_none() && stage[INPUT].is_some(), "moved over");
        let mut full = State::default();
        assert_eq!(pour(&mut full, &mut vec![Some(stack("t:ingot_cast")), Some(stack("t:iron_ingot"))], "t:molten_iron", 90, false), 0, "no space for an output");
    }

    #[test]
    fn a_platform_counts_as_an_empty_station_and_a_drained_tank_forgets_the_recipe() {
        let mut state = State::default();
        let mut items = vec![Some(stack("t:gold_platform")), None];
        assert_eq!(pour(&mut state, &mut items, "t:molten_iron", 810, false), 810, "block casting runs on a platform");
        state.fluid = None; // the host drained it
        state.tick(&Recipes, &mut items);
        assert_eq!((state.recipe, state.capacity, state.filter), (None, 0, None));
    }

    #[test]
    fn the_state_survives_the_saved_data() {
        let mut state = State::default();
        let mut items = vec![Some(stack("t:ingot_cast")), None];
        pour(&mut state, &mut items, "t:molten_iron", 90, false);
        state.timer = 12;
        let mut data = NbtCompound::new();
        state.save(&mut data);
        assert_eq!(State::load(&data, state.fluid.clone()), state);
        assert_eq!(State::load(&NbtCompound::new(), None), State::default(), "a new block entity has no timer running");
    }

    #[test]
    fn molding_with_the_hand_makes_casts_picks_them_up_and_pickup_returns_what_is_on_the_table() {
        let mut state = State::default();
        let mut items = vec![Some(stack("t:sand_cast")), None];
        let (mut held, mut give) = (None, Vec::new());
        // an empty hand picks the blank cast up
        state.interact(&Recipes, &mut items, &mut held, &mut give);
        assert_eq!((items[INPUT].clone(), held.clone()), (None, Some(stack("t:blank_sand_cast"))));
        // clay and a pattern: the pattern is kept on the table, one off the held stack
        let mut items = vec![Some(stack("t:clay")), None];
        let mut held = Some(ItemSlot::new(id("t:ingot"), 3));
        state.interact(&Recipes, &mut items, &mut held, &mut give);
        assert_eq!(items[INPUT].as_ref().unwrap().item, id("t:ingot_clay_cast"));
        assert_eq!(items[OUTPUT].as_ref().unwrap().item, id("t:ingot"));
        assert_eq!(held.as_ref().unwrap().count, 2);
        // sand uses the pattern up
        let mut items = vec![Some(stack("t:sand")), None];
        let mut held = Some(stack("t:ingot"));
        state.interact(&Recipes, &mut items, &mut held, &mut give);
        assert_eq!((items[INPUT].as_ref().unwrap().item.clone(), items[OUTPUT].clone(), held), (id("t:ingot_sand_cast"), None, None));
        // an unrelated held item with a pattern-less recipe on the table: the result goes to the player
        let mut items = vec![Some(stack("t:sand_cast")), None];
        let mut held = Some(stack("t:stick"));
        state.interact(&Recipes, &mut items, &mut held, &mut give);
        assert_eq!((give.clone(), items[INPUT].clone(), held.clone()), (vec![stack("t:blank_sand_cast")], None, Some(stack("t:stick"))));
        // nothing matches: a held item goes on an empty table, one of it; the output comes off first
        let mut items = vec![None, None];
        let mut held = Some(ItemSlot::new(id("t:stick"), 5));
        state.interact(&Recipes, &mut items, &mut held, &mut Vec::new());
        assert_eq!((items[INPUT].clone(), held.unwrap().count), (Some(stack("t:stick")), 4));
        let mut items = vec![Some(stack("t:ingot_cast")), Some(stack("t:iron_ingot"))];
        let mut give = Vec::new();
        state.interact(&Recipes, &mut items, &mut None, &mut give);
        assert_eq!((give, items[OUTPUT].clone(), items[INPUT].is_some()), (vec![stack("t:iron_ingot")], None, true));
    }

    #[test]
    fn nothing_happens_while_a_recipe_cools_or_fluid_is_in_the_tank_and_a_pulse_swaps_the_slots_when_idle() {
        let mut state = State::default();
        let mut items = vec![Some(stack("t:ingot_cast")), None];
        pour(&mut state, &mut items, "t:molten_iron", 90, false);
        state.tick(&Recipes, &mut items);
        let mut give = Vec::new();
        state.interact(&Recipes, &mut items, &mut None, &mut give);
        assert!(give.is_empty() && items[INPUT].is_some(), "the cooling recipe keeps the cast");
        let mut idle = State::default();
        let mut slots = vec![Some(stack("t:a")), None];
        idle.swap(&mut slots);
        assert_eq!((slots[INPUT].clone(), slots[OUTPUT].clone()), (None, Some(stack("t:a"))));
        assert_eq!(idle.analog_signal(&slots), 15);
        state.swap(&mut items);
        assert!(items[INPUT].is_some(), "a table that is casting does not swap");
    }
}
