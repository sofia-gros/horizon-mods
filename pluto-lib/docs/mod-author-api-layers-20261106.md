# PlutoLib: internal services and the Mod-author API

## Direction and scope

The requested design is two logical layers, not two additional crates or a replacement loader:

1. Internal services and host adapters own bounded validation, deterministic state transitions, native type conversion, version-specific Mixins targets and resource/lifecycle contracts.
2. Mod-author entry points provide namespaced registration, typed callbacks, constructors/builders and small re-exports. They delegate to the same implementations and owners.

The supported dependency direction remains Horizon -> Mixins -> PlutoLib -> a Mod choosing PlutoLib. Other Mixins Mods may call the supported Mixins surface directly. TConstruct chooses PlutoLib; that does not mean all old TConstruct consumers have migrated.

## Rules for additions

- Keep existing public paths, structs, error types and registration entry points. Add an ergonomic entry rather than silently replacing an old one.
- Use one registry and one owner per capability. Wrapper and legacy registrations participate in the same duplicate, ordering, freeze, bounds and removal rules.
- Do not turn pure planning/staging into an event or commit API. A proposal, snapshot or identifier does not grant native actor, slot, screen or network authority.
- Keep host types/dependencies in the existing allowed adapter layer. The pure crates do not import their native consumer. Current Mixins packaging restrictions remain in force.
- Keep validation and mutations explicit enough to audit mechanically: exact typed inputs/results, bounded work where needed, validation before publication, source/target hashes and named acceptance tests. Avoid hidden initialization, dynamic fallback owners and duplicate caches.
- A version adaptation must update pinned source/plan guards and run the corresponding actual host type/behavior gates. Do not weaken a guard merely to make an old version pass.

These are design constraints, not a promise that every existing API already meets a stable ABI or supports every future Horizon/Mixins revision. Several current public native adapters return native types, including Text. A change to those types can require explicit compatibility work in the public surface as well as the internal adapter. Pure API test success does not prove native binary compatibility.

## Current concrete entry points

- Inventory: inventory::plan_move, inventory::plan_between and inventory::stage_updates re-export the original types/functions. Old root and storage paths remain the same types. Both external examples were compiled and executed against one final crate.
- Payload observation: client_payload::OBSERVED.channel(...).register(...) delegates to the existing registry. It preserves borrowed bytes, the documented phase and subscriber order. It does not grant cancellation, sending, an open screen or reply authority.
- Recipe presentation: Replacement::for_entry(...).with_ingredient(...) constructs the existing move-owned proposal. Validation remains in the internal presentation operation. Native Book staging and consumer connection require their own proof.
- Fluid authoring: Declaration retains one validated Definition/render pair and delegates content/model generation and batch registration. One registry transaction owns the facts. It does not automatically publish assets or register a native renderer provider.
- Terrain authoring: the separate CELLS facade re-exports actual implementation types and delegates to the existing renderer registry. Its new actual-host verification is a separate gate; it is not included in the generic API proof below.

## Verified compatibility checkpoint

The exact generic composition at 7094f48e01dc17de65e0c0aa27eae418f07ec847 passed 147 inventory tests, 58 public API tests, one Guide example and one fluid example. All 207 Rust cases passed without ignored/filtered cases. Four external binaries were actually type-checked, and old/new storage examples each ran six assertions. Thirteen runner checks passed. Sources, exact file sets, locks, generated inputs and owned process cleanup were checked.

[Verified evidence](author-api-inventory-entry-result-20261106.md) links the immutable run and complete receipt. The earlier 203-test cutoff remains separately preserved. This generic composition does not prove the mixed native package, live Game/UI/LAN, complete TConstruct migration or main readiness.

## Language integration is a separate next unit

There is already one official language owner. The new internal adapter must delegate current lookup, ordered loading and native formatting to it. The Mod-author Key/Text layer retains keys and resolves through that adapter; it must not add a second CURRENT, dictionary, selected locale or hidden reload hook.

Authors provide assets/<namespace>/lang/<locale>.json in their own admitted content package. TOML is a preparation-time conversion to that native JSON format, not a new runtime parser or priority owner. Native source/namespace/pack/content overlay behavior must be documented and tested rather than silently reversed. A namespace directory does not automatically prefix or isolate translation keys.

The first runtime evidence must exercise actual Language/Text and item-name paths in two selected languages, including fallback, custom-name precedence, component key/style preservation and re-resolution. A dictionary lookup alone is not a translated-screen proof.

Known boundaries remain explicit: retained Guide Text is a resolved snapshot; same-locale resource reload is not detectable from locale code alone; old TConstruct English titles are also screen/LAN identities; some native format strings support only %s, positional %n$s and %%; and client-selected guest language is not automatically applied by the existing Guide LAN handler. These require separate consumer changes and acceptance tests, not a blanket promise that every UI is automatically translated.

Source preparation currently exists for the internal language adapter (22db77ec), the Key/Text layer (a630d1ab), and the item-display fixture is in preparation. They are excluded from the 207-test result. Actual native and screen-specific results must be added only after their corresponding gates execute.

## Safe integration order

Keep each source group immutable, add its old/new acceptance tests and examples, independently review the exact composition, run the smallest real-crate proof, then the relevant plan/apply/host and consumer gates. Preserve failed attempts and unrun gates. Publish coherent checkpoints as work progresses. PR regrouping and main integration are separate actions; this document does not close, retarget or merge any PR.
