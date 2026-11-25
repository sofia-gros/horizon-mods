//! The Tinkers items that hold fluid, from SlimeKnights TinkerFluids, TinkerSmeltery, TankItem, CopperCanItem, ContainerFoodItem,
//! MagmaBottleItem and FluidEvents (MIT; see LICENSE), registered as PlutoLib fluid containers:
//! - the eight seared and scorched tank items: a scaled tank of their block's capacity under `tank` (TankItemFluidHandler), which takes
//!   no fluid while the stack holds more than 16 (`limitStackSize`);
//! - the copper can: exactly one ingot of one fluid, kept as the fluid's id under `fluid`, scaled by the stack (CopperCanFluidHandler);
//! - the venom, slime and magma bottles (250 of their fluid, a glass bottle back) and the meat soup bowl (250, a bowl back), and powdered
//!   snow in the vanilla powder snow bucket (ConstantFluidContainerWrapper);
//! - the transfers the original ships as data (`fluid_transfers`): a glass bottle fills with 250 of a bottle's fluid, a bowl with meat soup.
//! The tank items with `limitStackSize` also register PlutoLib's per-stack maximum: a stack holding fluid stacks to 16, an empty one
//! delegates to the item's own maximum (menu stacking honours it today; the game's general stacking seam is PlutoLib's remaining
//! host work, see the support matrix).
//! The bottles and the soup are drunk (32 ticks, the soup feeds 8 hunger at 0.6 saturation modifier) and the bottles give the effects of
//! TinkerFood. Not ported, each named: the magma bottle's 15 seconds of fire (a finish callback cannot set an entity on fire), the effects
//! of Tinkers that are not ported yet are skipped by the game when given, the splash and lingering bottles (brewing and the cauldron
//! interactions that make them have no PlutoLib registry), drinking the potion bucket (nothing here can read a potion's contents out of
//! a stack's tag, so its effects at 2.5 times and the empty bucket it leaves are unported; the bucket as a container is registered with
//! the fluids, its tag the fluid's own), the tooltips with the
//! fluid, the fluid layer of the copper can's model, the sound of a transfer, and trading with the drains and ducts of a smeltery (their
//! use hook cannot reach the master's tank).
use crate::{
    fluid_transfers::{Kind, TRANSFERS},
    fluid_values::{BOTTLE, INGOT},
    tank,
};
use pluto_lib::{
    ActionResult, BlockEntityUse, EffectInstance, FluidItemDef, FluidItems, FluidStack, FluidTarget, Identifier, ItemBehavior, ItemBehaviors, ItemUse, ItemUseRegistry,
    TransferDirection, UseAction, UseItemFinishCallback,
};

fn id(text: &str) -> Identifier { Identifier::parse(text).expect("fixed item and fluid ids are valid") }
fn tc(path: &str) -> Identifier { Identifier::new("tconstruct", path).expect("fixed item and fluid ids are valid") }

/// `TankItem.limitStackSize`: a stack larger than this takes no fluid.
const TANK_FILL_LIMIT: u32 = 16;

/// The bottles of TinkerFluids: item, fluid and the effects drinking it gives (TinkerFood: effect, ticks, amplifier).
pub(crate) const BOTTLES: [(&str, &str, &[(&str, i32, u8)]); 6] = [
    ("venom_bottle", "venom", &[("minecraft:strength", 30 * 20, 0), ("minecraft:poison", 250, 0)]),
    ("earth_slime_bottle", "earth_slime", &[("tconstruct:experienced", 120 * 20, 0), ("minecraft:slowness", 120 * 20, 1)]),
    ("sky_slime_bottle", "sky_slime", &[("tconstruct:ricochet", 120 * 20, 0), ("minecraft:slowness", 120 * 20, 1)]),
    ("ichor_slime_bottle", "ichor", &[("minecraft:levitation", 10 * 20, 0), ("minecraft:slowness", 10 * 20, 1)]),
    ("ender_slime_bottle", "ender_slime", &[("tconstruct:enderference", 60 * 20, 0), ("minecraft:slowness", 60 * 20, 1)]),
    ("magma_bottle", "magma", &[]),
];

/// The effects drinking an item gives; empty for any other item.
pub(crate) fn drunk_effects(item: &str) -> &'static [(&'static str, i32, u8)] {
    BOTTLES.iter().find(|(bottle, ..)| item == format!("tconstruct:{bottle}")).map_or(&[], |(_, _, effects)| effects)
}

/// A single fluid tank as a trade partner: a held container fills it and drains it. `capacity` is its ceiling.
pub(crate) struct TankTarget { pub fluid: Option<FluidStack>, pub capacity: u32 }

impl FluidTarget for TankTarget {
    fn fill(&mut self, fluid: &FluidStack, execute: bool) -> u32 {
        let held = self.fluid.as_ref().map_or(0, |held| held.amount);
        if self.fluid.as_ref().is_some_and(|held| !held.same_fluid(fluid)) { return 0; }
        let taken = fluid.amount.min(self.capacity.saturating_sub(held));
        if execute && taken > 0 { self.fluid = Some(FluidStack { fluid: fluid.fluid.clone(), amount: held + taken, tag: fluid.tag.clone() }); }
        taken
    }
    fn drain(&mut self, wanted: Option<&Identifier>, amount: u32, execute: bool) -> Option<FluidStack> {
        let held = self.fluid.as_ref()?;
        if wanted.is_some_and(|wanted| wanted != &held.fluid) { return None; }
        let taken = held.amount.min(amount);
        if taken == 0 { return None; }
        let out = FluidStack { fluid: held.fluid.clone(), amount: taken, tag: held.tag.clone() };
        if execute {
            self.fluid = (held.amount > taken).then(|| FluidStack { fluid: out.fluid.clone(), amount: held.amount - taken, tag: out.tag.clone() });
        }
        Some(out)
    }
}

/// `FluidTransferHelper.interactWithTank` on a block entity with one declared tank (the melter): the held container empties into the tank
/// or fills from it; `Pass` when nothing moved, so the game goes on to open the window.
pub(crate) fn use_tank(used: &mut BlockEntityUse<'_>, capacity: u32) -> ActionResult {
    let mut target = TankTarget { fluid: used.fluid(0), capacity };
    let Some(done) = FluidItems::interact(used.held.as_ref(), &mut target, TransferDirection::Auto) else { return ActionResult::Pass };
    used.set_fluid(0, target.fluid);
    used.held = done.held;
    used.give.extend(done.give);
    ActionResult::Success
}

/// Registers every container, transfer and drink. Called once from the product's hooks (after the fluids, so the buckets' fillings are known).
/// `TankItem.getMaxStackSize` for the items with `limitStackSize`: a stack holding fluid (its data
/// carries the `tank` compound) stacks to 16; an empty stack delegates to the item's own maximum.
pub(crate) fn register_tank_stack_limit(item: Identifier) {
    let _ = ItemBehaviors::register(ItemBehavior::new(item).max_stack_size(|context| {
        context.custom_data.contains_key(tank::TAG_TANK).then_some(TANK_FILL_LIMIT)
    }));
}

pub(crate) fn register() {
    for (name, capacity) in tank::blocks() {
        FluidItems::register(FluidItemDef::tank(tc(&name), tank::TAG_TANK, capacity as u32).scaled().fill_stack_limit(TANK_FILL_LIMIT));
        register_tank_stack_limit(tc(&name));
    }
    FluidItems::register(FluidItemDef::portion(tc("copper_can"), "fluid", INGOT as u32));
    FluidItems::register(FluidItemDef::constant(id("minecraft:powder_snow_bucket"), tc("powdered_snow"), 1000, Some(id("minecraft:bucket"))));
    let glass_bottle = id("minecraft:glass_bottle");
    for (item, fluid, _) in BOTTLES {
        FluidItems::register(FluidItemDef::constant(tc(item), tc(fluid), BOTTLE as u32, Some(glass_bottle.clone())));
    }
    FluidItems::register(FluidItemDef::constant(tc("meat_soup"), tc("meat_soup"), 250, Some(id("minecraft:bowl"))));
    // the data transfers: each empty item takes its fillings in file order
    let mut empties: Vec<(&str, FluidItemDef)> = Vec::new();
    for transfer in TRANSFERS.iter().filter(|transfer| transfer.kind == Kind::Fill) {
        let names: Vec<&'static str> = transfer.fluids.to_vec();
        let accepts = move |fluid: &Identifier| fluid.namespace() == "tconstruct" && names.contains(&fluid.path());
        let result = id(transfer.result);
        match empties.iter().position(|(item, _)| *item == transfer.item) {
            Some(at) => { let (item, def) = empties.remove(at); empties.insert(at, (item, def.fills(accepts, transfer.amount, result))); }
            None => empties.push((transfer.item, FluidItemDef::empty_container(id(transfer.item), accepts, transfer.amount, result))),
        }
    }
    for (_, def) in empties { FluidItems::register(def); }
    register_drinks();
}

fn register_drinks() {
    for (item, _, _) in BOTTLES {
        // ContainerFoodItem: 32 ticks of drinking, always drinkable, a glass bottle back
        ItemUseRegistry::register(tc(item), ItemUse::new(32, UseAction::Drink).food(0, 0.0).can_always_eat().remainder(id("minecraft:glass_bottle"), 1));
    }
    // TinkerFood.MEAT_SOUP: nutrition 8, saturation modifier 0.6 (saturation 8 * 0.6 * 2), a bowl back, stacking to 1
    ItemUseRegistry::register(tc("meat_soup"), ItemUse::new(32, UseAction::Drink).food(8, 9.6).remainder(id("minecraft:bowl"), 1));
    UseItemFinishCallback::EVENT.register(|done| {
        for (effect, duration, amplifier) in drunk_effects(&done.item) {
            done.player.add_effect(&EffectInstance::new(effect, *duration).amplifier(*amplifier));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::{BlockEntityUse, ItemSlot, NbtCompound};

    fn stack(text: &str, count: u32) -> ItemSlot { ItemSlot::new(id(text), count) }
    fn fluid(text: &str, amount: u32) -> FluidStack { FluidStack { fluid: id(text), amount, tag: None } }

    #[test]
    fn the_shipped_transfers_fill_a_glass_bottle_and_a_bowl_and_empty_the_soup() {
        assert_eq!(TRANSFERS.len(), 8, "the two Twilight Forest ones are left out");
        crate::fluids::register();
        register();
        let bottle = stack("minecraft:glass_bottle", 1);
        for (item, fluid_name, _) in BOTTLES {
            let filled = FluidItems::fill(&bottle, &fluid(&format!("tconstruct:{fluid_name}"), 1000));
            assert_eq!((filled.amount, filled.container.item.to_string()), (250, format!("tconstruct:{item}")), "{fluid_name}");
            assert_eq!(FluidItems::drain(&filled.container, None, 250).unwrap().container.unwrap().item.to_string(), "minecraft:glass_bottle");
        }
        assert_eq!(FluidItems::fill(&bottle, &fluid("tconstruct:meat_soup", 249)).amount, 0, "a whole bottle or nothing");
        assert_eq!(FluidItems::fill(&bottle, &fluid("tconstruct:molten_iron", 1000)).amount, 0);
        let bowl = FluidItems::fill(&stack("minecraft:bowl", 1), &fluid("tconstruct:meat_soup", 500));
        assert_eq!((bowl.amount, bowl.container.item.to_string()), (250, "tconstruct:meat_soup".to_owned()));
        let emptied = FluidItems::drain(&stack("tconstruct:meat_soup", 1), None, 1000).unwrap();
        assert_eq!((emptied.fluid, emptied.container.unwrap().item.to_string()), (fluid("tconstruct:meat_soup", 250), "minecraft:bowl".to_owned()));
        // powdered snow comes out of the vanilla bucket as a bucket
        assert_eq!(FluidItems::drain(&stack("minecraft:powder_snow_bucket", 1), None, 1000).unwrap().fluid, fluid("tconstruct:powdered_snow", 1000));
    }

    #[test]
    fn tank_items_scale_with_the_stack_and_the_copper_can_holds_one_ingot() {
        register();
        let mut tanks = stack("tconstruct:seared_fuel_tank", 2);
        let filled = FluidItems::fill(&tanks, &fluid("minecraft:lava", 20_000));
        assert_eq!(filled.amount, 8000, "two tanks of four buckets");
        assert_eq!(FluidItems::contents(&filled.container), Some(fluid("minecraft:lava", 8000)));
        assert_eq!({ let mut one = filled.container.clone(); one.count = 1; tank::item_tank(&one).unwrap().amount() }, 4000, "kept per tank, as TankItem reads it");
        tanks.count = 17;
        assert_eq!(FluidItems::fill(&tanks, &fluid("minecraft:lava", 17_000)).amount, 0, "a stack above 16 takes no fluid");
        assert_eq!(FluidItems::capacity(&stack("tconstruct:scorched_ingot_gauge", 1)), 4320);
        let can = stack("tconstruct:copper_can", 1);
        assert_eq!(FluidItems::fill(&can, &fluid("tconstruct:molten_gold", 89)).amount, 0);
        let full = FluidItems::fill(&can, &fluid("tconstruct:molten_gold", 90)).container;
        assert_eq!(full.custom_data.get("fluid").and_then(|tag| tag.as_str()), Some("tconstruct:molten_gold"));
        assert_eq!(FluidItems::drain(&full, None, 90).unwrap().fluid, fluid("tconstruct:molten_gold", 90));
    }

    #[test]
    fn a_held_bucket_trades_with_a_tank_blocks_fluid_and_the_melter_passes_when_nothing_moves() {
        crate::fluids::register();
        register();
        let mut data = NbtCompound::new();
        let mut items = Vec::new();
        let mut used = BlockEntityUse { pos: [0; 3], data: &mut data, items: &mut items, held: Some(stack("tconstruct:molten_iron_bucket", 1)), sneaking: false, give: Vec::new() };
        assert_eq!(use_tank(&mut used, 4000), ActionResult::Success);
        assert_eq!(used.fluid(0), Some(fluid("tconstruct:molten_iron", 1000)));
        assert_eq!(used.held.as_ref().map(|held| held.item.to_string()), Some("minecraft:bucket".to_owned()), "the bucket is empty now");
        // the empty bucket takes it back
        assert_eq!(use_tank(&mut used, 4000), ActionResult::Success);
        assert_eq!(used.fluid(0), None);
        assert_eq!(used.held.as_ref().map(|held| held.item.to_string()), Some("tconstruct:molten_iron_bucket".to_owned()));
        // a stick does nothing, and an empty hand neither
        used.held = Some(stack("minecraft:stick", 1));
        assert_eq!(use_tank(&mut used, 4000), ActionResult::Pass);
        used.held = None;
        assert_eq!(use_tank(&mut used, 4000), ActionResult::Pass);
    }

    #[test]
    fn a_held_vanilla_bucket_trades_with_a_tank_too() {
        crate::fluids::register();
        register();
        let mut data = NbtCompound::new();
        let mut items = Vec::new();
        let mut used = BlockEntityUse { pos: [0; 3], data: &mut data, items: &mut items, held: Some(stack("minecraft:lava_bucket", 1)), sneaking: false, give: Vec::new() };
        // the lava bucket empties into the tank and the hand keeps an empty bucket
        assert_eq!(use_tank(&mut used, 4000), ActionResult::Success);
        assert_eq!(used.fluid(0), Some(fluid("minecraft:lava", 1000)));
        assert_eq!(used.held.as_ref().map(|held| held.item.to_string()), Some("minecraft:bucket".to_owned()), "the bucket is empty now");
        // the empty bucket takes the lava back as a lava bucket
        assert_eq!(use_tank(&mut used, 4000), ActionResult::Success);
        assert_eq!(used.fluid(0), None);
        assert_eq!(used.held.as_ref().map(|held| held.item.to_string()), Some("minecraft:lava_bucket".to_owned()));
        // water trades the same way
        used.held = Some(stack("minecraft:water_bucket", 1));
        assert_eq!(use_tank(&mut used, 4000), ActionResult::Success);
        assert_eq!(used.fluid(0), Some(fluid("minecraft:water", 1000)));
        assert_eq!(used.held.as_ref().map(|held| held.item.to_string()), Some("minecraft:bucket".to_owned()));
    }

    #[test]
    fn the_bottles_give_their_effects_and_the_original_foods_are_registered() {
        register();
        assert_eq!(drunk_effects("tconstruct:venom_bottle"), &[("minecraft:strength", 600, 0), ("minecraft:poison", 250, 0)]);
        assert!(drunk_effects("tconstruct:magma_bottle").is_empty());
        assert!(drunk_effects("minecraft:potion").is_empty());
        let soup = ItemUseRegistry::get("tconstruct:meat_soup").unwrap();
        assert_eq!((soup.use_time, soup.food.map(|food| food.nutrition)), (32, Some(8)));
        assert!(ItemUseRegistry::get("tconstruct:earth_slime_bottle").unwrap().can_always_eat);
    }
}
