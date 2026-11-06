# Outer tank patch delegation

Status: source checkpoint only. The new native fixture has not run. This is
separate from the earlier 146-test tank-compound result and does not extend
that result to these changed callers.

## Scope

Three existing outer-compound updates delegate to Pluto's existing
`patch_compound`, without a new API, registry, hook or storage authority:

- `Cannon::tank_to_nbt_preserving`: replace only the outer `tank` field after
  all existing item, raw-state, alias and nested-tank checks.
- `interact_drain`: construct the same saved proposal after the existing
  transfer, before the unchanged fresh-read comparison and write.
- `patch_smeltery_controller_tank`: retain the initial Compound shape gate;
  use a lazy one-element update iterator so the original Compound clone still
  precedes `state.tank.to_nbt()` evaluation.

For valid Compounds each path still clones the outer compound once. Unknown
keys, exact Java UTF-16 keys, nested values and floating-point bits remain
untouched. Native Compound is a sorted map, not a raw byte-stream duplicate-key
representation. Each update overwrites only the exact `tank` key.

No alias, count, slot, metadata, codec, recipe or capacity policy changes.
No save, notification, held-item, creative or give/drop ordering changes.
The latter effects remain TConstruct caller policy. `write` remains an
in-memory dirty-state update; this change adds no durability or rollback claim.

The controller's old code cloned non-Compound inputs before returning None;
the new explicit shape gate returns the same None without that pointless
clone. Allocation-failure/OOM timing is not claimed equivalent. The valid
Compound clone/encoding order is preserved with `once_with`, not eager tuple
construction. `Cannon::to_nbt_preserving` and `patch_casting_slot` are excluded:
naive delegation would add a clone or move validation across it.

## Source and planned comparison evidence

The complete old bodies were frozen from local source
`477069362ea08fcabf26e67ad910f3ee8ebfe452` before edits. The three oracle files
and `outer_tank_legacy.json` record independent method hashes and whole-file
restoration hashes. Replacing all three new bodies with their oracles must
restore both original production files exactly. Thus all outer save/notify/
held-item ordering and unmodified callers are guarded. The shared helper,
hopper source and existing mock-world scaffold are hash-bound too.

`test_outer_tank_patch.py` checks restoration, dependency identity, lazy
encoding order and rejection of alias/lossless/save/publication drift.
All five source guards pass, as do the five existing tank-compound guards.
No Rust compilation or native test has been attempted for this new source.

`run_outer_tank_patch.py` is prepared for a separate granted native slot. It
uses actual complete native state modules, actual ItemStack/NBT and the actual
single inventory API dependency. The current Cannon module is an exact byte
prefix; only the old method's declaration is renamed. Both complete Drain
and controller old/current functions are compiled. The Drain Game/world is
the existing bounded fixture, not actual Game or native authority.

Four new named cases are prepared: 128 Cannon raw/state comparisons, four
usable/stale root/nested-wire refusals plus opaque-item preservation, 24
controller malformed/unknown-value comparisons, and 576 whole Drain route
comparisons of return value, saved data, write count, held/carried/gift
snapshots and notification counters. These cases are **unexecuted**. Imported
existing tests must be reported separately if a later run succeeds.

The runner seals inputs, pins the official host revision and lock identities,
and checks them again afterward. Path and Cargo.lock equality do not guarantee
a cached dependency is Fresh: feature fingerprints can cause recompilation.
Any run needs a separate measured budget, serial grant, explicit jobs/debug/
incremental settings and the normal continuous 10 GiB + 128 MiB disk guard.
