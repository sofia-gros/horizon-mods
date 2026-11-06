# Explicit recipe-index compare-and-apply

`horizon_client::pluto_inventory::recipe_book::index` provides an explicit,
bounded operation on the values of a currently borrowed recipe index. It does
**not** provide persistent ownership, reload identity or automatic projection.
The official adapter is
`horizon_client::recipe_book::pluto_index_patch::with_index(index, callback)`.
The callback receives the host-independent `Index` trait, not a mutable game.

## Operation

1. Read a complete `Snapshot`. Every native `Known` field is represented,
   including a station whose notification is disabled, special flag, display
   count/range and optional result. Unsupported existing station strings are
   preserved exactly rather than normalized.
2. Supply `Patch.expected`, exact IDs to remove, definitions to insert and an
   explicit `Allocation`. An existing ID must also be in `remove` to replace it.
   No namespace prefix or previous content is treated as proof of ownership.
3. `compare_and_apply` checks the entire current snapshot, validates ranges,
   stages all records and conversions, then replaces the native map once.
   An ordinary returned error leaves the index unchanged. Allocation failure or
   process abort is not a recoverable transaction guarantee.

`Append` preserves unrelated ranges and starts after the largest retained end,
including retained empty-range positions. `FirstGap` uses the lowest available
contiguous range for each inserted ID in lexical order. Each differently sized
record searches from the first gap. New zero-display records use position zero
and consume no range. New special records must have zero displays.

The operation rejects negative/overlapping current ranges, checked integer
overflow, missing removals, unacknowledged existing IDs, stale values and budget
overflow. Mechanical bounds are 65,536 records, 16,384 IDs in each change set,
16 MiB of snapshot text and one million free-range comparisons. These are
library resource bounds, not a recipe family's content limit.

The current official `Known.station` is a static string. New values are limited
to the eight stations produced by native `crafting_station`; arbitrary strings
are not leaked or interned. Existing unknown stations remain untouched.

## Actual caller requirements

The previous fake-material and Travelers refreshes change crafting definitions
and metadata **before** their private index helper. A newly fallible operation
cannot safely replace that helper call alone. On failure, that sequence would
leave definitions and the index inconsistent.

A caller that changes those fields together must:

- Build and validate its new crafting and metadata candidates without changing
  live tables. Source recipe meaning, IDs, eligibility and limits stay there.
- Clone the current `RecipeIndex`, capture the expected snapshot, and apply the
  explicit patch to that candidate while holding the synchronous table borrow.
- If any step fails, discard all candidates. After success, publish the prepared
  crafting, metadata and index values in a no-fail move/assignment tail, without
  callbacks, reloads or observers between the assignments.
- Only afterward restore/grant/revoke knowledge or invalidate presentation as
  required by the caller's policy. This API does none of those actions itself.

The native `Recipes.crafting` and `Recipes.meta` fields are individually
cloneable and public, while `RecipeIndex` derives `Clone`. This makes that
bounded staging shape possible without cloning or altering other recipe tables.
The fixture exercises failure and success of this ordering with the exact
native index and modeled outer definition/metadata payloads. It does not claim
that either TConstruct refresh has already been migrated or proven in a game.

Do not use an old receipt to decide what this caller owns after reload. A
byte-identical index clone or reload passes value comparison by design. The
consumer must choose the current records it is authorized to replace; this API
does not establish that authority or lifecycle relationship.

## Official lifecycle and network limits

At Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0`, the main index is owned by
`Game.world_tables -> StationData -> RecipeIndex` by value. The index may be
replaced by reload without changing a parked ChunkMap. The no-pack fallback is
shared resource data, not a world-local owner. No global pointer/hash sidecar is
introduced.

The native load path synchronizes recipe content after reload listeners, but
`reload_world_tables` performs content/LAN synchronization **before** its final
reload listeners. LAN `ServerAdvancements` also has a separate display index.
Moreover, native content sync encodes source files, not these in-memory index
records. Therefore this operation is not multiplayer publication, guest display
synchronization, or a safe choice of the complete projection lifecycle. Those
boundaries require separate consumer integration and tests.

## Source evidence

- [Native index fields, loader and lookups](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/recipe_book.rs#L693-L860).
- [Native crafting/metadata fields](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/crafting.rs#L1327-L1342).
- [World-open callback order](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/game.rs#L12630-L12654) and [reload order](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/game.rs#L20254-L20306).
- [Native source-file recipe synchronization](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/game/native_recipes.rs#L376-L403).
- [Old fake-material caller](https://github.com/sofia-gros/horizon-mods/blob/8c56bb52b6b77fae6115e894af9e369a5c3f9e08/tconstruct/mixins/tconstruct/source-fake-material-world.rs#L80-L96) and [Travelers caller](https://github.com/sofia-gros/horizon-mods/blob/a8fc1e18e9c819c93a60cd50aea87dc8a099efaa/tconstruct/mixins/tconstruct/source-travelers-world.rs#L214-L246).

This slice adds one public API module and one native support module, with no
injections, state, registries, Game adapter, automatic activation or consumer
policy. See the validation record for exactly which checks have completed.

## Completed validation

The exact-index fixture passed 15 tests (11 pure API cases and four adapter/
modeled-publication cases), with four source/package tests. The complete
installed package at MOD `c55f4985001999b7ea9aa34f7470e8725a5684af` passed
official plan, complete-plan guards, apply and actual lib/bin type checking
against the official Horizon/Mixins pair above. The grouped run also passed
130 Python checks and 93 API tests across services.

This establishes installed composition and types, not live world publication,
consumer migration, reload identity, gameplay or networking. Exact evidence is
in [the validation record](recipe-index-patch-validation.json).
