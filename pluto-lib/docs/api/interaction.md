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

`UseBlockCallback` tells you which block was used. When the item in the hand matters, use `UseItemOnBlockCallback`: its context carries the item, how many are held, the dimension, the player (whose methods work inside the callback), whether the player is in creative mode and `held`, the stack that was used with its damage, max damage and custom data (a tool a mod made keeps its state there). This one turns dirt into grass when a seed is used on it, and uses the seed up:

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

`PlayerBlockBreakEvents::AFTER` runs once the block is gone. Every break event gets a `BlockBreakContext`: the position, dimension and block state, the player and the stack in their main hand, and `actions` for what to do to the player. For `AFTER` the held stack is the tool as of the report, after the break wore it, so a callback can read its custom data and queue an `EditStackData` for the main hand.

```rust
use pluto_lib::{ActionTarget, EntityAction, PlayerBlockBreakEvents, StackDataEdit, StackSlot, Nbt};

PlayerBlockBreakEvents::AFTER.register(|broken| {
    let Some(tool) = &broken.held else { return };
    broken.actions.borrow_mut().push(EntityAction::EditStackData {
        target: ActionTarget::Attacker, slot: StackSlot::MainHand, item: tool.item.clone(),
        edits: vec![StackDataEdit { path: vec!["mined".into()], value: Some(Nbt::Int(1)) }],
    });
});
```

> **Use with Caution.** `CANCELED` runs once when the combined `BEFORE` result vetoes a break, with the same position and before returning the cancellation to Horizon. An allowed break does not emit it. It is not a notification for unrelated vanilla restrictions or a later veto by another native mod. `AFTER` is not immediate: the game has no hook at the end of a break, so it runs at the breaking player's next tick or next break, at most one tick late, and only if the block at the position is no longer the one `BEFORE` saw (a break another mod cancelled later is not reported). `held` is the tool as of that tick. Creative breaks are heard too; it carries no block entity. Actions queued by `BEFORE` callbacks land even when a callback vetoes the break. Native-unvalidated.

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

A player breaking a block. `pos` is its position `[x, y, z]`, `dimension` the level (empty when unknown), `state` the block as it was (`None` when unreadable), `player` who breaks it, `held` the stack in their main hand and `actions` the `EntityAction`s a callback queues for the player (`BlockBreakContext::at` makes a context with a position only).

```rust
use pluto_lib::{BlockBreakContext, PlayerBlockBreakEvents};

PlayerBlockBreakEvents::BEFORE.register(|broken: &BlockBreakContext| broken.pos[1] >= 0);
assert!(PlayerBlockBreakEvents::BEFORE.invoke(&BlockBreakContext::at([1, 70, 1])));
```

### `BlockBreakContext::at`

A context that holds only a position: the other fields are empty. Useful in tests and for callbacks that only read `pos`.

```rust
use pluto_lib::BlockBreakContext;

let context = BlockBreakContext::at([1, 70, 1]);
assert!(context.player.is_none() && context.held.is_none() && context.actions.borrow().is_empty());
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
    held: None,
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
    player: ServerPlayer { uuid: 1, name: "Alex".into() }, creative: true, held: None,
};
assert_eq!(UseItemOnBlockCallback::EVENT.invoke(&used), ActionResult::Fail);
```

### `Hand`

The hand a player used an item in (`Main` or `Off`; Minecraft: `InteractionHand`).

```rust
use pluto_lib::Hand;

assert_ne!(Hand::Main, Hand::Off);
```

### `UseItemFinishContext`

A player finished eating or drinking an item. `item` is the item id and `player` the player (its methods work inside the callback). `stack` is the stack that was used (damage, max damage and custom data) and `hand` the hand it was used in. `slot` is what that hand holds now with all its components (`None` when the last of a stack was eaten), and `replacement` is where a callback puts a changed copy of it: the later callbacks see it, and the game writes it to the hand. `dimension` and `position` say where the player is, and `creative` whether nothing is used up.

```rust
use std::cell::RefCell;
use pluto_lib::{Hand, ServerPlayer, StackData, UseItemFinishContext};

let done = UseItemFinishContext {
    item: "example:cheese".into(), player: ServerPlayer { uuid: 1, name: "Alex".into() }, stack: StackData::default(), hand: Hand::Main,
    slot: None, replacement: RefCell::new(None), dimension: "minecraft:overworld".into(), position: [0.0, 64.0, 0.0], creative: false,
};
assert_eq!(done.item, "example:cheese");
assert_eq!(done.player.name, "Alex");
assert_eq!(done.hand, Hand::Main);
```

### `UseItemFinishCallback`

A player finished eating or drinking an item (NeoForge: `LivingEntityUseItemEvent.Finish`). Called after the food and the item's own effects have been given, only for a player on the server. A callback may wear the used stack or change what the hand holds through `replacement`.

```rust
use pluto_lib::UseItemFinishCallback;

UseItemFinishCallback::EVENT.register(|done| {
    if done.item == "example:cheese" {
        let _ = done.player.effects();
    }
    // Eating the tool wears it by two: change a copy of what the hand holds and hand it back.
    if done.item == "example:lunchbox" && !done.creative {
        if let Some(mut worn) = done.slot.clone() {
            worn.damage += 2;
            *done.replacement.borrow_mut() = Some(worn);
        }
    }
});
```

### `UseItemFinishCallback::EVENT`

The event to register on. Every callback is called in turn.

```rust
use std::cell::RefCell;
use pluto_lib::{Hand, ServerPlayer, StackData, UseItemFinishCallback, UseItemFinishContext};
use std::sync::atomic::{AtomicU32, Ordering};

static FINISHED: AtomicU32 = AtomicU32::new(0);
UseItemFinishCallback::EVENT.register(|done| if done.item == "example:snack" { FINISHED.fetch_add(1, Ordering::SeqCst); });
let done = UseItemFinishContext {
    item: "example:snack".into(), player: ServerPlayer { uuid: 1, name: "Alex".into() }, stack: StackData::default(), hand: Hand::Off,
    slot: None, replacement: RefCell::new(None), dimension: "minecraft:overworld".into(), position: [0.0, 64.0, 0.0], creative: true,
};
UseItemFinishCallback::EVENT.invoke(&done);
assert_eq!(FINISHED.load(Ordering::SeqCst), 1);
```

### `BreakSpeedContext`

How fast the player digs a block. `pos` is the block, `block` its state, `original` the speed as vanilla works it out (it does not change), `speed` the speed now (a `Cell`: change it with `set`) `held` the stack the player digs with (damage, max damage and custom data; None for an empty hand) and `player`, a `DiggingPlayer` with `on_ground` and `eyes_in_water` (None when the game does not say who digs). Vanilla already slows a dig in the air and under water, so a callback that needs those multipliers reads them from `original` and `player`. The dig's own facts ride along too: `block_light` and `sky_light` at the block (each 0 to 15, None when the game did not report them), `sky_darken` (how much of the sky light the time of day and the weather take off) and `biome_temperature`, the temperature of the biome at the block (None for a biome PlutoLib has no temperature for).

```rust
use pluto_lib::{BlockState, BreakSpeedContext};
use std::cell::Cell;

let dig = BreakSpeedContext { pos: [0, 64, 0], block: BlockState::parse("minecraft:stone").unwrap(), original: 1.5, speed: Cell::new(1.5), held: None, player: None, block_light: Some(0), sky_light: Some(15), sky_darken: 0, biome_temperature: Some(0.8) };
dig.speed.set(dig.speed.get() * 2.0);
assert_eq!((dig.original, dig.speed.get()), (1.5, 3.0));
assert_eq!(dig.pos, [0, 64, 0]);
assert_eq!(dig.block.block.path(), "stone");
```

### `DiggingPlayer`

What a break-speed callback may read about the player who digs: `on_ground` (the player's last accepted move put them on the ground) and `eyes_in_water` (the block at their eye height holds water or is waterlogged).

```rust
use pluto_lib::DiggingPlayer;

let airborne = DiggingPlayer { on_ground: false, eyes_in_water: false };
assert!(!airborne.on_ground && !airborne.eyes_in_water);
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
let dig = BreakSpeedContext { pos: [0; 3], block: BlockState::parse("example:slow_stone").unwrap(), original: 4.0, speed: Cell::new(4.0), held: None, player: None, block_light: None, sky_light: None, sky_darken: 0, biome_temperature: None };
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

The block-break events. `BEFORE` decides whether a break may proceed; a veto emits `CANCELED` once for that attempt; `AFTER` hears a break that removed the block.

```rust
use pluto_lib::PlayerBlockBreakEvents;

PlayerBlockBreakEvents::BEFORE.register(|_| true);
```

### `PlayerBlockBreakEvents::BEFORE`

Before a block is broken. Answer `false` to cancel the break; one `false` is enough.

```rust
use pluto_lib::{BlockBreakContext, PlayerBlockBreakEvents};

PlayerBlockBreakEvents::BEFORE.register(|b| b.pos != [0, 64, 0]); // protect the block at the origin
assert!(!PlayerBlockBreakEvents::BEFORE.invoke(&BlockBreakContext::at([0, 64, 0])));
assert!(PlayerBlockBreakEvents::BEFORE.invoke(&BlockBreakContext::at([1, 64, 0])));
```

### `PlayerBlockBreakEvents::AFTER`

After a player's break removed a block (Fabric: `PlayerBlockBreakEvents.AFTER`). Read the player, the block's `state` and the worn `held` tool from the context and queue `actions` for the player; see "Breaking Blocks" for when it runs and what it leaves out.

```rust
use pluto_lib::PlayerBlockBreakEvents;

PlayerBlockBreakEvents::AFTER.register(|broken| {
    if broken.state.as_ref().is_some_and(|state| state.block.path() == "stone") && broken.held.is_none() { return; }
});
```

### `PlayerBlockBreakEvents::CANCELED`

A break was vetoed by `BEFORE`. Receives the same `BlockBreakContext { pos }` once per rejected attempt, even when multiple listeners would reject it. Allowed attempts do not emit this event. The block has not been removed. Rejections outside PlutoLib's `BEFORE` event are not reported here.

```rust
use pluto_lib::PlayerBlockBreakEvents;

PlayerBlockBreakEvents::CANCELED.register(|_| {});
```
