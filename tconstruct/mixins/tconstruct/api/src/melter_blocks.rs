//! Block behaviour of the melter and the heater, from SlimeKnights ControllerBlock, TinyMultiblockControllerBlock and
//! HeaterBlock (MIT; see LICENSE): they face the player, and are "in a structure" while a fuel tank is below the melter or a
//! melter controller is above the heater. Particles, comparator output and tank interaction are not ported.
use pluto_lib::{BlockBehavior, BlockBehaviors, BlockState, BlockView, Direction, Identifier, Placement};

const FUEL_TANKS: &str = "tconstruct:fuel_tanks";
const HEATER_CONTROLLERS: &str = "tconstruct:heater_controllers";

/// `getHorizontalDirection().getOpposite()` of a placer turned `yaw` degrees (0 faces south, 90 west): the block faces the placer.
pub(crate) fn facing_toward(yaw: f32) -> &'static str {
    match ((yaw / 90.0).round() as i32).rem_euclid(4) { 0 => "north", 1 => "east", 2 => "south", _ => "west" }
}

fn bool_text(value: bool) -> &'static str { if value { "true" } else { "false" } }

fn in_tag(view: &dyn BlockView, pos: [i32; 3], tag: &str) -> bool { view.is_in_tag(pos, tag) }

/// The melter placed: facing the placer, in a structure when the block below is a valid fuel source.
pub(crate) fn melter_placement(state: BlockState, pos: [i32; 3], yaw: f32, view: &dyn BlockView) -> BlockState {
    state.with("facing", facing_toward(yaw)).with("in_structure", bool_text(in_tag(view, [pos[0], pos[1] - 1, pos[2]], FUEL_TANKS)))
}

/// The heater placed: facing the placer, in a structure when a heater controller is above.
pub(crate) fn heater_placement(state: BlockState, pos: [i32; 3], yaw: f32, view: &dyn BlockView) -> BlockState {
    state.with("facing", facing_toward(yaw)).with("in_structure", bool_text(in_tag(view, [pos[0], pos[1] + 1, pos[2]], HEATER_CONTROLLERS)))
}

/// A change below the melter: `in_structure` follows whether the new neighbour is a fuel tank, and no tank puts the fire out.
pub(crate) fn melter_shape(state: &BlockState, direction: Direction, neighbour_is_fuel_tank: bool) -> Option<BlockState> {
    if direction != Direction::Down { return None; }
    let mut next = state.clone().with("in_structure", bool_text(neighbour_is_fuel_tank));
    if !neighbour_is_fuel_tank { next = next.with("active", "false"); }
    Some(next)
}

/// A change above the heater.
pub(crate) fn heater_shape(state: &BlockState, direction: Direction, neighbour_is_controller: bool) -> Option<BlockState> {
    (direction == Direction::Up).then(|| state.clone().with("in_structure", bool_text(neighbour_is_controller)))
}

pub(crate) fn register() {
    let melter = Identifier::new("tconstruct", crate::melter_menu::MELTER).expect("valid melter id");
    BlockBehaviors::register(BlockBehavior::new(melter)
        .state_for_placement(|c| Placement::State(melter_placement(c.state.clone(), c.pos, c.yaw, c.view)))
        .update_shape(|c| {
            let tank = c.neighbour.as_ref().and_then(|state| c.view.is_state_in_tag(state, FUEL_TANKS)).unwrap_or(false);
            melter_shape(&c.state, c.direction, tank)
        }));
    let heater = Identifier::new("tconstruct", crate::melter_menu::HEATER).expect("valid heater id");
    BlockBehaviors::register(BlockBehavior::new(heater)
        .state_for_placement(|c| Placement::State(heater_placement(c.state.clone(), c.pos, c.yaw, c.view)))
        .update_shape(|c| {
            let controller = c.neighbour.as_ref().and_then(|state| c.view.is_state_in_tag(state, HEATER_CONTROLLERS)).unwrap_or(false);
            heater_shape(&c.state, c.direction, controller)
        }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::MemoryBlockView;

    fn melter() -> BlockState { BlockState::parse("tconstruct:seared_melter").unwrap() }

    #[test]
    fn a_block_faces_its_placer_and_the_melter_joins_a_fuel_tank_below() {
        // a placer looking south (yaw 0) is faced by a block facing north; looking west (90) by one facing east
        assert_eq!((facing_toward(0.0), facing_toward(90.0), facing_toward(180.0), facing_toward(-90.0)), ("north", "east", "south", "west"));
        let mut view = MemoryBlockView::default();
        view.tags.insert([0, 63, 0], ["tconstruct:fuel_tanks".to_owned()].into());
        let placed = melter_placement(melter(), [0, 64, 0], 180.0, &view);
        assert_eq!((placed.property("facing"), placed.property("in_structure")), (Some("south"), Some("true")));
        let loose = melter_placement(melter(), [5, 64, 0], 0.0, &view);
        assert_eq!(loose.property("in_structure"), Some("false"));
    }

    #[test]
    fn losing_the_tank_below_puts_the_melter_out_and_only_that_side_matters() {
        let lit = melter().with("in_structure", "true").with("active", "true");
        let gone = melter_shape(&lit, Direction::Down, false).unwrap();
        assert_eq!((gone.property("in_structure"), gone.property("active")), (Some("false"), Some("false")));
        let kept = melter_shape(&lit, Direction::Down, true).unwrap();
        assert_eq!((kept.property("in_structure"), kept.property("active")), (Some("true"), Some("true")), "a tank below keeps the fire");
        assert_eq!(melter_shape(&lit, Direction::North, false), None);
        let heater = BlockState::parse("tconstruct:seared_heater").unwrap();
        assert_eq!(heater_shape(&heater, Direction::Up, true).unwrap().property("in_structure"), Some("true"));
        assert_eq!(heater_shape(&heater, Direction::Down, true), None);
    }
}
