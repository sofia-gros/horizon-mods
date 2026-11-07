// Read-only built-in fluid facts. Custom world-fluid cells are a separate service.
use crate::motion::{PlayerMotion, WATER};
use horizon_world::{
    fluid::{FluidKind, FluidState},
    level::Behaviours,
    registry::BlockStateId,
};

/// Facts from the host's built-in fluid state, not a Pluto custom-fluid cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeFluidFacts {
    /// Exact native identity, retaining source/flowing distinctions and empty.
    pub id: &'static str,
    /// The native amount is full; this does not imply a source or full cube.
    pub full: bool,
}

fn state_facts(state: FluidState) -> NativeFluidFacts {
    NativeFluidFacts {
        id: state.kind().name(),
        full: state.is_full(),
    }
}

/// Read only the host's built-in classification for an already-resolved state.
/// Ordinary custom-fluid blocks may classify as native empty. Do not use this
/// result as a query for Pluto's separately registered custom-fluid families.
pub fn block(behaviours: &Behaviours, state: BlockStateId) -> NativeFluidFacts {
    state_facts(behaviours.fluid(state))
}

/// Native fluid identities, including empty and separate flowing identities.
/// This is not the custom-fluid registry or a resolved data-pack fluid tag.
pub fn native_ids() -> impl Iterator<Item = &'static str> {
    FluidKind::ALL.into_iter().map(FluidKind::name)
}

/// The native fluid block that remains when this solid/waterlogged block is
/// removed. This only resolves a state; the caller still owns permissions,
/// writes and rollback. It does not preserve Pluto custom-fluid cell overlays.
pub fn replacement_after_removal(behaviours: &Behaviours, state: BlockStateId) -> BlockStateId {
    behaviours.legacy_block(behaviours.fluid(state))
}

/// Three distinct samples of native water around the selected player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerWaterFacts {
    /// Current eye-in-water result, which can differ from the prior eye sample.
    pub eye_in_water: bool,
    /// Eye sample saved before the host updates base-tick fluid interaction.
    pub previous_eye_in_water: bool,
    /// The host's last body-touching-water sample, not the current eye result.
    pub touching_water: bool,
}

/// Read the supplied actor's existing samples without sampling the world or
/// advancing movement. Actor selection and absence handling remain the caller's.
/// These flags do not include Pluto's ordinary custom-fluid blocks.
pub fn player(player: &PlayerMotion) -> PlayerWaterFacts {
    PlayerWaterFacts {
        eye_in_water: player.is_eye_in_fluid(WATER),
        previous_eye_in_water: player.fluids.was_eye_in_water,
        touching_water: player.fluids.was_touching_water,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_identity_catalog_retains_empty_and_flowing_variants() {
        assert_eq!(
            native_ids().collect::<Vec<_>>(),
            [
                "minecraft:empty",
                "minecraft:flowing_water",
                "minecraft:water",
                "minecraft:flowing_lava",
                "minecraft:lava"
            ]
        );
    }

    #[test]
    fn full_native_flow_is_not_reclassified_as_a_source() {
        for kind in FluidKind::ALL {
            for amount in [1, 7, 8] {
                for falling in [false, true] {
                    let state = FluidState::new(kind, amount, falling);
                    let facts = state_facts(state);
                    assert_eq!(facts.id, kind.name());
                    assert_eq!(facts.full, kind != FluidKind::Empty && amount == 8);
                }
            }
        }
        let falling_water = state_facts(FluidState::new(FluidKind::FlowingWater, 8, true));
        assert!(falling_water.full);
        assert_ne!(falling_water.id, "minecraft:water");
        assert_eq!(state_facts(FluidState::EMPTY).id, "minecraft:empty");
        assert!(!state_facts(FluidState::EMPTY).full);
    }
}
