# Reuse the two-cell planner for chest insertion

Focused validation status: **42 native tests passed at `c8089606`**.
This consumer-only change is separate from the frozen v15 library cutoff.

`source-tinker-chests.rs::insert_slot` previously allocated one borrowed
snapshot for every slot plus the incoming stack, then asked `plan_move` about
only the incoming stack and selected destination. It now uses the existing
`plan_between` helper for those two snapshots. The shared planner still owns
all count arithmetic; no API, registry or native hook is added.

The real destination index remains the input to `ChestKind::accepts`.
The planner's logical source/destination indexes are 0 and 1, and its original
count checks remain explicit. They are not physical inventory indexes.
The whole-container first-equal search still runs before planning, preserving
Part uniqueness, tag admission, capacities and existing native equality.
The mutation and returned-remainder statements are unchanged.

Unlike the additive ordered-insertion API, this adoption does not tighten
malformed-input policy. Zero incoming counts still return unchanged. Existing
zero-count destination cells may still receive items; a zero limit produces
no move. Wrong container length or destination index remains a no-op.
Native item/component equality still omits retained opaque wire identity;
an admitted merge keeps the existing receiver's full native value. This is
neither a stronger lossless policy nor authority to write live storage.
No caller layout, notification, save, held-item or LAN sequence changes.

## Independent oracle and focused validation

The complete old `insert_slot` body was frozen from `6c09d53c`, before editing,
in `tconstruct/tests/rust/insert_slot_legacy.rs` with source/function/file hashes.
It uses the real unchanged `ChestKind` policy and actual shared `plan_move`.
The source guard restores all five intentionally changed chest helper bodies
to their originals and checks the historical whole file. It also checks the
exact adopted insertion body; unrelated edits or dependency drift fail closed.

The existing two oracle files and their receipts are unchanged. Fixture-only
imports route their old insertion dependency to the newly frozen old helper.
The actual consumer is compiled intact as `consumer_impl`, and the existing
wrapper tests still call its production wrapper functions. Three new tests
call its actual insertion helper directly against the frozen implementation:

- 9,000 combinations of Plain/Tinkers/Part capacity, real destination indexes
  other than logical 1, counts and limits including zero and `u32::MAX`, empty
  or component-distinct destinations, duplicates elsewhere and tag admission.
- 18 malformed shape/index cases requiring unchanged full values.
- An explicit opaque-wire merge case retaining the original receiver value.

The 42-test fixture passed, including the prior 39 tests, with actual clean
official ItemStack/NBT types and one real shared Cargo API crate. All 9,019 new
direct comparisons passed. The 12,772 earlier matrix comparisons were repeated,
not counted as new coverage. Both earlier oracle files and receipts are unchanged.

Eight source guards and eleven package Python checks passed. Strict official
path/lock identity and all 6,601 tracked MOD hashes were verified before and
after the actual fixture. The normal continuous 10 GiB + 128 MiB disk guard
held, with a minimum 10,889,220,096 bytes free. No retry was needed.

The [exact-cutoff receipt](slot-insertion-validation-20261106.json) records the
hashes, remote source checkpoint and scope. This is not full native-package,
TConstruct gameplay, save/reload or LAN proof. Notification and publication
call sites remain unchanged.
