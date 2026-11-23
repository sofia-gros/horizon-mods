//! The connection shapes of the pipes: each face carries a boolean block-state property that
//! says whether the pipe shows an arm toward that side. Placement sets every face from the
//! blocks already there, and a shape update keeps the changed face current. Whether a side
//! connects is decided from the neighbour's block id alone (the callbacks are handed states,
//! not query answers): this mod's own pipes of the same family, the vanilla containers and
//! tanks, and the Tinkers' Construct machines a pipe run is built against. The transport
//! itself does not trust this list — the tickers probe the live neighbour reach — so a block
//! missing here only ever costs the arm in the picture, never the transport.
use crate::common::{EXTRACTION_PIPE, FLUID_EXTRACTION_PIPE, FLUID_PIPE, ITEM_PIPE, NAMESPACE};
use pluto_lib::{BlockBehavior, BlockBehaviors, BlockPos, BlockState, BlockView, Direction, Identifier, Placement};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    Items,
    Fluids,
}

/// Vanilla blocks an item pipe shows an arm toward: the containers of the base game.
const VANILLA_ITEM: [&str; 10] = [
    "chest", "trapped_chest", "barrel", "hopper", "dispenser", "dropper", "furnace", "blast_furnace", "smoker",
    "shulker_box",
];
/// Tinkers' Construct blocks with item slots an item pipe shows an arm toward.
const TCONSTRUCT_ITEM: [&str; 15] = [
    "seared_melter", "smeltery_controller", "foundry_controller", "scorched_alloyer", "seared_table",
    "scorched_table", "seared_basin", "scorched_basin", "cast_chest", "part_chest", "tinkers_chest", "tinker_station",
    "part_builder", "modifier_worktable", "crafting_station",
];
/// Vanilla blocks a fluid pipe shows an arm toward.
const VANILLA_FLUID: [&str; 3] = ["cauldron", "water_cauldron", "lava_cauldron"];
/// Tinkers' Construct blocks with fluid tanks a fluid pipe shows an arm toward.
const TCONSTRUCT_FLUID: [&str; 20] = [
    "smeltery_controller", "foundry_controller", "seared_drain", "scorched_drain", "seared_melter", "scorched_alloyer",
    "seared_table", "scorched_table", "seared_basin", "scorched_basin", "seared_ingot_tank", "scorched_ingot_tank",
    "seared_fuel_tank", "scorched_fuel_tank", "seared_casting_tank", "scorched_proxy_tank", "seared_faucet",
    "scorched_faucet", "seared_channel", "scorched_channel",
];

/// Whether a pipe of `family` shows an arm toward a block with this state.
pub fn connectable(family: Family, state: &BlockState) -> bool {
    let block = &state.block;
    match block.namespace() {
        NAMESPACE => match family {
            Family::Items => block.path() == ITEM_PIPE || block.path() == EXTRACTION_PIPE,
            Family::Fluids => block.path() == FLUID_PIPE || block.path() == FLUID_EXTRACTION_PIPE,
        },
        "minecraft" => match family {
            Family::Items => VANILLA_ITEM.contains(&block.path()),
            Family::Fluids => VANILLA_FLUID.contains(&block.path()),
        },
        "tconstruct" => match family {
            Family::Items => TCONSTRUCT_ITEM.contains(&block.path()),
            Family::Fluids => TCONSTRUCT_FLUID.contains(&block.path()),
        },
        _ => false,
    }
}

fn connected(view: &dyn BlockView, pos: BlockPos, side: Direction, family: Family) -> bool {
    view.block_state(side.relative(pos)).is_some_and(|state| connectable(family, &state))
}

/// The state a pipe is placed with: an arm toward every side that already connects, and for an
/// extraction pipe a `facing` toward the block that was clicked (it pulls out of that block).
pub fn placement(state: BlockState, pos: BlockPos, face: Direction, view: &dyn BlockView, family: Family, extraction: bool) -> BlockState {
    let mut state = state;
    for side in Direction::ALL {
        state = state.with(side.name(), if connected(view, pos, side, family) { "true" } else { "false" });
    }
    if extraction {
        state = state.with("facing", face.opposite().name());
    }
    state
}

/// A neighbour changed on `direction`: that face's arm follows the new neighbour.
pub fn shape_update(state: &BlockState, direction: Direction, neighbour: Option<&BlockState>, family: Family) -> Option<BlockState> {
    let connects = neighbour.is_some_and(|state| connectable(family, state));
    Some(state.clone().with(direction.name(), if connects { "true" } else { "false" }))
}

pub fn register() {
    for (name, family, extraction) in [
        (ITEM_PIPE, Family::Items, false),
        (EXTRACTION_PIPE, Family::Items, true),
        (FLUID_PIPE, Family::Fluids, false),
        (FLUID_EXTRACTION_PIPE, Family::Fluids, true),
    ] {
        let block = Identifier::new(NAMESPACE, name).expect("valid pipe id");
        BlockBehaviors::register(
            BlockBehavior::new(block)
                .state_for_placement(move |context| {
                    Placement::State(placement(context.state.clone(), context.pos, context.face, context.view, family, extraction))
                })
                .update_shape(move |context| shape_update(&context.state, context.direction, context.neighbour.as_ref(), family)),
        );
    }
}
