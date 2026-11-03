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
