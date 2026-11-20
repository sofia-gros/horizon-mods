//! Block behaviour of the casting table and basin and of the faucet, from SlimeKnights TableBlock and FaucetBlock (MIT; see
//! LICENSE): tables and basins face the player, a faucet points away from the face it was placed on (up becomes down, the faucet
//! then hangs). Waterlogging, the comparator output and neighbour/redstone handling are not ported.
use crate::melter_blocks::facing_toward;
use pluto_lib::{BlockBehavior, BlockBehaviors, BlockState, Direction, Identifier, Placement};

/// FaucetBlock.getStateForPlacement: the clicked face, with up turned down.
pub(crate) fn faucet_placement(state: BlockState, face: Direction) -> BlockState {
    state.with("facing", if face == Direction::Up { Direction::Down } else { face }.name())
}

pub(crate) fn register() {
    for (name, ..) in crate::casting::BLOCKS {
        let id = Identifier::new("tconstruct", name).expect("valid casting block id");
        BlockBehaviors::register(BlockBehavior::new(id).state_for_placement(|c| Placement::State(c.state.clone().with("facing", facing_toward(c.yaw)))));
    }
    for name in crate::faucet::BLOCKS {
        let id = Identifier::new("tconstruct", name).expect("valid faucet id");
        BlockBehaviors::register(BlockBehavior::new(id).state_for_placement(|c| Placement::State(faucet_placement(c.state.clone(), c.face))));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_faucet_points_where_it_was_clicked_and_hangs_from_a_ceiling_when_placed_on_top() {
        let faucet = BlockState::parse("tconstruct:seared_faucet").unwrap();
        assert_eq!(faucet_placement(faucet.clone(), Direction::East).property("facing"), Some("east"));
        assert_eq!(faucet_placement(faucet, Direction::Up).property("facing"), Some("down"));
    }
}
