# Recipe-book cache invalidation

`horizon_client::pluto_inventory::recipe_book::Invalidation` is a borrowed,
host-independent capability. Call `invalidate_definitions()` after a consumer
decides its recipe definitions or display IDs require a presentation refresh.
It does not select recipes or decide when refresh is necessary.

The private native adapter is
`crate::game::recipe_book::pluto_invalidation::with_game(game, callback)`.
It borrows only the current `Game` for the synchronous callback and retains
nothing afterward. This host-qualified entry is an official-Horizon adapter,
not a portable ABI or an arbitrary mutable-Game consumer callback.

## Exact current adapter behavior

On official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0`, it calls native
`forget_book`, which drops the book model and derived recipe category cache.
It also resets the client-known comparison snapshot and visible tab/button
view, clears ghost and alternative previews, and forgets last-placed and
last-clicked recipes. Subsequent ordinary native reads rebuild the relevant
cache; invalidation neither eagerly rebuilds nor emits an event.

It preserves search text/focus/drag state, selected tab/page, animation state,
the client display list/version, recipe definitions and indexes, saved recipe
knowledge, open/filter settings, inventory and advancement progress. Remote
versus local triggering remains the caller's decision. In particular, clearing
the local comparison snapshot is not a remote recipe-packet resynchronization
or a grant/revocation of knowledge.

These are the equivalent cache-reset mechanics of:

- `source-fake-material-book-ui.rs::tconstruct_invalidate_fake_material_book`
  at MOD commit `8c56bb52b6b77fae6115e894af9e369a5c3f9e08`.
- `source-travelers-book-ui.rs::tconstruct_invalidate_travelers_book`
  at MOD commit `a8fc1e18e9c819c93a60cd50aea87dc8a099efaa`.

Their recipe/material eligibility, refresh timing, display-ID allocation,
world-load logic and preview policy remain TConstruct responsibilities. Those
older consumer helpers are absent from this central source base; this slice
does not import or activate either family. When those consumers are recovered,
only their reset bodies should delegate to this capability at their existing
call sites.

## Evidence and limits

This adds one pure trait, one private native support module and no injection,
owned state, registry or new crate. Three source-boundary tests cover the exact
reset body, manifest scope and native-independent API. They are written but
not run in this slice. Formatting and diff checks are separate from the queued
grouped official apply/type proof. No UI execution, gameplay, remote-session
or save/reload result is claimed.

The separate [saved-book handoff](saved-book-handoff.md) uses this cache reset
after one-shot restoration. It does not change this capability's reset set.
