# Saved recipe-book handoff

PlutoLib separates three phases: capture saved IDs before native validation,
wait for the consuming mod's current recipe projection, and consume a pending
snapshot once. It does not choose a recipe family or grant newly eligible IDs.
The API is `horizon_client::pluto_inventory::recipe_book::handoff`.

## Capture and ownership

Register an owner, a positive finite entry bound and an ID-selection function
before world opening. IDs, grammar and bounds are consumer policy. Duplicate
owner registration is rejected. Selectors run in owner order outside registry
and pending-state locks. Only IDs already present in saved `Settings.recipes`
can be selected; highlights are intersected with those selected IDs. An exceeded
bound or overlapping owner claim rejects the complete capture with a warning.
With no registered selector, native snapshot reads are skipped. Empty selections
retain an empty one-shot marker but no recipe IDs. Registration never reads a save again.
Removal changes future captures, not an already captured pending snapshot.

On official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0`, the adapter runs
at `Game::finish_open_world` HEAD, borrowing the successful `PreparedWorld`
without moving or modifying its native fields. This is after preparation but
before native recipe-ID filtering, not the old consumer's prepare RETURN hook.
The official prepare body contains closures and try expressions incompatible
with that older RETURN approach. No native body is copied or re-entered here.

Pending data lives in a Mutex-owned field on that exact prepared ChunkMap,
with its save root. The ChunkMap then moves into the active world. It is never
indexed by a native pointer, global actor or path-only registry. A different
ChunkMap with the same path has different pending data. Each owner is removed
once; the empty capture marker remains so repeated capture cannot revive IDs.
ChunkMap clear discards pending state. Failed open paths before installation
drop the prepared state rather than attach it to the old world.

The current native opening path installs the prepared world before filtering.
Its later `clear_entities` clears parked Levels and entity lists, not the newly
installed world's ChunkMap. A later dimension switch moves the owning world's
state with that world; it does not copy a saved book into a guest or another
world. This is source inspection, not a live dimension/load execution result.

## Readiness and restoration

Borrow `crate::game::pluto_saved_book::with_game` only in a primary local
non-client-role scope. Remote clients, client execution roles, split guests
and all LAN acting metadata are rejected without consuming pending data.

Call `restore_saved(owner, Readiness::Defer, callback)` until native world
opening has completed and the consumer has validated its current source data
and recipe projection. `Ready` is an explicit caller promise, not an automatic
lifecycle inference. Do not issue it from an earlier open/reload callback while
Game's old and new world fields are transitioning. Native current-world tables
must also exist, and the attached pending save root must match `game.world_root`.
These errors and Defer leave the one-shot snapshot untouched.

A Ready call takes one owner before acceptance callbacks. It stages the
intersection of saved IDs, the current native RecipeIndex and the consumer's
acceptance predicate before changing knowledge. The predicate runs once per
still-indexed saved ID in lexical order. Highlights cannot add another recipe.
Rejection, removal or a callback panic cannot replay that snapshot later.

Restoration inserts only accepted saved knowledge/highlights and invokes the
shared recipe-book UI invalidation capability. It does not change saved
open/filter preferences, inventory, advancement progress or recipe definitions.
It emits no unlock event, toast, reward or guest synchronization. No arbitrary
snapshot supplied by a caller is accepted as restoration authority.

## Consumer and validation limits

The original `#941` fake-material flow at MOD commit
`8c56bb52b6b77fae6115e894af9e369a5c3f9e08` remains policy evidence only.
Its family prefix, hexadecimal grammar, 1024-item limit, source readiness,
recipe generation and grant/revocation decisions are not imported or activated.
Future recovery must register its selector before opening and replace only the
handoff mechanics at the documented readiness boundary. Guest synchronization
remains unsupported.

A narrow official plan proved the immutable prepared binding and Mutex-owned
ChunkMap field are accepted: one HEAD and one state. Six exact API tests and
two exact std-only state tests pass. They cover subset/highlight selection,
limits/conflicts, unlocked callbacks, indexed acceptance, distinct world state,
moving ownership, one-shot consumption and clear behavior. They do not execute
Game opening or restoration. The final source bundle has also passed the grouped official proof below.
No gameplay/save/reload pass is claimed.

## Final grouped official proof

Runtime commits `7713ac48` and `f309f818` passed in the complete actual PlutoLib
package at central cutoff `d039d95c634851bc2ee30c2aa11180e58b43a314`,
using official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0` and Mixins
`e8cd0aa38cffef49cb2e088ef716baa107148c02`. The package contains two crates,
43 support modules, 27 injections and three owned states.

- Official plan passed in 19.54 seconds.
- Apply and complete-plan/source guards passed; apply took 21.03 seconds.
- Actual native library and binary type checking exited zero in 74 seconds.
- The combined source suite passed 94 checks and the actual shared API passed
  68 tests. These totals include earlier features, not new handoff-only tests.
- The handoff-specific six API, two std-only state and three source-boundary
  checks above passed separately.

The first complete-plan attempt rejected the capture support module under
`world_loading.rs`, whose unresolved glob import prevents safe module insertion.
The final placement is a sibling under `game.rs`, borrowing the restricted
`world_loading::PreparedWorld` type through its documented parent visibility.
No native function body, visibility or Mixins guard was relaxed. This placement
fix changes neither capture timing nor pending ownership.

Logs are under the central checkout's
`.work/plutolib-foundation-proof/grouped-v6b/`. The specific evidence is recorded
in [saved-book-validation.json](saved-book-validation.json). Compilation does
not prove live open/restore ordering, save/reload behavior, consumer activation
or guest synchronization; those remain separate integration work.
