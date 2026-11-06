# Outer tank delegation: bounded native validation

## Result and immutable scope

[Run 42186716846](https://github.com/sofia-gros/horizon-mods/actions/runs/42186716846),
job `127788705395`, passed at runner head
`a583c0f272f847bcb7fb72435747172f698ab903` on 2026-11-06.
It tested unchanged runtime source
`794a0c526ca0a96d0ca9c9ff40010611b5968a89` against official Horizon
`624c4f6265c02296fbe8a80f029dd0d987fb94c0`.

- 160 Rust tests passed: 156 inherited and all four required additions.
- 0 failed, ignored, measured or filtered cases; one full-suite execution.
- 30 runner Python checks and five original source guards passed.
- Actual cold rustc invocations compiled official core, world, client-core and
  items, the real historical inventory API, and the fixture test crate.
- Both independent fixture preparations agreed. All four required gates,
  exact dependency identities/paths, source/file-set/lock/OUT_DIR seals and final
  clean checks passed. The execution phase rejected any actual recompilation.
- Final receipt: `passed=true`, `verified=true`, `error=null`.

The three consumer edits delegate only existing outer `tank` compound updates
to PlutoLib's `patch_compound`: Cannon preservation, Drain's outer-controller
proposal and the controller tank patch. The complete old methods remain the
comparison oracles. Caller order, shape/error gates, unknown fields and raw
data distinctions remain in the fixture. The new cases cover 128 Cannon matrix
comparisons, four retained-wire variants plus opaque item preservation, 24
controller comparisons, and 576 Drain route comparisons.

**The Drain route uses the existing mock Game/world.** This is a real native
ItemStack/NBT boundary proof, not an installed Mod, complete client build, live
actor/slot authority, durable disk transaction, rollback or full original-PR
repair. The prior 146-case proof and authoring API examples are separate cutoffs.

## Guard and failure history

The first guarded run at `f7f5d90b`,
[42185710725](https://github.com/sofia-gros/horizon-mods/actions/runs/42185710725),
stopped before Rust compilation: Cargo metadata returned success, but merged
stderr progress prevented whole-stdout JSON parsing. Its false receipt and full
log remain in [the first-attempt record](outer-tank-first-native-attempt-20261106.json).

Only runner capture changed for the separately reviewed and authorized retry.
Both metadata commands now have distinct file-backed stdout and stderr under
the same timeout, process-group, memory and disk monitoring. The entire stdout
must be valid JSON; no prefix searching or output filtering is used. The 55
runtime inputs, 28-file historical inventory API, full old oracles and 160 cases
did not change. There was no automatic retry or guard relaxation.

The successful run admitted the real-root/every-ancestor topology, with
15,749,718,016 bytes initially available. The minimum observed available RAM was
14,524,874,752 bytes; minimum free disk was 90,941,816,832 bytes. The disk floor
remained 10 GiB + 128 MiB. These are observed admission/polling results, **not a
hard aggregate memory cap**. No cgroup/systemd settings were changed.

## Retained evidence

[The machine-readable record](outer-tank-native-validation-20261106.json)
contains the exact final receipt, all 160 test names, phase/actual-rustc witness,
and complete UTF-8 job log as `gzip+base64`. Full compressed readback was checked.

- Uncompressed log: 711,713 bytes; SHA-256
  `132e83f228c601349bf40039d12cbcd2dffe6caf0bf43b5ea8401a062ef993cd`.
- Gzip: 78,555 bytes; SHA-256
  `c040bc9de22339e833671e8c025c59c5b2ec848ae1adf46e6632e9b930f274c1`.
- Full source seal:
  `4b2b866214feeb77f2244c1d395f75a38de62726171ea0f58b6ea8de5180e44d`.
- Official source seal:
  `e1f94b0f68a4088615408abe8fec63488bbfd1faba041899240a6b99dd4ac385`.
- Generated OUT_DIR seal:
  `2e0cccec72c5324ed5fe44c504d31bc7a7e1592914235ca20fa7f1186fd21bff`.

This closure changes documentation only. It does not rerun the trigger or change
production source, any PR state, caches or the user's discarded engine fork.
