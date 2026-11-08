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
        api: "item_api_v1::MiningSpeed::MODIFY (host tick_mining)",
        support: Support::BlockedByMixins,
        behaviour: "callbacks are accepted and never invoked; mining speed is the game's own",
        unblocked_by: "horizon-mixins accepting a MODIFY_VARIABLE hook on tick_mining's `speed` local (E_LOCAL_NOT_FOUND today)",
    },
    SupportEntry {
        api: "item_api_v1::MiningSpeed::MODIFY (client destroy_progress)",
        support: Support::BlockedByMixins,
        behaviour: "callbacks are accepted and never invoked; the client's predicted progress is the game's own",
        unblocked_by: "horizon-mixins matching the `speed` accesses in destroy_progress (E_MATCH_COUNT: 4 found, 2 required)",
    },
    SupportEntry {
        api: "block_api::Block::PICK_STACK_COLOR (host picked_stack)",
        support: Support::BlockedByMixins,
        behaviour: "callbacks are accepted and never invoked; the picked item is unchanged",
        unblocked_by: "horizon-mixins supporting injections into functions that use `?` or closures (E_UNSUPPORTED_CONTROL_FLOW)",
    },
    SupportEntry {
        api: "fluid_api_v1::FluidDef::with_world_blocks / with_bucket",
        support: Support::Stub,
        behaviour: "the request is kept; no source or flowing blocks and no bucket of a mod's fluid exist",
        unblocked_by: "Horizon offering world blocks and buckets for a mod's fluid (it has only tanks for them today)",
    },
    SupportEntry {
        api: "lifecycle_events_v1::ServerLifecycleEvents::SERVER_STARTED / SERVER_STOPPED",
        support: Support::Stub,
        behaviour: "callbacks are accepted and never invoked",
        unblocked_by: "a Horizon hook point that reports the server being fully started and fully stopped (only about-to-start and stopping exist)",
    },
    SupportEntry {
        api: "events_interaction_v0::PlayerBlockBreakEvents::AFTER / CANCELED",
        support: Support::Stub,
        behaviour: "callbacks are accepted and never invoked; BEFORE works",
        unblocked_by: "wiring Horizon's block-destroyed notifications (a hook point to map, not yet done)",
    },
    SupportEntry {
        api: "entity_events_v1::ServerLivingEntityEvents::AFTER_DEATH",
        support: Support::Stub,
        behaviour: "callbacks are accepted and never invoked; ALLOW_DAMAGE works",
        unblocked_by: "mapping Horizon's mob_killed / player_died hook points (not yet done)",
    },
    SupportEntry {
        api: "resource_loader_v0::ResourceType::ServerData",
        support: Support::Stub,
        behaviour: "listeners are accepted and never invoked; ClientResources works",
        unblocked_by: "a Horizon hook point that reports a data reload (recipes, loot tables, tags)",
    },
    SupportEntry {
        api: "item_api_v1::ItemTooltipCallback (reading or editing the lines already there)",
        support: Support::Stub,
        behaviour: "callbacks can add lines but do not see the existing ones",
        unblocked_by: "a PlutoLib text type that can be read back from Horizon's Text (add-only works today)",
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
