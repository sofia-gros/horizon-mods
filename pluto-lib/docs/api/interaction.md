# Interaction events: using items and blocks, attacking, breaking blocks

Callbacks decide what happens when a player acts. The use and attack events answer with an `ActionResult` (the first answer that is not `Pass` wins); the block-break `BEFORE` event answers yes or no (any no cancels).

## Basic Example

Stop players from using a block in a place, and let everything else through:

```rust
use pluto_lib::{ActionResult, UseBlockCallback};

UseBlockCallback::EVENT.register(|used| {
    if used.pos[1] < 0 { ActionResult::Fail } else { ActionResult::Pass }
});
```

## What an Answer Means

The use and attack events answer with an `ActionResult`, and the first answer that is not `Pass` wins. `Fail` cancels the action. `Success` and `Consume` mean your callback did the job (`Success` also swings the hand). `Pass` leaves it to the next callback and finally to the game.

```rust
use pluto_lib::ActionResult;

assert!(ActionResult::Consume.is_accepted());
assert!(!ActionResult::Consume.should_swing_hand());
assert!(ActionResult::Success.should_swing_hand());
```

## Breaking Blocks

`PlayerBlockBreakEvents::BEFORE` is a yes or no question instead: answer `false` to cancel the break.

```rust
use pluto_lib::PlayerBlockBreakEvents;

PlayerBlockBreakEvents::BEFORE.register(|broken| broken.pos != [0, 64, 0]);
```

> **Use with Caution.** `AFTER` and `CANCELED` are accepted but not called yet (support matrix).

## Reference

## Data

### `UseItemContext`

A player used an item in the air. `item` is the item id, `count` the stack size.

```rust
use pluto_lib::{ActionResult, UseItemCallback, UseItemContext};

UseItemCallback::EVENT.register(|used: &UseItemContext| {
    if used.item == "minecraft:stick" && used.count > 1 { ActionResult::Fail } else { ActionResult::Pass }
});
let result = UseItemCallback::EVENT.invoke(&UseItemContext { item: "minecraft:stick".into(), count: 2 });
assert_eq!(result, ActionResult::Fail);
```

### `UseBlockContext`

A player used a block. `pos` is the block position `[x, y, z]`, `face` the clicked face as a direction vector (for example `[0, 1, 0]` is the top).

```rust
use pluto_lib::{ActionResult, UseBlockCallback, UseBlockContext};

UseBlockCallback::EVENT.register(|used: &UseBlockContext| {
    if used.face == [0, 1, 0] && used.pos[1] < 0 { ActionResult::Fail } else { ActionResult::Pass }
});
let result = UseBlockCallback::EVENT.invoke(&UseBlockContext { pos: [4, -3, 9], face: [0, 1, 0] });
assert_eq!(result, ActionResult::Fail);
```

### `AttackEntityContext`

A player attacked an entity. `entity` is the entity id, `kind` its type id, `pos` its position `[x, y, z]`.

```rust
use pluto_lib::{ActionResult, AttackEntityCallback, AttackEntityContext};

AttackEntityCallback::EVENT.register(|attack: &AttackEntityContext| {
    if attack.kind == "minecraft:villager" { ActionResult::Fail } else { ActionResult::Pass }
});
let result = AttackEntityCallback::EVENT.invoke(&AttackEntityContext { entity: 12, kind: "minecraft:villager".into(), pos: [0.0, 64.0, 0.0] });
assert_eq!(result, ActionResult::Fail);
```

### `BlockBreakContext`

A block is being broken. `pos` is its position `[x, y, z]`.

```rust
use pluto_lib::{BlockBreakContext, PlayerBlockBreakEvents};

PlayerBlockBreakEvents::BEFORE.register(|broken: &BlockBreakContext| broken.pos[1] >= 0);
assert!(PlayerBlockBreakEvents::BEFORE.invoke(&BlockBreakContext { pos: [1, 70, 1] }));
```

## Events

### `UseItemCallback`

Holder of the use-item event.

```rust
use pluto_lib::UseItemCallback;

UseItemCallback::EVENT.register(|_| pluto_lib::ActionResult::Pass);
```

### `UseItemCallback::EVENT`

Runs when a player uses an item. Answer `Fail` to cancel, `Success` / `Consume` to take the action yourself, `Pass` to leave it to the game.

```rust
use pluto_lib::{ActionResult, UseItemCallback, UseItemContext};

UseItemCallback::EVENT.register(|_| ActionResult::Consume);
assert_eq!(UseItemCallback::EVENT.invoke(&UseItemContext { item: "minecraft:apple".into(), count: 1 }), ActionResult::Consume);
```

### `UseBlockCallback`

Holder of the use-block event.

```rust
use pluto_lib::UseBlockCallback;

UseBlockCallback::EVENT.register(|_| pluto_lib::ActionResult::Pass);
```

### `UseBlockCallback::EVENT`

Runs when a player uses (right-clicks) a block.

```rust
use pluto_lib::{ActionResult, UseBlockCallback, UseBlockContext};

UseBlockCallback::EVENT.register(|_| ActionResult::Success);
assert!(UseBlockCallback::EVENT.invoke(&UseBlockContext { pos: [0, 0, 0], face: [0, 1, 0] }).should_swing_hand());
```

### `AttackEntityCallback`

Holder of the attack-entity event.

```rust
use pluto_lib::AttackEntityCallback;

AttackEntityCallback::EVENT.register(|_| pluto_lib::ActionResult::Pass);
```

### `AttackEntityCallback::EVENT`

Runs when a player attacks an entity. `Fail` cancels the attack.

```rust
use pluto_lib::{ActionResult, AttackEntityCallback, AttackEntityContext};

AttackEntityCallback::EVENT.register(|_| ActionResult::Fail);
let attack = AttackEntityContext { entity: 1, kind: "minecraft:cow".into(), pos: [0.0; 3] };
assert!(!AttackEntityCallback::EVENT.invoke(&attack).is_accepted());
```

### `PlayerBlockBreakEvents`

The block-break events. `BEFORE` works; `AFTER` and `CANCELED` are accepted but never invoked yet (support matrix).

```rust
use pluto_lib::PlayerBlockBreakEvents;

PlayerBlockBreakEvents::BEFORE.register(|_| true);
```

### `PlayerBlockBreakEvents::BEFORE`

Before a block is broken. Answer `false` to cancel the break; one `false` is enough.

```rust
use pluto_lib::{BlockBreakContext, PlayerBlockBreakEvents};

PlayerBlockBreakEvents::BEFORE.register(|b| b.pos != [0, 64, 0]); // protect the block at the origin
assert!(!PlayerBlockBreakEvents::BEFORE.invoke(&BlockBreakContext { pos: [0, 64, 0] }));
assert!(PlayerBlockBreakEvents::BEFORE.invoke(&BlockBreakContext { pos: [1, 64, 0] }));
```

### `PlayerBlockBreakEvents::AFTER`

Stub: after a block was broken. Accepted, not invoked yet.

```rust
use pluto_lib::PlayerBlockBreakEvents;

PlayerBlockBreakEvents::AFTER.register(|_| {});
```

### `PlayerBlockBreakEvents::CANCELED`

Stub: a break was canceled. Accepted, not invoked yet.

```rust
use pluto_lib::PlayerBlockBreakEvents;

PlayerBlockBreakEvents::CANCELED.register(|_| {});
```
