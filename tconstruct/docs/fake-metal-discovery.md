# Fake-metal recipe book index and previews

Partial high-priority #379 acquisition support, stacked on the world-local
crafting projection. This slice does **not** add automatic acquisition discovery.

## Implemented

- After rebuilding the two fake-metal conversion families, rebuild their reserved
  `tconstruct:fake_metal_native/` index entries in the same world's RecipeIndex.
  Only complete, matching generated ID/material/shape/marker/count records receive
  one display. Other recipes' display numbers remain unchanged. Stale synthetic
  entries disappear when a material or source direction is no longer eligible.
- Native Book construction already carries the output Material. Add Material to
  every ingredient display stack, including both the overlay and ghost preview.
- Forget the old book, known-display cache and stale preview/click state after
  projection. Existing native recipe commands can discover the indexed IDs after
  the world snapshot is ready; no recipe knowledge is granted by this refresh.
- Clicking or shift-clicking these entries on the local authoritative game shows
  a read-only ghost recipe. Slots are neither cleared nor filled. The native
  placement query returns None for marked conversions, including other callers
  such as server menu placement. They are excluded from the craftable filter.
  Manual crafting still runs the existing exact Material assembly guard.

## Deliberate boundaries

Native stock counting and auto-placement compare item IDs, not Materials. They
must not combine e.g. bronze and invar fake ingots or clear the player's grid to
attempt the wrong material. This is preview-only support until a material-aware
placement transaction exists. It is not a claim that the recipe is uncraftable
by hand.

The two source acquisition advancements and their dynamic variant rewards are
still absent. Source recipe IDs are not registered as fabricated craftable aliases.
Saved-book validation runs before the late source snapshot/projection. A bounded
one-shot handoff now captures only previously saved synthetic knowledge/highlights
at successful world preparation, carries it in that PreparedWorld's ChunkMap,
and restores it only after the current world's generated index validates it.
The handoff is consumed once; missing/disabled recipes stay dropped. It does not
re-award completed advancements, learn newly eligible materials or resurrect
recipes removed by `/recipe take`. Source goal mapping remains separate.

Restoration waits outside borrowed LAN guest player scopes. A host world-update
hook can finish a pending handoff if the first snapshot was captured inside such
a scope. This does not restore a guest's own saved book and does not add network
synchronization. No files are reread, written or migrated by the handoff.

Only the exact two generated prefixes with bounded lowercase hexadecimal material
suffixes are candidates. At most 1,024 saved variants are retained, matching the
projection's two directions times 512 visible materials. An over-budget snapshot
fails closed. Pending highlights must also be saved known IDs and currently valid;
settings and unrelated recipe knowledge are untouched.

Remote clients do not install authoritative projections. Network recipe display
IDs/content synchronization remains a separate integration gap; this change is
not LAN recipe-book support. The reserved prefix belongs to this generator.

## Review and deferred checks

Reviewed host RecipeIndex, Book, ghost/overlay rendering, placement and refresh
boundaries. Inline Rust tests cover record rejection, stable unrelated display
IDs, isolated indexes and stale removal. Saved-book tests additionally cover
candidate bounds, highlights, current-world validation, no new grants, one-shot
consumption and preservation of unrelated knowledge/settings. Python tests cover
module/hook wiring, preview safety and the explicit missing discovery boundary.

Tests, compilation, native injection application, in-game UI, save/reopen and LAN
were **not run**. No CI inspection, version checks or merge. A combined integration
pass must verify these with the preceding crafting/material acquisition stack.
