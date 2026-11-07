# Plan a move between slots

Use `horizon_client::pluto_inventory` when a container needs to decide how many items can move between two logical slots. It does not own an inventory or depend on a screen. Your mod chooses the number and order of slots, what each slot accepts, and how many items a stack may hold.

```rust
use horizon_client::pluto_inventory::{plan_move, StackSnapshot};

let gem = "example:gem";
let slots = [
    Some(StackSnapshot { item: &gem, count: 12 }),
    Some(StackSnapshot { item: &gem, count: 60 }),
];
let movement = plan_move(
    &slots, 0, 1, 12,
    |_, _| 64,            // effective destination limit
    |_, _| true,          // destination accepts the stack
    |left, right| left == right,
).expect("valid slot indexes");
assert_eq!(movement.unwrap().count, 4);
```

The plan reports the source, destination, count, and both original counts. Neither the snapshot nor the live inventory is changed. Equal source and destination, empty source, a rejected stack, incompatible identities, no room, and a zero request produce no move. An out-of-range slot returns `MoveError::SlotOutOfBounds`.

For real stacks, `item` can be a borrowed host stack. Compare its complete item and component identity, excluding count; choose the destination's effective limit from the item and slot limits. Acceptance may depend on your container's current contents. The caller must recheck the original stacks, acceptance and limit and commit the move in the same authoritative transaction. The plan alone cannot prevent stale snapshots or perform rollback.

## Adapt an incoming stack

A container that currently inserts an incoming stack can keep its existing acceptance and capacity rules. The [complete `plan_insert` example](../examples/inventory/plan_insert.rs) appends a borrowed incoming stack to a temporary snapshot and passes its index as the source. For example, the call site can keep its own policy:

```rust,ignore
let plan = plan_insert(&slots, &incoming, destination, effective_limit,
    |index, stack| container_accepts(&slots, index, stack))?;
```

`effective_limit` is the consumer's resolved item/slot limit; `container_accepts` stands for its existing rule, including any material or uniqueness check. The example calls the host's `same_item_same_components` for identity. The source index in this plan is the temporary index `slots.len()`, not a real container slot: use the plan's `count` to update the caller-owned incoming stack and its destination only after rechecking the snapshot and policy in the authoritative transaction. The planner does not choose a slot, apply the move or refresh stale rules.

## API reference

- `StackSnapshot { item: &T, count: u32 }` borrows a complete stack identity and records its count.
- `plan_move(slots, from, to, requested, limit, accepts, same)` returns `Result<Option<MovePlan>, MoveError>`. The callbacks receive the destination and source identity for limit and acceptance, and both identities for equality.
- `plan_between(source, destination, requested, limit, accepts, same)` uses two snapshots from separate containers. Its logical indexes are 0 for the source and 1 for the destination; it does not itself commit or roll back either container.
- `MovePlan` contains `from`, `to`, `count`, `source_count_before`, and `destination_count_before` (`None` for an empty destination).
- `MoveError::SlotOutOfBounds` indicates an invalid index. `Ok(None)` means no move under the supplied conditions.

The [inventory demo](../examples/inventory/README.md) shows a consumer validating and committing a plan with host stacks. This first operation plans one source-to-destination move. It does not implement shift-click ordering, swaps, slot storage, adjacent inventories, networking or a GUI. An unreleased native bundle adds one `pluto-inventory-api` crate to `horizon-client-lib` and re-exports it through `horizon_client::pluto_inventory` for both the game and server-side transaction modules. This host-qualified path is not yet a host-independent stable facade. On Linux, the demo was built from the packaged library and checked locally and with a LAN guest, including host save/reload and guest reconnect. TConstruct uses this planner for Chest insertion and quick moves; a joint Linux test on the new target covered Chest and LAN guest inventory movement. The demo uses existing host synchronization; the planner itself does not provide networking.

## Revalidate a staged transaction

The same API exports three host-independent helpers extracted from TConstruct's
hopper transactions. They borrow their inputs and do not own storage, invoke
native events, decide slot policy, or commit a move:

- `select_candidate(candidates, check)` considers `(index, value)` candidates in
  order. The callback returns `Some(true)` to select, `Some(false)` to continue
  after cancellation, or `None` to abort the entire attempt after a stale
  snapshot. No later callback runs after selection or abort.
- `snapshot_matches(expected, current)` requires an exact, present live value.
  Missing storage fails closed, even when the expected inventory is empty.
  Compare original raw storage as well as decoded stacks if the decoder may
  discard extension fields.
- `encode_lossless(value, encode, decode, same)` returns an encoded proposal only
  if decoding succeeds and `same` confirms the complete original identity.
  For stacks, compare count, damage, and all components, and reject retained
  opaque wire state separately. A codec round trip does not guarantee network
  transport or restore information already discarded when loading raw data.

TConstruct's Chute and casting hopper routes use these helpers while retaining
their machine-specific restrictions, native callbacks, block checks, storage
writes, and rollback. Recheck after callbacks and commit in the same
single-authority transaction. These comparisons do not prevent another actor
from changing state between a check and a separate write.

The helpers have host-independent tests for cancellation, stale and missing
snapshots, changed count/damage/components, rejected decoding, and untouched
inputs. This is not evidence of live game or LAN behavior on a new host.

### Native item storage adapter

`horizon_client::pluto_stack_storage::encode_item_lossless(&stack)` applies the
codec guard to Horizon's actual ItemStack/NBT types. It rejects empty stacks and
all retained wire backing throughout the nested stack tree, including stale
backing hidden by `wire_patch()`.
Accepted proposals preserve count, damage and native component identity.
It returns a new NBT value without changing the stack or storage.

Before replacing a saved item, independently check that the existing raw record
matches the decoded item. Unknown raw fields may already have been discarded by
a decoder, and this helper cannot restore them. It also does not promise network
round trips, choose capacities or slot policy, or save a machine.

`decode_item_canonical(&raw)` decodes an item only when native re-encoding
compares equal to the input `Tag`. This uses the existing host codec and Tag
value equality, including tag types, UTF-16 keys and floating-point bit
patterns. It does not compare serialized file bytes or recover discarded
fields. Smeltery's three existing decode/re-encode checks reuse this adapter;
empty-slot forms, count/component validation, aliases, raw-slot preservation
and authoritative commits remain caller policy.

Canonical saved data is different from save eligibility and a live snapshot.
For example, serializing an opaque or runtime-bearing stack may already have
lost its wire backing or bundle selection before this decoder sees the Tag.
Keep the separate no-opaque and snapshot guards. A successful canonical decode
is not a transport guarantee or permission to overwrite an existing raw slot.

`contains_retained_wire(&stack)` is the shared read-only query for opaque backing
in the stack or its container contents, charged projectiles, pot decorations and
bundle contents. It includes stale backing. It does not establish that a stack
is saveable or that its data can be edited and transported losslessly.

Smeltery's existing no-opaque page and single-item insertion guards reuse this
query. They now reject nested backing as well as outer-stack backing; ordinary
representable nested components remain accepted. This strengthens the intended
no-opaque rewrite boundary without changing the caller's canonical raw-item
checks, count splitting, page layout, access checks or authoritative commit.
Read-only display projection still clones opaque stacks unchanged.

`same_item_snapshot(before, after)` additionally compares retained and currently
usable wire bytes at every node, which native component equality excludes.
`inventory_snapshot_matches(before, current)` uses that comparison for every
slot, including slot order, empty cells and vector length, and rejects missing
storage. TConstruct's Chute, casting, Cannon and Casting Tank hopper callback
guards use it so a change in opaque backing presence, exposed bytes or current
usability in any slot during a callback cannot pass as unchanged. The Melter's held-item check
uses the same item comparison. These checks preserve the caller's separate
block, chunk, raw storage, callback, machine and rollback rules.

Snapshot equality also checks runtime-only bundle selection at every node;
native component equality omits it, but a stale clone could overwrite it.
Save eligibility deliberately does not compare this unsaved runtime field.
The comparison covers observable native state, not the host's private retained-wire
decoded baseline. Different private baselines can expose the same bytes and
both currently be stale, so they can compare equal here. Do not treat these
helpers as complete private-state identity or arbitrary live write authority.
Their existing caller revalidation and commit restrictions remain essential;
persistence continues to reject all retained wire.
Editing damage or components can also make retained wire unusable even
when the original bytes remain present; retaining those bytes alone is not a
transport guarantee.

### Additive retained-baseline comparison (experimental)

`horizon_client::pluto_stack_snapshot::same_item_with_retained_baselines`
compares the existing observable state plus the exact retained decoded baseline
at every ordinary nested stack and every recursively retained baseline. It
requires the Pluto module installed inside the actual native `item_data` module;
the existing observable API and its callers do not silently gain this behavior.

The native child module `horizon_items::item_data::pluto_wire_read::retained`
borrows the stored baseline and patch without copying or changing either. It has
no dependency on a client, mod policy, registry or the public inventory crate.
This preserves the native dependency direction. It does not claim that the
retained patch is currently usable.

The additive comparator reuses `same_item_snapshot` for the observable policy
and its existing nested-stack traversal. Equal independently allocated values
compare equal. A completed node-pair cache avoids expanding every path through a
shared immutable baseline graph. Addresses are only cache keys after all values
and descendants have passed, never evidence of equality. Ordinary native
component comparison retains its own recursive traversal and allocation costs;
this is not a bounded-cost validator for untrusted arbitrarily deep data.

This is an experimental read-only comparison, not a transaction token or live
write permission. No consumer has adopted it, and persistence still rejects all
retained wire. The private declaration and complete `item_data.rs` source are
pinned by a mandatory native-plan guard. Complete package verification and all
eight actual applied-items regression cases passed at `4d82580b`; this does not
validate consumer adoption or live writes. See [the private-baseline contract
and proof procedure](item-private-baseline.md).

Casting, Heater, Cannon and Casting Tank save-eligibility checks delegate to
`encode_item_lossless`. Their previous native codec predicates had the same
acceptance criteria, including rejecting zero counts: the native decoder
normalizes zero to one, which fails exact count equality. Eligibility for saving
is deliberately different from snapshot identity. An unchanged item with
retained wire can match a snapshot but must not be serialized by this adapter.
Machine schemas and their conservative raw-item patching remain in TConstruct.

Smeltery inventory serialization also delegates its item save predicate to
`encode_item_lossless`, then keeps its separate canonical Tag check. This
intentionally rejects nested retained wire that its former outer-stack-only
guard missed. Empty slots, original raw-slot identity, aliases, progress rules
and the authoritative commit remain local; ordinary representable stacks keep
the previous codec behavior. This follow-up is separate from the earlier
canonical-decode equivalence proof.

The nested-wire correction deliberately tightens that earlier acceptance
boundary. The original adapter inspected wire only on the outer stack, while
native equality ignores opaque backing in nested stacks too. The earlier
root-only fixture results below do not prove nested preservation. Canonical
nested stacks retain the host codec's existing rules: charged projectiles use
the bounded item codec, while container, pot and bundle contents use native
stack templates. This adapter does not invent a second codec or normalize data.

A focused Linux fixture passed 14 tests against official Horizon
`624c4f6265c02296fbe8a80f029dd0d987fb94c0`, with transitive dependencies checked
against that host's lockfile. Ten tests exercise the generic API; four exercise
the actual native item adapter and extracted Chute snapshot helper, including
changed count/components/raw state and retained, changed or stale wire backing.
This fixture does not apply the full native bundle or prove full client, live
world or LAN behavior. Native package integrity and module-header checks also
passed for this extraction.


The subsequent [grouped inventory/storage validation](inventory-storage-validation-20261105.md)
checks the combined predicate-enabled API as one actual Cargo crate together
with its machine consumers. It records 427 passing fixture executions (with
shared production tests repeated across fixtures), the exact MOD/official host
revisions, the codec-parity matrix, and the unselected-slot wire callback
regression. This replaces the early source-inclusion fixture wiring with real
Cargo dependency wiring; it is still focused evidence, not a full-client or
live-world claim.

For explicit native NBT record changes, see [storage compound patches](nbt-storage.md).
The shared helper preserves untouched fields while each consumer retains its
schema, aliases, raw-slot rules and authoritative commit policy.

## Stage changes across storage partitions

`pluto_inventory::storage::stage_updates(changes, partition_key, load_copy,
apply_to_copy)` returns a sorted map of staged proposals only after all changes
succeed. Changes run in their supplied order and each partition is loaded once.
A missing partition or rejected change drops every proposal and stops later
callbacks. Empty input returns an empty map without invoking callbacks.

`load_copy` must supply independent proposal data. `apply_to_copy` must modify
that proposal, not live storage through shared handles. The helper cannot undo
callback side effects outside those values. It does not lock, persist, publish
or guarantee an atomic multi-record commit. The consumer revalidates and commits
through its existing authoritative path.

TConstruct uses this staging operation for `write_many`. Chunk addressing,
metadata IDs and schemas, preservation rules, actual chunk writes and dirty
marking stay in TConstruct. This preserves input ordering and failure behavior
without moving machine policy into the library.

The shared crate also provides a [read-only block-event facade](block-feedback.md)
with borrowed actor/equipment facts. Those item facts reuse the semantic
component DTO and must not be used as native snapshot/commit identity.


## Armor-wear observations

The grouped development API adds `pluto_inventory::armor_wear`, a read-only
consumption-time before/after event using the shared equipment views. See
[its phase, source guards and pending native validation](armor-wear-observation.md).
It is not a durability delta, commit receipt or write capability.

## Project a read-only slot view

`pluto_inventory::projection::project_slots(slots, mapping)` clones one cell for
each optional source index in `mapping`, in that order. `None`, an out-of-bounds
index and an empty source cell each produce an empty output cell. Repeated
indices appear repeatedly; values keep their normal `Clone` semantics. The
source is borrowed and unchanged.

The caller defines the mapping and output length. Chest and Smeltery views use
this operation while retaining their existing columns, rows, scroll arithmetic,
54-slot transport and reserved container cells in TConstruct. Stacks are cloned
whole, including opaque wire and runtime state; display projection does not run
the saved-item codec or impose save eligibility.

The mapping is not a write permission or a transaction snapshot. Keep layout
and reserved-cell rules separately, reject padding/virtual destinations, and
revalidate live storage before any authoritative commit. This operation does
not change the existing caller commit guards or implement click/drag policy,
network transport or a GUI.

The [focused projection proof](inventory-projection-validation-20261105.md)
records the exact source/official target and 75 passing API/native fixture
executions, including preserved layout, padding, reserved cells and opaque data.

The [recipe-book invalidation capability](recipe-book-invalidation.md) resets
only derived presentation caches through a scoped adapter. Recipe/index changes,
knowledge, preview/placement policy and refresh timing remain with consumers.

The [canonical item decode proof](item-canonical-decode-validation-20261105.md)
compares all three delegated checks with their previous native behavior and
records the exact cutoff, edge cases and isolated proof limits.

The later [grouped storage/potion proof](storage-potion-validation-20261105.md)
records 474 fixture executions at one combined cutoff, including the intentional
nested Smeltery save strengthening and actual potion adapter wiring.
