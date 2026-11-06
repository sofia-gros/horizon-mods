# Explicit staged cloud construction

This bounded PlutoLib group separates a reusable cloud request from native host
construction. It does **not** spawn a cloud, allocate an entity ID, resolve an
actor, enable a TConstruct recipe, apply effects or change cloud ticking.

## Two boundaries

`horizon_client::pluto_cloud` owns `Descriptor`, `Timing`, `Effect`, `Label`,
`Attribution` and `Validated`. Every behavior value is explicit; there is no
Default implementation or built-in TConstruct preset. Catalog resolution uses
the existing shared potion-facts adapter rather than another native registry
owner. `validate` preserves request order, duplicate effects, raw durations,
amplifiers, RGB, radius settings, labels and attribution.

The private `game::potions::pluto_cloud_factory::construct` consumes a validated
request and returns an opaque `StagedCloud`. It fills real native `Breath`,
`CloudSettings` and `PotionCloud` fields, with fresh empty victim ledgers and ID
zero. It does not hold Game. Its separately reviewed game-private consuming accessor supports the
[source-stage insertion boundary](cloud-insertion.md), without changing construction values. This is an
unregistered prepared object, not a playable feature or an entity handle.

Attribution is a **request**, never proof of authority: explicitly unowned,
player with an optional guest connection, or mob with an entity ID. Nothing
selects the current player, invents a guest, or silently supplies None. Any
future insertion service and caller must separately prove authoritative actor
binding. Existing read-only equipment Actor facts are not that proof.

## Supported input contract

- Finite positions within signed 32-bit block-coordinate cells.
- Finite radius in 0..32 and finite radius-change values. No clamping.
- Nonnegative age, wait and reapplication ticks; lifetime -1 or nonnegative.
  Duration-on-use remains signed. Effect-duration scale is finite and nonnegative.
  Native wrapping/cadence semantics are not reimplemented by the descriptor.
- RGB at most `0xffffff`. Labels are literal or no-argument translation keys,
  at most 4096 UTF-8 bytes; translation keys cannot be empty. Rich Text components
  are not currently represented. The library chooses no label or color formula.
- One to 256 known effects, with duration -1 or nonnegative and amplifier 0..255.
  This is a bounded construction contract, not a source-mod allowlist. Instant
  effects can be represented, but no application is enabled by this service.
- Duplicate IDs retain order and values. Aliases that resolve to the same effect
  must agree on visibility/ambient flags: the native cloud stores these flags by
  identity, so conflicting duplicate flags are rejected rather than changed.

Validation errors return no staged object. Successful validation does not prove
that a mob/guest exists, a target chunk is loaded, a caller has permission, or a
request matches a particular source mod. None of those decisions are inferred.

## Existing TConstruct migration decisions

Pinned source `MobEffectCloudFluidEffect.makeCloud` explicitly sets context
entity ownership, radius 1, wait 10, radius-on-use -0.5 and shrink by radius over
lifetime. The existing port documents duration 600 and stores four times its
already-scaled durations to compensate for native quarter scaling. These are
consumer choices, not shared defaults. Altering that duration pair also changes
its overflow admission and cannot be treated as a mechanical extraction.

Official lingering defaults use reapplication 20 and duration-on-use 0. Their
source vanilla constructor defaults were not independently checked by this
bounded audit. Actual actor attribution differs between the pinned TConstruct
source and the current ownerless port. No consumer is wired to this constructor
until those decisions are explicitly documented and actor context is proved.

Official ticking shrinks radius before each eligible dose. A radius-1 cloud with
radius-on-use -0.5 can disappear after its first target, so duration 600 is not a
guaranteed visible lifetime. The host vector already participates in world close,
parked dimensions and LAN tracking; runtime and save/reload behavior for generated
cloud payloads/owners still need their own verification.

Sources: [TConstruct makeCloud](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/modifiers/fluid/block/MobEffectCloudFluidEffect.java),
[official native settings and ticking](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/game/potions.rs).

## Validation scope

Target: official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0`, Mixins
`e8cd0aa38cffef49cb2e088ef716baa107148c02`. Source-boundary checks guard explicit
inputs, single catalog ownership, exact module placement and absence of mutation
or consumer activation. `tools/prepare_cloud_fixture.py` resolves the actual
shared descriptor and catalog adapter against clean, exact official core types,
using the official lock and fixed fixture-identity exception. Six native tests
cover preserved values, explicit attribution, invalid bounds, budgets, missing
effects, conflicting alias flags and legal edge cases. Preparation is not a test.

The actual host factory additionally contains two private-module tests for every
native field and attribution projection. These require a real official host
harness; descriptor tests alone do not exercise or verify this factory. Package
plan/preflight/apply and complete library/binary type checks passed at integrated
cutoff `fb96dd82`. An initial disk-guard interruption resumed against the same
5,036 unchanged staged source files and completed successfully. The two factory
tests remain unrun; type checking is not their execution. At exact source
`8b2cb14d`, all six descriptor tests passed in 0.50 seconds using actual official
core types. Source/lock/input guards passed before and after execution; four
new source-boundary checks and five catalog guard regressions also passed. See
[proof scope](cloud-construction-proof.json).
No mock renderer/game structs establish compatibility, and no gameplay proof is
claimed.
