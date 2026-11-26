//! Fluid inside item stacks (Fabric: `Storage<FluidVariant>` of an item through `ContainerItemContext`; Forge: `IFluidHandlerItem`).
//!
//! A mod says which items hold fluid and how ([`FluidItemDef`]); every stack of such an item then answers what it holds, takes a fluid
//! and gives it back. The fluid lives in the stack's own custom data (or, for a bottle, in which item it is), so it is saved, copied
//! and synchronised with the stack and needs no host feature of its own. Every transfer is a pure function from a stack to the stack
//! afterwards, so asking and doing are the same call and the caller keeps what it wants.
//!
//! [`FluidItems::interact`] is the hand-held part: a stack against a tank (a [`FluidTarget`]), the way a bucket empties into a tank or
//! fills from it when the player uses a block.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::base::{Identifier, Nbt, NbtCompound, NbtCompoundExt};
use super::block_entity_api_v1::{FluidStack, ItemSlot};

// Decides whether a fluid may go into a container; called with the fluid's id.
type FluidAccepts = Arc<dyn Fn(&Identifier) -> bool + Send + Sync>;

#[derive(Clone)]
enum Kind {
    /// A variable amount up to `capacity` per item, kept as `{FluidName, Amount, Tag?}` under `key` (the fluid's own tag, when it has one). With `scaled`, a stack of `n` items is one
    /// tank of `n` times the capacity, filled and drained together in steps of `n`.
    Tank { key: String, capacity: u32, scaled: bool, fill_stack_limit: Option<u32> },
    /// Exactly `amount` per item of one fluid, kept as the fluid's id under `key` (a `{FluidName, Tag}` compound when the fluid
    /// carries a tag): filled and drained whole, never in part.
    Portion { key: String, amount: u32 },
    /// Always holds `amount` of `fluid`; draining it hands back `empty` (or nothing).
    Constant { fluid: Identifier, amount: u32, empty: Option<Identifier> },
    /// Always holds `amount` of `fluid` with the stack's own custom data as the fluid's tag (the item is the
    /// container and its tag is the fluid stack's tag, as a potion bucket's is); draining it hands the tag back
    /// with the fluid and hands back `empty` (or nothing).
    TaggedConstant { fluid: Identifier, amount: u32, empty: Option<Identifier> },
    /// Holds nothing but becomes a filled item when `amount` of an accepted fluid goes in.
    Fills(Vec<FillRule>),
}

#[derive(Clone)]
struct FillRule {
    accepts: FluidAccepts,
    amount: u32,
    filled: Identifier,
    /// The filled stack takes the fluid's tag as its own custom data (the filled item is a tagged container of
    /// the fluid, see [`FluidItemDef::tagged_constant`]); without it the filled stack keeps the empty one's data.
    carry_tag: bool,
}

/// One kind of item that holds fluid and how it holds it. Build it with [`FluidItemDef::tank`], [`FluidItemDef::portion`],
/// [`FluidItemDef::constant`], [`FluidItemDef::tagged_constant`] or [`FluidItemDef::empty_container`] and register it with [`FluidItems::register`].
#[derive(Clone)]
pub struct FluidItemDef {
    /// The item this describes, such as `example:can`.
    pub item: Identifier,
    kind: Kind,
}

impl FluidItemDef {
    /// An item holding any one fluid up to `capacity` millibuckets, kept in the stack's custom data under `key` as `{FluidName, Amount, Tag?}`
    /// (Forge's own fluid stack tag, so an item written by another tool reads the same).
    pub fn tank(item: Identifier, key: &str, capacity: u32) -> Self {
        Self { item, kind: Kind::Tank { key: key.to_owned(), capacity, scaled: false, fill_stack_limit: None } }
    }

    /// An item holding exactly `amount` millibuckets of one fluid or nothing, kept in the stack's custom data under `key` as the fluid's id.
    /// A stack of several holds `amount` each and fills and drains whole.
    pub fn portion(item: Identifier, key: &str, amount: u32) -> Self {
        Self { item, kind: Kind::Portion { key: key.to_owned(), amount } }
    }

    /// An item that always holds `amount` of `fluid`, such as a bottle of it; emptied it becomes `empty` (a glass bottle) or vanishes.
    pub fn constant(item: Identifier, fluid: Identifier, amount: u32, empty: Option<Identifier>) -> Self {
        Self { item, kind: Kind::Constant { fluid, amount, empty } }
    }

    /// An item that always holds `amount` of `fluid` together with a tag: the stack's own custom data is the fluid's
    /// tag, so a stack with no custom data holds the plain fluid and any other stack holds the fluid with that tag.
    /// Emptied it becomes `empty` (an empty bucket) or vanishes. Filling happens on the empty item's side (see
    /// [`FluidItemDef::empty_container_tagged`]).
    pub fn tagged_constant(item: Identifier, fluid: Identifier, amount: u32, empty: Option<Identifier>) -> Self {
        Self { item, kind: Kind::TaggedConstant { fluid, amount, empty } }
    }

    /// An empty item (a glass bottle, a bucket, a bowl) that becomes `filled` when `amount` millibuckets of a fluid `accepts` goes in. Chain
    /// [`FluidItemDef::fills`] for further fillings of the same item.
    pub fn empty_container(item: Identifier, accepts: impl Fn(&Identifier) -> bool + Send + Sync + 'static, amount: u32, filled: Identifier) -> Self {
        Self { item, kind: Kind::Fills(vec![FillRule { accepts: Arc::new(accepts), amount, filled, carry_tag: false }]) }
    }

    /// An empty item that becomes `filled` when `amount` millibuckets of a fluid `accepts` goes in, taking the fluid's
    /// tag as the filled stack's own custom data (the filled item is a tagged container of the fluid, see
    /// [`FluidItemDef::tagged_constant`]). Chain [`FluidItemDef::fills`] or [`FluidItemDef::fills_tagged`] for further
    /// fillings of the same item.
    pub fn empty_container_tagged(item: Identifier, accepts: impl Fn(&Identifier) -> bool + Send + Sync + 'static, amount: u32, filled: Identifier) -> Self {
        Self { item, kind: Kind::Fills(vec![FillRule { accepts: Arc::new(accepts), amount, filled, carry_tag: true }]) }
    }

    /// The same empty item with one more filling, tried after the earlier ones. Does nothing to other kinds of item.
    pub fn fills(mut self, accepts: impl Fn(&Identifier) -> bool + Send + Sync + 'static, amount: u32, filled: Identifier) -> Self {
        if let Kind::Fills(rules) = &mut self.kind {
            rules.push(FillRule { accepts: Arc::new(accepts), amount, filled, carry_tag: false });
        }
        self
    }

    /// The same empty item with one more filling, tried after the earlier ones, whose filled stack takes the fluid's
    /// tag as its own custom data. Does nothing to other kinds of item.
    pub fn fills_tagged(mut self, accepts: impl Fn(&Identifier) -> bool + Send + Sync + 'static, amount: u32, filled: Identifier) -> Self {
        if let Kind::Fills(rules) = &mut self.kind {
            rules.push(FillRule { accepts: Arc::new(accepts), amount, filled, carry_tag: true });
        }
        self
    }

    /// The same tank, where a stack of `n` items is one tank of `n` times the capacity: the fluid is kept per item and a transfer must
    /// be a multiple of `n`. Does nothing to other kinds of item.
    pub fn scaled(mut self) -> Self {
        if let Kind::Tank { scaled, .. } = &mut self.kind {
            *scaled = true;
        }
        self
    }

    /// The same tank, refusing to take fluid while the stack holds more than `limit` items (a large stack is not safe to fill). Draining
    /// is unaffected. Does nothing to other kinds of item.
    pub fn fill_stack_limit(mut self, limit: u32) -> Self {
        if let Kind::Tank { fill_stack_limit, .. } = &mut self.kind {
            *fill_stack_limit = Some(limit);
        }
        self
    }

    /// The custom data key that holds the fluid, for the kinds that keep it there.
    pub fn key(&self) -> Option<&str> {
        match &self.kind {
            Kind::Tank { key, .. } | Kind::Portion { key, .. } => Some(key),
            _ => None,
        }
    }
}

static DEFS: Mutex<Option<HashMap<String, FluidItemDef>>> = Mutex::new(None);

/// What a fluid transfer into a stack did: how much went in and the stack afterwards.
#[derive(Clone, Debug, PartialEq)]
pub struct Filled {
    /// Millibuckets that went in; 0 when the stack took nothing.
    pub amount: u32,
    /// The stack afterwards, the same count as before, possibly another item (an empty bottle that is now full).
    pub container: ItemSlot,
}

/// What a fluid transfer out of a stack did: the fluid that came out and the stack afterwards.
#[derive(Clone, Debug, PartialEq)]
pub struct Drained {
    /// The fluid taken.
    pub fluid: FluidStack,
    /// The stack afterwards. A bottle that is now empty is the empty item; a stack that vanishes (nothing is left of it) is `None`.
    pub container: Option<ItemSlot>,
}

/// One side of a transfer that is not an item: a tank to fill and drain, where [`FluidItems::interact`] moves fluid to or from the held stack.
/// A block entity's tank implements it over its saved data.
pub trait FluidTarget {
    /// How much of `fluid` the tank takes; with `execute` false it only asks.
    fn fill(&mut self, fluid: &FluidStack, execute: bool) -> u32;
    /// Takes up to `amount` millibuckets, of `wanted` when named and otherwise of what the tank gives first; with `execute` false it only asks.
    fn drain(&mut self, wanted: Option<&Identifier>, amount: u32, execute: bool) -> Option<FluidStack>;
}

/// Which way a held stack and a tank trade.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransferDirection {
    /// The stack empties into the tank if it can, otherwise it is filled from the tank.
    Auto,
    /// Only the stack into the tank.
    ToTank,
    /// Only the tank into the stack.
    FromTank,
}

/// What a trade between a held stack and a tank did.
#[derive(Clone, Debug, PartialEq)]
pub struct Interaction {
    /// What the hand holds afterwards; `None` for an empty hand.
    pub held: Option<ItemSlot>,
    /// Stacks to hand to the player (the filled bottle when the hand held several empty ones).
    pub give: Vec<ItemSlot>,
    /// The fluid that moved, and `true` when it went into the tank.
    pub moved: FluidStack,
    /// Whether the fluid went from the stack into the tank (`false`: from the tank into the stack).
    pub into_tank: bool,
}

/// Registers the items that hold fluid and does the transfers.
pub struct FluidItems;

impl FluidItems {
    /// Makes `def.item` a fluid container. Registering the same item again replaces the earlier definition.
    pub fn register(def: FluidItemDef) {
        if let Ok(mut all) = DEFS.lock() {
            all.get_or_insert_with(HashMap::new).insert(def.item.to_string(), def);
        }
    }

    /// The definition registered for `item`, if any.
    pub fn get(item: &str) -> Option<FluidItemDef> {
        DEFS.lock().ok()?.as_ref()?.get(item).cloned()
    }

    /// Every definition registered so far, in no particular order.
    pub fn registered() -> Vec<FluidItemDef> {
        DEFS.lock().ok().and_then(|all| all.as_ref().map(|all| all.values().cloned().collect())).unwrap_or_default()
    }

    /// Whether the stack's item holds, or can be filled with, fluid.
    pub fn is_container(stack: &ItemSlot) -> bool {
        Self::get(&stack.item.to_string()).is_some()
    }

    /// Whether the stack is a bucket: a filled bucket (a constant container that hands back
    /// `minecraft:bucket` when drained) or the shared empty bucket itself. Tank items, portions
    /// and other containers are not buckets: they trade through a block's own use, while a
    /// bucket's trade with a clicked block also rides the bucket path (see
    /// `fluid_world_api_v1::use_bucket`), because the game's own bucket use would otherwise
    /// pour the fluid into the world without ever asking the block.
    pub(crate) fn is_bucket(stack: &ItemSlot) -> bool {
        let Some(def) = Self::get(&stack.item.to_string()) else { return false };
        match &def.kind {
            Kind::Constant { empty, .. } | Kind::TaggedConstant { empty, .. } => {
                empty.as_ref().is_some_and(|empty| empty.to_string() == "minecraft:bucket")
            }
            Kind::Fills(_) => stack.item.to_string() == "minecraft:bucket",
            _ => false,
        }
    }

    /// What the stack holds; `None` for an empty container and for an item that holds no fluid. A scaled tank of several items reports
    /// the amount of all of them.
    pub fn contents(stack: &ItemSlot) -> Option<FluidStack> {
        let def = Self::get(&stack.item.to_string())?;
        let count = stack.count.max(1);
        match &def.kind {
            Kind::Tank { key, scaled, .. } => {
                let (fluid, amount, tag) = read_tank(&stack.custom_data, key)?;
                let scale = if *scaled { count } else { 1 };
                Some(FluidStack { fluid, amount: amount.saturating_mul(scale), tag })
            }
            Kind::Portion { key, amount } => {
                let (fluid, tag) = read_portion(&stack.custom_data, key)?;
                Some(FluidStack { fluid, amount: amount.saturating_mul(count), tag })
            }
            Kind::Constant { fluid, amount, .. } => Some(FluidStack { fluid: fluid.clone(), amount: amount.saturating_mul(count), tag: None }),
            Kind::TaggedConstant { fluid, amount, .. } => Some(FluidStack { fluid: fluid.clone(), amount: amount.saturating_mul(count), tag: stack_tag(stack) }),
            Kind::Fills(_) => None,
        }
    }

    /// How much the stack holds at most, 0 for an item that is not a container. A bottle's is the amount of its one filling.
    pub fn capacity(stack: &ItemSlot) -> u32 {
        let Some(def) = Self::get(&stack.item.to_string()) else { return 0 };
        let count = stack.count.max(1);
        match &def.kind {
            Kind::Tank { capacity, scaled, .. } => capacity.saturating_mul(if *scaled { count } else { 1 }),
            Kind::Portion { amount, .. } => amount.saturating_mul(count),
            Kind::Constant { amount, .. } | Kind::TaggedConstant { amount, .. } => *amount,
            Kind::Fills(rules) => rules.iter().map(|rule| rule.amount).max().unwrap_or(0),
        }
    }

    /// The stack with `fluid` put in as far as it goes. A tank takes up to what it has room for (of the fluid it holds, or any when empty);
    /// a portion takes exactly its amount when empty; an empty container takes the first filling that accepts the fluid and has enough, and
    /// becomes the filled item. Constant containers and every other item take nothing. Only a stack of one is filled unless the tank is
    /// scaled.
    pub fn fill(stack: &ItemSlot, fluid: &FluidStack) -> Filled {
        let none = Filled { amount: 0, container: stack.clone() };
        let Some(def) = Self::get(&stack.item.to_string()) else { return none };
        let count = stack.count.max(1);
        match &def.kind {
            Kind::Tank { key, capacity, scaled, fill_stack_limit } => {
                if !*scaled && count != 1 || fill_stack_limit.is_some_and(|limit| count > limit) {
                    return none;
                }
                let scale = if *scaled { count } else { 1 };
                let current = read_tank(&stack.custom_data, key);
                if current.as_ref().is_some_and(|(held, _, held_tag)| held != &fluid.fluid || held_tag != &fluid.tag) {
                    return none;
                }
                let held = current.map_or(0, |(_, amount, _)| amount);
                let offered = fluid.amount - fluid.amount % scale;
                let room = capacity.saturating_mul(scale).saturating_sub(held.saturating_mul(scale));
                let taken = offered.min(room);
                if taken == 0 {
                    return none;
                }
                let mut container = stack.clone();
                write_tank(&mut container.custom_data, key, Some((&fluid.fluid, (held.saturating_mul(scale) + taken) / scale, fluid.tag.as_ref())));
                Filled { amount: taken, container }
            }
            Kind::Portion { key, amount } => {
                let total = amount.saturating_mul(count);
                if read_portion(&stack.custom_data, key).is_some() || fluid.amount < total {
                    return none;
                }
                let mut container = stack.clone();
                write_portion(&mut container.custom_data, key, &fluid.fluid, fluid.tag.as_ref());
                Filled { amount: total, container }
            }
            Kind::Constant { .. } | Kind::TaggedConstant { .. } => none,
            Kind::Fills(rules) => {
                if count != 1 {
                    return none;
                }
                let Some(rule) = rules.iter().find(|rule| (rule.accepts)(&fluid.fluid) && fluid.amount >= rule.amount) else { return none };
                let mut container = stack.clone();
                container.item = rule.filled.clone();
                if rule.carry_tag { container.custom_data = fluid.tag.clone().unwrap_or_default(); }
                Filled { amount: rule.amount, container }
            }
        }
    }

    /// The fluid the stack gives up to `max` millibuckets, with the stack afterwards. `wanted` names the fluid asked for: a different one
    /// gives nothing. A tank gives up to `max` (in steps of its count when scaled); a portion and a constant container give their whole
    /// amount or nothing, so `max` must cover it; a constant container becomes its empty item.
    pub fn drain(stack: &ItemSlot, wanted: Option<&Identifier>, max: u32) -> Option<Drained> {
        let def = Self::get(&stack.item.to_string())?;
        let count = stack.count.max(1);
        match &def.kind {
            Kind::Tank { key, scaled, .. } => {
                if !*scaled && count != 1 {
                    return None;
                }
                let scale = if *scaled { count } else { 1 };
                let (fluid, held, tag) = read_tank(&stack.custom_data, key)?;
                if wanted.is_some_and(|wanted| wanted != &fluid) {
                    return None;
                }
                let held_total = held.saturating_mul(scale);
                let taken = (max - max % scale).min(held_total);
                if taken == 0 {
                    return None;
                }
                let mut container = stack.clone();
                let left = (held_total - taken) / scale;
                write_tank(&mut container.custom_data, key, (left > 0).then_some((&fluid, left, tag.as_ref())));
                Some(Drained { fluid: FluidStack { fluid, amount: taken, tag }, container: Some(container) })
            }
            Kind::Portion { key, amount } => {
                let total = amount.saturating_mul(count);
                let (fluid, tag) = read_portion(&stack.custom_data, key)?;
                if max < total || wanted.is_some_and(|wanted| wanted != &fluid) {
                    return None;
                }
                let mut container = stack.clone();
                container.custom_data.remove(key);
                Some(Drained { fluid: FluidStack { fluid, amount: total, tag }, container: Some(container) })
            }
            Kind::Constant { fluid, amount, empty } => {
                if count != 1 || max < *amount || wanted.is_some_and(|wanted| wanted != fluid) {
                    return None;
                }
                let container = empty.as_ref().map(|item| {
                    let mut left = stack.clone();
                    left.item = item.clone();
                    left.custom_data = NbtCompound::new();
                    left
                });
                Some(Drained { fluid: FluidStack { fluid: fluid.clone(), amount: *amount, tag: None }, container })
            }
            Kind::TaggedConstant { fluid, amount, empty } => {
                if count != 1 || max < *amount || wanted.is_some_and(|wanted| wanted != fluid) {
                    return None;
                }
                let container = empty.as_ref().map(|item| {
                    let mut left = stack.clone();
                    left.item = item.clone();
                    left.custom_data = NbtCompound::new();
                    left
                });
                Some(Drained { fluid: FluidStack { fluid: fluid.clone(), amount: *amount, tag: stack_tag(stack) }, container })
            }
            Kind::Fills(_) => None,
        }
    }

    /// A trade between the stack in the hand and a tank, as `FluidUtil.interactWithFluidHandler` does it: in [`TransferDirection::Auto`] the held
    /// stack empties into the tank if the tank takes any of it, otherwise it fills from what the tank holds. `None` when nothing moved (an
    /// empty hand, an item that holds no fluid, a full or empty tank, a fluid nobody takes).
    ///
    /// A container that is not a scaled tank trades one at a time: from a stack of several, one is split off and its result is handed
    /// to the player in `give`. A scaled tank trades as the whole stack.
    pub fn interact(held: Option<&ItemSlot>, target: &mut dyn FluidTarget, direction: TransferDirection) -> Option<Interaction> {
        let held = held?;
        let def = Self::get(&held.item.to_string())?;
        let whole = matches!(def.kind, Kind::Tank { scaled: true, .. });
        let mut unit = held.clone();
        if !whole {
            unit.count = 1;
        }
        let (result, moved, into_tank) = match direction {
            TransferDirection::ToTank => Self::into_tank(&unit, target)?,
            TransferDirection::FromTank => Self::from_tank(&unit, target)?,
            TransferDirection::Auto => match Self::into_tank(&unit, target) {
                Some(done) => done,
                None => Self::from_tank(&unit, target)?,
            },
        };
        if whole || held.count == 1 {
            return Some(Interaction { held: result, give: Vec::new(), moved, into_tank });
        }
        let mut rest = held.clone();
        rest.count -= 1;
        Some(Interaction { held: Some(rest), give: result.into_iter().collect(), moved, into_tank })
    }

    fn into_tank(unit: &ItemSlot, target: &mut dyn FluidTarget) -> Option<(Option<ItemSlot>, FluidStack, bool)> {
        let offered = Self::drain(unit, None, u32::MAX)?;
        let taken = target.fill(&offered.fluid, false);
        if taken == 0 {
            return None;
        }
        let wanted = Self::drain(unit, Some(&offered.fluid.fluid), taken)?;
        let sent = target.fill(&wanted.fluid, true);
        if sent == 0 {
            return None;
        }
        Some((wanted.container, FluidStack { fluid: wanted.fluid.fluid, amount: sent, tag: wanted.fluid.tag }, true))
    }

    fn from_tank(unit: &ItemSlot, target: &mut dyn FluidTarget) -> Option<(Option<ItemSlot>, FluidStack, bool)> {
        let available = target.drain(None, u32::MAX, false)?;
        let filled = Self::fill(unit, &available);
        if filled.amount == 0 {
            return None;
        }
        let taken = target.drain(Some(&available.fluid), filled.amount, true)?;
        if taken.amount != filled.amount {
            return None;
        }
        Some((Some(filled.container), taken, false))
    }
}

/// The fluid tag a tagged container's stack carries: its whole custom data, or none when it carries none.
fn stack_tag(stack: &ItemSlot) -> Option<NbtCompound> {
    (!stack.custom_data.is_empty()).then(|| stack.custom_data.clone())
}

fn read_tank(data: &NbtCompound, key: &str) -> Option<(Identifier, u32, Option<NbtCompound>)> {
    let tag = data.get_compound(key)?;
    let name = tag.get_string("FluidName")?;
    let amount = tag.get_int("Amount", 0);
    if amount <= 0 || name == "minecraft:empty" {
        return None;
    }
    // The fluid's own tag rides as a nested `Tag` compound (Forge's fluid stack format); absent in older data.
    let fluid_tag = tag.get_compound("Tag").cloned();
    Some((Identifier::parse(name).ok()?, amount.min(i64::from(i32::MAX)) as u32, fluid_tag))
}

fn write_tank(data: &mut NbtCompound, key: &str, tank: Option<(&Identifier, u32, Option<&NbtCompound>)>) {
    match tank {
        Some((fluid, amount, fluid_tag)) => {
            let mut tag = NbtCompound::new();
            tag.insert("FluidName".to_owned(), Nbt::String(fluid.to_string()));
            tag.insert("Amount".to_owned(), Nbt::Int(amount.min(i32::MAX as u32) as i32));
            if let Some(fluid_tag) = fluid_tag {
                tag.insert("Tag".to_owned(), Nbt::Compound(fluid_tag.clone()));
            }
            data.insert(key.to_owned(), Nbt::Compound(tag));
        }
        None => {
            data.remove(key);
        }
    }
}

/// A portion's fluid: a bare id string for a plain fluid, or a `{FluidName, Tag}` compound when the
/// fluid carries a tag. Data written before portions carried tags is the bare string and reads the same.
fn read_portion(data: &NbtCompound, key: &str) -> Option<(Identifier, Option<NbtCompound>)> {
    if let Some(tag) = data.get_compound(key) {
        let name = tag.get_string("FluidName")?;
        return Some((Identifier::parse(name).ok()?, tag.get_compound("Tag").cloned()));
    }
    let name = data.get_string(key)?;
    Some((Identifier::parse(name).ok()?, None))
}

fn write_portion(data: &mut NbtCompound, key: &str, fluid: &Identifier, tag: Option<&NbtCompound>) {
    match tag {
        Some(tag) => {
            let mut compound = NbtCompound::new();
            compound.insert("FluidName".to_owned(), Nbt::String(fluid.to_string()));
            compound.insert("Tag".to_owned(), Nbt::Compound(tag.clone()));
            data.insert(key.to_owned(), Nbt::Compound(compound));
        }
        None => {
            data.insert(key.to_owned(), Nbt::String(fluid.to_string()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(text: &str) -> Identifier {
        Identifier::parse(text).unwrap()
    }
    fn stack(text: &str, count: u32) -> ItemSlot {
        ItemSlot::new(id(text), count)
    }
    fn fluid(text: &str, amount: u32) -> FluidStack {
        FluidStack { fluid: id(text), amount, tag: None }
    }

    /// A single fluid tank.
    struct Tank {
        held: Option<FluidStack>,
        capacity: u32,
    }
    impl FluidTarget for Tank {
        fn fill(&mut self, fluid: &FluidStack, execute: bool) -> u32 {
            let current = self.held.as_ref().map_or(0, |held| held.amount);
            if self.held.as_ref().is_some_and(|held| !held.same_fluid(fluid)) {
                return 0;
            }
            let taken = fluid.amount.min(self.capacity - current);
            if execute && taken > 0 {
                self.held = Some(FluidStack { fluid: fluid.fluid.clone(), amount: current + taken, tag: fluid.tag.clone() });
            }
            taken
        }
        fn drain(&mut self, wanted: Option<&Identifier>, amount: u32, execute: bool) -> Option<FluidStack> {
            let held = self.held.as_mut()?;
            if wanted.is_some_and(|wanted| wanted != &held.fluid) {
                return None;
            }
            let taken = held.amount.min(amount);
            let out = FluidStack { fluid: held.fluid.clone(), amount: taken, tag: held.tag.clone() };
            if execute {
                held.amount -= taken;
                if held.amount == 0 {
                    self.held = None;
                }
            }
            Some(out)
        }
    }

    #[test]
    fn a_scaled_tank_fills_and_drains_the_whole_stack_in_steps_of_its_count() {
        FluidItems::register(FluidItemDef::tank(id("fi_test:tank"), "tank", 4000).scaled().fill_stack_limit(16));
        let three = stack("fi_test:tank", 3);
        let filled = FluidItems::fill(&three, &fluid("fi_test:lava", 1000));
        assert_eq!(filled.amount, 999, "a stack of three takes thirds: the remainder of the offer stays out");
        assert_eq!(FluidItems::contents(&filled.container), Some(fluid("fi_test:lava", 999)));
        assert_eq!(filled.container.custom_data.get_compound("tank").unwrap().get_int("Amount", 0), 333, "kept per item");
        assert_eq!(FluidItems::capacity(&three), 12000);
        assert_eq!(FluidItems::fill(&filled.container, &fluid("fi_test:water", 30)).amount, 0, "another fluid is refused while it holds one");
        let drained = FluidItems::drain(&filled.container, None, 500).unwrap();
        assert_eq!(drained.fluid.amount, 498, "steps of three");
        assert_eq!(FluidItems::contents(&drained.container.unwrap()), Some(fluid("fi_test:lava", 501)));
        // draining the rest leaves no tag at all
        let empty = FluidItems::drain(&filled.container, None, u32::MAX).unwrap().container.unwrap();
        assert!(empty.custom_data.is_empty());
        assert_eq!(FluidItems::contents(&empty), None);
        // a stack above the fill limit cannot be filled, but can be drained
        let mut big = stack("fi_test:tank", 17);
        assert_eq!(FluidItems::fill(&big, &fluid("fi_test:lava", 17)).amount, 0);
        write_tank(&mut big.custom_data, "tank", Some((&id("fi_test:lava"), 10, None)));
        assert_eq!(FluidItems::drain(&big, None, 170).unwrap().fluid.amount, 170);
    }

    #[test]
    fn a_portion_is_filled_and_drained_whole_and_scaled_by_the_stack() {
        FluidItems::register(FluidItemDef::portion(id("fi_test:can"), "fluid", 90));
        let can = stack("fi_test:can", 1);
        assert_eq!(FluidItems::fill(&can, &fluid("fi_test:iron", 89)).amount, 0, "less than an ingot goes nowhere");
        let full = FluidItems::fill(&can, &fluid("fi_test:iron", 500));
        assert_eq!(full.amount, 90);
        assert_eq!(full.container.custom_data.get_string("fluid"), Some("fi_test:iron"));
        assert_eq!(FluidItems::fill(&full.container, &fluid("fi_test:iron", 90)).amount, 0, "a full can takes no more");
        assert_eq!(FluidItems::drain(&full.container, None, 89), None);
        assert_eq!(FluidItems::drain(&full.container, Some(&id("fi_test:gold")), 90), None);
        let out = FluidItems::drain(&full.container, Some(&id("fi_test:iron")), 90).unwrap();
        assert_eq!(out.fluid, fluid("fi_test:iron", 90));
        assert!(out.container.unwrap().custom_data.is_empty());
        let four = stack("fi_test:can", 4);
        assert_eq!(FluidItems::fill(&four, &fluid("fi_test:iron", 359)).amount, 0);
        assert_eq!(FluidItems::fill(&four, &fluid("fi_test:iron", 360)).amount, 360, "a stack of four is one container of four ingots");
    }

    #[test]
    fn bottles_fill_from_the_matching_fluid_and_hand_back_the_empty_item() {
        let bottle = id("fi_test:bottle");
        let glass = id("fi_test:glass_bottle");
        FluidItems::register(FluidItemDef::constant(bottle.clone(), id("fi_test:venom"), 250, Some(glass.clone())));
        FluidItems::register(FluidItemDef::empty_container(glass.clone(), |fluid| fluid.path() == "venom", 250, bottle.clone())
            .fills(|fluid| fluid.path() == "magma", 250, id("fi_test:magma_bottle")));
        let empty = stack("fi_test:glass_bottle", 1);
        assert_eq!(FluidItems::fill(&empty, &fluid("fi_test:venom", 249)).amount, 0, "a whole bottle or nothing");
        let venom = FluidItems::fill(&empty, &fluid("fi_test:venom", 1000));
        assert_eq!((venom.amount, venom.container.item.to_string().as_str()), (250, "fi_test:bottle"));
        assert_eq!(FluidItems::fill(&empty, &fluid("fi_test:magma", 250)).container.item.path(), "magma_bottle", "the second filling");
        assert_eq!(FluidItems::fill(&empty, &fluid("fi_test:honey", 250)).amount, 0);
        assert_eq!(FluidItems::contents(&venom.container), Some(fluid("fi_test:venom", 250)));
        assert_eq!(FluidItems::contents(&empty), None);
        let drained = FluidItems::drain(&venom.container, None, 1000).unwrap();
        assert_eq!(drained.fluid, fluid("fi_test:venom", 250));
        assert_eq!(drained.container.unwrap().item, glass);
        // a stack of two bottles does not trade as a whole
        assert_eq!(FluidItems::drain(&stack("fi_test:bottle", 2), None, 1000), None);
        assert!(FluidItems::is_container(&empty) && !FluidItems::is_container(&stack("fi_test:stick", 1)));
    }

    #[test]
    fn a_held_stack_empties_into_the_tank_first_and_otherwise_fills_from_it() {
        let bottle = id("fi_test:b2");
        let glass = id("fi_test:g2");
        FluidItems::register(FluidItemDef::constant(bottle.clone(), id("fi_test:venom"), 250, Some(glass.clone())));
        FluidItems::register(FluidItemDef::empty_container(glass.clone(), |fluid| fluid.path() == "venom", 250, bottle.clone()));
        // two bottles in the hand, an empty tank: one empties, the glass bottle goes to the player
        let mut tank = Tank { held: None, capacity: 1000 };
        let done = FluidItems::interact(Some(&stack("fi_test:b2", 2)), &mut tank, TransferDirection::Auto).unwrap();
        assert!(done.into_tank);
        assert_eq!(tank.held, Some(fluid("fi_test:venom", 250)));
        assert_eq!(done.held.as_ref().map(|held| (held.item.to_string(), held.count)), Some(("fi_test:b2".to_owned(), 1)));
        assert_eq!(done.give.iter().map(|give| give.item.to_string()).collect::<Vec<_>>(), ["fi_test:g2"]);
        // a glass bottle against that tank fills from it
        let done = FluidItems::interact(Some(&stack("fi_test:g2", 3)), &mut tank, TransferDirection::Auto).unwrap();
        assert!(!done.into_tank);
        assert_eq!(tank.held, None, "the tank gave its quarter bucket");
        assert_eq!(done.held.unwrap().count, 2);
        assert_eq!(done.give[0].item, bottle);
        // a tank that refuses (full) and an empty one move nothing, an empty hand too
        let mut full = Tank { held: Some(fluid("fi_test:venom", 1000)), capacity: 1000 };
        assert_eq!(FluidItems::interact(Some(&stack("fi_test:b2", 1)), &mut full, TransferDirection::ToTank), None);
        let mut other = Tank { held: Some(fluid("fi_test:magma", 500)), capacity: 1000 };
        assert_eq!(FluidItems::interact(Some(&stack("fi_test:g2", 1)), &mut other, TransferDirection::Auto), None, "no filling takes magma");
        assert_eq!(FluidItems::interact(None, &mut full, TransferDirection::Auto), None);
        assert_eq!(FluidItems::interact(Some(&stack("fi_test:stick", 1)), &mut full, TransferDirection::Auto), None);
    }

    #[test]
    fn a_fluid_tag_is_kept_by_tanks_and_portions_and_a_different_tag_does_not_mix() {
        let mut tag = NbtCompound::new();
        tag.insert("Potion".to_owned(), Nbt::String("minecraft:healing".to_owned()));
        let potion = FluidStack { fluid: id("minecraft:water"), amount: 500, tag: Some(tag.clone()) };
        FluidItems::register(FluidItemDef::tank(id("fi_test:tagged_tank"), "tank", 1000));
        let filled = FluidItems::fill(&stack("fi_test:tagged_tank", 1), &potion);
        assert_eq!(filled.amount, 500);
        assert_eq!(FluidItems::contents(&filled.container), Some(potion.clone()), "the tag survives the tank");
        assert_eq!(FluidItems::fill(&filled.container, &FluidStack::new(id("minecraft:water"), 100)).amount, 0, "an untagged stack of the same fluid does not mix");
        assert_eq!(FluidItems::fill(&filled.container, &FluidStack { amount: 100, ..potion.clone() }).amount, 100, "the same tag tops up");
        let drained = FluidItems::drain(&filled.container, None, 250).unwrap();
        assert_eq!(drained.fluid, FluidStack { amount: 250, ..potion.clone() });
        // a portion stores a tagged fluid as a compound and reads the bare-string form back as plain
        FluidItems::register(FluidItemDef::portion(id("fi_test:tagged_can"), "fluid", 90));
        let can = FluidItems::fill(&stack("fi_test:tagged_can", 1), &FluidStack { amount: 90, ..potion.clone() });
        assert_eq!(can.amount, 90);
        assert_eq!(FluidItems::contents(&can.container), Some(FluidStack { amount: 90, ..potion }));
        let plain = FluidItems::fill(&stack("fi_test:tagged_can", 1), &fluid("fi_test:iron", 90));
        assert_eq!(plain.container.custom_data.get_string("fluid"), Some("fi_test:iron"), "a plain fluid keeps the bare-string form");
    }

    #[test]
    fn a_tagged_bucket_holds_its_fluid_with_the_stacks_own_tag_and_an_empty_bucket_takes_the_tag_on_filling() {
        let bucket = id("fi_test:potion_bucket");
        let empty_bucket = id("fi_test:bucket");
        FluidItems::register(FluidItemDef::tagged_constant(bucket.clone(), id("fi_test:potion"), 1000, Some(empty_bucket.clone())));
        FluidItems::register(FluidItemDef::empty_container_tagged(empty_bucket.clone(), |fluid| fluid.path() == "potion", 1000, bucket.clone()));
        let mut tag = NbtCompound::new();
        tag.insert("Potion".to_owned(), Nbt::String("minecraft:healing".to_owned()));
        let potion = FluidStack { fluid: id("fi_test:potion"), amount: 1000, tag: Some(tag.clone()) };
        // an empty bucket filled with a tagged stack of the fluid takes the tag as its own data
        let filled = FluidItems::fill(&stack("fi_test:bucket", 1), &potion);
        assert_eq!((filled.amount, filled.container.item.clone(), filled.container.custom_data.clone()), (1000, bucket.clone(), tag.clone()));
        assert_eq!(FluidItems::contents(&filled.container), Some(potion.clone()), "the bucket holds the fluid with the tag");
        assert_eq!(FluidItems::capacity(&filled.container), 1000);
        // a bucket with no data holds the plain fluid, and one filled from a plain stack stays plain
        assert_eq!(FluidItems::contents(&stack("fi_test:potion_bucket", 1)), Some(FluidStack { tag: None, ..potion.clone() }));
        let plain = FluidItems::fill(&stack("fi_test:bucket", 1), &FluidStack { tag: None, ..potion.clone() });
        assert_eq!(plain.amount, 1000);
        assert!(plain.container.custom_data.is_empty());
        // draining hands the tag back with the fluid and leaves a plain empty bucket
        let drained = FluidItems::drain(&filled.container, None, 1000).unwrap();
        assert_eq!(drained.fluid, potion);
        let left = drained.container.unwrap();
        assert_eq!((left.item, left.custom_data.is_empty()), (empty_bucket, true));
        assert_eq!(FluidItems::drain(&filled.container, Some(&id("fi_test:water")), 1000), None, "another fluid is not in it");
        assert_eq!(FluidItems::fill(&filled.container, &potion).amount, 0, "a full bucket takes nothing");
    }

    #[test]
    fn a_scaled_tank_in_the_hand_trades_as_the_whole_stack() {
        FluidItems::register(FluidItemDef::tank(id("fi_test:t3"), "tank", 1000).scaled());
        let mut tank = Tank { held: Some(fluid("fi_test:lava", 500)), capacity: 10_000 };
        let done = FluidItems::interact(Some(&stack("fi_test:t3", 2)), &mut tank, TransferDirection::FromTank).unwrap();
        let held = done.held.unwrap();
        assert_eq!(held.count, 2, "no split off");
        assert!(done.give.is_empty());
        assert_eq!(FluidItems::contents(&held), Some(fluid("fi_test:lava", 500)));
        assert_eq!(tank.held, None);
        let back = FluidItems::interact(Some(&held), &mut Tank { held: None, capacity: 300 }, TransferDirection::ToTank).unwrap();
        assert_eq!(back.moved.amount, 300, "only what the tank takes leaves the stack");
        assert_eq!(FluidItems::contents(&back.held.unwrap()), Some(fluid("fi_test:lava", 200)));
    }
}
