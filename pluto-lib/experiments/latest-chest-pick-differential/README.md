# Chest pick-color: 41f finite policy differential fixture

**Source prepared only. Rust/metadata/plan/apply/host execution has not run.**
This separate fixture targets official Horizon `41f822b659789780f58d32c63ddfb6887ea1b8bf`.
Its immutable runtime input is the excluded chest candidate
`68bddc1b7543c4822bf3c15f6312d4d4d35b76d8`, which remains a historical f4dc candidate.
It does not repin that candidate, modify its seven files, change ordinary manifests,
or claim the old mining36/Template/default proofs apply to 41f.

## Actual inputs and exact transformations

The original Option helper and metadata reader come from Mod main
`08afa992d5091e03360f3473ea93fd17aa793987`. The new helper is the exact sibling
`latest-chest-pick/source-chest-pick.rs` from 68bdd, SHA256
`b7091136b67115babbe19dae8365f490c09b4a75e2c94d2ab41b39d086ffc1cc`.
Its item guard, metadata read expression, Tag::Int handling and color assignment
are byte-identical to the old helper; only its signature and Some extraction
became an owned ItemStack input/output and mutable borrow.

For testing, each helper is transformed only at three exact, invertible locations:
its function name/fixture visibility, its Game parameter, and its single world
metadata-read expression. The old Option and new owned policy bodies otherwise
remain unchanged. The replacement reader receives an explicit borrowed Tag and
counts calls. The production metadata reader's full source is hash-pinned. Only
its three world/chunk/first-record acquisition lines are excluded; the complete
nested-key/legacy-ID/color selection suffix is included byte-for-byte.

**This is not a modeled Game.** Chunk coordinates, loaded state, first-record
selection, world metadata ownership, actor substitution, reach, Ctrl, callback
cancellation and network synchronization are outside this fixture. The counter
proves only whether the finite policy requests its supplied metadata. None maps
to no new owned-policy invocation, reflecting the source-proven final-LOAD design;
it does not execute or prove that Mixins selection.

The fixture imports real `horizon_items::items::ItemStack`, native `StackData` /
`Wire` / `BundleSelected`, and `horizon_world::nbt::{Tag, JavaString}`. Its tiny
root `items` reexport supplies the same native type to the **whole unchanged**
official `pick_item.rs`, including its three original tests. No substitute item,
NBT, equality, pick-selection implementation, Game type or stand-in native codec
is provided. Before a future compile, actual official package path/lock identity
must be verified by a separate preparer/controller.

Immutable 41f downloads show items.rs, item_data.rs, nbt.rs and pick_item.rs are
byte-identical to the corresponding f4dc files. The exact hashes are recorded in
inputs.json. This is only a source comparison, not a current-version runtime proof
or a claim about changed protocol/item.rs and other native transport paths.

## Complete proposed Rust inventory

One unfiltered fixture execution must include **17 named tests**:

- 3 unchanged official `native_pick::tests` cases.
- 14 additional finite differential cases from `regression_tests.rs`.

The additional cases cover native-None policy bypass; unrelated block items;
all signed i32 color boundaries; missing/malformed color; existing color/no-extra
preservation; nested/legacy precedence including malformed nested values; the
old reader's permissive nested-ID behavior; foreign UTF-16/float/nested metadata;
count/damage/name/container/custom data and runtime bundle selection; retained
wire bytes versus stale usable wire; survival mismatch/no grant and exact-color
selection; and creative cloning of supplied empty/full collected payload shapes.

The two collected payload shapes are inputs, **not execution of Game's
include_data branch**. Retained Wire is a synthetic native policy input, not a
claim that picked_stack constructs it. A changed color can invalidate usable
wire while the raw backing remains present. Native equality excludes Wire and
BundleSelected, so those omitted facts are checked explicitly. These tests are
not a general private-baseline identity, arbitrary NBT roundtrip, save/transport,
Game authority or complete-component migration proof.

## Source validation and future gates

The renderer performs no Git, network, Cargo, metadata, Rust or plan operation.
It accepts only the frozen input hashes, reconstructs five fixture source files
in memory, verifies each helper's exact inverse and checks the entire expected
17-name inventory. Fourteen pure Python checks passed, including altered source,
wrong pin/execution claim, incorrect metadata acquisition, policy-tail drift,
wrong names and ambiguous substitutions. That success is not Rust type checking.
The generated hashes and these source-only results are in source-verification.json.

A future authorized CI must fetch exact official41f and main08afa, verify raw
input/full file sets and locked dependency identities, then regenerate identical
fixture bytes. It must compile the real native dependencies and this harness,
record actual compiler paths, list all17 names without filtering, run the sealed
binary once with zero failures/ignored/filtered, and verify sources/locks/output
and bounded process-group cleanup afterward. No workflow, trigger or run grant
is included in this source group.

Independent source review of the candidate found no additional policy/ownership
blocker, but its four-LOAD/last-LOAD IR selection, helper Game typing and full
composition remain separate gates. No old feature is removed to pass a guard.
