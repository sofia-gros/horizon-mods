# Fluid contact/query contract

`pluto_api::fluid::contact` is the read-only geometry portion of the shared fluid
feature group. It adds no host injections, effects, entity movement or
consumer-specific rules. The caller supplies a stable world view using either
an existing `World` adapter or a closure returning `Sample`.

## Queries

- `query_point(world, point)` and `query_point_with(point, sample)` return `Dry`,
  `Unloaded` or a fluid cell with its cell coordinate, upper surface and positive
  depth below that surface. Invalid coordinates never call the provider.
- `query_box(world, bounds, limits)` and `query_box_with(bounds, limits, sample)`
  return known positive intersections grouped by fluid ID. Each entry has its
  volume in cubic blocks and count of touched cells. IDs are sorted.
- Box results also report sampled and unloaded counts. Only `is_complete()`
  establishes full coverage. An empty map with unloaded cells **does not prove
  the box is dry**. No automatic wet/dry, damage or swimming decision is made.

For example, a consumer can query its own eye position without giving the
library an entity or choosing any effect:

```rust
use pluto_api::fluid::contact::{query_point, PointContact};

match query_point(world, eye_position)? {
    PointContact::Fluid { cell, depth, .. } => {
        // Record facts for the consumer's own policy. No effect is applied here.
        record_contact(cell.fluid, depth);
    }
    PointContact::Dry => record_dry(),
    PointContact::Unloaded => retain_unknown(),
}
```

A closure adapter is useful when only read access is available. It must preserve
`Sample::Unloaded` rather than turn missing data into air. The caller owns
snapshot consistency; a query does not acquire a world lock, load chunks,
cache world state, write cells or schedule ticks.

## Geometry and limits

Bounds require finite coordinates and a strictly positive extent on every axis.
Cell enumeration is half-open at the box's maximum and deterministic in Y/Z/X
order. Negative coordinates use floor, not truncation. The query supports the
full signed 32-bit cell-coordinate range, including an exclusive upper bound of
`i32::MAX + 1`; point coordinates outside that range are rejected.

`Limits::default()` permits at most 4,096 cells. Callers may choose a budget from
1 through `MAX_QUERY_CELLS` (65,536). The complete cell count is checked before
any provider call, using a wide integer so huge boxes cannot overflow into a
small budget. Invalid budgets, non-finite/inverted/zero bounds, out-of-range
coordinates and budget exhaustion are explicit errors. Invalid fluid levels are
errors, not silently dry observations. These bounds limit the query's own work;
the provider is still responsible for making each individual sample bounded.

The current cells are bottom-filled: source level 0 is 8/9 high, horizontal
levels 1–7 are `(8-level)/9` high and falling level 8 is full height. A tangent
upper face is outside the fluid. Volumes use normal floating-point geometry.
Tests compare every level with the existing generated stepped models.

This does not implement corner-smoothed liquid surfaces, connected-column
immersion, native water/lava conversion, inversion, fog or contact effects.
A fluid moving upward must not be given an inverted contact shape merely
because its density is negative. New geometry requires an explicit shared
contract, rather than a consumer guessing from flow direction.

## Validation and integration boundary

On 2026-11-05 the public API suite passed 19 tests, including 10 contact tests:
geometry/model consistency, surface tangency, multiple families, negative and
fractional bounds, unloaded coverage, budget enforcement, large-product
arithmetic, malformed input, coordinate edges and proof that the `World`
wrappers never call write/scheduling methods. No game executable was run.

This follows the shared world-fluid service's official 624c/e8 source proof.
The contact addition is host-independent Rust and was checked as a public API
crate; it has not received a separate host/gameplay proof. Package it with the
shared foundation's recursive `api/src` discovery so `fluid/contact.rs` is
included. The earlier fixed `API_FILES` list alone is insufficient. Final
package, host and full-stack validation belongs to the combined fluid feature
batch, not a separate per-function PR.

## Read-only official-world adapter candidate

The native game-scoped `pluto_world_fluid` module now provides
`query_point(&Game, point)` and `query_box(&Game, bounds, limits)`. These wrappers
borrow the world's existing resources and delegate to the same contact kernel.
They select no actor, effect, movement response or authority, and no consuming
mod is activated by this addition. A mod's own game-scoped adapter can consume
these facts; the higher gameplay package remains the owner of `pluto_api`.

The private `pluto_world_fluid_query::View` holds only immutable references to
actual native `World`, `BlockRegistry` and `Behaviours`. All three must come from
the same world/resource context. It is also used by the existing flow adapter's
`sample` method, so read-only queries and writes' precondition checks share one
registered-cell decoder and sample ordering:

1. Check `world.shape.min_y` and exclusive `shape.max_y()`, preserving the exact
   existing `GameLevel` bounds. Outside-height cells remain solid.
2. Check loaded chunks before calling `World::block`, whose missing-chunk
   fallback is air. Unloaded cells remain unknown.
3. Test the native `Behaviours::is_air` flag, not `World::is_air`'s single-ID
   comparison, preserving nondefault air variants.
4. Decode only registered custom blocks with parseable level 0 through 8.
   Missing, malformed and unknown cells retain the existing solid fallback.

Registration remains visible per sample, as in the original adapter. This
extraction does not silently introduce a registry snapshot or change poisoned
registry behavior. Callers still own the stability of the queried world view.

The read-only entrypoints never call `Game::with_level`. In the selected official
host that method flushes spill and LAN particle/sound queues after its closure,
so invoking it inside a query would introduce unrelated observable mutations.
No tick scheduler or mutable world access is accepted by the view.

Five source-boundary checks pass, including reversal of the named extraction
changes back to the complete original adapter's hash. Existing bucket, tick,
neighbor, permissions and mutation policy bytes remain unchanged. Three Rust
checks over the actual `BlockRegistry` passed at `0b815cc4`: level parsing,
malformed and unknown states, and per-sample registration visibility. The view
compiled against actual official `World`, `BlockRegistry` and `Behaviours` types
and the actual Pluto API crate. Integrated cutoff `c55f4985` subsequently passed
official plan/preflight/apply and complete library/binary type checks, including
the existing flow adapter delegation and Game query wrappers. The later minimal-resource fixture tests the actual read-only view; production
resource compatibility and Game wrapper runtime remain unproved. Earlier contact-kernel or cloud proof alone does not verify
this source. No copied native structs or geometry-only tests establish
world-view compatibility.

`tools/prepare_fluid_query_fixture.py` prepares a narrow decoder fixture against
clean official Horizon sources, the official lock and the actual Pluto API
crate. It admits only the exact fixture and Pluto API package identities in
addition to official locked dependencies, and hashes the API manifest and all
Rust sources before/after execution. It compiles the real native view types and
runs only the three decoder tests; it does not substitute a mock World or
Behaviours. At exact source `0b815cc4`, all three decoder tests passed in 1.25 seconds.
Source/API/lock guards passed before and after execution. Three Python
fixture-identity/input guard checks also pass. The first attempt exposed a
test-helper-only native NBT key conversion, corrected from owned String to
borrowed string slice; its failed log is preserved separately. See
[scoped native proof](fluid-contact-native-proof.json).

### Minimal native-loader fixture candidate

The optional `--with-world-view` fixture mode adds five tests of in-memory
sampling against actual `World`, `BlockRegistry` and `Behaviours`. Its synthetic
resources are passed through the real loaders, never copied native structs.
The runner extracts the 126 unique block names required by the pinned official
`Named` declaration, preserving native aliases. It adds native liquid block
levels 0–15 required by the loader and two custom nine-level fluid families.
Minimal collision NBT follows the official collision test schema. Tag bundles,
AIR flags and otherwise-empty gameplay rows are explicit fixture data. Missing
or invalid inputs fail; absent resources never skip a test successfully.

The cases cover exact dimension height despite widened chunk storage,
negative/unloaded chunks, nondefault air variants, registered-only custom cell
recognition, point/box geometry and unchanged world revision/storage markers.
At exact source `b9949dcd`, **all eight native tests passed**: the three decoder
cases and all five minimal-resource view cases. None were ignored. The guarded
run completed in 1.50 seconds; source, actual API and official lock guards passed
before and after. Five Python fixture guards also pass, including strict name
extraction and absence of skip/fake-type fallbacks. These tests establish only
the minimal fixture's semantics. Production resource compatibility, game actor
binding, rendering, movement and contact effects remain separate. The exact
generated resources and source hashes are recorded in
[world-view fixture proof](fluid-contact-world-view-proof.json). No game assets are fetched
or broadly generated by this fixture.
