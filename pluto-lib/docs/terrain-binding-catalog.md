# Prepared custom-terrain bindings

This is bounded groundwork for rendering custom world fluids. It prepares an
immutable lookup from registered custom block-state IDs to their declared sprite
layers. It does **not** tessellate fluids, change a draw pass, or turn custom
blocks into native water/lava. Existing nine stepped ordinary-block models,
contact heights and current rendering are unchanged.

## Ownership and dependency direction

The single authoritative definition registry remains `pluto_api::fluid`, owned
by the gameplay layer. Its projection supplies generic explicit block-property
selectors, texture names and opaque group/variant IDs to two public renderer
support modules:

- `horizon_client::mesher::pluto_terrain_bindings`: bounded descriptors and results.
- `horizon_client::mesher::pluto_terrain_binding_host`: preparation and read-only
  inspection of the resulting catalog for a borrowed native Mesher.

The renderer imports neither the gameplay API nor TConstruct. Its cache is a
**derived resource cache**, not a second content/physics registry. Fluid family
content, materials, acquisition and effects remain consumer policy.

`PackResources::assemble` HEAD receives the actual `Arc<Assets>`,
`Arc<RenderAssets>` and `Arc<MeshTables>` before publishing the WorkerContext.
Each cache entry holds weak references to all three. A lookup upgrades them and
checks all three exact owners while the upgraded owners are alive. Equal values,
a generation counter, a block registry pointer alone, and recycled allocation
addresses cannot stand in for this identity. Arc clones match; a reload's new
RenderAssets or MeshTables does not. The render generation is diagnostic only.

At most 64 live owner triples are retained; expired entries are pruned at later
preparation. A full live budget rejects a new publication without evicting
another live catalog. Weak references do not keep resource values alive, though
Arc control allocations remain until pruning. Native 624c has no `Arc::get_mut`
or `Arc::make_mut` use on these immutable resource owners; adding weak references
would matter to another mod expecting sole Arc ownership, so that is not a
supported unreviewed composition.

## Preparation and failure

The gameplay projection captures the single registry snapshot and revision in
one read-lock transaction. It emits exactly the nine declared level selectors
0 through 8 per definition, then resolves each selector against the actual
BlockRegistry, MeshTables facts and RenderAssets sprite map. The lower service
never interprets fluid levels or supplies a water/lava fallback.

Resolution rejects missing states/facts/textures, duplicate variant selectors,
colliding resolved states, non-model states, native nonempty FluidKind, and
sprite layers outside the loaded atlas or native terrain layer range. All
resolution is staged before one cache publication. A failure is an explicit
`Status::Unavailable(Error)`, not a successful empty catalog. A truly empty
registry can produce `Ready` with no bindings. Lock poisoning, an unmatched
owner, a conflicting same-owner snapshot or the live-owner budget returns a
separate `CacheError`; publication errors are logged by the upper adapter.

Projection is bounded to 16,384 bindings and 1 MiB of selector text, checked
before ninefold descriptor copies. Capturing the existing registry snapshot
still has its existing cost proportional to the number of registered definitions.
Resolution happens during resource assembly, not per cell. No current meshing
or draw path calls `with_frame`, and no new per-cell lock/callback is installed.

A `Ready` binding means the declared state, model flag and declared atlas layer
exist in those owners. It does **not** prove that a baked model uses that sprite,
that every model variant is usable, or that the native loader assembled these
particular fixture assets in a running game.

## Registry changes and read-only inspection

`RegistrySnapshot::revision()` supplies an opaque monotonic validity handle.
A successful addition invalidates the old handle and publishes a fresh handle
with the new registry map under the same write lock. Empty, identical and failed
batches do not invalidate it. Existing immutable snapshot contents remain
readable. Registry poisoning never means a current revision. This does not
freeze registration or change contact/flow's existing per-sample lookups.

The derived render catalog belongs to the registry snapshot taken at resource
assembly. A later successful registration makes that catalog unavailable even
if the new family has not yet been used. It requires explicit resource
repreparation with a new owner triple and all required states/textures present.
Preparing again on the same owners cannot replace or reactivate their original
snapshot. This is deliberately different from dynamic contact/flow lookup.

`with_frame(mesher, callback)` is **read-only inspection or staged computation,
not a frame-long lease**. It releases the cache lock before checking the upper
revision or invoking the callback. A stale revision before inspection supplies
`Unavailable(StaleRegistry)`. A change during inspection returns
`Err(StaleRegistry)` and discards the callback's return value.

The callback must not publish mesh or other side effects. It may compute an
unpublished result and use it only after `Ok`; the post-check cannot roll back
side effects. Even `Ok` is a point-in-time observation, not a lock preventing a
subsequent registration or a permission to retain numerical IDs across owners.
A future drawing adapter needs its own explicit staging/publication contract.

## Official lifecycle and composition evidence

All source evidence is pinned to Horizon
`624c4f6265c02296fbe8a80f029dd0d987fb94c0` and Mixins
`e8cd0aa38cffef49cb2e088ef716baa107148c02`:

- `game.rs:2183–2268`: PackResources owns all three resources; assemble constructs
  the worker context from the supplied owners. `context_with` clones the same
  owners while replacing only biome/light context.
- `game.rs:20780–20800,22220–22225`: initial content preparation precedes resource
  assembly. `game/mods.rs:41–57,132` executes declarations before content loading.
- `game/packs.rs:485–537`: staged reload calls content preparation, constructs new
  RenderAssets and MeshTables and enters the same assembly boundary. A failed
  staged resource set is not made current by this service.
- `streaming.rs:994` and `mesher.rs:1407`: workers retain the Arc owners and the
  Mesher borrows their exact values.
- `mesher.rs:1503–1511` and `mesher/fluid.rs:250–259`: native liquid dispatch and
  tint/emission specialize fixed water/lava kinds. This service does not enter
  that path for custom blocks.

`tools/check_terrain_binding_plan.py` must run on the **complete installed plan**
and pristine official source. It pins these source bytes, the three exact
module placements/sources and the exact non-cancellable assembly HEAD. Any other
hook on the same resolved assembly function, including an alias target spelling,
is rejected pending composition review. The guard does not sandbox arbitrary
trusted native mod code or claim all hypothetical Arc mutations are impossible.

## Verification and remaining work

The bounded proof record is [terrain-binding-validation.json](terrain-binding-validation.json).
On 2026-11-05 the actual official planner accepted three modules and one HEAD.
Twelve fixture tests passed: eight pure cache/revision cases and four adapter
cases using the real official BlockRegistry/BlockState/FluidState with explicitly
modeled Assets/RenderAssets/MeshTables/Mesher shells. The revised real Pluto API
suite passed 31 tests, including two new revision tests. Eight package/source/plan
guard tests passed, including three packaged-ID regressions. These are not complete native type or gameplay proofs.

The complete actual package passed official plan, all nine mandatory guards,
apply and native lib+bin type checking (68 seconds) at shared cutoff
`529ad7229422121d6bda28e5b5c7af5279fa0c76`. Its archive SHA-256 is
`3c769c1399095cea630b2de17dc14a6a520876aa27c6bd0735d2e07bbc9b7fed`.
All five terrain/API runtime files were checked byte-for-byte against that
archive. The full group contains 2 bundled crates, 57 modules, 32 hooks and
3 states; 159 Python and 98 inventory-API tests passed across the whole group.
The initial v10 preflight correctly rejected package-prefixed IDs; the central
37ad5021 fix accepts only the exact reviewed component or packaged IDs. Runtime
source bytes did not change. Logs and the 5,041-file stage seal are recorded in
the proof JSON. This establishes native type compatibility, not execution of a
real resource assembly. Live reload, chunk meshing, GPU output and performance
remain untested.
Custom-fluid corner heights, culling, flow UVs, translucent ordering, emission,
fog, and inversion have **not** been implemented by this catalog. No consumer
migration or TConstruct rendering recovery is claimed.
