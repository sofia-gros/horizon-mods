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

This first operation plans one source-to-destination move. It does not implement shift-click ordering, swaps, slot storage, adjacent inventories, networking or a GUI. An unreleased native bundle adds one `pluto-inventory-api` crate to `horizon-client-lib` and re-exports it through `horizon_client::pluto_inventory` for both the game and server-side transaction modules. This host-qualified path is not yet a host-independent stable facade; game and Tinkers integration are not yet verified.
