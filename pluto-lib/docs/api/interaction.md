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

## Using an Item on a Block

`UseBlockCallback` tells you which block was used. When the item in the hand matters, use `UseItemOnBlockCallback`: its context carries the item, how many are held, the dimension, the player (whose methods work inside the callback) and whether the player is in creative mode. This one turns dirt into grass when a seed is used on it, and uses the seed up:

```rust
use pluto_lib::{ActionResult, BlockState, Identifier, UseItemOnBlockCallback, World};

UseItemOnBlockCallback::EVENT.register(|used| {
    if used.item != "example:grass_seeds" {
        return ActionResult::Pass;
    }
    let world = World::new(&used.dimension);
    let dirt = world.get_block_state(used.pos).filter(|s| s.block.to_string() == "minecraft:dirt");
    if dirt.is_none() || !world.set_block_state(used.pos, &BlockState::new(Identifier::new("minecraft", "grass_block").unwrap())) {
        return ActionResult::Pass;
    }
    world.play_sound("minecraft:block.grass.place", [0.5, 0.5, 0.5], 1.0, 1.0);
    if !used.creative {
        used.player.take_item(&used.item, 1);
    }
    ActionResult::Success
});
```

## How Fast a Block Is Dug

`BreakSpeedCallback` is asked every tick of a dig, on the server and on the client that predicts it, with the speed the game worked out. Change it with `speed.set(..)`; every callback is called in turn. A speed of 0 makes no progress, so the block is not broken. Not asked in creative mode or for an unbreakable block. This one makes a golden pickaxe dig stone twice as fast:

```rust
use pluto_lib::BreakSpeedCallback;

BreakSpeedCallback::EVENT.register(|dig| {
    if dig.block.block.path() == "stone" {
        dig.speed.set(dig.speed.get() * 2.0);
    }
});
```

> **Use with Caution.** It runs every tick of every dig: only compute.

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

### `UseItemOnBlockContext`

A player used an item on a block. `item` is the item id, `count` how many are held, `pos` the block, `face` the face that was clicked as a unit offset, `dimension` the dimension id, `player` the player (its methods work inside the callback) and `creative` whether the player is in creative mode.

```rust
use pluto_lib::{ServerPlayer, UseItemOnBlockContext};

let used = UseItemOnBlockContext {
    item: "example:grass_seeds".into(),
    count: 3,
    pos: [1, 64, 1],
    face: [0, 1, 0],
    dimension: "minecraft:overworld".into(),
    player: ServerPlayer { uuid: 1, name: "Alex".into() },
    creative: false,
};
assert_eq!((used.count, used.pos, used.face), (3, [1, 64, 1], [0, 1, 0]));
assert_eq!(used.item, "example:grass_seeds");
assert_eq!(used.dimension, "minecraft:overworld");
assert_eq!(used.player.name, "Alex");
assert!(!used.creative);
```

### `UseItemOnBlockCallback`

A player used an item on a block, before the game handles it. Answer `Success` after doing the job and the game does not handle it; the first answer that is not `Pass` wins.

```rust
use pluto_lib::{ActionResult, UseItemOnBlockCallback};

UseItemOnBlockCallback::EVENT.register(|used| if used.item == "example:wand" { ActionResult::Success } else { ActionResult::Pass });
```

### `UseItemOnBlockCallback::EVENT`

The event itself, to register on or to invoke in a test.

```rust
use pluto_lib::{ActionResult, ServerPlayer, UseItemOnBlockCallback, UseItemOnBlockContext};

UseItemOnBlockCallback::EVENT.register(|used| if used.item == "example:probe_item" { ActionResult::Fail } else { ActionResult::Pass });
let used = UseItemOnBlockContext {
    item: "example:probe_item".into(), count: 1, pos: [0, 0, 0], face: [0, 1, 0], dimension: "minecraft:overworld".into(),
    player: ServerPlayer { uuid: 1, name: "Alex".into() }, creative: true,
};
assert_eq!(UseItemOnBlockCallback::EVENT.invoke(&used), ActionResult::Fail);
```

### `UseItemFinishContext`

A player finished eating or drinking an item. `item` is the item id and `player` the player (its methods work inside the callback).

```rust
use pluto_lib::{ServerPlayer, UseItemFinishContext};

let done = UseItemFinishContext { item: "example:cheese".into(), player: ServerPlayer { uuid: 1, name: "Alex".into() } };
assert_eq!(done.item, "example:cheese");
assert_eq!(done.player.name, "Alex");
```

### `UseItemFinishCallback`

A player finished eating or drinking an item (NeoForge: `LivingEntityUseItemEvent.Finish`). Called after the food and the item's own effects have been given, only for a player on the server.

```rust
use pluto_lib::UseItemFinishCallback;

UseItemFinishCallback::EVENT.register(|done| {
    if done.item == "example:cheese" {
        let _ = done.player.effects();
    }
});
```

### `UseItemFinishCallback::EVENT`

The event to register on. Every callback is called in turn.

```rust
use pluto_lib::{ServerPlayer, UseItemFinishCallback, UseItemFinishContext};
use std::sync::atomic::{AtomicU32, Ordering};

static FINISHED: AtomicU32 = AtomicU32::new(0);
UseItemFinishCallback::EVENT.register(|done| if done.item == "example:snack" { FINISHED.fetch_add(1, Ordering::SeqCst); });
let done = UseItemFinishContext { item: "example:snack".into(), player: ServerPlayer { uuid: 1, name: "Alex".into() } };
UseItemFinishCallback::EVENT.invoke(&done);
assert_eq!(FINISHED.load(Ordering::SeqCst), 1);
```

### `BreakSpeedContext`

How fast the player digs a block. `pos` is the block, `block` its state, `original` the speed as vanilla works it out (it does not change) and `speed` the speed now (a `Cell`: change it with `set`).

```rust
use pluto_lib::{BlockState, BreakSpeedContext};
use std::cell::Cell;

let dig = BreakSpeedContext { pos: [0, 64, 0], block: BlockState::parse("minecraft:stone").unwrap(), original: 1.5, speed: Cell::new(1.5) };
dig.speed.set(dig.speed.get() * 2.0);
assert_eq!((dig.original, dig.speed.get()), (1.5, 3.0));
assert_eq!(dig.pos, [0, 64, 0]);
assert_eq!(dig.block.block.path(), "stone");
```

### `BreakSpeedCallback`

How fast a player digs a block (NeoForge: `PlayerEvent.BreakSpeed`).

```rust
use pluto_lib::BreakSpeedCallback;

BreakSpeedCallback::EVENT.register(|dig| dig.speed.set(dig.speed.get() + 1.0));
```

### `BreakSpeedCallback::EVENT`

The event itself, to register on or to invoke in a test.

```rust
use pluto_lib::{BlockState, BreakSpeedCallback, BreakSpeedContext};
use std::cell::Cell;

BreakSpeedCallback::EVENT.register(|dig| if dig.block.block.path() == "slow_stone" { dig.speed.set(0.0) });
let dig = BreakSpeedContext { pos: [0; 3], block: BlockState::parse("example:slow_stone").unwrap(), original: 4.0, speed: Cell::new(4.0) };
BreakSpeedCallback::EVENT.invoke(&dig);
assert_eq!(dig.speed.get(), 0.0);
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
