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
