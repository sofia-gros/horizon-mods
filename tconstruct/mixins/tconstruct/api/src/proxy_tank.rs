//! The proxy tank, from SlimeKnights ProxyTankBlock, ProxyTankBlockEntity and ProxyItemTank (MIT; see LICENSE):
//! a block whose fluid storage is the fluid container item placed inside it. Neighbours fill and drain the
//! item through the block, the comparator reads the item's fill, and using the block trades fluid with the
//! held container or swaps the stored item around. Only containers that fill and drain a little at a time are
//! accepted (`ProxyItemTank.isItemValid`): the `tconstruct:proxy_tank_blacklist` tag keeps out the bucket, the
//! glass bottle, the bowl and the copper can — and anything whose container item is one of those, as a filled
//! bucket leaves a bucket — and the tag's `#tconstruct:modifiable/ammo` members are kept out through the item
//! snapshot.
//!
//! Not ported, each named: the click split of the original's `use` (the corner posts trade fluid, the center
//! and the bottom swap the item) — the use hook carries no hit position or face, so a fluid trade is tried
//! first and the item swap second, as the fluid cannons do; the tank listing a neighbour reads — PlutoLib's
//! reader hook is given the saved data but not the item slots, and here the item is the storage, so fills and
//! drains (which do receive the slots) work while the listing cannot be answered; the stored item and its
//! fluid drawn in the corner posts (the render data is already generated; no renderer is registered for this
//! block) and the comparator output itself, which needs the host — the strength rule is `comparator` here.

use crate::melter_blocks::facing_toward;
use pluto_lib::{
    ActionResult, BlockBehavior, BlockBehaviors, BlockEntityDrain, BlockEntityFill, BlockEntityType, BlockEntityTypeRegistry, BlockEntityUse, FluidItems,
    FluidStack, FluidTarget, Identifier, ItemSlot, Placement, TransferDirection,
};

pub(crate) const BLOCK: &str = "scorched_proxy_tank";
/// The direct members of `tconstruct:proxy_tank_blacklist`; the tag's `#tconstruct:modifiable/ammo` part is a
/// tag lookup, as a tag's members are data.
const BLACKLIST: [&str; 4] = ["minecraft:bucket", "minecraft:glass_bottle", "minecraft:bowl", "tconstruct:copper_can"];

fn stored(items: &[Option<ItemSlot>]) -> Option<&ItemSlot> { items.first().and_then(Option::as_ref) }

/// Whether `item` is in `tconstruct:modifiable/ammo`, from the captured item snapshot; false while none was
/// captured.
fn in_ammo_tag(item: &Identifier) -> bool {
    let tag = Identifier::new("tconstruct", "modifiable/ammo").expect("valid tag id");
    crate::common::item_snapshot().and_then(|tags| tags.get_tag(&tag).map(|members| members.contains(item))).unwrap_or(false)
}

/// The item a stack leaves behind (`Item.getCraftingRemainingItem`), read off what draining it whole leaves:
/// a filled bucket leaves a bucket; a tank item leaves itself, which is no remainder.
fn crafting_remainder(stack: &ItemSlot) -> Option<Identifier> {
    let container = FluidItems::drain(stack, None, u32::MAX)?.container?;
    (container.item != stack.item).then_some(container.item)
}

/// `ProxyItemTank.isItemValid`: a fluid container that is not blacklisted, whose container item is not either.
pub(crate) fn accepts(stack: &ItemSlot, in_ammo_tag: impl Fn(&Identifier) -> bool) -> bool {
    if !FluidItems::is_container(stack) { return false; }
    let banned = |item: &Identifier| BLACKLIST.contains(&item.to_string().as_str()) || in_ammo_tag(item);
    if banned(&stack.item) { return false; }
    match crafting_remainder(stack) { Some(remainder) => !banned(&remainder), None => true }
}

/// The stored item's fluid (`ProxyItemTank.getFluidInTank(0)`): the block's one tank is the item's.
pub(crate) fn contents(items: &[Option<ItemSlot>]) -> Option<FluidStack> { stored(items).and_then(FluidItems::contents) }

/// The stored item's capacity (`ProxyItemTank.getTankCapacity(0)`), 0 with no item inside.
pub(crate) fn capacity(items: &[Option<ItemSlot>]) -> u32 { stored(items).map_or(0, FluidItems::capacity) }

/// `ProxyTankBlockEntity.calculateComparatorStrength`: 0 with no capacity, else 1 + 14 * amount / capacity —
/// an item holding nothing still reads 1.
pub(crate) fn comparator(items: &[Option<ItemSlot>]) -> i32 {
    let capacity = capacity(items);
    if capacity == 0 { return 0; }
    1 + 14 * contents(items).map_or(0, |fluid| fluid.amount) as i32 / capacity as i32
}

/// The stored item as a trade partner: fills and drains go through it, and what it becomes replaces it.
struct ItemTarget { stack: Option<ItemSlot> }

impl FluidTarget for ItemTarget {
    fn fill(&mut self, fluid: &FluidStack, execute: bool) -> u32 {
        let Some(current) = &self.stack else { return 0 };
        let filled = FluidItems::fill(current, fluid);
        if execute && filled.amount > 0 { self.stack = Some(filled.container); }
        filled.amount
    }
    fn drain(&mut self, wanted: Option<&Identifier>, amount: u32, execute: bool) -> Option<FluidStack> {
        let drained = FluidItems::drain(self.stack.as_ref()?, wanted, amount)?;
        if execute { self.stack = drained.container.clone(); }
        Some(drained.fluid)
    }
}

/// `ProxyItemTank.fill`: into the stored item.
pub(crate) fn fill_tank(fill: &mut BlockEntityFill<'_>) -> u32 {
    let Some(current) = stored(fill.items).cloned() else { return 0 };
    let filled = FluidItems::fill(&current, &fill.fluid);
    if filled.amount > 0 {
        if let Some(cell) = fill.items.get_mut(0) { *cell = Some(filled.container); }
    }
    filled.amount
}

/// `ProxyItemTank.drain`: out of the stored item; an item the drain consumes entirely leaves the slot empty.
pub(crate) fn drain_tank(drain: &mut BlockEntityDrain<'_>) -> Option<FluidStack> {
    let current = stored(drain.items)?.clone();
    let drained = FluidItems::drain(&current, drain.wanted.as_ref(), drain.amount)?;
    if let Some(cell) = drain.items.get_mut(0) { *cell = drained.container.clone(); }
    Some(drained.fluid)
}

/// `ProxyTankBlockEntity.interact` without the click split (see the module notes): with an item inside, the
/// held container trades fluid with it first; then the items swap — a held container goes in (one, the slot's
/// limit), an empty hand takes the item out, and with both full the stored stack goes to the player's
/// inventory (what does not fit stays in the original; here it drops at the player's feet with the rest).
fn use_proxy_tank(used: &mut BlockEntityUse<'_>) -> ActionResult {
    if let Some(current) = stored(used.items).cloned() {
        if used.held.is_some() {
            let mut target = ItemTarget { stack: Some(current) };
            if let Some(done) = FluidItems::interact(used.held.as_ref(), &mut target, TransferDirection::Auto) {
                if let Some(cell) = used.items.get_mut(0) { *cell = target.stack; }
                used.held = done.held;
                used.give.extend(done.give);
                return ActionResult::Success;
            }
        }
    }
    match (stored(used.items).cloned(), used.held.clone()) {
        (None, Some(held)) if accepts(&held, in_ammo_tag) => {
            let mut one = held.clone();
            one.count = 1;
            if let Some(cell) = used.items.get_mut(0) { *cell = Some(one); }
            used.held = if held.count > 1 { let mut rest = held; rest.count -= 1; Some(rest) } else { None };
        }
        (Some(stored), None) => {
            if let Some(cell) = used.items.get_mut(0) { *cell = None; }
            used.held = Some(stored);
        }
        (Some(stored), Some(_)) => {
            if let Some(cell) = used.items.get_mut(0) { *cell = None; }
            used.give.push(stored);
        }
        _ => {}
    }
    ActionResult::Success
}

pub(crate) fn register() {
    let block = Identifier::new("tconstruct", BLOCK).expect("valid proxy tank id");
    BlockEntityTypeRegistry::register(
        BlockEntityType::new(block.clone()).item_slots(1).sync_data()
            .fluid_filler(fill_tank)
            .fluid_drainer(drain_tank)
            .on_use(use_proxy_tank),
    );
    // ProxyTankBlock.getStateForPlacement: facing the placer
    BlockBehaviors::register(BlockBehavior::new(block).state_for_placement(|c| Placement::State(c.state.clone().with("facing", facing_toward(c.yaw)))));
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::{BlockEntityUse, NbtCompound};

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
    fn stack(text: &str, count: u32) -> ItemSlot { ItemSlot::new(id(text), count) }
    fn fluid(text: &str, amount: u32) -> FluidStack { FluidStack { fluid: id(text), amount, tag: None } }

    fn registered() {
        crate::fluids::register();
        crate::fluid_items::register();
        crate::casting_tank::register();
        register();
    }

    #[test]
    fn the_block_entity_is_registered_with_one_slot() {
        registered();
        assert!(BlockEntityTypeRegistry::get("tconstruct:scorched_proxy_tank").is_some());
    }

    #[test]
    fn only_incremental_containers_are_accepted_and_the_blacklist_wins() {
        registered();
        let no_ammo = |_: &Identifier| false;
        assert!(accepts(&stack("tconstruct:seared_fuel_tank", 1), no_ammo), "a tank item fills a little at a time");
        assert!(accepts(&stack("tconstruct:seared_casting_tank", 1), no_ammo));
        assert!(!accepts(&stack("tconstruct:copper_can", 1), no_ammo), "the can only trades whole: blacklisted");
        assert!(!accepts(&stack("minecraft:bucket", 1), no_ammo), "the empty bucket is blacklisted itself");
        assert!(!accepts(&stack("tconstruct:molten_iron_bucket", 1), no_ammo), "a filled bucket leaves a bucket");
        assert!(!accepts(&stack("minecraft:glass_bottle", 1), no_ammo));
        assert!(!accepts(&stack("minecraft:stick", 1), no_ammo), "not a container at all");
        // the tag's ammo part, through the lookup the production check feeds from the item snapshot
        assert!(!accepts(&stack("tconstruct:seared_fuel_tank", 1), |item| item.path() == "seared_fuel_tank"));
    }

    #[test]
    fn fills_and_drains_go_through_the_stored_item_and_the_comparator_reads_it() {
        registered();
        let mut data = NbtCompound::new();
        let mut items: Vec<Option<ItemSlot>> = vec![Some(stack("tconstruct:seared_fuel_tank", 1))];
        assert_eq!(comparator(&items), 1, "an empty container still reads 1");
        let mut fill = BlockEntityFill { pos: [0; 3], data: &mut data, items: &mut items, fluid: fluid("minecraft:lava", 2500), face: None, execute: true };
        assert_eq!(fill_tank(&mut fill), 2500);
        assert_eq!(contents(&items), Some(fluid("minecraft:lava", 2500)));
        assert_eq!(capacity(&items), 4000);
        assert_eq!(comparator(&items), 1 + 14 * 2500 / 4000);
        let mut drain = BlockEntityDrain { pos: [0; 3], data: &mut data, items: &mut items, wanted: None, amount: 500, execute: true };
        assert_eq!(drain_tank(&mut drain), Some(fluid("minecraft:lava", 500)));
        assert_eq!(contents(&items), Some(fluid("minecraft:lava", 2000)));
        // no item inside: nothing fills, nothing drains, no strength
        let mut items: Vec<Option<ItemSlot>> = vec![None];
        let mut fill = BlockEntityFill { pos: [0; 3], data: &mut data, items: &mut items, fluid: fluid("minecraft:lava", 100), face: None, execute: true };
        assert_eq!(fill_tank(&mut fill), 0);
        let mut drain = BlockEntityDrain { pos: [0; 3], data: &mut data, items: &mut items, wanted: None, amount: 100, execute: true };
        assert_eq!(drain_tank(&mut drain), None);
        assert_eq!(comparator(&items), 0);
    }

    #[test]
    fn using_the_block_swaps_the_item_and_trades_fluid_with_it() {
        registered();
        let mut data = NbtCompound::new();
        let mut items: Vec<Option<ItemSlot>> = vec![None];
        // a held container goes in, one at a time; a plain item does not
        let mut used = BlockEntityUse { pos: [0; 3], data: &mut data, items: &mut items, held: Some(stack("minecraft:stick", 1)), sneaking: false, give: Vec::new() };
        assert_eq!(use_proxy_tank(&mut used), ActionResult::Success);
        assert_eq!(used.items[0], None);
        used.held = Some(stack("tconstruct:seared_fuel_tank", 2));
        assert_eq!(use_proxy_tank(&mut used), ActionResult::Success);
        assert_eq!(used.items[0], Some(stack("tconstruct:seared_fuel_tank", 1)));
        assert_eq!(used.held, Some(stack("tconstruct:seared_fuel_tank", 1)));
        // the held bucket trades with the item inside: the bucket empties into it and comes back empty
        used.held = Some(stack("tconstruct:molten_iron_bucket", 1));
        assert_eq!(use_proxy_tank(&mut used), ActionResult::Success);
        assert_eq!(contents(used.items), Some(fluid("tconstruct:molten_iron", 1000)));
        assert_eq!(used.held.as_ref().map(|held| held.item.to_string()), Some("minecraft:bucket".to_owned()));
        // an empty hand takes the item back out
        used.held = None;
        assert_eq!(use_proxy_tank(&mut used), ActionResult::Success);
        assert_eq!(used.items[0], None);
        assert_eq!(used.held.as_ref().map(|held| held.item.to_string()), Some("tconstruct:seared_fuel_tank".to_owned()));
        assert_eq!(FluidItems::contents(used.held.as_ref().unwrap()), Some(fluid("tconstruct:molten_iron", 1000)), "the fluid stayed in the item");
    }
}
