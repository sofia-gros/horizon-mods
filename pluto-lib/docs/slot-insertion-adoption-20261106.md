# Reuse the two-cell planner for chest insertion

Source-preparation status: **native differential tests not yet run**.
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

## Independent oracle and planned validation

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

The planned 42-test fixture also repeats the prior 39 tests, with actual
clean official ItemStack/NBT types and one real shared Cargo API crate.
Host path/lock and full source identity checks remain mandatory. This document
is not a completed test receipt or evidence of full TConstruct/gameplay proof.
