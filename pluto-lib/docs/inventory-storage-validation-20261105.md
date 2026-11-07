# Inventory and storage batch validation, 2026-11-05

## Exact scope

The grouped storage fixtures passed on combined MOD foundation
`856845a4615cd2776e87e6ba6e016f0e20d87636` against a clean official
`horizon-rust/horizon` checkout at
`624c4f6265c02296fbe8a80f029dd0d987fb94c0`.

This foundation combines the transaction helpers, native item adapters,
advancement scope and predicate API, and machine consumers. The fixtures depend
on the actual `pluto-inventory-api` Cargo package, including its `serde_json`
dependency and predicate module. They do not copy, stub or disable those APIs.
The native storage adapter is compiled from its production source with real
Horizon item and NBT types; world access and event callbacks remain bounded
fixtures. Resolution is seeded from the official host lockfile, then checked
for identical transitive package identities, allowing only the fixture and the
exact local Pluto package.

## Preserved semantics and intended strengthening

- Casting, Heater, Cannon and Casting Tank delegate their native item save
  predicates to `encode_item_lossless`. Their prior predicates and the shared
  adapter agree across normal items, count boundaries, damage/components,
  identifier normalization, air, and current or stale retained wire backing.
  Zero-count rejection is equivalent: the native decoder normalizes zero to one,
  so the previous exact count comparison already rejected it.
- Melter's held-item snapshot comparison delegates to `same_item_snapshot`
  with the same count, component and retained/usable wire criteria.
- Remaining casting, Cannon and Casting Tank hopper inventory rechecks use
  `inventory_snapshot_matches`. This intentionally detects opaque-wire-only
  callback changes which native `ItemStack` equality ignores, including changes
  in unselected slots. The callback's changed state is preserved, not overwritten.
- Snapshot identity is not save eligibility. An unchanged wire-backed item can
  match a snapshot but cannot be serialized by the lossless save adapter.
- Machine schemas, raw-field preservation, capacities, recipes, sided-slot
  policy, world/chunk checks, native callback order, saves and rollback remain
  in TConstruct. The API does not provide locking, atomic persistence or network
  transport.

## Commands and results

All four runners used the same official checkout and cached target, with
`CARGO_BUILD_JOBS=2`, `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0` and
`CARGO_PROFILE_TEST_DEBUG=0`:

```sh
python3 tconstruct/tests/run_cannon_held.py
bash tconstruct/tests/run_casting_preservation_rust.sh
bash tconstruct/tests/run_seared_reinforcement_rust.sh
python3 tconstruct/tests/run_inventory_transaction_guards.py
```

`HORIZON_HOST` must point to the clean checkout above. The runners reject a
wrong revision; the proof also required a clean MOD tree at the recorded head.

- Cannon and Casting Tank fixture: **156 passed**. Includes
  `machine_save_adapters_preserve_native_codec_eligibility` and
  `cannon_hopper_preserves_unselected_slot_wire_changes`.
- Casting preservation: **57 passed**.
- Seared reinforcement and dependent production state/recipe tests: **210 passed**.
- Native item adapter and extracted Chute transaction guard: **4 passed**.

These are **427 passing test executions**, not 427 distinct behaviors: production
module tests overlap between fixtures. No failures or ignored cases were
reported. Expected bounded-fixture unused-code warnings remain.

The grouped source checks also passed: 35 preservation/adapter/fixture-wiring
Python tests, 274 injected-module references in 22 manifests, and `git diff
--check`. Available disk space remained above 10 GiB throughout this proof.

## Limits

This is focused storage-boundary evidence for the exact combined head. It does
not apply the complete mixin bundle, build the full client, exercise a live
world or LAN session, or verify every open machine PR. Packaged integration and
broader runtime evidence must be reported separately. Historical old-official
checks are not counted as verification on this latest target.

## Partitioned proposal staging follow-up

A later focused run passed at combined MOD head
`5a534b098fd0eed2c706f75f598ad384362747c6`, including the
`pluto_inventory::storage::stage_updates` extraction. The two native fixtures
used clean official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0` and
checked their dependency identities against the host lockfile.

- The complete actual inventory API Cargo crate: **33 unit tests passed**.
  This host-independent run copied the unchanged full package to a temporary
  directory, retaining all dependency-backed modules and registries.
- Casting preservation and the real metadata store: **68 passed**. Required
  tests confirmed repeated updates retain input order across partitions and a
  missing partition or empty change set does not publish any staged records.
- Native item/NBT transaction fixture: **10 passed**.

Total: **111 passing executions**, with overlap between source modules in the
native fixtures. The generic staging tests also cover load-once behavior,
missing input, rejection after a partial proposal, stopped later callbacks,
empty input and non-Clone keys/proposals.

The API stages independent caller-owned values only. It cannot undo external
callback side effects and does not promise atomic authoritative publication.
Chunk addressing, metadata schemas, persistence, dirty marking and commit
ordering remain in TConstruct.

The run used two jobs, debug information and incremental compilation disabled,
and the continuous 250 ms disk guard. It exited zero without triggering the
10 GiB + 128 MiB threshold; minimum free space was 11,278,422,016 bytes. Separately, 45 Python preservation
and wiring tests, eight native package checks and 286 injected-module references
in 22 manifests passed their guards.

This is not a full native composition or client/runtime result. In particular,
it does not establish compatibility of unrelated advancement state injections;
their official-host constructor constraints require separate composition proof.

## Nested stack identity correction

The bounded native transaction fixture passed **16 tests** at MOD source head
`cd278a8d119c528834daa75f56150fd4f95502dc`, against the same clean official
Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0` with exact host-lock
dependency identities. Four shared-item source guards and `git diff --check`
also passed. This is a correction to the earlier root-only guards, not a claim
that the older proofs covered nested opaque data.

Official `StackData` holds child stacks in `container`, `charged`,
`pot_decorations` and `bundle_contents`. Its equality deliberately ignores
`Wire` and runtime-only `BundleSelected`. Pluto now uses one native child
traversal for the following distinctions:

- `contains_retained_wire` finds current or stale backing at any depth.
- Save eligibility rejects retained wire anywhere in the tree before encoding.
- Snapshot equality compares retained bytes, usable wire patches and runtime
  bundle selection at every node, in addition to native component identity.
- Save eligibility continues to ignore deliberately unsaved bundle selection.
  A selected bundle can remain codec-representable while differing from an old
  transaction snapshot.

The six new regression tests cover every child field, wire addition/removal,
changed bytes, changed usability without changed bytes, an unselected inventory
slot changed during a callback, a 32-level mixed tree, normal nested values,
slot-position changes and the snapshot/save distinction for bundle selection.
Native count semantics are preserved: charged projectiles use the item codec's
1–99 range, while native container/pot/bundle templates retain count 100.
All four nested fields reject a zero count when roundtrip normalization changes
it. The tests do not substitute a separate codec.

One initial test assumed every nested field used the same count range. The
official source and fixture showed that assumption was wrong; the test and
documentation were corrected, and all 16 tests then passed. The fixture runner
now prints captured diagnostics before returning a failed Cargo result.

The run used two jobs, no incremental compilation or debug information, and a
continuous disk guard. It exited zero with minimum free space
11,105,816,576 bytes, above 10 GiB + 128 MiB. This proof does not re-run the full
machine fixture group, apply the complete library package, or establish
live-world/LAN transport. In particular, keeping raw wire bytes after editing
components does not mean the native sender can still use those bytes.

## Observable snapshot limit

The snapshot proofs above cover native component values, runtime bundle
selection, retained wire bytes and current usable-patch presence/bytes. They do
not compare the private decoded baseline stored inside official `Wire`. Two
different private baselines with the same exposed patch bytes can both be stale
and compare equal through the current public API. These helpers therefore do
not establish arbitrary live write authority or full private Wire identity.
No runtime guard was changed by this documentation correction; save eligibility
still rejects all retained backing. A future exact private-state comparison
requires a separately validated native read seam, not pointer-address equality.
