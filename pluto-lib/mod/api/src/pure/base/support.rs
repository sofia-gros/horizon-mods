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
        api: "block_api_v1::ShapeUpdateContext::schedule_tick / BlockView::is_state_in_tag",
        support: Support::Stub,
        behaviour: "request recording and supplied-state tag lookup are source-wired to live shape ticks/registry; custom views may lack tags; native validation remains pending",
        unblocked_by: "native attachment source review and consolidated native plan/type/game validation",
    },
    SupportEntry {
        api: "entity_events_v1::MobSpawnCallback / MobSpawnPreparation / MobSpawnCompletion / MobSpawnAction",
        support: Support::Stub,
        behaviour: "registered split-phase dispatch and owned completion/resource cleanup are pure-tested; generic provider and runtime/natural/spawner/event/marked-structure/finalizing-frog-conversion callers are source-wired; selected completion follows inherited tails and precedes owned join, with once-only owner identity and unchanged non-finalizing NBT/other-conversion paths; actual spawner publication and converted-tail helpers are trace-host tested, not native-validated; full caller review and native field/state/plan/type/build/game validation remain pending",
        unblocked_by: "native attachment review and consolidated native validation of each caller, including spawner/raid/structure/conversion, zero-ID pending mobs, no duplicate initialization/reroll, join/cancellation ordering, child effects and complete stack transport; LOAD and a narrow finalize hook are not substitutes",
    },
    SupportEntry {
        api: "fluid_api_v1::FluidRegistry::snapshot / FluidRegistrySnapshot::contains",
        support: Support::Stub,
        behaviour: "owned native-plus-defined fluid membership, missing-base refusal, invalid/duplicate rejection and nested/unwind restoration are pure-tested; native base IDs source-use the full selected-root command export rather than render catalogs/hardcoded vanilla lists; no tag membership or world-fluid support is implied",
        unblocked_by: "native attachment source/site review and native plan/build/game checks, export availability, source fluid registration and reload retention; ordered tag loading, full fluid capabilities and world simulation remain independent consumers",
    },
    SupportEntry {
        api: "item_api_v1::ItemRegistry::snapshot / ItemRegistrySnapshot::default_stack / ItemRegistrySnapshot::crafting_remainder",
        support: Support::Stub,
        behaviour: "owned prototype snapshots, full-batch validation, copy isolation and nested/unwind restoration are pure-tested; guarded source captures selected Items durability/custom NBT and item-level crafting remainders separately from use remainders; complete property maps validate known absence, unknown references and retained ownership; a final owned-map LOAD now retains custom_data from the same loader's raw Arc, without rereading files; missing/malformed payloads disable capture; native attachment/conversion unvalidated",
        unblocked_by: "native attachment review and native LOAD/capture/type/build/game checks; StackData remains reduced rather than all native components",
    },
    SupportEntry {
        api: "resource_loader_v0::ResourceManagerHelper::reload_json_stacks",
        support: Support::Stub,
        behaviour: "owned bottom-to-top JSON/error layers for all folders captured before callbacks, cross-folder reads independent of listener order, empty versus unavailable folders and nested/unwind cleanup are pure-tested; normal effective JSON never falls back through a broken top layer; guarded native reload-pass source prepares both views once per distinct directory from one pack stack; client transport is inert",
        unblocked_by: "native attachment owner/site review and consolidated native tests for pack priorities, replace/append tag merges, unreadable/malformed layers, empty folders and nested scopes; caller-specific tag/library assembly remains separate",
    },
    SupportEntry {
        api: "resource_loader_v0::ResourceManagerHelper::reload_registry",
        support: Support::Stub,
        behaviour: "owned membership/tag lookups and nested/unwind cleanup are pure-tested; a source-declared native reload wrapper scopes full entity-type IDs/effective world-pack tags and active item membership/tags with native fallback, plus native fluid IDs joined with PlutoLib-defined fluids and the host-loaded fluid tags (unknown members dropped, failed tag read leaves tags unavailable); invalid contents isolate that registry; outside callbacks/unsupported registries return None; native plan/type/build/game validation is pending",
        unblocked_by: "native attachment source/site review and consolidated native reload tests, full entity registry rather than render catalogs, effective versus fallback pack tags, absent versus empty tags, nested callbacks and scope teardown; other registries need their own validated sources",
    },
    SupportEntry {
        api: "resource_loader_v0::ResourceManagerHelper::register_json_reload_listener",
        support: Support::Stub,
        behaviour: "folder registration, duplicate/path rejection, batch validation and unlocked JSON callbacks are pure-tested; server registration/application use existing native API 400, with owned snapshots supplied by a guarded source-declared reload-pass wrapper, but native compilation/pack/reload validation is pending; client JSON declarations are inert",
        unblocked_by: "native attachment source review and consolidated native tests for initial/reload/empty batches, enabled pack overlays, invalid top JSON without fallback and listener failures; client JSON also needs its own folder discovery/apply transport",
    },
    SupportEntry {
        api: "trade_offer_helper::TradeOfferHelper",
        support: Support::Stub,
        behaviour: "providers/snapshots and the actual native-used pool-selection core are pure-tested; source-declared concrete Game HEAD preserves per-set RandomSequences and map ownership; native factory/tag/stack transport is source-wired but plan/type/build/game/save/LAN validation is pending",
        unblocked_by: "general pre-selection factory pool adapter preserving native trade-set RandomSequences, amount/duplicate policy, ordered registry tags and stack data, followed by native attachment review and consolidated native/game validation; do not append completed offers",
    },
    SupportEntry {
        api: "item_api_v1::ItemBehavior::is_ender_mask",
        support: Support::Stub,
        behaviour: "registered true/false/delegation and actual adapter branch/math are pure-tested; native Enderman HEAD/transport are source-declared only; AI snapshots have item ID/UUID/observer but no full stack",
        unblocked_by: "native attachment source/site/owner review, consolidated plan/build/game validation including native pumpkin fallback, explicit false, gaze cone/occlusion and LAN; stack-dependent queries also need full native stack transport",
    },
    SupportEntry {
        api: "item_api_v1::ItemBehavior::max_stack_size",
        support: Support::Stub,
        behaviour: "registration and dispatch are pure-tested (an answer by the stack's custom data; none and zero delegate to the item's own maximum); the item behaviour transport carries the query and the menu adapter honours it when stacking in a menu, both source-wired and native-unvalidated; the game's own stacking (inventory merging, pickup) has no PlutoLib seam yet, so it still uses the item default everywhere else",
        unblocked_by: "a PlutoLib seam at the game's stack-limit query that calls the installed transport dispatcher, followed by native attachment review and consolidated build/game validation",
    },
    SupportEntry {
        api: "data_attachment_api_v1::AttachmentRegistry::create_synced / EntityInfo attachment methods",
        support: Support::Stub,
        behaviour: "entity and sync-policy access source-forwards native attachment holders/codecs/sync; pure tests cover per-world/holder values, policy conflicts and JSON snapshots only; native entity save/reopen, holder lifecycle and LAN remain unverified",
        unblocked_by: "native attachment source review followed by consolidated native/game gates for UUID identity, load/unload, missing holders, save/reopen and tracking-client synchronization; attribute/render consumers are separate",
    },
    SupportEntry {
        api: "living_entity_behavior::LivingEntityBehaviors",
        support: Support::Stub,
        behaviour: "load-time and successful in-place-NBT-reload class-base callbacks source-produce owned armor/toughness writes in ordinary attachment scopes before entity-load observers, retaining modifiers and validating each batch before base mutation; later size/metal setters are unconnected and native Host/order remain unvalidated; transient class attributes are source-applied before server join using native instance sets; armor/toughness retain native effective processing and scalar resistance receives transient ops/has/amount/remove forwarding; constructor timing, gear caches, observers and save/sync remain unvalidated; per-type jump-power, fall-calculation and contact-attack queries dispatch outside locks; native HEAD helpers forward subtype size/block factor and the effective double attack attribute; a source-declared fall LOAD follows event/class math and precedes block effects; helper facades and actual Tinkers registrations pass pure tests, not native signature/site/game validation",
        unblocked_by: "native attachment source/plan review and consolidated native/game validation; remaining entity factory, movement/fall, attributes, state/sync, spawn and rendering adapters are separate",
    },
    SupportEntry {
        api: "entity_type_api::EntityTypeRegistry::register_derived",
        support: Support::Stub,
        behaviour: "records owned derived types and source-wires symbolic type/default-attribute-supplier/network/command output plus declaration-keyed roots; preserves inherited raw fields and exact supplier bit strings, with no fabricated nonliving suppliers; native execution/catalog/model validation, tracking policy, custom attributes, spawn/state/simulation adapters remain incomplete or unverified",
        unblocked_by: "a general PlutoLib-owned registry/loader adapter validating the real factory and publishing every type consumer, plus separate model, state, attributes, spawn and simulation adapters; the four-template content whitelist is not final host-impossibility evidence",
    },
    SupportEntry {
        api: "block_entity_renderer_api_v1::BlockEntityCanvas::keep (native registry/client-lifetime invalidation)",
        support: Support::Stub,
        behaviour: "keep suppression and generation/token cores are pure-tested; registry replacement advances a process-local generation; source lookup/entry-generation/revision hooks cover existing caches and client commits/resets but actual owner/constructor/function plan validation and native compilation remain pending",
        unblocked_by: "native attachment review of the source-declared cache owner/constructor and concrete hooks, then native compilation and game checks for registration replacement, existing entries, client resets and nested draw behavior",
    },
    SupportEntry {
        api: "block_entity_api_v1::BlockEntityType::client_ticker / block_entity_renderer_api_v1::BlockEntityCanvas::client_data / block_api_v1::BlockView::has_neighbor_signal",
        support: Support::Stub,
        behaviour: "client callbacks and owned transaction state are pure-tested; native state/constructor declarations and renderer snapshot forwarding exist in source but await native attachment plan/native validation; client tick dispatch is source-wired at the existing animation pass with server/title/pause/freeze guards, and replacement/unload reset hooks preserve same-level respawns in source; all native lifecycle validation remains pending; borrowed native signal-query forwarding exists in source but is unvalidated, and unavailable/busy client_data stays None",
        unblocked_by: "a reviewed general client-level adapter with instance lifetime/reset/unload ownership, true neighbor-signal queries, read-only render access and no save/server-wire serialization; dedicated servers must never dispatch it",
    },
    SupportEntry {
        api: "block_entity_api_v1::BlockEntityTypeRegistry / block_entity_renderer_api_v1::BlockEntityRendererRegistry (native-recognized custom names)",
        support: Support::Stub,
        behaviour: "registration is recorded; registered-custom-only factory policy is pure-tested and predicate/hook wiring exists in source; the mandatory block_entities guard deliberately blocks native use until the actual native attachment plan fingerprint is reviewed; custom-name Mod rendering remains unverified",
        unblocked_by: "a general PlutoLib-owned mod-only factory-priority adapter preserving unregistered/vanilla fallback, plus saved-entity and network lifecycle verification; no native attachment is yet validated",
    },
    SupportEntry {
        api: "skull_model_registry::SkullModelRegistry / SkullModelAnimation",
        support: Support::Stub,
        behaviour: "custom item/worn skull definitions and actual codec/parts/bounds/submit/worn dispatch helpers are pure-tested with host facades; source connects the shared native special-renderer path and requesting renderer-owned geometry, preserving native-kind fallback; native types, codec behavior, baking/conversion, attachment/reload and pixels remain unvalidated",
        unblocked_by: "native attachment combined source/plan review and consolidated native inventory/hand/worn/block/reload/save/LAN validation; no registration-only gameplay claim",
    },
    SupportEntry {
        api: "block_api_v1::PistonBehavior / BlockBehavior::piston_behavior / piston_behavior_override",
        support: Support::Stub,
        behaviour: "static mod-block piston reactions are registered and actual transport/gameplay helpers are pure-tested; source content projection preserves individual state rows and includes immutable settings in the content-root identity; native types, sites and piston resolution remain unvalidated behind native attachment review",
        unblocked_by: "native attachment source/plan review of Plan::override_gameplay, then consolidated native piston/loot/save/LAN validation; vanilla/unregistered settings reject content-root generation",
    },
    SupportEntry {
        api: "item_api_v1::StandingAndWallBlockItem::register",
        support: Support::Stub,
        behaviour: "records owned floor/wall/attachment declarations; owned-table overlay and cache-suppression policy are pure-tested, native resolver/helpers are drafted, provider and symbolic resource-export hooks are source-wired with transactional ID-name validation and declaration-keyed roots; the old numeric STORE remains undeclared because build()? is unsupported; real plan/site/native placement validation remains pending",
        unblocked_by: "a PlutoLib-owned per-resource binding adapter covering cached and uncached tables, resolving all item/block IDs before publication and preserving native item placement, survival, obstruction, game-master checks and consumption; content wall_block is sign-only and state_for_placement cannot change block identity",
    },
    SupportEntry {
        api: "block_api_v1::BlockBehavior::is_slime_block / is_sticky_block / can_stick_to",
        support: Support::BlockedByMixins,
        behaviour: "source identity and directional pair predicates are registered and pure-tested; native piston/entity-push paths do not invoke them",
        unblocked_by: "a reviewed PlutoLib adapter preserving complete native resolver semantics; direct generic Sim method injections reject E_UNSUPPORTED_SIGNATURE",
    },
    SupportEntry {
        api: "worldgen_api_v1::StructurePieceTypeRegistry (natural generation and saved-start lifecycle)",
        support: Support::Stub,
        behaviour: "explicit registered templates dispatch clipped DATA markers with shared world/RNG, configured features and per-piece state; natural placement is unconnected and saved-start reloads are unverified",
        unblocked_by: "natural start/cache integration and verified saved-start restoration across lifecycle/resource changes",
    },
    SupportEntry {
        api: "worldgen_api_v1::StructureTypeRegistry (avoid-biome sets, native-kind overrides and shared-resource source changes)",
        support: Support::Stub,
        behaviour: "explicit planning and plain/registered templates are connected; natural plain-template selection/cache/restore is checked, while typed natural marker placement is under verification; avoid-biome sets and native-kind overrides remain unsupported; ambiguous source snapshots reject; lifecycle runtime coverage is documented in worldgen",
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
        api: "fluid_world_api_v1::FluidWorld (the first tick of a placed source, entity contact)",
        support: Support::Stub,
        behaviour: "the flow, the buckets and the fluid's block behaviour are pure-tested and run through existing block hooks; a source a bucket places asks for its first tick through World::schedule_tick: the HostWorld adapter queues the request (the mod-facing host has no scheduling call) and the level's random/scheduled tick seams apply it in that dimension, so the source starts flowing without waiting for a neighbour change; entity contact is notified through BlockBehavior::entity_inside (see its row), but applying contact effects (burning, effects, swimming, buoyancy, fog) needs a consumer that acts on the entity, which the read-only notification does not provide",
        unblocked_by: "native build and game validation of the queue drain at the level tick seams; a consumer-side entity action path for contact effects (the notification itself is delivered)",
    },
    SupportEntry {
        api: "living_entity_drops::LivingEntityDropsCallback / LivingEntityDrops",
        support: Support::Stub,
        behaviour: "ordered borrowed registry/dispatcher, actual Tinkers consumer and gateway/claim helpers are pure-tested; source-declared native death transport shares Nav head-spent flags, owns removed-source claims in CombatTracker, gates only the head table and preserves canonical killer type IDs; exact native types, attachment and capture ordering remain unvalidated behind native attachment review",
        unblocked_by: "native attachment native death transport preserving killer class/power, independent dropped-skull counters, capture/drop order, loot-disabled event semantics and the requesting world's ownership; then native/game verification",
    },
    SupportEntry {
        api: "particle_factory_registry::ParticleFactoryRegistry::{register_item,register_entity_override,item}",
        support: Support::Stub,
        behaviour: "owned item-factory stack registration/copy lookup work; native client named-request bridge and Ground-model resolution are source-declared, with trace-helper tests only; exact-type cube landing kind overrides and shared host/remote zero-count event consumption are source-wired; actual selector/event/constructor chain is trace-tested, not native conversion/model/filters/RNG/rendering; other engine-direct emitters/bursts/network paths incomplete",
        unblocked_by: "native attachment source/site review, native client adapter validation and real original-factory sprite/physics/RNG gates; connect actual entity emission and network/type consumers",
    },
    SupportEntry {
        api: "color_provider_api_v1::ColorProviderRegistry::{register_item,item_color}",
        support: Support::Stub,
        behaviour: "owned-stack item registration/direct lookup work; shared baked-layer LOAD adapter and merged-component forwarding are source-declared, native-unvalidated; structural helper test covers RGB/ARGB, sparse indices, decline/special/native fallback and shared-geometry isolation, not native rendering; indices above 4095 retain native tint",
        unblocked_by: "native attachment source/site review, actual Mixins/type validation and real inventory/hand/CIT rendering gates with full stack/default/patch coverage",
    },
    SupportEntry {
        api: "color_provider_api_v1::ColorProviderRegistry::register_block (item/hand, particles, CTM overlays)",
        support: Support::Stub,
        behaviour: "world-chunk base faces invoke providers; common Game particle_sprite tint override is source-declared with supplied registry/position/index zero, native decline and ordinary grass dirt-crumb exception; structural helper test is not native validation; independent falling-dust/CTM/other paths remain unconnected",
        unblocked_by: "native attachment particle source/site review and native/game checks, plus adapters at independent tint paths; the world-mesh hook alone does not cover them",
    },
    SupportEntry {
        api: "block_api_v1::BlockBehavior::path_node_type",
        support: Support::Stub,
        behaviour: "typed registration preserves explicit blocked answers and native fallback; borrowed-world static query adapter is implemented and passes planning; guarded compilation and game verification are pending, and static queries have no entity identity",
        unblocked_by: "PlutoLib-owned borrowed world-view adapter at the native path-node query, followed by guarded build and real route verification",
    },
    SupportEntry {
        api: "block_api_v1::BlockBehavior::fall_on / update_entity_after_fall_on (items and remaining entity families)",
        support: Support::Stub,
        behaviour: "player and concrete mob landing adapters exist; four-kind cow immunity/bounce and stone damage control pass in game; item post-fall expression adapter passes full build and four-kind bounce/stack/control game gate; item landing-damage distance is unavailable; other species and side contacts are separate",
        unblocked_by: "PlutoLib-owned typed adapters at the other entities' native landing boundaries; no Host is needed to edit calculated damage",
    },
    SupportEntry {
        api: "block_api_v1::BlockBehavior::entity_inside_motion (non-player entities and contact sound/network effects)",
        support: Support::Stub,
        behaviour: "resident-player falling-side velocity/suppression and same-tick native sound requests pass on all four horizontal faces; audible/broader LAN delivery is unverified; cow/item four-face falling-side and same-entity native sound gates pass; other species, flat contact and broader LAN remain unverified or unresolved",
        unblocked_by: "typed entity motion adapters and independently verified sound/network transport; velocity edits alone do not imply those effects",
    },
    SupportEntry {
        api: "block_api_v1::BlockBehavior::entity_inside",
        support: Support::Stub,
        behaviour: "callbacks are invoked by PlutoLib's player, mob and item movement seams (the seams that dispatch entity_inside_motion), with the level's dimension, the block's full state and the entity's id and kind, on the server side; source-wired, not native or game validated. The native block_entity_inside trait hook is deliberately not the carrier: its generated method receives no host and only a state id, which cannot be resolved to a block state there",
        unblocked_by: "consolidated native build and game validation of the three movement seams; entity families whose movement PlutoLib does not seam (projectiles) are not notified",
    },
    SupportEntry {
        api: "recipe_api_v1::register_ingredient_kind / register_assembly",
        support: Support::Stub,
        behaviour: "both registries are pure-tested; the game's ingredient dispatch (load-time item lists and craft-time stack tests for mod-defined kinds) and the crafting gate's assembly substitution (a provider's compound replacing the result template's custom data, denial vetoing the match) are source-wired through the native inventory registry and the established crafting-match boundary; native compilation and game validation are pending",
        unblocked_by: "consolidated native build and game validation of the ingredient hooks and the assembly arm of the crafting-match boundary",
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
