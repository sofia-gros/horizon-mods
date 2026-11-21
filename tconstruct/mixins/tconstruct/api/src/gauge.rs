//! The gauges, from SlimeKnights Mantle `GaugeBlock`, Tinkers `RenderingGaugeBlock` and `GaugeBlockEntity` (MIT; see
//! LICENSE): a thin panel mounted on the face the placer clicked, showing the fluid of the tank behind it (the neighbour
//! opposite its facing). The copper gauge is a plain block. The obsidian gauge's block entity exists to cache that
//! neighbour tank for its renderer, as the original's does: a renderer here sees only its own block entity, so the
//! ticker below mirrors the first tank behind the gauge (fluid, amount and the tank's capacity) into the gauge's own
//! saved data — the tank blocks' own `tank` compound, with `capacity` beside it — and `rendering::render_gauge` draws
//! it. The original caches the neighbour's handler itself and invalidates the cache with the capability; the mirror
//! is the same cache, refreshed every tick instead.
use pluto_lib::{
    BlockBehavior, BlockBehaviors, BlockEntityTick, BlockEntityType, BlockEntityTypeRegistry, BlockState, Direction, Identifier, Neighbours, NeighbourTank, Nbt,
    NbtCompound, NbtCompoundExt, Placement,
};

/// Both gauges of `TinkerSmeltery`, in registration order.
pub(crate) const GAUGES: [&str; 2] = ["copper_gauge", "obsidian_gauge"];

/// The gauge with a block entity (`RenderingGaugeBlock`): the obsidian one.
pub(crate) const RENDERING_GAUGE: &str = "obsidian_gauge";

/// `GaugeBlock.getStateForPlacement`: the gauge faces the way the clicked face points, away from the block it hangs on.
pub(crate) fn gauge_placement(state: BlockState, face: Direction) -> BlockState { state.with("facing", face.name()) }

/// Writes the tank a gauge shows into its saved data (the tank blocks' `tank` compound, and the tank's
/// `capacity` beside it); an empty or missing tank clears both. Answers whether anything changed, so a
/// gauge whose neighbour did not move leaves its data alone.
pub(crate) fn set_mirror(data: &mut NbtCompound, tank: Option<NeighbourTank>) -> bool {
    let held = tank.and_then(|tank| tank.fluid.map(|fluid| (fluid, tank.capacity)));
    let mut wanted = NbtCompound::new();
    if let Some((fluid, capacity)) = &held {
        let amount = fluid.amount.min(i32::MAX as u32) as i32;
        crate::tank::Tank { fluid: Some(crate::tank::Fluid { fluid: fluid.fluid.clone(), amount }), capacity: 0 }.write(&mut wanted);
        wanted.insert("capacity".to_owned(), Nbt::Int((*capacity).min(i32::MAX as u32) as i32));
    }
    let changed = data.get("tank") != wanted.get("tank") || data.get("capacity") != wanted.get("capacity");
    if changed {
        for key in ["tank", "capacity"] {
            match wanted.remove(key) {
                Some(value) => { data.insert(key.to_owned(), value); }
                None => { data.remove(key); }
            }
        }
    }
    changed
}

/// The gauge's cache refresh: the first tank of the block behind it (opposite its facing, asked on the face
/// looking back at the gauge — `GaugeBlockEntity.getTank`).
fn tick_gauge(tick: &mut BlockEntityTick<'_>) {
    let Some(facing) = Neighbours::block_state(tick.pos).and_then(|state| state.property("facing").and_then(Direction::parse)) else { return };
    let (behind, face) = Neighbours::beside(tick.pos, facing.opposite());
    let tank = Neighbours::fluid_tanks(behind, face).and_then(|tanks| tanks.into_iter().next());
    set_mirror(tick.data, tank);
}

pub(crate) fn register() {
    for name in GAUGES {
        let id = Identifier::new("tconstruct", name).expect("valid gauge id");
        BlockBehaviors::register(BlockBehavior::new(id).state_for_placement(|c| Placement::State(gauge_placement(c.state.clone(), c.face))));
    }
    let gauge = Identifier::new("tconstruct", RENDERING_GAUGE).expect("valid gauge id");
    BlockEntityTypeRegistry::register(BlockEntityType::new(gauge).sync_data().ticker(tick_gauge));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_gauge_faces_the_clicked_face_and_only_the_obsidian_one_has_a_block_entity() {
        for (face, name) in [(Direction::North, "north"), (Direction::Up, "up"), (Direction::East, "east")] {
            let state = gauge_placement(BlockState::parse("tconstruct:copper_gauge").unwrap(), face);
            assert_eq!(state.property("facing"), Some(name));
        }
        register();
        assert!(BlockEntityTypeRegistry::get("tconstruct:obsidian_gauge").is_some());
        assert!(BlockEntityTypeRegistry::get("tconstruct:copper_gauge").is_none(), "the copper gauge is a plain block");
        assert!(BlockBehaviors::get("tconstruct:copper_gauge").is_some());
    }

    #[test]
    fn the_mirror_keeps_the_tank_behind_in_the_tank_blocks_own_format_and_only_writes_on_change() {
        use pluto_lib::FluidStack;
        let tank = |amount: u32| Some(NeighbourTank { fluid: Some(FluidStack { fluid: id("tconstruct:molten_iron"), amount, tag: None }), capacity: 4320 });
        let mut data = NbtCompound::new();
        assert!(set_mirror(&mut data, tank(2160)), "the first mirror is written");
        let held = crate::tank::Tank::read(&data, data.get_int("capacity", 0) as i32);
        assert_eq!(held.fluid, Some(crate::tank::Fluid { fluid: id("tconstruct:molten_iron"), amount: 2160 }), "readable by the tank blocks' own reader");
        assert_eq!(held.capacity, 4320);
        assert!(!set_mirror(&mut data, tank(2160)), "an unchanged neighbour changes nothing");
        assert!(set_mirror(&mut data, tank(2000)), "a drained neighbour is written");
        assert!(set_mirror(&mut data, Some(NeighbourTank { fluid: None, capacity: 4320 })), "an emptied tank clears the mirror");
        assert!(data.get("tank").is_none() && data.get("capacity").is_none());
        assert!(!set_mirror(&mut data, None), "and no tank behind clears nothing twice");
    }

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
}
