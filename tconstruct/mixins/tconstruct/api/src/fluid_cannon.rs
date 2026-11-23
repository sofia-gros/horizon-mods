//! The fluid cannons, from SlimeKnights FluidCannonBlock, KnightMetalFluidCannonBlock and FluidCannonBlockEntity (MIT; see
//! LICENSE): a seared tank that also shoots its fluid. The tank half is the tank's own (`crate::tank`): one fluid up to the
//! block's capacity, saved under `tank`, the `light` state following the fluid. The cannon adds a single item slot (the
//! original's `item` handler, one stack of anything), a six-way facing taken from the placer's look, and the redstone
//! trigger: a rising neighbour signal sets `triggered` and the shot follows four ticks later, once per rising edge.
//! Shooting is `FluidCannonBlockEntity.shoot` whole (see [`shoot`]): the fluid's effects run against the block in front
//! when they can, else the fluid flies as a `FluidEffectProjectile` (`crate::fluid_projectile`) of effects-amount times
//! power, else the shot fails with the dispenser fail sound. A shot the block in front consumes sends the
//! original's fluid particles at the hit location, carrying the tank's fluid (the send is real; the fluid
//! particle's per-spawn sprite awaits a host channel — see `crate::particles`). What has no reach is not
//! faked: the fail's BLOCK_ACTIVATE game event (no game event surface), and the face sturdiness that
//! gates the projectile is answered by approximation, as [`shoot`] notes. The use hook is told no click position, so it
//! cannot split the block into the original's tank zone (the top half) and item zone: a held fluid container trades with
//! the tank first, and anything else swaps with the item slot. The end cannon's knightmetal bite is ported (the
//! entity_inside callback below), and so is the plain knightmetal block's identical bite — KnightMetalBlock.entityInside
//! is the body KnightMetalFluidCannonBlock's is copied from: an entity inside takes KnightMetalBlock.BLOCK_DAMAGE of
//! tconstruct:knightmetal damage, queued as a hurt action and landed by the game's own hurt call where the contact
//! actions drain. The host's hurt call reaches living entities only, where the original calls hurt on any entity: a
//! non-living one takes nothing from the original's call in practice either, but here the refusal is the host's, not
//! the damage type's. Both blocks' path type is ported. What a broken cannon drops of its slot follows the host's
//! slot storage, as the melter's inventory does.
use crate::fluid_effects::{self, WorldAccess};
use crate::fluid_ingredient::FluidTags;
use crate::fluid_projectile;
use crate::fluid_values::BUCKET;
use crate::fluid_world::{block_context, holder, slot_write_back, stack_of_slot, LiveWorld};
use crate::tank::Tank;
use pluto_lib::{
    ActionResult, ActionTarget, BlockBehavior, BlockBehaviors, BlockEntityDrain, BlockEntityFill, BlockEntityTick, BlockEntityType, BlockEntityTypeRegistry, BlockPos, BlockState, BlockView, Direction,
    EntityAction, EntityInsideContext, FluidStack, Identifier, ItemSlot, NeighbourTank, Nbt, NbtCompound, ParticleData, PathNodeType, Placement, World,
};

/// The cannons of `TinkerSmeltery`: name, tank capacity, and the shot's power, velocity and inaccuracy
/// (`FluidCannonBlock` of the seared properties twice, `KnightMetalFluidCannonBlock` of the seared ones for the end).
pub(crate) const CANNONS: [(&str, i32, f32, f32, f32); 3] = [
    ("seared_fluid_cannon", BUCKET * 2, 1.0, 1.1, 4.5),
    ("scorched_fluid_cannon", BUCKET * 2, 2.0, 1.5, 6.0),
    ("end_fluid_cannon", BUCKET * 4, 1.5, 3.0, 3.0),
];

/// The ticks between a rising signal and the shot (`scheduleTick(pos, this, 4)`).
const SHOT_DELAY: i32 = 4;
/// The shot countdown in the block entity's data; the original keeps it in the level's tick queue instead.
const TAG_SHOT_DELAY: &str = "shot_delay";

/// The capacity of the cannon block called `name`.
pub(crate) fn capacity_of(name: &str) -> Option<i32> { CANNONS.iter().find(|(cannon, ..)| *cannon == name).map(|(_, capacity, ..)| *capacity) }

fn cannon_tank(name: &str, data: &NbtCompound) -> Tank { Tank::read(data, capacity_of(name).unwrap_or(BUCKET * 2)) }

/// `SearedTankBlock.getAnalogOutputSignal` over the cannon's tank — a cannon is a seared tank, so
/// the original answers a comparator with `ITankBlockEntity.comparatorStrength`. No host surface
/// answers a comparator read yet, the same missing analog output as every tank's (the melter's, the
/// alloyer's), so the strength stands as the port's record of the rule, as the casting tank's does.
pub(crate) fn comparator(name: &str, data: &NbtCompound) -> i32 { cannon_tank(name, data).comparator() }

pub(crate) fn read_tanks(name: &str, data: &NbtCompound) -> Vec<NeighbourTank> {
    let tank = cannon_tank(name, data);
    vec![NeighbourTank { fluid: tank.fluid.map(|fluid| FluidStack { fluid: fluid.fluid, amount: fluid.amount as u32, tag: None }), capacity: tank.capacity as u32 }]
}

pub(crate) fn fill_tank(name: &str, fill: &mut BlockEntityFill<'_>) -> u32 {
    let mut tank = cannon_tank(name, fill.data);
    let filled = tank.fill(&fill.fluid.fluid, fill.fluid.amount.min(i32::MAX as u32) as i32, true);
    if filled > 0 { tank.write(fill.data); }
    filled as u32
}

pub(crate) fn drain_tank(name: &str, drain: &mut BlockEntityDrain<'_>) -> Option<FluidStack> {
    let mut tank = cannon_tank(name, drain.data);
    let out = tank.drain(drain.wanted.as_ref(), drain.amount.min(i32::MAX as u32) as i32, true)?;
    tank.write(drain.data);
    Some(FluidStack { fluid: out.fluid, amount: out.amount as u32, tag: None })
}

/// The item half of `FluidCannonBlockEntity.interact`: an empty slot takes the whole hand stack, an empty hand takes the
/// slot's stack, and with both full the stored stack goes to the player's inventory (the original leaves what does not
/// fit in the slot; the use hook's `give` drops that overflow at the player's feet instead). True when anything moved.
fn swap_items(slot: &mut Option<ItemSlot>, held: &mut Option<ItemSlot>, give: &mut Vec<ItemSlot>) -> bool {
    match (slot.is_none(), held.is_none()) {
        (true, false) => { *slot = held.take(); true }
        (false, true) => { *held = slot.take(); true }
        (false, false) => { give.push(slot.take().expect("a full slot")); true }
        (true, true) => false,
    }
}

/// `FluidCannonBlockEntity.interact`: the fluid container trade of the tank zone first (the use hook hears no click
/// position to tell the zones apart), then the item swap.
fn use_cannon(name: &str, used: &mut pluto_lib::BlockEntityUse<'_>) -> ActionResult {
    struct Block(Tank);
    impl pluto_lib::FluidTarget for Block {
        fn fill(&mut self, fluid: &FluidStack, execute: bool) -> u32 { self.0.fill(&fluid.fluid, fluid.amount.min(i32::MAX as u32) as i32, execute).max(0) as u32 }
        fn drain(&mut self, wanted: Option<&Identifier>, amount: u32, execute: bool) -> Option<FluidStack> {
            self.0.drain(wanted, amount.min(i32::MAX as u32) as i32, execute).map(|out| FluidStack { fluid: out.fluid, amount: out.amount as u32, tag: None })
        }
    }
    let mut target = Block(cannon_tank(name, used.data));
    if let Some(done) = pluto_lib::FluidItems::interact(used.held.as_ref(), &mut target, pluto_lib::TransferDirection::Auto) {
        target.0.write(used.data);
        used.held = done.held;
        used.give.extend(done.give);
        return ActionResult::Success;
    }
    let moved = match used.items.first_mut() {
        Some(slot) => swap_items(slot, &mut used.held, &mut used.give),
        None => false,
    };
    if moved { ActionResult::Success } else { ActionResult::Pass }
}

/// What a shot did: it found no fluid, no effects data to shoot with, applied its fluid to the block
/// in front, fired it as a projectile, or failed (the original's failure sound, which every shot
/// that does not fire plays — an empty tank included).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Shot { NoFluid, NoEffects, AppliedBlock, Fired, Failed }

/// The shot numbers of the cannon called `name`: power, velocity, inaccuracy.
fn shot_numbers(name: &str) -> Option<(f32, f32, f32)> {
    CANNONS.iter().find(|(cannon, ..)| *cannon == name).map(|(_, _, power, velocity, inaccuracy)| (*power, *velocity, *inaccuracy))
}

/// The centre of a block, where the original plays its sounds and aims its contexts from.
fn centre(pos: BlockPos) -> [f64; 3] { [pos[0] as f64 + 0.5, pos[1] as f64 + 0.5, pos[2] as f64 + 0.5] }

/// The slot's stack as an engine stack (the context's fallback stack), empty when the slot is.
fn slot_stack(items: &[Option<ItemSlot>]) -> fluid_effects::Stack {
    items.first().and_then(|slot| slot.as_ref()).map(stack_of_slot).unwrap_or_else(fluid_effects::Stack::empty)
}

/// Whether the block at `pos` presents a sturdy face to a shot: the original asks the face's
/// sturdiness (`isFaceSturdy`), which is not a PlutoLib query, so the port answers that a block
/// which cannot be replaced and is no liquid presents one — the full blocks that make up the
/// common case; a shaped block whose hit face is not full (a slab's open half) is read as sturdy
/// here though the original would fire past it.
fn face_sturdy(world: &LiveWorld, pos: BlockPos) -> bool {
    let Some(state) = world.block_state(pos) else { return false };
    state.block.to_string() != "minecraft:air" && !world.is_liquid_block(pos) && !world.can_be_replaced(pos)
}

/// The failure of a shot: `LevelEvent.SOUND_DISPENSER_FAIL` plays the dispenser fail sound at the
/// cannon; the BLOCK_ACTIVATE game event that follows it in the original has no reach.
fn fail(world: &mut LiveWorld, pos: BlockPos) -> Shot {
    world.play_sound(centre(pos), "minecraft:block.dispenser.fail", 1.0, 1.0);
    Shot::Failed
}

/// `FluidCannonBlockEntity.shoot`: the tank fluid's effects, found as the original finds them, run
/// against the block in front when the entry has block effects and a block with a shape stands
/// there (spending what they consume, the context's stack going back into the slot, the fluid
/// particles sent at the hit location, the PARTICLES_SHOOT level event aside — no level event
/// surface); else, when the face
/// in front is not sturdy, a projectile of `effects amount x power` millibuckets flies with the
/// llama spit sound, and the tank is spent by what flew. A shot that cannot spawn its projectile
/// (the kind unpublished or the chunk unloaded — see `crate::fluid_projectile`) fails without
/// spending, as does a fluid without effects.
pub(crate) fn shoot(
    name: &str,
    pos: BlockPos,
    facing: Direction,
    data: &mut NbtCompound,
    items: &mut Vec<Option<ItemSlot>>,
    world: &mut LiveWorld,
    effects: Option<(&fluid_effects::Data, &FluidTags)>,
) -> Shot {
    let mut tank = cannon_tank(name, data);
    let Some(held) = tank.fluid.clone() else {
        // The original plays the failure sound for an empty tank too: its shoot falls through to it.
        world.play_sound(centre(pos), "minecraft:block.dispenser.fail", 1.0, 1.0);
        return Shot::NoFluid;
    };
    let fluid = FluidStack { fluid: held.fluid.clone(), amount: held.amount.max(0) as u32, tag: None };
    let Some((effects, tags)) = effects else { return Shot::NoEffects };
    let Some((has_block, has_entity, amount_per_level)) = effects.describe(&fluid, tags) else { return fail(world, pos) };
    if !has_block && !has_entity { return fail(world, pos); }
    let (power, velocity, inaccuracy) = shot_numbers(name).unwrap_or((1.0, 1.1, 4.5));
    let target = facing.relative(pos);
    if has_block {
        let shaped = world.block_state(target).map_or(false, |state| state.block.to_string() != "minecraft:air") && !world.is_liquid_block(target);
        if shaped {
            // Mantle's trace result: the hit on the target's face toward the cannon, at its centre.
            let opposite = facing.opposite();
            let step = opposite.offset();
            let location = [
                target[0] as f64 + 0.5 + 0.5 * step[0] as f64,
                target[1] as f64 + 0.5 + 0.5 * step[1] as f64,
                target[2] as f64 + 0.5 + 0.5 * step[2] as f64,
            ];
            let mut context = block_context(world, holder(slot_stack(items), None, centre(pos)), target, opposite, location);
            let consumed = effects.apply_to_block(&fluid, power, &mut context, tags);
            if consumed > 0 {
                // The context's stack goes back into the slot; take it out first, as the context
                // holds the world the send below needs.
                let holder_stack = context.env.holder.stack.clone();
                // The original's shot particles (FluidCannonBlockEntity.shoot): ten fluid particles at
                // the hit location, carrying the tank's fluid as it was shot, before it is spent.
                world.world().send_particles("tconstruct:fluid", &ParticleData::Fluid(fluid.clone()), location, [0.1, 0.2, 0.1], 0.2, 10);
                tank.drain(Some(&held.fluid), consumed, true);
                tank.write(data);
                if let Some(slot) = items.first_mut() { slot_write_back(slot, &holder_stack); }
                return Shot::AppliedBlock;
            }
        }
    }
    if !face_sturdy(world, target) {
        let amount = held.amount.min((amount_per_level as f32 * power) as i32);
        if amount > 0 {
            let flying = FluidStack { amount: amount as u32, ..fluid.clone() };
            if fluid_projectile::launch(world.world(), pos, facing, flying, power, velocity, inaccuracy, slot_stack(items)).is_some() {
                let step = facing.offset();
                let at = [
                    pos[0] as f64 + 0.5 + 0.7 * step[0] as f64,
                    pos[1] as f64 + 0.5 + 0.7 * step[1] as f64,
                    pos[2] as f64 + 0.5 + 0.7 * step[2] as f64,
                ];
                let pitch = 1.0 / (world.random_float() * 0.4 + 1.2) + 0.5;
                world.play_sound(at, "minecraft:entity.llama.spit", 1.0, pitch);
                tank.drain(Some(&held.fluid), amount, true);
                tank.write(data);
                return Shot::Fired;
            }
        }
    }
    fail(world, pos)
}

/// One tick of the trigger, from `FluidCannonBlock.neighborChanged` and `tick`: the pending count is set to
/// [`SHOT_DELAY`] when the signal rises, counts down while it holds, fires at zero and then stays latched at zero until
/// the signal drops. Returns the next pending value and whether the cannon shoots on this tick.
fn trigger_step(triggered: bool, pending: Option<i32>) -> (Option<i32>, bool) {
    if !triggered { return (None, false); }
    match pending {
        None => (Some(SHOT_DELAY), false),
        Some(1) => (Some(0), true),
        Some(left) if left > 1 => (Some(left - 1), false),
        Some(_) => (Some(0), false),
    }
}

fn tick_cannon(name: &str, tick: &mut BlockEntityTick<'_>) {
    let Some(world) = World::ticking() else { return };
    let Some(state) = world.get_block_state(tick.pos) else { return };
    // `TankBlockEntity.updateLight`: the light state follows the fluid.
    let wanted = cannon_tank(name, tick.data).light().to_string();
    if state.property("light").is_some_and(|light| light != wanted) { world.set_block_state(tick.pos, &state.clone().with("light", &wanted)); }
    let pending = tick.data.get(TAG_SHOT_DELAY).and_then(|value| value.as_i64()).map(|value| value as i32);
    let (next, fire) = trigger_step(state.property("triggered") == Some("true"), pending);
    match next {
        Some(left) => { tick.data.insert(TAG_SHOT_DELAY.to_owned(), Nbt::Int(left)); }
        None => { tick.data.remove(TAG_SHOT_DELAY); }
    }
    if fire {
        let facing = Direction::parse(state.property("facing").unwrap_or("north")).unwrap_or(Direction::North);
        let mut live = LiveWorld::new(world.clone());
        match crate::common::fluid_effects() {
            Some(effects) => {
                let effects = effects.read().expect("fluid effects lock poisoned");
                let tags = crate::common::smeltery_data().and_then(|data| data.read().ok().map(|data| data.tags.clone())).unwrap_or_default();
                shoot(name, tick.pos, facing, tick.data, tick.items, &mut live, Some((&effects, &tags)));
            }
            None => {
                shoot(name, tick.pos, facing, tick.data, tick.items, &mut live, None);
            }
        }
    }
}

/// `FluidCannonBlock.getStateForPlacement`: the facing is the placer's nearest looking direction, turned around.
pub(crate) fn cannon_placement(state: BlockState, yaw: f32, pitch: f32) -> BlockState {
    state.with("facing", Direction::nearest_looking(yaw, pitch)[0].opposite().name()).with("triggered", "false")
}

/// Whether the cannon at `pos` has a neighbour signal: the original asks the position and the one above it. An answer
/// the host cannot give for either is not read as no signal.
fn neighbour_signal(view: &dyn BlockView, pos: [i32; 3]) -> Option<bool> {
    let here = view.has_neighbor_signal(pos);
    let above = view.has_neighbor_signal([pos[0], pos[1] + 1, pos[2]]);
    if here == Some(true) || above == Some(true) { Some(true) } else if here.is_some() && above.is_some() { Some(false) } else { None }
}

/// `FluidCannonBlock.neighborChanged` as a shape update: `triggered` follows the neighbour signal; the shot itself is
/// the block entity tick's, as the scheduled tick's callback cannot reach the block entity's tank and item.
pub(crate) fn cannon_trigger(state: &BlockState, signal: Option<bool>) -> Option<BlockState> {
    match (signal, state.property("triggered") == Some("true")) {
        (Some(true), false) => Some(state.clone().with("triggered", "true")),
        (Some(false), true) => Some(state.clone().with("triggered", "false")),
        _ => None,
    }
}

/// `KnightMetalBlock.BLOCK_DAMAGE`: what an entity inside a knightmetal block or the end cannon takes.
const KNIGHTMETAL_BITE: f32 = 4.0;

/// `KnightMetalBlock.entityInside`, and `KnightMetalFluidCannonBlock`'s copy of it: the entity inside
/// is hurt by the knightmetal damage source (TinkerDamageTypes.KNIGHTMETAL), with no attacker.
fn knightmetal_bite(context: &EntityInsideContext) {
    context.act(EntityAction::Hurt { target: ActionTarget::Victim, amount: KNIGHTMETAL_BITE, damage_type: "tconstruct:knightmetal".to_owned() });
}

pub(crate) fn register() {
    for (name, ..) in CANNONS {
        let block = Identifier::new("tconstruct", name).expect("valid cannon id");
        BlockEntityTypeRegistry::register(
            BlockEntityType::new(block.clone()).sync_data().item_slots(1)
                .ticker(move |tick| tick_cannon(name, tick))
                .fluid_reader(move |data| read_tanks(name, data))
                .fluid_filler(move |fill| fill_tank(name, fill))
                .fluid_drainer(move |drain| drain_tank(name, drain))
                .on_use(move |used| use_cannon(name, used)),
        );
        let mut behaviour = BlockBehavior::new(block)
            .state_for_placement(|c| Placement::State(cannon_placement(c.state.clone(), c.yaw, c.pitch)))
            .update_shape(|c| cannon_trigger(&c.state, neighbour_signal(c.view, c.pos)));
        if name == "end_fluid_cannon" {
            // `KnightMetalFluidCannonBlock.getBlockPathType`: mobs avoid it as damaging.
            behaviour = behaviour.path_node_type(|_| Some(PathNodeType::Damaging));
            // `KnightMetalFluidCannonBlock.entityInside`: the knightmetal bite.
            behaviour = behaviour.entity_inside(knightmetal_bite);
        }
        BlockBehaviors::register(behaviour);
    }
    // KnightMetalBlock itself: the same bite and the same path type, on the plain block, which
    // otherwise rides its vanilla template. Its placement is waterlogged in water, as
    // KnightMetalBlock.getStateForPlacement leaves it.
    BlockBehaviors::register(BlockBehavior::new(Identifier::new("tconstruct", "knightmetal_block").expect("valid block id"))
        .state_for_placement(|c| Placement::State(c.state.clone().with("waterlogged", if c.in_water { "true" } else { "false" })))
        .entity_inside(knightmetal_bite)
        .path_node_type(|_| Some(PathNodeType::Damaging)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::{BlockEntityFill, MemoryBlockView};

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }

    fn cannon() -> BlockState { BlockState::parse("tconstruct:seared_fluid_cannon").unwrap() }

    #[test]
    fn the_cannons_carry_the_original_capacities_and_shot_numbers() {
        assert_eq!((capacity_of("seared_fluid_cannon"), capacity_of("scorched_fluid_cannon"), capacity_of("end_fluid_cannon"), capacity_of("seared_fuel_tank")), (Some(2000), Some(2000), Some(4000), None));
        let numbers: Vec<(f32, f32, f32)> = CANNONS.iter().map(|(_, _, power, velocity, inaccuracy)| (*power, *velocity, *inaccuracy)).collect();
        assert_eq!(numbers, [(1.0, 1.1, 4.5), (2.0, 1.5, 6.0), (1.5, 3.0, 3.0)]);
    }

    #[test]
    fn the_knightmetal_bite_queues_four_knightmetal_damage_and_registers_on_both_blocks() {
        let context = EntityInsideContext {
            dimension: "minecraft:overworld".to_owned(), state: BlockState::parse("tconstruct:end_fluid_cannon").unwrap(),
            pos: [0, 64, 0], entity: 7, kind: "minecraft:zombie".to_owned(), bounds: [0.0; 6], fall_distance: 0.0,
            actions: std::cell::RefCell::new(Vec::new()),
        };
        knightmetal_bite(&context);
        assert_eq!(context.actions.borrow().as_slice(), [EntityAction::Hurt { target: ActionTarget::Victim, amount: 4.0, damage_type: "tconstruct:knightmetal".to_owned() }]);
        register();
        assert!(BlockBehaviors::get("tconstruct:end_fluid_cannon").is_some());
        assert!(BlockBehaviors::get("tconstruct:knightmetal_block").is_some());
    }

    #[test]
    fn a_cannon_is_a_tank_of_its_own_capacity_with_an_item_beside_it() {
        let mut data = NbtCompound::new();
        let mut items = vec![None];
        let mut fill = BlockEntityFill { pos: [0; 3], data: &mut data, items: &mut items, fluid: FluidStack { fluid: id("tconstruct:molten_iron"), amount: 5000, tag: None }, face: None, execute: true };
        assert_eq!(fill_tank("end_fluid_cannon", &mut fill), 4000);
        assert_eq!(read_tanks("end_fluid_cannon", &data)[0].fluid.as_ref().unwrap().amount, 4000);
        let mut drain = BlockEntityDrain { pos: [0; 3], data: &mut data, items: &mut items, wanted: None, amount: 2000, execute: true };
        assert_eq!(drain_tank("end_fluid_cannon", &mut drain).unwrap().amount, 2000);
        register();
        assert!(BlockEntityTypeRegistry::get("tconstruct:seared_fluid_cannon").is_some());
        // the use hook over the registered shape: a plain item goes into the one slot, and comes back out
        let mut used = pluto_lib::BlockEntityUse { pos: [0; 3], data: &mut data, items: &mut items, held: Some(ItemSlot::new(id("minecraft:stick"), 2)), sneaking: false, give: Vec::new() };
        assert_eq!(use_cannon("seared_fluid_cannon", &mut used), ActionResult::Success);
        assert_eq!(items[0].as_ref().map(|slot| slot.count), Some(2));
        let mut used = pluto_lib::BlockEntityUse { pos: [0; 3], data: &mut data, items: &mut items, held: None, sneaking: false, give: Vec::new() };
        assert_eq!(use_cannon("seared_fluid_cannon", &mut used), ActionResult::Success);
        assert_eq!(used.held.as_ref().map(|slot| slot.count), Some(2));
    }

    #[test]
    fn the_comparator_reads_the_cannons_fill() {
        let mut data = NbtCompound::new();
        let mut items = vec![None];
        assert_eq!(comparator("end_fluid_cannon", &data), 0, "an empty cannon reads nothing");
        let mut fill = BlockEntityFill { pos: [0; 3], data: &mut data, items: &mut items, fluid: FluidStack { fluid: id("tconstruct:molten_iron"), amount: 2000, tag: None }, face: None, execute: true };
        assert_eq!(fill_tank("end_fluid_cannon", &mut fill), 2000);
        assert_eq!(comparator("end_fluid_cannon", &data), 1 + 14 * 2000 / 4000);
        let mut fill = BlockEntityFill { pos: [0; 3], data: &mut data, items: &mut items, fluid: FluidStack { fluid: id("tconstruct:molten_iron"), amount: 2000, tag: None }, face: None, execute: true };
        assert_eq!(fill_tank("end_fluid_cannon", &mut fill), 2000);
        assert_eq!(comparator("end_fluid_cannon", &data), 15, "a full cannon reads full");
    }

    #[test]
    fn the_item_swap_takes_puts_and_yields_like_the_originals_interact() {
        let stored = ItemSlot::new(id("minecraft:bucket"), 3);
        let (mut slot, mut held, mut give) = (None, Some(stored.clone()), Vec::new());
        assert!(swap_items(&mut slot, &mut held, &mut give));
        assert_eq!(slot.as_ref().map(|s| s.count), Some(3), "an empty slot takes the whole hand stack");
        assert!(held.is_none());
        assert!(swap_items(&mut slot, &mut held, &mut give));
        assert!(slot.is_none());
        assert_eq!(held.as_ref().map(|s| s.count), Some(3), "an empty hand takes it back");
        let mut slot = Some(stored);
        let mut held = Some(ItemSlot::new(id("minecraft:stick"), 1));
        assert!(swap_items(&mut slot, &mut held, &mut give));
        assert!(slot.is_none());
        assert_eq!(give.len(), 1, "with both full the stored stack goes to the player");
        assert_eq!(give[0].count, 3);
        assert_eq!(held.as_ref().map(|s| s.count), Some(1), "and the hand keeps its own");
        let (mut slot, mut held) = (None, None);
        assert!(!swap_items(&mut slot, &mut held, &mut give));
    }

    /// A data set with one block entry (molten obsidian sets blocks) and one entity entry (blazing
    /// blood lights entities), as the projectile's tests build theirs.
    fn effects_data() -> fluid_effects::Data {
        let files = vec![
            (id("tconstruct:blazing_blood"), serde_json::json!({
                "fluid": {"fluid": "tconstruct:blazing_blood", "amount": 250},
                "entity_effects": [{"type": "tconstruct:fire", "action": "add", "time": 2}],
            })),
            (id("tconstruct:molten_obsidian"), serde_json::json!({
                "fluid": {"fluid": "tconstruct:molten_obsidian", "amount": 50},
                "block_effects": [{"type": "tconstruct:set_block", "block": "minecraft:obsidian"}],
            })),
        ];
        let context = crate::conditions::Context { items: None, config: Default::default() };
        fluid_effects::Data { effects: fluid_effects::Effects::load(&files, &context), loaded: true }
    }

    fn filled(name: &str, fluid: &str, amount: u32) -> NbtCompound {
        let mut data = NbtCompound::new();
        let mut items = Vec::new();
        let mut fill = BlockEntityFill { pos: [0; 3], data: &mut data, items: &mut items, fluid: FluidStack { fluid: id(fluid), amount, tag: None }, face: None, execute: true };
        fill_tank(name, &mut fill);
        data
    }

    #[test]
    fn a_shot_waits_four_ticks_fires_once_per_rising_signal_and_spends_nothing_without_effects() {
        let (mut pending, mut fire) = trigger_step(false, None);
        assert_eq!((pending, fire), (None, false));
        (pending, fire) = trigger_step(true, pending);
        assert_eq!((pending, fire), (Some(SHOT_DELAY), false), "the rising signal schedules the shot");
        for left in [3, 2, 1] {
            (pending, fire) = trigger_step(true, pending);
            assert_eq!((pending, fire), (Some(left), false));
        }
        (pending, fire) = trigger_step(true, pending);
        assert_eq!((pending, fire), (Some(0), true), "four ticks later it fires");
        (pending, fire) = trigger_step(true, pending);
        assert_eq!((pending, fire), (Some(0), false), "and not again while the signal holds");
        (pending, fire) = trigger_step(false, pending);
        assert_eq!((pending, fire), (None, false), "the falling signal re-arms it");
        // the shot itself: no fluid, then no effects data, then a fluid no entry matches; the tank is untouched throughout
        let effects = effects_data();
        let tags = FluidTags::default();
        let mut items = vec![None];
        let mut ops = pluto_lib::MemoryWorldOps::default();
        let mut live = LiveWorld::new(World::new("minecraft:overworld"));
        let mut data = NbtCompound::new();
        let shot = pluto_lib::with_world(&mut ops, || shoot("seared_fluid_cannon", [0, 64, 0], Direction::North, &mut data, &mut items, &mut live, Some((&effects, &tags))));
        assert_eq!(shot, Shot::NoFluid);
        let mut data = filled("seared_fluid_cannon", "minecraft:lava", 1000);
        let shot = pluto_lib::with_world(&mut ops, || shoot("seared_fluid_cannon", [0, 64, 0], Direction::North, &mut data, &mut items, &mut live, None));
        assert_eq!(shot, Shot::NoEffects);
        let shot = pluto_lib::with_world(&mut ops, || shoot("seared_fluid_cannon", [0, 64, 0], Direction::North, &mut data, &mut items, &mut live, Some((&effects, &tags))));
        assert_eq!(shot, Shot::Failed, "lava has no entry: the shot fails, with its sound");
        assert_eq!(cannon_tank("seared_fluid_cannon", &data).amount(), 1000);
        assert_eq!(ops.sounds.iter().filter(|(sound, _)| sound.as_str() == "minecraft:block.dispenser.fail").count(), 2, "the empty tank and the effect-less fluid both fail audibly");
    }

    #[test]
    fn a_shot_applies_its_fluid_to_the_block_in_front_and_spends_what_it_uses() {
        let effects = effects_data();
        let tags = FluidTags::default();
        let mut data = filled("seared_fluid_cannon", "tconstruct:molten_obsidian", 1000);
        let mut items = vec![None];
        let mut ops = pluto_lib::MemoryWorldOps::default();
        ops.blocks.insert(("minecraft:overworld".to_owned(), [0, 64, -1]), "minecraft:stone".to_owned());
        let mut live = LiveWorld::new(World::new("minecraft:overworld"));
        let shot = pluto_lib::with_world(&mut ops, || shoot("seared_fluid_cannon", [0, 64, 0], Direction::North, &mut data, &mut items, &mut live, Some((&effects, &tags))));
        assert_eq!(shot, Shot::AppliedBlock);
        assert_eq!(cannon_tank("seared_fluid_cannon", &data).amount(), 950, "one level of molten obsidian is 50");
        assert_eq!(ops.blocks[&("minecraft:overworld".to_owned(), [0, 64, -1])], "minecraft:obsidian");
        assert!(ops.summons.is_empty(), "a block application fires no projectile");
    }

    #[test]
    fn a_shot_fires_a_projectile_of_amount_times_power_and_fails_against_a_sturdy_face() {
        let effects = effects_data();
        let tags = FluidTags::default();
        let mut items = vec![None];
        // open air in front: the projectile flies with amount per level times the end cannon's
        // power 1.5 — the float power the original fires with, not a rounded 1 or 2
        let mut data = filled("end_fluid_cannon", "tconstruct:blazing_blood", 1000);
        let mut ops = pluto_lib::MemoryWorldOps::default();
        let mut live = LiveWorld::new(World::new("minecraft:overworld"));
        let shot = pluto_lib::with_world(&mut ops, || shoot("end_fluid_cannon", [0, 64, 0], Direction::North, &mut data, &mut items, &mut live, Some((&effects, &tags))));
        assert_eq!(shot, Shot::Fired);
        assert_eq!(cannon_tank("end_fluid_cannon", &data).amount(), 625, "amount per level 250 times power 1.5 is 375");
        assert_eq!(ops.summons.len(), 1);
        assert_eq!(ops.summons[0].kind, "tconstruct:fluid_spit");
        assert!(ops.sounds.iter().any(|(sound, _)| sound.as_str() == "minecraft:entity.llama.spit"));
        // a sturdy block in front and no block effects to apply to it: the shot fails, spending nothing
        let mut data = filled("seared_fluid_cannon", "tconstruct:blazing_blood", 1000);
        let mut ops = pluto_lib::MemoryWorldOps::default();
        ops.blocks.insert(("minecraft:overworld".to_owned(), [0, 64, -1]), "minecraft:stone".to_owned());
        let mut live = LiveWorld::new(World::new("minecraft:overworld"));
        let shot = pluto_lib::with_world(&mut ops, || shoot("seared_fluid_cannon", [0, 64, 0], Direction::North, &mut data, &mut items, &mut live, Some((&effects, &tags))));
        assert_eq!(shot, Shot::Failed);
        assert_eq!(cannon_tank("seared_fluid_cannon", &data).amount(), 1000);
        assert!(ops.summons.is_empty());
    }

    #[test]
    fn the_trigger_state_follows_the_neighbour_signal_and_the_cannon_faces_its_placer() {
        let off = cannon().with("triggered", "false");
        assert_eq!(cannon_trigger(&off, Some(true)).unwrap().property("triggered"), Some("true"));
        assert_eq!(cannon_trigger(&off, Some(false)), None);
        assert_eq!(cannon_trigger(&off, None), None, "an unavailable answer is not a falling signal");
        let on = cannon().with("triggered", "true");
        assert_eq!(cannon_trigger(&on, Some(false)).unwrap().property("triggered"), Some("false"));
        assert_eq!(cannon_trigger(&on, Some(true)), None);
        // looking north the cannon faces south, back at the placer; looking down it faces up
        assert_eq!(cannon_placement(cannon(), 180.0, 0.0).property("facing"), Some("south"));
        assert_eq!(cannon_placement(cannon(), 180.0, 90.0).property("facing"), Some("up"));
        assert_eq!(cannon_placement(cannon(), 0.0, 0.0).property("triggered"), Some("false"));
        // the signal of the position above counts, as the original asks both
        let mut view = MemoryBlockView::default();
        view.neighbor_signals.insert([0, 65, 0], true);
        assert_eq!(neighbour_signal(&view, [0, 64, 0]), Some(true));
        assert_eq!(neighbour_signal(&view, [5, 64, 5]), None);
    }
}
