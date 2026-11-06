# Remaining custom-fluid mesher boundary

Source-only audit at official Horizon 624c4f62 / Mixins e8cd0aa3, after the
prepared catalog and explicit geometry builder. No new target plan, replacement
adapter, mesh hook or draw behavior was introduced by this audit.

## Why the easy hooks do not establish cell replacement

- Native `mesher/fluid.rs::tesselate` (around 216) has destructured tuple and
  position parameters. The official helper binding rule requires named patterns.
  Its body also chooses native water versus lava behavior; it is not a generic
  custom-fluid entry point.
- `Mesher::face` (around 1648) has a destructured `(id,state)` parameter and is
  called only after a baked model face exists. It cannot create a new fluid cell
  surface by merely observing the old model.
- `StateModels::models_at<'a>` (catalog.rs:448) has an explicit function lifetime,
  rejected by the official target signature rule. Its other callers also make
  unsafely global model suppression inappropriate.
- `Mesher::mesh_reusing` contains closures. Ordinary interior call/value rewrite
  validation rejects such bodies; the plain-closure RETURN exception does not
  establish that those interior hooks work. These are static rule matches, not
  newly executed planner failures.
- A RETURN hook might be planner-supported, but this has not been tested. It
  cannot by itself safely replace the old custom-block model: the resulting
  TerrainVertex carries no originating state/cell identity. Removing vertices
  by geometric coincidence would be an unsafe guess. Appending fluid geometry
  while keeping the old model would duplicate surfaces.

## A candidate that needs an explicit decision and proof

A version-pinned Pluto-owned cancellable HEAD adapter of `mesh_reusing` could
preserve the exact official body and add a generic claimed-cell dispatch after
`hidden(...)` and before the native fluid/model branches. This follows the
already-used shared caller-adapter pattern, but no feasibility success is
claimed for this target yet.

The target parameters are named and its impl is lifetime-only. A helper must
borrow `&mut out`, then take the owned SectionMesh only when it will return
`Break`; taking it on a `Continue` path would lose the original caller buffer.
When no custom-cell consumer is registered, the adapter must Continue the
original function without replacing its body or clearing any buffer.

A sound implementation would have to preserve all native early exits, scratch
reuse, block-entity/light collection, opaque visibility, existing water/lava and
ordinary-model paths, animation collection and later sorting/bounds work.
Claimed-cell replacement must skip exactly its old model path, never unrelated
cells. The native caller body and inserted hunks must be regeneration-checked,
with complete-plan conflict rejection; a copied body is not a stable upstream
extension point.

A section-scoped immutable catalog can avoid per-cell registry locks. Yet the
current read-only catalog callback is not a mutation lease: custom output must
remain unpublished until revision checks complete, and a stale revision needs
an explicit fallback rather than publishing partially replaced surfaces. A
native staging/writer contract is still needed for pass, color/light/emission,
sprite flags, animation, bounds and sort metadata. TConstruct family and shape
choices belong in a separate consumer.

This candidate must first pass an actual narrow official plan, exact-body type
and no-claim differential tests, then bounded claimed-cell replacement tests.
None of those gates is satisfied by the current pure geometry test. It remains
a future design choice, not an installed or verified rendering fix.
