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
    fn a_drain_or_duct_faces_its_placer_and_starts_outside_a_structure() {
        let state = io_placement(BlockState::parse("tconstruct:seared_drain").unwrap().with("in_structure", "true"), 180.0);
        assert_eq!((state.property("facing"), state.property("in_structure")), (Some("south"), Some("false")));
    }
}
