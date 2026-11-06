# Advancement item-tag/index slice, 2026-11-05

This is part of the existing broad advancement recovery feature, not a separate
PR or a completed acquisition-stack migration.

## Constructor blocker and bounded correction

Actual official planning rejected the initial AdvState field: after the required
empty constructor array was serialized, E_STATE required nonempty explicit
constructor sites. Accepting derived Default's initializer is not sufficient.
AdvState has no source constructor literal. The initial candidate was never
applied or declared native-validated.

The corrected design removes that unsupported state entirely. One ChunkMap owns
immutable owner snapshots and an Arc-owned Any payload holding exact snapshot,
tree and custom index identities. The render layer uses std types only; only the
game adapter downcasts the payload. A wrong type or changed identity rebuilds.
There is no global or actor cache, unsafe pointer access or client dependency in
render. ChunkMap has the supported explicit Default constructor.

Custom indexing never writes native adv.inventory_index. No-provider/released
paths therefore retain the original native cache. Replace/clear and the explicit
invalidate_candidates API affect only the custom cache, not the event hash or
progress. Parked A-to-B-to-A worlds retain their own custom cache, and dropping a
detached world releases its snapshot/cache even if a returned index remains alive.
Unknown, owned-empty and unmanaged states stay distinct; conflict rejection is
atomic. Source producer/allowlist policy remains outside PlutoLib.

## Focused checks

- 23 real shared Cargo API tests passed before the documentation-only API scope
  clarification (no API implementation changed in this correction).
- 9 corrected bounded real-adapter tests pass: no-provider native fallback,
  unknown/empty, replacement/idempotence, actor/progress invariance, A-to-B-to-A,
  clear/release, tree/explicit invalidation, conflict rejection, wrong-payload
  recovery and detached-world lifetime. Host Game/index types are modeled;
  the shared API, adapters and native state sources are the actual files.
- 6 unified-package guards pass, including no unsupported AdvState declaration,
  exact module bytes and supported explicit constructor coverage.
- 2 separate TConstruct policy projection tests passed; publisher activation
  remains intentionally absent from the runtime capture path in step 2.

The corrected actual packaged plan passed in 16.25 seconds on official Horizon
624c4f6265c02296fbe8a80f029dd0d987fb94c0 and Mixins
e8cd0aa38cffef49cb2e088ef716baa107148c02. The archive was central 1e602b61
plus the bounded 799d2443 correction (five corrected source files and removal of
the unsupported state), with 2 crates, 25 modules, 24 injections and 2 supported
states. Archive SHA256:
59d1134f7703a9ebef2518f3f041053ad9809106c036687a2cbe0dd8ad4f9a62.
This is actual source/package planning, not a metadata-only probe.

The complete-plan exclusivity/source guard passed on that exact plan, and its
6 regression tests passed. Both copied methods require sole ownership, exact
function fingerprints, actor bindings, pristine file hash and helper bytes; all
competing hook phases fail closed. Canonical predicate fallback is separate.

Evidence is preserved under .work/official-advancement-recovery/
guard-cutoff-1e602b61: corrected-cutoff.json, corrected-plutolib.hnmod,
corrected-official-plan.json and corrected-plan.log. This plan does not include
subsequent central features. Final grouped apply/type-check is still pending.
Existing predicate/event proofs do not cover this new state/hook set.
Game/LAN behavior, producer readiness, full runtime wiring and old consumer-hook
removal remain pending step 3. Original PR-head classifications are unchanged.
