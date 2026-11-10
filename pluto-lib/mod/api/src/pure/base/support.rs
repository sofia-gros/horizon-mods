//! The support matrix: which PlutoLib calls are real and which are stubs, and why.
//!
//! PlutoLib always provides the full Fabric-API-shaped call. Where Horizon or Horizon Mixins cannot do what the call means
//! yet, the call exists and does nothing (or returns the documented default); it is listed here so that missing behaviour
//! is never silent. When the host gains support only PlutoLib changes.

/// Whether a call does what it says on the pinned Horizon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Support {
    /// Works on the pinned Horizon.
    Implemented,
    /// Callable but inert: the host offers nothing to hook yet.
    Stub,
    /// Callable but inert: the pinned Horizon Mixins refuses the hook that would implement it.
    BlockedByMixins,
}

/// One row of the matrix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SupportEntry {
    /// The Fabric-style call, `module::Type::MEMBER`.
    pub api: &'static str,
    pub support: Support,
    /// What the call does while it is not `Implemented`.
    pub behaviour: &'static str,
    /// What host change makes it `Implemented`.
    pub unblocked_by: &'static str,
}

/// Every call that is not fully implemented. Add a row in the same change that adds a stub.
pub const MATRIX: &[SupportEntry] = &[
    SupportEntry {
        api: "worldgen_api_v1::StructureTypeRegistry (natural generation, native-kind overrides, shared-resource source changes and custom piece/marker codecs)",
        support: Support::Stub,
        behaviour: "custom explicit placement/plain-template restoration adapters are under verification; natural starts, native-kind overrides and custom marker codecs remain unconnected; ambiguous shared-resource source snapshots reject",
        unblocked_by: "verified native/game adapters preserving candidate weighting, biome policy, caching, template markers and saved-piece state",
    },
    SupportEntry {
        api: "worldgen_api_v1::RootPlacementTypeRegistry (ordinary native JSON and natural generation)",
        support: Support::Stub,
        behaviour: "explicit per-root callbacks bind holders/providers before placement; ordinary native root JSON/natural generation does not decode registered overrides",
        unblocked_by: "registry-aware root codecs and generation-context integration without changing native simulation semantics",
    },
    SupportEntry {
        api: "block_api_v1::BlockEditor::place_tree_with_roots / worldgen_api_v1::RootPlacementWorld (custom/memory editors and nested callback placement)",
        support: Support::Stub,
        behaviour: "unsupported tree editors reject without writes; unbound holder/provider queries return errors; nested callback feature/tree placement is rejected",
        unblocked_by: "editor implementations with real registry bindings and nested placement tracking",
    },
    SupportEntry {
        api: "block_api_v1::BlockEditor::place_tree / place_tree_with_plan (custom/memory editors; nested tree-callback placement)",
        support: Support::Stub,
        behaviour: "default/custom-memory editors reject trees; live extra-decorator editors reject nested features/trees without writing",
        unblocked_by: "editor override or nested placement tracking merged into the outer tree's finalization sets",
    },
    SupportEntry {
        api: "worldgen_api_v1::TreeDecoratorTypeRegistry (native trees and natural generation)",
        support: Support::Stub,
        behaviour: "explicit place_tree appends predecoded decorators and tracks writes (live oak fixture checked); ordinary native tree JSON/natural generation does not decode registered decorator types",
        unblocked_by: "registry-aware tree codec/placement integration preserving original decorator order and generation contexts",
    },
    SupportEntry {
        api: "worldgen_api_v1::FeatureTypeRegistry (natural generation, native commands and nested native feature graphs)",
        support: Support::Stub,
        behaviour: "direct and live BlockEditor dispatch invoke registered codecs/generators; native commands, generation workers and nested native graphs do not dispatch custom types",
        unblocked_by: "PlutoLib-owned adapters inside the corresponding feature execution contexts, retaining their RNG and world semantics",
    },
    SupportEntry {
        api: "fluid_api_v1::FluidDef::with_world_blocks / with_bucket",
        support: Support::Stub,
        behaviour: "the request is kept; no source or flowing blocks and no bucket of a mod's fluid exist",
        unblocked_by: "Horizon offering world blocks and buckets for a mod's fluid (it has only tanks for them today)",
    },
    SupportEntry {
        api: "color_provider_api_v1::ColorProviderRegistry::register_block (item/hand, particles, CTM overlays)",
        support: Support::Stub,
        behaviour: "world-chunk base faces invoke providers; these other rendering paths retain native tint and do not invoke them",
        unblocked_by: "PlutoLib-owned adapters at the remaining rendering paths; the first world-mesh tint hook does not cover them",
    },
    SupportEntry {
        api: "block_api_v1::BlockBehavior::entity_inside",
        support: Support::Stub,
        behaviour: "callbacks are accepted and never invoked",
        unblocked_by: "a PlutoLib injection that reads the block state in Horizon's block_entity_inside (the hook gives the state id only)",
    },
    SupportEntry {
        api: "events_interaction_v0::PlayerBlockBreakEvents::AFTER",
        support: Support::Stub,
        behaviour: "callbacks are accepted and never invoked; BEFORE and its veto notification CANCELED work",
        unblocked_by: "wiring Horizon's block-destroyed notifications (a hook point to map, not yet done)",
    },
];

/// Rows for a given support class.
pub fn entries_with(support: Support) -> impl Iterator<Item = &'static SupportEntry> {
    MATRIX.iter().filter(move |e| e.support == support)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_row_says_what_it_does_and_what_unblocks_it() {
        for e in MATRIX {
            assert!(!e.api.is_empty() && !e.behaviour.is_empty() && !e.unblocked_by.is_empty(), "{e:?}");
            assert_ne!(e.support, Support::Implemented, "only stubs belong in the matrix: {e:?}");
        }
    }
}
