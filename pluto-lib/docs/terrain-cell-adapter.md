# Bounded native terrain cell adapter

This is the next shared rendering layer after the prepared binding catalog and
fixed-size geometry builder. It targets official Horizon
`624c4f6265c02296fbe8a80f029dd0d987fb94c0` and Mixins
`e8cd0aa38cffef49cb2e088ef716baa107148c02`. It does not register any consumer,
coerce custom blocks into native water/lava, or establish GPU behavior.

## Contract and dependency direction

`horizon_client::mesher::pluto_terrain_cells::register(group, provider)` registers
one pure geometry callback for an opaque group already in the prepared catalog.
The renderer owns this one policy registry; the authoritative fluid definitions
remain in the existing upper API. No reverse renderer-to-game dependency or copy
of that fluid registry is introduced.

Registration closes on the first `mesh_reusing` invocation, even if the registry
is empty. Duplicate groups, poisoned state, more than 64 providers, control
characters, and group names exceeding 256 bytes are rejected. Late registration
is rejected rather than changing only newly rebuilt sections. Registration must
finish before meshing starts; no runtime remove/replace capability is provided.

A callback receives the exact prepared binding, integer world cell position and
a read-only immediate 3x3x3 prepared-render neighborhood. Neighbor absence means
no catalog binding, not an unloaded world or air guarantee. Official padding can
repeat the center chunk's edge into missing neighbors. It returns None to keep
the native model, or an existing fixed-size geometry Batch with coordinates in
**cell-local [0,1]**, not section coordinates. Empty geometry explicitly omits the
eligible model. Callbacks must be pure, bounded and thread-safe. There is no
mutable host, send, texture load, native mesh or retained resource capability.

## Eligible models and unchanged native behavior

Only a custom prepared state with native `FluidKind::Empty`, exactly one selected
baked model, at most six faces, one face per facing and the exact prepared sprite
is eligible. Multi-part, duplicate-direction, multi-material and missing models
remain native. Every emitted direction must have a source face before any vertex
is written. Resource packs cannot silently turn an ambiguous model into a chosen
material. Policy must decide whether it is appropriate to replace a model; this
API does not recognize fluid names, level grammar or content namespaces.

The version-pinned copied `mesh_reusing` body retains all native preparation,
early returns, light/entity collection, native water/lava tessellation, randomized
model selection, block offsets, visibility and leaf rules, and buffer finalizing.
The one added dispatch is immediately before its ordinary model-face loop. No
provider keeps the original body through HEAD Continue. No claim keeps the exact
original model loop in the copied body. An empty or unavailable binding catalog
also keeps native behavior.

Claimed geometry changes only positions and UVs. It retains each selected face's
cull direction, facing, light, AO/shade/tint/color, render type, pass, resolved
sprite, opacity/translucency and connected-texture/fusion metadata. The exact
native cull and stacked-pane filter is reused, then `Mesher::face` computes the
native lighting, connected textures, overlays, packed vertex colors/light/UVs,
render pass and animation notes. This is the ordinary terrain-material pipeline,
not native water/lava shader flags. Back faces keep their source face's lighting
and cull metadata. The API does not reinterpret culling for shortened surfaces.

There is no per-face String clone: the temporary baked Material keeps an empty
nonallocating name because the pinned native emission reads only its
`force_translucent` field; its resolved layer remains unchanged. The full mesher
source and absence of other mesher-file hooks are mandatory guards for that
assumption. The original native path never receives this temporary metadata.

## Bounded staging and concurrency

The catalog and frozen provider map are acquired once per section; each eligible candidate does bounded map lookups. There is no
registry/cache lock or revision callback per cell. Position/UV preparation uses
fixed arrays, with at most 12 input quads per cell, 8,192 custom input quads per
section and 262,144 actual custom native-emitted vertices per section. Native
connected-texture expansion remains native and bounded by its pinned code. A
limit is checked after each native face's bounded emission. On overflow the
entire private section output is discarded and the original native body reuses
its allocation. Callback work itself is a provider contract, not forcibly timed.

The exact Assets/RenderAssets/MeshTables owner triple is checked by the existing
catalog, and its registry validity is checked before and after the private mesh
computation. Invalid geometry, unavailable source directions or output overflow
also discard the whole staged section. Native output is then recomputed using
reused output/scratch buffers. No partial custom mesh is returned.

This is a point-in-time validity check, **not a render-long lease**, and it cannot
roll back arbitrary callback side effects. It does not evict meshes previously
published to a GPU. Late authoritative registration makes the old prepared
catalog unavailable for new computations; the caller must prepare new resources
and perform its usual remeshing lifecycle. Runtime callback changes are rejected.

## Source and composition guard

`tools/check_terrain_cell_plan.py` must run on the complete installed plan before
apply. It verifies official source hashes, the target fingerprint and bindings,
exact raw or packaged module/hook identities, exact runtime module bytes, and
regenerates the copied body plus native face filter from reviewed transformations.
The first implementation reserves every injection in `mesher.rs` to this one
lowest-priority cancellable HEAD, rejecting alias spellings, interior hooks,
RETURN changes and unknown neighboring method hooks. Other compositions need a
new proof; they are not silently accepted. Native code attribution is retained.

## Current validation and remaining limits

- Actual no-op HEAD feasibility plan: one injection, exit 0 in 7.26 seconds;
  target receiver, region, section, mutable output and scratch bindings accepted.
  Artifacts: `/tmp/pluto-mesher-head-probe-20261106`. No apply/type claim follows.
- 17 small std-only tests passed (five new registry/budget/coordinate tests plus existing
  cache/geometry tests). Nineteen terrain source/package/guard tests passed.
- Exact source/body/filter regeneration passed against the official export.
- Complete actual package plan passed at source `d63a7c8e`: 2 crates, 63 modules,
  34 injections, 3 states, 20.50 seconds. Both terrain guards passed against the
  packaged source and identities. Archive SHA-256:
  `058f90042accfb876c1e6941da2e22ad10692b1391ee818e5147c7d9b55b9d6b`.
  Artifacts: `/tmp/pluto-terrain-cell-plan-20261106`.
- Apply and the native host check are still pending. CPU mesher differential
  fixtures and GPU display are not yet run. No full prepared official mesh
  resource root is available; ignored official reuse tests are not evidence.

No content callback, corner-height/flow policy, inverted-volume support, fog,
fluid current, multiplayer rendering, resource-reload gameplay or performance
result is claimed by this groundwork. Those are separate consumer/runtime gates.
