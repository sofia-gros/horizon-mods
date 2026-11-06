# Native fluid facts boundary

PlutoLib owns the host decoding and access used to read native fluid states and
player water samples. TConstruct retains the decisions made with those facts.
The public support module is `horizon_client::pluto_fluid_facts`, installed in the
shared client library by PlutoLib's existing native bundle. Its structs contain
only strings and booleans. Its host-bound entry points accept the existing host
`Behaviours`, `BlockStateId` and `PlayerMotion` types.

This is a focused compatibility boundary, not a claim that all TConstruct
host dependencies have moved behind a host-independent facade. There is no
reverse dependency from the client library to the higher gameplay package's
`pluto-api` crate, no second owner of these fact types, and no new hook/state.

## Contract

- `block(behaviours, state)` returns the exact built-in fluid ID and the host's
  full-amount flag. Source and flowing IDs remain distinct. A full falling flow
  is not reclassified as a source, and full amount does not mean a full cube.
- `native_ids()` lists the host's built-in identities, including empty and both
  flowing identities. It does not resolve tags or enumerate custom fluids.
- `replacement_after_removal(behaviours, state)` resolves the native remaining
  fluid block. It does not remove anything, authorize a write or own rollback.
- `player(player)` captures current eye-in-water, previous eye-in-water and last
  body-touching-water as three separate facts. It does not sample the world,
  advance movement or select an actor. The caller decides what an absent player
  means. The prior eye sample must not be replaced with the current eye sample.

These APIs describe the official host's built-in fluid system only. Ordinary
custom-fluid blocks used by [the experimental world-fluid service](world-fluids.md)
may classify as native empty. This API does not make them water, add waterlogging
or preserve a custom-fluid overlay when another block is removed. Custom cells
and their [contact queries](fluid-contact-query.md) remain a separate contract.

## Existing consumers moved to this boundary

The migration is limited to these five files in `tconstruct/mixins/tconstruct/`:

| Consumer | Shared host access removed | Policy retained in TConstruct |
| --- | --- | --- |
| `source-native-hook-bodies.rs` | native fluid enum, state fullness, identity catalog | geode full-water condition, exact source-water waterlogging, slime-grass tag/spread rules |
| `source-ender-propagule.rs` | native fluid-preserving replacement | tree placement, permissions and failure restoration |
| `source-splash-conditional.rs` | native fluid-preserving replacement | splash predicates, selected replacement, permissions and accounting |
| `source-mining-context-shared.rs` | current/prior eye-water field access | mining multipliers, Aqua Affinity and Hydraulic conditions |
| `source-projectile-airborne.rs` | last touching-water field access | riding, ground and climbability conditions |

The source-water test remains an exact `minecraft:water` identity test, not a
family membership test. The native catalog still contains `minecraft:empty`.
The mining caller retains its previous absence/default behavior. The existing
actor selection and all writes stay in the consumers.

Other fluid-related boundaries remain separate work: native tank reads already
use Pluto's `pluto_fluid`, and item-fluid NBT uses `pluto_stack_fluid`; their
metadata/storage changes belong to the storage feature group. TConstruct's
multi-fluid tank combines generic storage with its alloy/foundry rules and is
not moved by this extraction. No source-specific material, effect or recipe is
introduced into PlutoLib.

## Native climbability construction

The private gameplay support module `crate::game::pluto_motion_query` owns the
host `MotionWorld` construction used by `climbable(game)`. It borrows the game's
selected actor, world, assets, collision binding, motion resources and active
`health_resources` for the duration of one query. It returns
`horizon_client::Result<Option<bool>>`: no actor is `Ok(None)`, query errors are
returned unchanged, and successful queries return the native result.

The health context comes from the current game's world/pack-aware resource, as
in official [`game/consume.rs`](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/game/consume.rs#L156-L165)
and [`game/prediction.rs`](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/game/prediction.rs#L76-L84).
It is not fabricated from defaults or borrowed from another world. No host
references escape in the result. The query uses an empty entity-collision list,
matching the previous consumer query; native climbability reads the actor's feet
and block tags rather than using entity-collision geometry.

This does not advance a movement tick. It is also not strictly read-only: the
native climbability query can remember the climbable block on the actor. That
existing side effect is retained. No fluid sampling, new physics or gameplay
rule is added.

`source-projectile-airborne.rs` still owns riding, missing-player, grounded and
native-water exclusions. It maps an unavailable or failed climbability query to
`true`, preserving its conservative ineligible result, and still calls its own
`airborne_conditions` formula. It no longer builds a host motion context or
knows the added health field. Other TConstruct host dependencies are unaffected.

Three focused source/manifest checks for this boundary passed on 2026-11-05.
Official apply/type checking of the new shared module remains with the grouped
validation owner. The extraction repairs the known missing-field construction
in source, but does not establish that all dependent PR heads compile.

## Read-only native tank snapshots

The existing `horizon_client::pluto_fluid::first` API and its raw signed fields
remain available without changed parsing/default behavior. The new
`first_snapshot(world, position, face)` validates that same first exposed tank
projection once and returns `Result<Option<TankSnapshot>, InvalidTank>`:

- `Ok(None)` means the raw reader found no available exposed tank. This includes
  unloaded/missing providers; it is not proof of an empty tank.
- `Ok(Some(snapshot))` has positive capacity, a nonnegative amount no greater
  than capacity, and a fluid identity whenever amount is positive. Zero amount
  normalizes any stale identity to `None`, as the previous gauge code did.
- `Err` means a native provider was observed but its projected capacity or
  contents are invalid. A consumer must not treat this as a missing provider
  and silently fall through to another local data source.

`TankSnapshot` uses unsigned amount/capacity and carries
`metadata_complete = false`. The official native first-tank projection contains
only fluid identity and amount, not fluid tags. An omitted tag is not evidence
of an untagged potion. This flag does not promise that a metadata transport was
added; no tags are read, invented or written by this API.

`source-gauge.rs` now carries completeness alongside the selected tank instead
of checking the native provider again to guess its provenance. It keeps local
TConstruct readers marked complete, native snapshots incomplete, and its
existing generic potion-name fallback for incomplete metadata. The capacity,
amount, empty normalization and invalid-provider fallback rules are moved from
the consumer to PlutoLib without changing them. Gauge display and TConstruct
machine selection remain consumer policy.

This is validation of the existing native projection, not an opaque-NBT
validator. The raw reader retains the official missing-row/default behavior.
The API does not resolve arbitrary capabilities, load chunks, add cauldron
support, synchronize worlds, or fill/drain a tank. The storage feature group
continues to own writes and compound metadata preservation.

Official [`game/machines/native_fluids.rs`](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/game/machines/native_fluids.rs)
defines the matching ID/amount projection and missing-entry defaults. This
client-library helper intentionally covers only declared mod block-entity tanks,
not every operation of the higher gameplay package's native fluid API.

Four new source-boundary checks and two existing gauge-name checks passed on
2026-11-05. Three native Rust snapshot conversion tests are included for batch
validation and have not been run in an applied official host. No new hooks,
manifest entries or dependency edges are needed for this addition.

## Evidence and validation limits

The extraction was checked against official Horizon
`624c4f6265c02296fbe8a80f029dd0d987fb94c0` and the selected Mixins revision
`e8cd0aa38cffef49cb2e088ef716baa107148c02`. Original TConstruct consumer expressions
were inspected at horizon-mods `08afa992d5091e03360f3473ea93fd17aa793987`.

Official sources:

- [`fluid.rs`](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-world/src/fluid.rs): built-in identities, full amount and source/flowing distinctions.
- [`level/mod.rs`](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-world/src/level/mod.rs): `Behaviours::fluid` and `legacy_block`.
- [`motion/fluids.rs`](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/motion/fluids.rs): current eye query and separately saved eye/body samples.
- [`client_lib.rs`](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/client_lib.rs): public motion module and client-library attachment boundary.

The follow-up includes two native Rust contract tests for batch validation.
Four new source/manifest boundary checks and five focused existing consumer
source checks passed on 2026-11-05. Source checks are not official compilation
or gameplay proof. The earlier selected-entry world-fluid proof does
not cover this new module. No new official apply/check or gameplay run is claimed.

The original consumer at `08afa992` constructed `MotionWorld` without the
`health` field required by official `motion.rs:1014-1023`. The subsequent shared
climbability adapter above removes that constructor from the consumer and uses
the correct active-game health source. This source repair is pending grouped
official host validation; it does not retroactively verify original published
heads or prove a complete latest-host TConstruct bundle.
