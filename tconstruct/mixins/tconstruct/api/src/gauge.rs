//! The gauges, from SlimeKnights Mantle `GaugeBlock`, Tinkers `RenderingGaugeBlock` and `GaugeBlockEntity` (MIT; see
//! LICENSE): a thin panel mounted on the face the placer clicked, showing the fluid of the tank behind it (the neighbour
//! opposite its facing). The copper gauge is a plain block; the obsidian gauge's block entity exists only to cache that
//! neighbour tank for its renderer, so the type is registered with no hooks of its own. Drawing the fluid is the
//! renderer's work: the fluid cuboid data already sits in `render_data`, and no renderer is registered for the gauge yet.
use pluto_lib::{BlockBehavior, BlockBehaviors, BlockEntityType, BlockEntityTypeRegistry, BlockState, Direction, Identifier, Placement};

/// Both gauges of `TinkerSmeltery`, in registration order.
pub(crate) const GAUGES: [&str; 2] = ["copper_gauge", "obsidian_gauge"];

/// The gauge with a block entity (`RenderingGaugeBlock`): the obsidian one.
pub(crate) const RENDERING_GAUGE: &str = "obsidian_gauge";

/// `GaugeBlock.getStateForPlacement`: the gauge faces the way the clicked face points, away from the block it hangs on.
pub(crate) fn gauge_placement(state: BlockState, face: Direction) -> BlockState { state.with("facing", face.name()) }

pub(crate) fn register() {
    for name in GAUGES {
        let id = Identifier::new("tconstruct", name).expect("valid gauge id");
        BlockBehaviors::register(BlockBehavior::new(id).state_for_placement(|c| Placement::State(gauge_placement(c.state.clone(), c.face))));
    }
    let gauge = Identifier::new("tconstruct", RENDERING_GAUGE).expect("valid gauge id");
    BlockEntityTypeRegistry::register(BlockEntityType::new(gauge));
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
}
