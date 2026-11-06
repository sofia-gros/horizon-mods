# Explicit cell-face geometry

`horizon_client::mesher::pluto_terrain_cell_geometry` is a small, allocation-free
builder for an **unpublished** cell-face batch. It belongs to the same shared
terrain feature as the [prepared binding catalog](terrain-binding-catalog.md).
It adds one pure support module and no cell/mesh/draw hook, state or registry.

## Intended place in the custom-fluid path

A future custom-fluid meshing adapter can obtain a current resource-bound
catalog, inspect an immutable neighborhood, decide which family/level and faces
are supported, compute final corner heights and UVs, and call this builder.
It must then stage native vertices with verified sprite/pass/color/light data
and publish only at a separately validated native mesh boundary. This module
does **not** establish those missing lookup, staging or publication contracts.
It must not be used to write mesh output from the catalog's read-only callback.

The generic module receives only explicit geometry. TConstruct family IDs,
materials and source semantics are absent. In particular, it does not infer
water/lava behavior, the native weighted-height rule, epsilon, flow direction,
backface visibility, color, lighting, emission or translucency from a state ID.

## Contract

`build(&Cell) -> Result<Batch, Error>` validates every supplied value before
returning a batch. Input limits are explicit:

- Cell origin is section-local, with each coordinate 0 through 15.
- Final NW/NE/SW/SE corner heights and bottom are finite and within `[0,1]`;
  bottom cannot exceed any corner. The caller supplies any top epsilon adjustment.
- Side inset is finite and in `[0,0.5)`. It shifts side planes, not top/down faces.
- Each selected face supplies finite normalized `[0,1]` UVs and its backface choice.
- Top diagonal order is explicitly NW–SE or NE–SW. No shader-pack choice is inferred.

The output has at most twelve quads stored in a fixed array, with no Vec, heap
allocation, locks, callbacks or native buffer writes. Front order is Up, Down,
North, South, West, East; a requested back immediately follows its front.
Backfaces reverse the same position/UV pairs as `[0,3,2,1]`. Top-diagonal changes
rotate the position/UV pairs together, preserving winding. The batch carries
face/direction facts only, not native shader flags or a resource ownership token.

This restricted API is not an arbitrary, inverted-volume or gas-ceiling mesher.
Degenerate zero-height geometry is representable; whether to omit it is caller
policy. Numeric IDs/sprites cannot be attached or made valid merely by creating
a batch. No current native rendering function calls this module, so existing
chunk behavior and hot paths are unchanged. Allocation-free structure is proven
by the implementation; end-to-end performance has not been benchmarked.

## Official expression proof

`tools/prepare_terrain_geometry_oracle.py` reads the exact pinned official
`crates/horizon-client-render/src/mesher/fluid.rs` at Horizon
`624c4f6265c02296fbe8a80f029dd0d987fb94c0`. It checks the full file SHA and each
reviewed span SHA, then copies the native coordinate/UV/winding expressions
**verbatim** into a small test wrapper. The wrapper supplies primitive arguments;
it does not execute the native tessellator or any world/flow/shading decision.
Original MIT attribution is retained in the module and fixture.

The differential test checks all 64 face masks, both diagonal orders, with and
without backs, three section positions, four asymmetric/flat corner sets and
four inset values: **12,288 cases**, comparing all emitted position and UV
floating-point bits and face order. Four additional pure tests check capacity,
back winding, omission, section bounds and invalid inputs. The five Rust tests
passed on 2026-11-05. Five source/package tests also passed, including a regression rejecting inner
file attributes in the injected module.

The copied source expressions use native bottom/side UV conventions in the
oracle. Arbitrary explicitly supplied valid UVs are preserved by the builder,
but no new flow-UV algorithm is claimed. The official source hash is also pinned
by the existing mandatory complete-plan terrain-binding guard.

## Still unproven

The first installed v11 plan rejected two module-wide inner doc comments as
unsupported file attributes (E_MODULE), before apply. They were converted to
ordinary comments without changing runtime arithmetic; all five Rust and five
source/package checks were rerun successfully.

The corrected complete package subsequently passed the official plan (20.05s),
all eleven mandatory guards, apply (21.41s) and actual native lib+bin type check
(64s) at `3ba060dfca5e567bf9ff0b4853f429a221564c25`. Its archive SHA-256 is
`e42240e75471359eb9b3c82486e30a1940ba5308c1688fccda25bc83acd22890`.
The geometry source was checked byte-for-byte against that installed archive.
The group passed 187 Python checks including the 106-test inventory API runner;
these are whole-group counts, not all geometry-specific. The 5,042-file stage
seal, initial rejection and final results are retained in the
[proof record](terrain-cell-geometry-validation.json). No real chunk, worker, complete native resource assembly,
GPU drawing, reload or performance test has run for this module. A production
custom-fluid adapter must still address neighborhood/family policy, corner
smoothing, occlusion, output suppression of the old block model, pass/shading/
light metadata, sort/bounds/animation bookkeeping, resource invalidation and
failure behavior. Neither this builder nor the catalog completes custom-fluid
rendering or TConstruct recovery.
