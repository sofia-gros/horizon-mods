# Experimental world-fluid service

PlutoLib owns the reusable world-fluid registry, level encoding, finite downward
flow, source transactions and official-host adaptation. A consumer owns its
fluid definitions, textures, acquisition and effects. This is not a Forge API or
native `FluidKind` registration. It does not require a modified Horizon fork.

## Public API

`pluto_api::fluid::Definition` contains namespaced fluid, block, bucket and texture
identities, a tick delay, horizontal level drop and light emission. Register the
same immutable definition at startup on every side using `fluid::register`.
Identical registration is idempotent; conflicting identities fail instead of
replacing live content. Register before world/resource construction, for example
at the official `native_blocks` preparation boundary. The definition registry is
process-scoped and is not a dynamic data-pack reload registry.

### Atomic registration and stable lookups

Use `fluid::register_all(definitions)` for a finite group of families. The batch
validates every definition before locking and publishes a candidate registry
only after all fluid, block and bucket identity checks succeed. An invalid
member or conflict with existing content or another batch member publishes
none of the new families. Identical duplicates are idempotent, including
repetition inside one batch. The existing single-family `register` uses this
same transaction. An empty batch is a successful no-op.

`fluid::registry_snapshot()` captures an immutable `RegistrySnapshot`. Its
`get`, `for_block` and `for_bucket` lookups refer to the same captured set, while
`iter` yields definitions in deterministic fluid-ID order without retaining a
global lock. Later registrations do not change existing snapshots; capture
after all startup participants have registered when a complete catalog is
needed. Capturing another snapshot is explicit, not an automatic reload.

A snapshot also exposes an opaque `revision()` handle. `is_current()` becomes
false after a successful new-family addition, without changing the snapshot's
contents. Empty, idempotent and failed batches preserve the handle. It is a
point-in-time observation, not a lock, render lease or global registration freeze.
The [prepared terrain binding catalog](terrain-binding-catalog.md) uses it to
reject stale resource-derived facts; existing contact/flow reads stay dynamic.

The transaction covers the process-local definition registry only. A snapshot
is not world data, a save format, numerical host state IDs, or confirmation that
content files and host blocks were installed. Host content validation remains a
separate boundary. This API does not authorize dynamic world registry changes,
removal or replacement of live definitions.

The grouped public API suite passed 24 tests on 2026-11-05, including five new
registry cases: rollback on existing conflicts, internal collisions/invalid
members, idempotent ordered snapshots, stable earlier views, and concurrent
snapshot visibility of a complete batch. These are contract checks, not a new
host/gameplay or full-stack validation. The wider fluid feature batch owns the
combined package and host checks.

`World` exposes loaded-cell reads, conditional writes and scheduling without
Horizon types or numerical state IDs. `place_source`, `pickup_source` and `tick`
use that interface. Level 0 is a source, 1–7 horizontal flow and 8 a falling
column. Only sources fill buckets. Spreading writes into air or weaker flow of
the same family. Foreign fluids, solid blocks and unloaded chunks are preserved.
Missing chunk reads cause a retry, not a chunk load.

The private adapter uses ordinary registered block states and scheduled BLOCK
ticks. Its writes go through official `Behaviours::set_block`, shape/neighbour
updates and existing server update delivery. The host saves these block states
and its scheduled block queue. That reuse is an architectural basis, not a claim
that save/reload or multiplayer gameplay has been exercised for this service.
Bucket use checks the authoritative actor/hand, build permission, native bucket
cancellation and current stack before committing. Inventory commits immediately
after a successful world write, before fallible event processing. A refused
write does not consume a bucket. Creative use follows the host filled-result
handling. The actual fluid ID reaches guest bucket-empty permission listeners.

## Built-in fluid facts

The separate [native fluid facts adapter](native-fluid-facts.md) isolates existing
TConstruct reads of the host's built-in fluid states and player-water samples.
It does not classify this service's custom block cells as native fluids.

## Contact and query facts

`pluto_api::fluid::contact` provides read-only point and axis-aligned box queries.
It uses the existing `World::sample` interface or a caller-supplied read closure;
it does not expose a new Horizon hook or enable any entity effect. The public
contract, limits and verification scope are in [contact queries](fluid-contact-query.md).

## Generate consumer content

The API generates deterministic ordinary block declarations and nine stepped
models. Use the generic development utility, outside the installed bundle:

```sh
cargo run --manifest-path pluto-lib/mixins/pluto_native_creative/api/Cargo.toml \
  --example fluid_content -- definition.json content registration.rs
```

This writes models, blockstate and block declaration under `content`, and a
consumer-owned `register_world_fluid` helper. Add that helper to the consumer's
`native_blocks` HEAD injection. Supply the bucket item, bucket model and textures
in the consumer's own content pack. Never put consumer assets or policy into
PlutoLib's shared `.hnmod`. The installed public crate contains only reusable
source; the generation utility is a repository development tool.

## Current limits

- The visual prototype is nine flat/stepped ordinary block models, not native
  liquid tessellation. No corner smoothing, flow UVs or submerged fog.
- No swimming, current, buoyancy, burning, drowning or other contact effect.
- No upward flow, slope search, source conversion, mixing, waterlogging,
  dispensers, plant replacement or source-compatible per-fluid behaviours.
- Air-only bucket placement; no vanilla liquid-container fallback. Sources have
  outline shapes for pickup; flowing cells have none. Collision is empty.
- Vanilla pathfinding and heightmaps do not classify these blocks as fluids.
- Runtime disabling/re-enabling consumer mods and dynamic definition reload are
  not a supported lifecycle yet. Keep installed definitions fixed for a world.
- Tested kernel contracts and a selected-entry host proof do not verify the
  entire PlutoLib/TConstruct stack, native installation, save/reload or clients.

Host-sensitive code stays in `world-fluid.rs` and three private adapter hooks.
The public API does not expose private host types. Future host changes normally
stay behind that boundary, but save schemas and changed behaviour contracts may
still require consumer migration. No permanent compatibility promise is made.

## Drawing compatibility boundary

The [shared drawing adapter](rendering.md) extracts tinted GUI crops, repeated
fills, unshaded cuboid vertices and packed-light conversion. It is independent
of fluid IDs and machine policy. Consumer textures, dimensions and original
render hooks remain outside Pluto. Its native equivalence and package proofs
are recorded separately from the earlier world-fluid service proof.
