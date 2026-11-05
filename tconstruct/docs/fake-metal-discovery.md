# Fake-metal recipe book index and previews

Partial high-priority #379 acquisition support, stacked on the world-local
crafting projection. The two source acquisition goals now map to the current
world's validated generated variants; material-aware auto-placement remains absent.

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
  the world snapshot is ready; index refresh itself grants no recipe knowledge.
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

## Source discovery

The two source advancement IDs remain `tconstruct:recipes/misc/tools/parts/`
`fake_ingot_to_block` and `fake_block_to_ingots`. Their source inventory criteria
are preserved exactly: any fake ingot or any fake storage block, respectively.
Acquisition OR an unlocked current native variant rewards the complete current
family. Native recipe events and advancement rewards perform the actual grant.
There is no fixed six-alloy list and no grant during index/tree refresh.

The source `has_item` and `has_the_recipe` criterion names remain, preserving
existing earned progress. Each validated material variant gets one additional
recipe-unlocked alternative with a stable hexadecimal material suffix. All
alternatives are in one OR requirement group. Rewards include only the current
world's matching direction, at most 512 entries. Original source recipe IDs are
not registered as fabricated craftable aliases.

Before a valid index exists, or when the family has no eligible variants, both
original criteria use `minecraft:impossible` and rewards are empty. Existing
progress is not revoked. The ready index enables those goals through the normal
advancement-tree return hook and after recipe projection/reload. Unchanged trees
retain Arc identity; changed trees keep advancement list positions and rebuild
trigger/recipe indexes, then invalidate the inventory predicate cache. A per-player
advancement-state cache holds the last mapped tree Arc and recipe generation, so
normal repeated tree reads do not regenerate hundreds of JSON criteria. Reloads,
new snapshots and different trees invalidate that cache without global state.

Only the complete host-supported source goal shape, or this adapter's exact
prior generated shape, is remapped. Datapack changes to predicates, requirements,
parent, display, experience, loot or recipe rewards are left untouched. Fields
ignored by the native advancement parser are outside this recognition contract.

Newly eligible variants are not automatically awarded after their family goal
was already completed; this preserves explicit recipe removals rather than
re-granting every completed goal on reload. That dynamic expansion case remains
separate. Inventory criteria run on native inventory events, not synthetic
reload events.

## Saved knowledge

Saved-book validation runs before the late source snapshot/projection. A bounded
one-shot handoff now captures only previously saved synthetic knowledge/highlights
at successful world preparation, carries it in that PreparedWorld's ChunkMap,
and restores it only after the current world's generated index validates it.
The handoff is consumed once; missing/disabled recipes stay dropped. It does not
re-award completed advancements, learn newly eligible materials or resurrect
recipes removed by `/recipe take` after the one-shot restore.

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
module/hook wiring, preview safety and source mapping. Discovery tests additionally
cover exact source fixtures, OR criteria, family isolation, unavailable context,
idempotent mapping, stable advancement indices and modified-goal rejection.

Tests, compilation, native injection application, in-game UI, save/reopen and LAN
were **not run**. No CI inspection, version checks or merge. A combined integration
pass must verify these with the preceding crafting/material acquisition stack.
