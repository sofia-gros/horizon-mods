# Fluid render contracts: first migration checkpoint

This records the frozen first cutoff a6e25c8e / 52c551d1. The later, still
unverified native-material extension is described in `terrain-materials.md`.
Its explicit model/provider admission supersedes the temporary blanket
UnsupportedAppearance gate described below; no old proof is promoted.

This is a bounded shared-library source checkpoint on verified ordinary v15
(local `8715b123`, official Horizon `624c4f62` / Mixins `e8cd0aa3`). It does not
activate a TConstruct provider or claim a new native/CPU/GPU proof. The published
CPU/Game CI checkpoints remain unchanged and target their previous source.

## Registration capacity and publication

The geometry provider map now permits 256 exact groups, instead of 64. This
admits the known 70-family requirement while retaining a fixed budget shared by
all mods. A group is at most 256 bytes: registered names total at most 64 KiB,
and an unpublished candidate batch adds at most another 64 KiB, plus bounded
BTreeMap/node/function-pointer overhead. This is not a claim that the complete
allocator footprint is exactly 128 KiB.

`register_all(&[(&str, Provider)])` validates batch length, identifiers, duplicate
batch groups, existing groups, capacity and frozen state before extending the
live map. Any returned error publishes no member. An allocation panic is not a
recoverable Result transaction. The same callback may handle many exact groups;
each still consumes one entry. Existing single registration delegates to this
same operation. First mesh access still freezes even an empty registry; no late
addition/removal is enabled, including an empty batch after freeze.

No per-cell lock or allocation was added. Lookup remains a bounded BTreeMap
lookup, now over at most 256 rather than 64 keys. Per-cell 12-quads, per-section
8,192 custom quads and 262,144 custom emitted vertices remain unchanged. This
source statement is not a measured timing result.

## Display facts remain with the single authoritative fluid registry

Existing `Definition` struct literals, `register`, `register_all`, flow/contact
lookup and generated ordinary-block content remain supported. Legacy registration
gets the existing still texture, white RGB and Bottom render orientation.

`RenderDefinition` adds a flowing-texture ID, packed RGB tint and Bottom/Top
orientation. `register_with_render` and `register_all_with_render` publish them
with the corresponding Definition under the same registry write transaction and
revision token. There is no second global registry. Snapshot capture clones both
maps and the token under the same read lock; `snapshot.render(id)` borrows facts
from that exact immutable snapshot.

Display facts are immutable. Identical repeats are harmless; a legacy repeat does
not erase a custom style. Conflicting render facts, even a later attempt to add
custom rendering to a legacy-default registration, reject the whole batch. There
is no post-registration style replacement. Successful additions invalidate the
old revision once; failed/idempotent batches do not. Render orientation describes
geometry, not flow/contact surface semantics or permission to move a fluid upward.

**Material-aware rendering is not installed by this checkpoint.** The current
upper projection explicitly returns `UnsupportedAppearance` for any nondefault
style, making its prepared catalog unavailable instead of silently ignoring the
flow texture/tint/orientation. Default definitions retain their old projection.
The existing native fallback still applies. Storing metadata is not proof that
all 70 families display correctly, nor a migration of their content/save states.

## Oriented geometry, without fluid policy

`build_oriented(cell, Bottom)` is exactly the existing `build(cell)`. Top first
uses all the same validation, then reflects Y about the cell's midpoint, swaps
Up/Down faces, and reverses vertex/UV pairs to preserve outward winding. Back-face
flags stay paired. The original order is reflected (Down then Up, then sides),
not resorted. Coordinates remain fixed-size; no world reads or allocation occur.
This supports ceiling-anchored volumes mechanically, not an upward-flow kernel,
source corner smoothing or proof of the original mod's visual policy.

## Opaque UV proof boundary

Source review found that native `Sprites::region_opaque` proves opacity only over
the baked face's original UV rectangle, in every frame. Reusing that flag for a
larger UV region could incorrectly omit alpha testing. The new adapter refuses a
claim when an originally opaque face has no explicit render type and the new UV
rectangle is not an exact subset of the original finite rectangle. It does so
before emitting custom vertices; the whole staged section takes native fallback.

The subset check handles reversed coordinates and degenerate rectangles, rejects
nonfinite values, and uses no epsilon expansion. Nonopaque faces retain their
native classification; explicit render types retain their deliberate override.
This does not resolve a new sprite's alpha classification or Fusion association.
Future still/flow material selection must use actual prepared native material
facts, not swap only a layer number while copying old opacity/Fusion metadata.

## Evidence and remaining gates

- 19 existing terrain Python/source/package checks passed.
- Exact official copied-body and face-filter regeneration passed. This is not a
  new official Mixins plan/apply/type check.
- Fifteen new Rust tests are written: eight registration/revision cases, four
  provider/UV cases and three orientation cases. Execution is pending the serial
  resource gate. Prior Rust/native success is not reused as evidence for them.
- No new native loader, mesh differential, timing or GPU execution occurred.
- Next material-aware checkpoint must independently preserve baked face culling,
  render passes, alpha, tint, Fusion/connected-texture identity, and prepared
  owner/revision lifetime. It must also replace the explicit Unsupported gate
  only after the new adapter can honor those facts.
