//! Block behaviour of the smeltery and foundry controllers, from SlimeKnights ControllerBlock and HeatingControllerBlock (MIT; see
//! LICENSE): they face the placer and start outside a structure. The structure check, the controller's inventory and what it
//! drops on breaking (the original also keeps a custom name and a retexture, neither of which exists here) are the block
//! entity's. Rotating and mirroring a block that is in a structure, particles and the error display are not ported.
use pluto_lib::{BlockBehavior, BlockBehaviors, Identifier, Placement};

/// A controller placed: facing the placer, not in a structure (`getStateForPlacement`).
pub(crate) fn controller_placement(state: pluto_lib::BlockState, yaw: f32) -> pluto_lib::BlockState {
    state.with("facing", crate::melter_blocks::facing_toward(yaw)).with("in_structure", "false").with("active", "false")
}

/// A drain, duct or chute placed (OrientableSmelteryBlock.getStateForPlacement): facing the placer, outside a structure.
pub(crate) fn io_placement(state: pluto_lib::BlockState, yaw: f32) -> pluto_lib::BlockState {
    state.with("facing", crate::melter_blocks::facing_toward(yaw)).with("in_structure", "false")
}

pub(crate) fn register() {
    for name in crate::smeltery_io::DRAINS.into_iter().chain(crate::smeltery_io::DUCTS).chain(crate::smeltery_io::CHUTES) {
        let id = Identifier::new("tconstruct", name).expect("valid drain, duct or chute id");
        BlockBehaviors::register(BlockBehavior::new(id).state_for_placement(|c| Placement::State(io_placement(c.state.clone(), c.yaw))));
    }
    for name in [crate::smeltery_menu::SMELTERY, crate::smeltery_menu::FOUNDRY] {
        let id = Identifier::new("tconstruct", name).expect("valid controller id");
        BlockBehaviors::register(BlockBehavior::new(id).state_for_placement(|c| Placement::State(controller_placement(c.state.clone(), c.yaw))));
    }
    // the alloyer is a tiny multiblock controller like the melter (AlloyerBlock extends TinyMultiblockControllerBlock):
    // the melter's placement and shape rules are its rules too
    let alloyer = Identifier::new("tconstruct", crate::alloyer_menu::ALLOYER).expect("valid alloyer id");
    BlockBehaviors::register(BlockBehavior::new(alloyer)
        .state_for_placement(|c| Placement::State(crate::melter_blocks::melter_placement(c.state.clone(), c.pos, c.yaw, c.view)))
        .update_shape(|c| {
            let tank = c.neighbour.as_ref().and_then(|state| c.view.is_state_in_tag(state, "tconstruct:fuel_tanks")).unwrap_or(false);
            crate::melter_blocks::melter_shape(&c.state, c.direction, tank)
        }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::BlockState;

    #[test]
    fn a_controller_faces_its_placer_and_starts_unformed() {
        let state = controller_placement(BlockState::parse("tconstruct:smeltery_controller").unwrap(), 90.0);
        assert_eq!((state.property("facing"), state.property("in_structure"), state.property("active")), (Some("east"), Some("false"), Some("false")));
        register();
    }

    #[test]
    fn a_chute_faces_its_placer_and_starts_outside_a_structure() {
        let state = io_placement(BlockState::parse("tconstruct:scorched_chute").unwrap().with("in_structure", "true"), 90.0);
        assert_eq!((state.property("facing"), state.property("in_structure")), (Some("east"), Some("false")));
        register();
    }

    #[test]
    fn the_alloyer_joins_a_fuel_tank_below_and_loses_its_fire_with_it() {
        let mut view = pluto_lib::MemoryBlockView::default();
        view.tags.insert([0, 63, 0], ["tconstruct:fuel_tanks".to_owned()].into());
        let state = BlockState::parse("tconstruct:scorched_alloyer").unwrap();
        let placed = crate::melter_blocks::melter_placement(state, [0, 64, 0], 180.0, &view);
        assert_eq!((placed.property("facing"), placed.property("in_structure")), (Some("south"), Some("true")));
        let lit = placed.with("active", "true");
        let gone = crate::melter_blocks::melter_shape(&lit, pluto_lib::Direction::Down, false).unwrap();
        assert_eq!((gone.property("in_structure"), gone.property("active")), (Some("false"), Some("false")));
        register();
    }

    #[test]
    fn a_drain_or_duct_faces_its_placer_and_starts_outside_a_structure() {
        let state = io_placement(BlockState::parse("tconstruct:seared_drain").unwrap().with("in_structure", "true"), 180.0);
        assert_eq!((state.property("facing"), state.property("in_structure")), (Some("south"), Some("false")));
    }
}
