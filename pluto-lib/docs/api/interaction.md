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

A player used an item in the air. `item` is the item id, `count` the stack size, `stack` the stack as used, with its damage and custom data (a tool's stats and modifiers live there), and `player` who used it (`None` when the host named no acting player and the world has not exactly one player). The host's use event names no hand; the hand holding the stack is the one whose carried stack equals `stack`.

```rust
use pluto_lib::{ActionResult, StackData, UseItemCallback, UseItemContext};

UseItemCallback::EVENT.register(|used: &UseItemContext| {
    if used.item == "minecraft:stick" && used.count > 1 { ActionResult::Fail } else { ActionResult::Pass }
});
let used = UseItemContext { item: "minecraft:stick".into(), count: 2, stack: StackData { item: "minecraft:stick".into(), count: 2, ..StackData::default() }, player: None };
assert_eq!(UseItemCallback::EVENT.invoke(&used), ActionResult::Fail);
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

A player breaking a block. `pos` is its position `[x, y, z]`, `dimension` the level (empty when unknown), `state` the block as it was (`None` when unreadable), `player` who breaks it, `held` the stack in their main hand and `actions` the `EntityAction`s a callback queues for the player (`BlockBreakContext::at` makes a context with a position only). `face` is the face of the block the player hit as the dig started (`None` when the game did not report one), `yaw` and `pitch` are where the player looks in degrees, and `extra_breaks` holds the extra block positions a `BEFORE` callback adds with `break_extra`.

```rust
use pluto_lib::{BlockBreakContext, PlayerBlockBreakEvents};

PlayerBlockBreakEvents::BEFORE.register(|broken: &BlockBreakContext| broken.pos[1] >= 0);
assert!(PlayerBlockBreakEvents::BEFORE.invoke(&BlockBreakContext::at([1, 70, 1])));
let context = BlockBreakContext::at([1, 70, 1]);
assert!(context.face.is_none() && context.yaw == 0.0 && context.pitch == 0.0 && context.extra_breaks.borrow().is_empty());
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
use pluto_lib::{ActionResult, StackData, UseItemCallback, UseItemContext};

UseItemCallback::EVENT.register(|_| ActionResult::Consume);
let used = UseItemContext { item: "minecraft:apple".into(), count: 1, stack: StackData { item: "minecraft:apple".into(), count: 1, ..StackData::default() }, player: None };
assert_eq!(UseItemCallback::EVENT.invoke(&used), ActionResult::Consume);
```

### `DispenseRegistry`

The registry of dispenser behaviours, keyed by the item dispensed. The game asks its mods before a dispenser ejects (its dispense hook); PlutoLib answers for the items registered here — spawning the shot, consuming one item and playing the projectile-launch event — and an item nobody claims keeps the game's own dispensing.

```rust
use pluto_lib::{DispenseBehavior, DispenseRegistry, Identifier};

DispenseRegistry::register(
    Identifier::parse("example:glow_ball").unwrap(),
    DispenseBehavior::Shoot { entity: Identifier::parse("example:glow_ball").unwrap(), power: 1.1, inaccuracy: 6.0 },
);
assert!(DispenseRegistry::get("example:glow_ball").is_some());
assert!(DispenseRegistry::get("example:unclaimed").is_none());
```

### `DispenseBehavior`

What a dispenser does with one item. `Shoot { entity, power, inaccuracy }` fires the item as a projectile of the entity type `entity` (the original's `ShootProjectileDispenserBehavior`): the projectile spawns at the dispenser's mouth carrying the item, flies the shot [`dispense_shot`](#dispense_shot) computes, and one item is consumed.

```rust
use pluto_lib::{DispenseBehavior, Identifier};

let shoot = DispenseBehavior::Shoot { entity: Identifier::parse("example:efln_ball").unwrap(), power: 1.1, inaccuracy: 6.0 };
assert!(matches!(shoot, DispenseBehavior::Shoot { power, .. } if power == 1.1));
```

### `dispense_shot`

`fn dispense_shot(pos: [i32; 3], facing: Direction, power: f32) -> ([f64; 3], [f64; 3])`. Where a shot spawns and the motion it starts with: the dispenser's dispense position (the block's centre stepped 0.7 along the facing) and the facing stepped up by 0.1, normalised and scaled by `power`. The spread the inaccuracy would add needs a random source the dispense hook does not hand over, so the shot is the mean shot and flies true.

```rust
use pluto_lib::{dispense_shot, Direction};

let (origin, motion) = dispense_shot([10, 64, -3], Direction::East, 1.1);
assert!((origin[0] - 11.2).abs() < 1e-9 && (origin[1] - 64.5).abs() < 1e-9);
let length = (motion[0] * motion[0] + motion[1] * motion[1] + motion[2] * motion[2]).sqrt();
assert!((length - 1.1).abs() < 1e-9, "the shot flies at its power");
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

What a break-speed callback may read about the player who digs: `on_ground` (the player's last accepted move put them on the ground), `eyes_in_water` (the block at their eye height holds water or is waterlogged), `effect_levels` the effects on the player as (effect id, amplifier), and `rain_exposed` whether rain falls on the player.

```rust
use pluto_lib::DiggingPlayer;

let airborne = DiggingPlayer { on_ground: false, eyes_in_water: false, ..DiggingPlayer::default() };
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

### `DiggingFacts`

What the world reports about a dig besides the block itself. `block_light` and `sky_light` are the light at the block (each 0 to 15, `None` when the game did not report them), `sky_darken` is how much of the sky light the time of day and the weather take off, and `biome_temperature` is the temperature of the biome at the block (`None` for a biome with no known temperature).

```rust
use pluto_lib::events_interaction_v0::DiggingFacts;

let facts = DiggingFacts { block_light: Some(3), sky_light: Some(15), sky_darken: 4, biome_temperature: Some(0.8) };
assert_eq!((facts.block_light, facts.sky_light, facts.sky_darken), (Some(3), Some(15), 4));
assert_eq!(facts.biome_temperature, Some(0.8));
assert_eq!(DiggingFacts::default(), DiggingFacts { block_light: None, sky_light: None, sky_darken: 0, biome_temperature: None });
```

### `UseItemOnEntityContext`

A player used an item on an entity (right click, not an attack). `entity` is the entity id, `kind` its type id, `hand` the hand the item is used with, `held` the stack in that hand (`None` for an empty hand), `player` the player (`None` when the game names none) and `actions` the `EntityAction`s a callback queues for the player.

```rust
use std::cell::RefCell;
use pluto_lib::{Hand, ServerPlayer, StackData, UseItemOnEntityContext};

let click = UseItemOnEntityContext {
    entity: 5, kind: "minecraft:sheep".into(), hand: Hand::Main,
    held: Some(StackData { item: "test:kama".into(), count: 1, ..Default::default() }),
    player: Some(ServerPlayer { uuid: 9, name: "Alex".into() }), actions: RefCell::new(Vec::new()),
};
assert_eq!((click.entity, click.hand), (5, Hand::Main));
assert_eq!(click.kind, "minecraft:sheep");
assert_eq!(click.held.as_ref().map(|held| held.item.as_str()), Some("test:kama"));
assert_eq!(click.player.as_ref().map(|player| player.uuid), Some(9));
assert!(click.actions.borrow().is_empty());
```

### `InteractEntityContext`

A player interacted with an entity at the game's own generic claim point, before any hand-specific handling, so the context names no hand. `entity` is the entity id, `kind` its type id, `stack` the stack the interaction is made with (`None` when the game names none) and `player` the player (`None` when the game names none).

```rust
use pluto_lib::InteractEntityContext;

let click = InteractEntityContext { entity: 41, kind: "minecraft:armor_stand".into(), stack: None, player: None };
assert_eq!(click.entity, 41);
assert_eq!(click.kind, "minecraft:armor_stand");
assert!(click.stack.is_none() && click.player.is_none());
```

### `FishingContext`

A player's fishing rod changed state. `player` is the player fishing, `state` the state's number (see `FISHING_CAST` and `FISHING_BITE`), `hand` the hand holding the rod, `rod` the rod stack in that hand (`None` when the game did not read it), `cancel` a flag a callback sets to true to cancel the state and `actions` the `EntityAction`s a callback queues for the player.

```rust
use std::cell::{Cell, RefCell};
use pluto_lib::{FishingContext, Hand, ServerPlayer, StackData, FISHING_CAST};

let fish = FishingContext {
    player: ServerPlayer { uuid: 21, name: "Fisher".into() }, state: FISHING_CAST, hand: Hand::Main,
    rod: Some(StackData { item: "minecraft:fishing_rod".into(), count: 1, ..Default::default() }),
    cancel: Cell::new(false), actions: RefCell::new(Vec::new()),
};
assert_eq!((fish.player.uuid, fish.state, fish.hand), (21, FISHING_CAST, Hand::Main));
assert_eq!(fish.rod.as_ref().map(|rod| rod.item.as_str()), Some("minecraft:fishing_rod"));
assert!(!fish.cancel.get() && fish.actions.borrow().is_empty());
```

### `FISHING_CAST`

The fishing state for a cast: `pub const FISHING_CAST: i32 = 0`. A `FishingContext` whose `state` is this value means the bobber was cast.

```rust
use pluto_lib::FISHING_CAST;

assert_eq!(FISHING_CAST, 0);
```

### `FISHING_BITE`

The fishing state for a bite: `pub const FISHING_BITE: i32 = 1`. A `FishingContext` whose `state` is this value means a fish bit.

```rust
use pluto_lib::{FISHING_BITE, FISHING_CAST};

assert_eq!(FISHING_BITE, 1);
assert_ne!(FISHING_BITE, FISHING_CAST);
```

### `BlockBreakContext::break_extra`

Takes another block with this break: `fn break_extra(&self, pos: [i32; 3])`. The block itself, a duplicate and anything past 64 extra blocks are refused; the game breaks the extra blocks through its own destroy path and they fire no break events of their own.

```rust
use pluto_lib::BlockBreakContext;

let context = BlockBreakContext::at([10, 64, 10]);
context.break_extra([11, 64, 10]);
context.break_extra([11, 64, 10]);
context.break_extra([10, 64, 10]);
assert_eq!(*context.extra_breaks.borrow(), vec![[11, 64, 10]]);
```

### `UseEntityCallback`

Holder of the use-item-on-entity event (Fabric: `UseEntityCallback`). It is heard once for each hand the game tries, the main hand first.

```rust
use pluto_lib::UseEntityCallback;

UseEntityCallback::EVENT.register(|_| pluto_lib::ActionResult::Pass);
```

### `UseEntityCallback::EVENT`

The event itself: `pub const EVENT: &'static Event<UseItemOnEntityContext, ActionResult>`. Runs when a player uses an item on an entity; the first answer that is not `Pass` takes the click and neither the entity's own interaction nor the item's runs.

```rust
use std::cell::RefCell;
use pluto_lib::{ActionResult, Hand, ServerPlayer, StackData, UseEntityCallback, UseItemOnEntityContext};

UseEntityCallback::EVENT.register(|click| {
    if click.kind == "minecraft:sheep" && click.held.as_ref().is_some_and(|held| held.item == "test:kama") { ActionResult::Success } else { ActionResult::Pass }
});
let click = UseItemOnEntityContext {
    entity: 5, kind: "minecraft:sheep".into(), hand: Hand::Main,
    held: Some(StackData { item: "test:kama".into(), count: 1, ..Default::default() }),
    player: Some(ServerPlayer { uuid: 9, name: "Alex".into() }), actions: RefCell::new(Vec::new()),
};
assert_eq!(UseEntityCallback::EVENT.invoke(&click), ActionResult::Success);
```

### `InteractEntityCallback`

Holder of the generic entity-interaction event. It runs at the game's own claim point, before its hand-specific handling.

```rust
use pluto_lib::InteractEntityCallback;

InteractEntityCallback::EVENT.register(|_| pluto_lib::ActionResult::Pass);
```

### `InteractEntityCallback::EVENT`

The event itself: `pub const EVENT: &'static Event<InteractEntityContext, ActionResult>`. Runs when a player interacts with an entity; the first answer that is not `Pass` takes the interaction and the game does not handle it.

```rust
use pluto_lib::{ActionResult, InteractEntityCallback, InteractEntityContext};

InteractEntityCallback::EVENT.register(|context| if context.entity == 41 { ActionResult::Success } else { ActionResult::Pass });
let click = InteractEntityContext { entity: 41, kind: "minecraft:armor_stand".into(), stack: None, player: None };
assert_eq!(InteractEntityCallback::EVENT.invoke(&click), ActionResult::Success);
let other = InteractEntityContext { entity: 42, kind: "minecraft:armor_stand".into(), stack: None, player: None };
assert_eq!(InteractEntityCallback::EVENT.invoke(&other), ActionResult::Pass);
```

### `PlayerFishEvents`

The fishing events (Bukkit: `PlayerFishEvent`). Every callback runs, and any one of them may cancel the state through the context's `cancel` flag.

```rust
use pluto_lib::PlayerFishEvents;

PlayerFishEvents::EVENT.register(|_| {});
```

### `PlayerFishEvents::EVENT`

The event itself: `pub const EVENT: &'static Event<FishingContext, ()>`. Runs when a player's fishing rod changes state; a cancelled cast throws no bobber and a cancelled reel-in catches nothing.

```rust
use std::cell::{Cell, RefCell};
use pluto_lib::{FishingContext, Hand, PlayerFishEvents, ServerPlayer, StackData, FISHING_BITE, FISHING_CAST};

PlayerFishEvents::EVENT.register(|fish| if fish.player.uuid == 21 && fish.state == FISHING_CAST { fish.cancel.set(true); });
let fish = |state: i32| FishingContext {
    player: ServerPlayer { uuid: 21, name: "Fisher".into() }, state, hand: Hand::Main,
    rod: Some(StackData { item: "minecraft:fishing_rod".into(), count: 1, ..Default::default() }),
    cancel: Cell::new(false), actions: RefCell::new(Vec::new()),
};
let cast = fish(FISHING_CAST);
PlayerFishEvents::EVENT.invoke(&cast);
assert!(cast.cancel.get(), "the cast is cancelled");
let bite = fish(FISHING_BITE);
PlayerFishEvents::EVENT.invoke(&bite);
assert!(!bite.cancel.get(), "the bite is not");
```

## Entity reach and entity events

### `EntityReach::in_box`

`fn in_box(min: [f64; 3], max: [f64; 3]) -> Vec<WorldEntity>`. Every living entity and item entity whose position is inside the box (min corner, max corner), in the host's order; empty outside a scope.

```rust
use pluto_lib::{EntityReach, Identifier, ItemSlot, LivingFacts, MemoryEntityOps, WorldEntity, with_entities};

let mut ops = MemoryEntityOps::default();
ops.entities.push(([1.5, 64.5, 1.5], WorldEntity {
    id: 1,
    kind: Identifier::parse("minecraft:item").unwrap(),
    item: Some(ItemSlot::new(Identifier::parse("minecraft:iron_ingot").unwrap(), 3)),
    living: None,
}));
ops.entities.push(([2.5, 64.5, 2.5], WorldEntity {
    id: 2,
    kind: Identifier::parse("minecraft:zombie").unwrap(),
    item: None,
    living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }),
}));
ops.entities.push(([9.5, 64.5, 9.5], WorldEntity {
    id: 3,
    kind: Identifier::parse("minecraft:cow").unwrap(),
    item: None,
    living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }),
}));
with_entities(&mut ops, || {
    let found = EntityReach::in_box([0.0, 60.0, 0.0], [4.0, 70.0, 4.0]);
    assert_eq!(found.iter().map(|entity| entity.id).collect::<Vec<_>>(), [1, 2], "the cow stands outside the box");
});
```

### `EntityReach::discard`

`fn discard(id: u64) -> bool`. Removes the item entity `id`, which was taken whole; false outside a scope, when there is no such entity, or when it is not an item entity.

```rust
use pluto_lib::{EntityReach, Identifier, ItemSlot, LivingFacts, MemoryEntityOps, WorldEntity, with_entities};

let mut ops = MemoryEntityOps::default();
ops.entities.push(([1.5, 64.5, 1.5], WorldEntity {
    id: 1,
    kind: Identifier::parse("minecraft:item").unwrap(),
    item: Some(ItemSlot::new(Identifier::parse("minecraft:iron_ingot").unwrap(), 3)),
    living: None,
}));
ops.entities.push(([2.5, 64.5, 2.5], WorldEntity {
    id: 2,
    kind: Identifier::parse("minecraft:zombie").unwrap(),
    item: None,
    living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }),
}));
ops.entities.push(([9.5, 64.5, 9.5], WorldEntity {
    id: 3,
    kind: Identifier::parse("minecraft:cow").unwrap(),
    item: None,
    living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }),
}));
with_entities(&mut ops, || {
    assert!(EntityReach::discard(1));
    assert!(!EntityReach::discard(1), "already gone");
    assert!(!EntityReach::discard(2), "a living entity is not discarded");
});
assert_eq!(ops.discarded, vec![1]);
```

### `EntityReach::set_item`

`fn set_item(id: u64, stack: &ItemSlot) -> bool`. Leaves `stack` in the item entity `id` (what did not fit where it was taken to); false outside a scope or where there is no such item entity.

```rust
use pluto_lib::{EntityReach, Identifier, ItemSlot, LivingFacts, MemoryEntityOps, WorldEntity, with_entities};

let mut ops = MemoryEntityOps::default();
ops.entities.push(([1.5, 64.5, 1.5], WorldEntity {
    id: 1,
    kind: Identifier::parse("minecraft:item").unwrap(),
    item: Some(ItemSlot::new(Identifier::parse("minecraft:iron_ingot").unwrap(), 3)),
    living: None,
}));
ops.entities.push(([2.5, 64.5, 2.5], WorldEntity {
    id: 2,
    kind: Identifier::parse("minecraft:zombie").unwrap(),
    item: None,
    living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }),
}));
ops.entities.push(([9.5, 64.5, 9.5], WorldEntity {
    id: 3,
    kind: Identifier::parse("minecraft:cow").unwrap(),
    item: None,
    living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }),
}));
with_entities(&mut ops, || {
    let left = ItemSlot::new(Identifier::parse("minecraft:iron_ingot").unwrap(), 1);
    assert!(EntityReach::set_item(1, &left));
    assert!(!EntityReach::set_item(2, &left), "a living entity holds no item stack");
});
assert_eq!(ops.left.len(), 1);
assert_eq!(ops.left[0].1.count, 1);
```

### `EntityReach::hurt`

`fn hurt(id: u64, amount: f32, damage_type: &str) -> bool`. Hurts entity `id` by `amount` of the damage type `damage_type`, with no attacker; whether it took any. False outside a scope, for an item entity, or when the host refuses an invulnerable, dead or creative target.

```rust
use pluto_lib::{EntityReach, Identifier, ItemSlot, LivingFacts, MemoryEntityOps, WorldEntity, with_entities};

let mut ops = MemoryEntityOps::default();
ops.entities.push(([1.5, 64.5, 1.5], WorldEntity {
    id: 1,
    kind: Identifier::parse("minecraft:item").unwrap(),
    item: Some(ItemSlot::new(Identifier::parse("minecraft:iron_ingot").unwrap(), 3)),
    living: None,
}));
ops.entities.push(([2.5, 64.5, 2.5], WorldEntity {
    id: 2,
    kind: Identifier::parse("minecraft:zombie").unwrap(),
    item: None,
    living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }),
}));
ops.entities.push(([9.5, 64.5, 9.5], WorldEntity {
    id: 3,
    kind: Identifier::parse("minecraft:cow").unwrap(),
    item: None,
    living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }),
}));
with_entities(&mut ops, || {
    assert!(EntityReach::hurt(2, 5.0, "minecraft:in_fire"));
    assert!(!EntityReach::hurt(1, 5.0, "minecraft:in_fire"), "an item entity cannot be hurt");
});
assert_eq!(ops.hurt, vec![(2, 5.0, "minecraft:in_fire".to_owned())]);
```

### `EntityReach::position_of`

`fn position_of(id: u64) -> Option<[f64; 3]>`. Where entity `id` stands (its feet position); `None` outside a scope or when the host cannot reach it.

```rust
use pluto_lib::{EntityReach, Identifier, ItemSlot, LivingFacts, MemoryEntityOps, WorldEntity, with_entities};

let mut ops = MemoryEntityOps::default();
ops.entities.push(([1.5, 64.5, 1.5], WorldEntity {
    id: 1,
    kind: Identifier::parse("minecraft:item").unwrap(),
    item: Some(ItemSlot::new(Identifier::parse("minecraft:iron_ingot").unwrap(), 3)),
    living: None,
}));
ops.entities.push(([2.5, 64.5, 2.5], WorldEntity {
    id: 2,
    kind: Identifier::parse("minecraft:zombie").unwrap(),
    item: None,
    living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }),
}));
ops.entities.push(([9.5, 64.5, 9.5], WorldEntity {
    id: 3,
    kind: Identifier::parse("minecraft:cow").unwrap(),
    item: None,
    living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }),
}));
assert_eq!(EntityReach::position_of(2), None, "outside a scope");
with_entities(&mut ops, || {
    assert_eq!(EntityReach::position_of(2), Some([2.5, 64.5, 2.5]));
    assert_eq!(EntityReach::position_of(99), None, "no such entity");
});
```

### `EntityReach::teleport`

`fn teleport(id: u64, pos: [f64; 3]) -> bool`. Moves entity `id` to `pos`; a mob's navigation stops and its fall distance resets, as the game's own teleport does. False outside a scope, for a player, or when the host cannot move it.

```rust
use pluto_lib::{EntityReach, Identifier, ItemSlot, LivingFacts, MemoryEntityOps, WorldEntity, with_entities};

let mut ops = MemoryEntityOps::default();
ops.entities.push(([1.5, 64.5, 1.5], WorldEntity {
    id: 1,
    kind: Identifier::parse("minecraft:item").unwrap(),
    item: Some(ItemSlot::new(Identifier::parse("minecraft:iron_ingot").unwrap(), 3)),
    living: None,
}));
ops.entities.push(([2.5, 64.5, 2.5], WorldEntity {
    id: 2,
    kind: Identifier::parse("minecraft:zombie").unwrap(),
    item: None,
    living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }),
}));
ops.entities.push(([9.5, 64.5, 9.5], WorldEntity {
    id: 3,
    kind: Identifier::parse("minecraft:cow").unwrap(),
    item: None,
    living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }),
}));
with_entities(&mut ops, || {
    assert!(EntityReach::teleport(2, [7.5, 70.0, -3.5]));
    assert_eq!(EntityReach::position_of(2), Some([7.5, 70.0, -3.5]));
    assert!(!EntityReach::teleport(99, [0.0, 64.0, 0.0]), "no such entity to move");
});
```

### `EntityReach::passengers_of`

`fn passengers_of(vehicle: u64) -> Vec<u64>`. The direct passengers of `vehicle`, in the order they mounted; empty outside a scope, when the entity carries nobody, or when the host cannot say.

```rust
use pluto_lib::{EntityReach, Identifier, ItemSlot, LivingFacts, MemoryEntityOps, WorldEntity, with_entities};

let mut ops = MemoryEntityOps::default();
ops.entities.push(([1.5, 64.5, 1.5], WorldEntity {
    id: 1,
    kind: Identifier::parse("minecraft:item").unwrap(),
    item: Some(ItemSlot::new(Identifier::parse("minecraft:iron_ingot").unwrap(), 3)),
    living: None,
}));
ops.entities.push(([2.5, 64.5, 2.5], WorldEntity {
    id: 2,
    kind: Identifier::parse("minecraft:zombie").unwrap(),
    item: None,
    living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }),
}));
ops.entities.push(([9.5, 64.5, 9.5], WorldEntity {
    id: 3,
    kind: Identifier::parse("minecraft:cow").unwrap(),
    item: None,
    living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }),
}));
with_entities(&mut ops, || {
    assert!(EntityReach::passengers_of(3).is_empty());
    assert!(EntityReach::start_riding(2, 3));
    assert_eq!(EntityReach::passengers_of(3), vec![2]);
});
```

### `EntityReach::vehicle_of`

`fn vehicle_of(passenger: u64) -> Option<u64>`. The vehicle `passenger` rides, if it rides one; `None` outside a scope, when it rides nothing, or when the host cannot say.

```rust
use pluto_lib::{EntityReach, Identifier, ItemSlot, LivingFacts, MemoryEntityOps, WorldEntity, with_entities};

let mut ops = MemoryEntityOps::default();
ops.entities.push(([1.5, 64.5, 1.5], WorldEntity {
    id: 1,
    kind: Identifier::parse("minecraft:item").unwrap(),
    item: Some(ItemSlot::new(Identifier::parse("minecraft:iron_ingot").unwrap(), 3)),
    living: None,
}));
ops.entities.push(([2.5, 64.5, 2.5], WorldEntity {
    id: 2,
    kind: Identifier::parse("minecraft:zombie").unwrap(),
    item: None,
    living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }),
}));
ops.entities.push(([9.5, 64.5, 9.5], WorldEntity {
    id: 3,
    kind: Identifier::parse("minecraft:cow").unwrap(),
    item: None,
    living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }),
}));
with_entities(&mut ops, || {
    assert_eq!(EntityReach::vehicle_of(2), None);
    assert!(EntityReach::start_riding(2, 3));
    assert_eq!(EntityReach::vehicle_of(2), Some(3));
});
```

### `EntityReach::start_riding`

`fn start_riding(passenger: u64, vehicle: u64) -> bool`. Puts `passenger` on `vehicle`; true when the mount was accepted. The host refuses what the game refuses — the same entity twice, a passenger that already rides something, a cycle, and any mount its own mount event cancels — and an accepted mount may only become observable through `vehicle_of` from the next tick. False outside a scope.

```rust
use pluto_lib::{EntityReach, Identifier, ItemSlot, LivingFacts, MemoryEntityOps, WorldEntity, with_entities};

let mut ops = MemoryEntityOps::default();
ops.entities.push(([1.5, 64.5, 1.5], WorldEntity {
    id: 1,
    kind: Identifier::parse("minecraft:item").unwrap(),
    item: Some(ItemSlot::new(Identifier::parse("minecraft:iron_ingot").unwrap(), 3)),
    living: None,
}));
ops.entities.push(([2.5, 64.5, 2.5], WorldEntity {
    id: 2,
    kind: Identifier::parse("minecraft:zombie").unwrap(),
    item: None,
    living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }),
}));
ops.entities.push(([9.5, 64.5, 9.5], WorldEntity {
    id: 3,
    kind: Identifier::parse("minecraft:cow").unwrap(),
    item: None,
    living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }),
}));
with_entities(&mut ops, || {
    assert!(EntityReach::start_riding(2, 3), "the zombie mounts the cow");
    assert!(!EntityReach::start_riding(2, 1), "already riding");
    assert!(!EntityReach::start_riding(3, 3), "an entity cannot ride itself");
    assert!(!EntityReach::start_riding(3, 2), "the cow mounting its own rider would close a cycle");
});
assert_eq!(ops.mounts, vec![(2, 3)]);
```

### `EntityReach::stop_riding`

`fn stop_riding(passenger: u64) -> bool`. Takes `passenger` off whatever it rides; true when it was riding and the dismount was accepted, which the host may likewise apply at its next rider synchronisation. False outside a scope.

```rust
use pluto_lib::{EntityReach, Identifier, ItemSlot, LivingFacts, MemoryEntityOps, WorldEntity, with_entities};

let mut ops = MemoryEntityOps::default();
ops.entities.push(([1.5, 64.5, 1.5], WorldEntity {
    id: 1,
    kind: Identifier::parse("minecraft:item").unwrap(),
    item: Some(ItemSlot::new(Identifier::parse("minecraft:iron_ingot").unwrap(), 3)),
    living: None,
}));
ops.entities.push(([2.5, 64.5, 2.5], WorldEntity {
    id: 2,
    kind: Identifier::parse("minecraft:zombie").unwrap(),
    item: None,
    living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }),
}));
ops.entities.push(([9.5, 64.5, 9.5], WorldEntity {
    id: 3,
    kind: Identifier::parse("minecraft:cow").unwrap(),
    item: None,
    living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }),
}));
with_entities(&mut ops, || {
    assert!(EntityReach::start_riding(2, 3));
    assert!(EntityReach::stop_riding(2));
    assert_eq!(EntityReach::vehicle_of(2), None);
    assert!(!EntityReach::stop_riding(2), "no longer riding");
});
assert!(ops.mounts.is_empty());
```

### `ServerEntityEvents::ENTITY_TICK_PRE`

An entity is about to tick: return `false` from a callback to cancel that entity's tick (the post event is then not heard for it). Heard for every kind the game ticks, derived types included; the entity and world reaches are open while the callbacks run.

```rust
use pluto_lib::{EntityInfo, ServerEntityEvents};

ServerEntityEvents::ENTITY_TICK_PRE.register(|entity: &EntityInfo| entity.kind != "minecraft:item");
let zombie = EntityInfo { id: 9, kind: "minecraft:zombie".into(), dimension: "minecraft:overworld".into() };
let item = EntityInfo { id: 10, kind: "minecraft:item".into(), dimension: "minecraft:overworld".into() };
assert!(ServerEntityEvents::ENTITY_TICK_PRE.invoke(&zombie));
assert!(!ServerEntityEvents::ENTITY_TICK_PRE.invoke(&item));
```

### `ServerEntityEvents::ENTITY_TICK_POST`

An entity has ticked: a notification, heard after the entity's own tick ran, and not at all for a tick that `ENTITY_TICK_PRE` cancelled.

```rust
use pluto_lib::{EntityInfo, ServerEntityEvents};

ServerEntityEvents::ENTITY_TICK_POST.register(|entity: &EntityInfo| {
    let _ = (entity.id, &entity.kind, &entity.dimension);
});
ServerEntityEvents::ENTITY_TICK_POST.invoke(&EntityInfo { id: 9, kind: "minecraft:cow".into(), dimension: "minecraft:overworld".into() });
```

### `CarriedSlot`

One place a player carries a stack, as the wider events that report a player's whole carrying name them: `Equipment(StackSlot)` is a hand or an armor slot, and `Inventory(u32)` is a main-inventory slot, 0 to 35, the hotbar first.

```rust
use pluto_lib::{CarriedSlot, StackSlot};

assert_eq!(CarriedSlot::Equipment(StackSlot::MainHand), CarriedSlot::Equipment(StackSlot::MainHand));
assert_ne!(CarriedSlot::Equipment(StackSlot::Head), CarriedSlot::Inventory(0));
assert_ne!(CarriedSlot::Inventory(0), CarriedSlot::Inventory(35));
```

### `CarriedStack`

A stack a player carries, with the place it sits in: `slot` is where the stack sits and `stack` is the stack itself, with all its components. An event that enumerates a player's stacks reports one of these for each non-empty place.

```rust
use pluto_lib::{CarriedSlot, CarriedStack, Identifier, ItemSlot, StackSlot};

let carried = CarriedStack {
    slot: CarriedSlot::Equipment(StackSlot::MainHand),
    stack: ItemSlot::new(Identifier::parse("minecraft:iron_pickaxe").unwrap(), 1),
};
assert_eq!(carried.slot, CarriedSlot::Equipment(StackSlot::MainHand));
assert_eq!(carried.stack.count, 1);
```

### `CarriedEdit`

A whole new stack for a place a player carries, which a callback queues in an event's edits: `slot` is the place to write and `stack` is the stack to put there. The host writes it after the callbacks ran; a count of 0 empties the place, and writing a place the event did not list is allowed.

```rust
use pluto_lib::{CarriedEdit, CarriedSlot, Identifier, ItemSlot};

let edit = CarriedEdit {
    slot: CarriedSlot::Inventory(0),
    stack: ItemSlot::new(Identifier::parse("minecraft:torch").unwrap(), 0),
};
assert_eq!(edit.slot, CarriedSlot::Inventory(0));
assert_eq!(edit.stack.count, 0, "a count of 0 empties the place");
```

## Living class landings and death drops

### `CauseFallDamageContext`

The landing a class `cause_fall_damage` decides on: `entity` and `entity_type` name the landing entity, `distance` is the fall distance after any living-fall event adjustment, `multiplier` is the damage multiplier the landing carries, `velocity` is the entity's motion as it lands, and `suppressing_bounce` reports a sneaking player (a mob never suppresses).

```rust
use pluto_lib::{CauseFallDamageContext, Identifier};
let context = CauseFallDamageContext { entity: 7, entity_type: Identifier::parse("example:bouncer").unwrap(),
    distance: 5.0, multiplier: 1.0, velocity: [0.0, -1.5, 0.0], suppressing_bounce: false };
assert_eq!(context.distance, 5.0);
assert!(!context.suppressing_bounce);
```

### `FallOutcome`

What a `cause_fall_damage` override does instead of applying fall damage: `Cancel` applies nothing beyond the block's own `fallOn`; `Bounce` rebounds with the given motion, airborne again, playing `sound` at the entity when present.

```rust
use pluto_lib::{FallOutcome, Identifier};
let outcome = FallOutcome::Bounce { velocity: [0.0, 0.9, 0.0], sound: Some(Identifier::parse("example:bounce").unwrap()) };
assert!(matches!(outcome, FallOutcome::Bounce { .. }));
assert_eq!(FallOutcome::Cancel, FallOutcome::Cancel);
```

### `EquipmentDropChanceContext`

The equipment drop chance one class computes for one slot at death: `entity_type` is the dying type, `slot` is the equipment slot name, `slime_size` is present only for native cube/slime implementations, and `chance` is the recorded chance (or the game's default for the slot) the class may replace for this death's roll only.

```rust
use pluto_lib::{EquipmentDropChanceContext, Identifier};
let context = EquipmentDropChanceContext { entity_type: Identifier::parse("example:armored").unwrap(),
    slot: "head".into(), slime_size: Some(4), chance: 2.0 };
assert_eq!(context.chance, 2.0);
assert_eq!(context.slot, "head");
```

### `LivingEntityBehavior::cause_fall_damage`

Registers the class's own `causeFallDamage`: returning an outcome replaces the inherited fall-damage application for the landing; returning `None` keeps it.

```rust
use pluto_lib::{FallOutcome, Identifier, LivingEntityBehavior};
let behavior = LivingEntityBehavior::new(Identifier::parse("example:bouncer").unwrap())
    .cause_fall_damage(|context| (context.distance > 2.0).then_some(FallOutcome::Cancel));
let _ = behavior;
```

### `LivingEntityBehavior::equipment_drop_chance`

Registers the class's own per-slot equipment drop chance at death; returning `None` keeps the recorded chance for that slot.

```rust
use pluto_lib::{Identifier, LivingEntityBehavior};
let behavior = LivingEntityBehavior::new(Identifier::parse("example:armored").unwrap())
    .equipment_drop_chance(|context| (context.chance > 0.25).then_some(0.25));
let _ = behavior;
```

### `LivingEntityBehaviors::cause_fall_damage`

Dispatches the registered `cause_fall_damage` override for the context's type; `None` keeps the inherited application.

```rust
use pluto_lib::{CauseFallDamageContext, Identifier, LivingEntityBehaviors};
let context = CauseFallDamageContext { entity: 1, entity_type: Identifier::parse("example:unregistered").unwrap(),
    distance: 9.0, multiplier: 1.0, velocity: [0.0; 3], suppressing_bounce: false };
assert_eq!(LivingEntityBehaviors::cause_fall_damage(&context), None);
```

### `LivingEntityBehaviors::equipment_drop_chance`

Dispatches the registered per-slot drop chance for the context's type; `None` keeps the recorded chance.

```rust
use pluto_lib::{EquipmentDropChanceContext, Identifier, LivingEntityBehaviors};
let context = EquipmentDropChanceContext { entity_type: Identifier::parse("example:unregistered").unwrap(),
    slot: "head".into(), slime_size: None, chance: 0.085 };
assert_eq!(LivingEntityBehaviors::equipment_drop_chance(&context), None);
```

## Mob splits

### `MobSplitContext`

The dying parent and the split the game is about to perform: `parent` is the parent entity, `position` its position, `size` its size and `child_size` the children's size (half of it).

```rust
use pluto_lib::{EntityInfo, MobSplitContext};
let context = MobSplitContext { parent: EntityInfo { id: 7, kind: "example:parent".into(), dimension: "minecraft:overworld".into() },
    position: [1.0, 64.0, 2.0], size: 4, child_size: 2 };
assert_eq!(context.child_size, 2);
assert_eq!(context.parent.id, 7);
```

### `MobSplitRandom`

Random draws for inheritance decisions, from the parent's own stream.

```rust
use pluto_lib::MobSplitRandom;
struct Fixed;
impl MobSplitRandom for Fixed {
    fn next_float(&mut self) -> f32 { 0.25 }
    fn next_int(&mut self, bound: i32) -> Result<i32, String> { Ok(bound - 1) }
}
let mut random = Fixed;
assert_eq!(random.next_float(), 0.25);
assert_eq!(random.next_int(3), Ok(2));
```

### `MobSplitChild`

One spawned child, dressed before it joins the world; its `EntityInfo` carries the child's real id, so attachment writes land on the child.

```rust
use pluto_lib::{EntityInfo, MobSplitChild, StackData};
struct Child { info: EntityInfo }
impl MobSplitChild for Child {
    fn info(&self) -> EntityInfo { self.info.clone() }
    fn set_equipment(&mut self, _: &str, _: StackData) -> Result<(), String> { Ok(()) }
    fn set_custom_name(&mut self, _: Option<String>) -> Result<(), String> { Ok(()) }
    fn set_no_ai(&mut self, _: bool) -> Result<(), String> { Ok(()) }
    fn set_invulnerable(&mut self, _: bool) -> Result<(), String> { Ok(()) }
}
let child = Child { info: EntityInfo { id: 8, kind: "example:parent".into(), dimension: "minecraft:overworld".into() } };
assert_eq!(child.info().id, 8);
```

### `MobSplitInheritance`

The inheritance view handed to split callbacks after the game spawned the children: `context` is the split, `head` is the parent's head stack (children receive copies), `head_drop_chance` is the parent's recorded head drop chance, `custom_name`, `no_ai` and `invulnerable` are the parent's flags, `random` draws from the parent's stream, and `children` are the spawned children in the game's spawn order.

```rust
use pluto_lib::{EntityInfo, MobSplitChild, MobSplitContext, MobSplitInheritance, MobSplitRandom, StackData};
struct Fixed;
impl MobSplitRandom for Fixed {
    fn next_float(&mut self) -> f32 { 0.5 }
    fn next_int(&mut self, bound: i32) -> Result<i32, String> { Ok(bound - 1) }
}
struct Child { info: EntityInfo }
impl MobSplitChild for Child {
    fn info(&self) -> EntityInfo { self.info.clone() }
    fn set_equipment(&mut self, _: &str, _: StackData) -> Result<(), String> { Ok(()) }
    fn set_custom_name(&mut self, _: Option<String>) -> Result<(), String> { Ok(()) }
    fn set_no_ai(&mut self, _: bool) -> Result<(), String> { Ok(()) }
    fn set_invulnerable(&mut self, _: bool) -> Result<(), String> { Ok(()) }
}
let context = MobSplitContext { parent: EntityInfo { id: 7, kind: "example:parent".into(), dimension: "minecraft:overworld".into() },
    position: [1.0, 64.0, 2.0], size: 4, child_size: 2 };
let mut random = Fixed;
let mut children: Vec<Box<dyn MobSplitChild>> = vec![Box::new(Child { info: EntityInfo { id: 8, kind: "example:parent".into(), dimension: "minecraft:overworld".into() } })];
let view = MobSplitInheritance { context: &context, head: None, head_drop_chance: 0.085,
    custom_name: None, no_ai: false, invulnerable: false, random: &mut random, children: &mut children };
assert_eq!(view.children.len(), 1);
assert_eq!(view.head_drop_chance, 0.085);
```

### `MobSplitCallback`

A split observer with two phases, both defaulting to keeping the game's split: `prepare` tunes the child count or cancels the split; `inherit` dresses the spawned children.

```rust
use pluto_lib::MobSplitCallback;
struct Observer;
impl MobSplitCallback for Observer {}
let _observer = Observer;
```

### `MobSplitCallbacks`

The registration-order split dispatch: preparation callbacks run in order before the game spawns, inheritance callbacks in order after.

```rust
use pluto_lib::MobSplitCallbacks;
let _registry = MobSplitCallbacks;
```

### `MobSplitCallbacks::register`

Registers a split callback under a unique name; duplicate names reject.

```rust
use pluto_lib::{MobSplitCallback, MobSplitCallbacks};
struct Observer;
impl MobSplitCallback for Observer {}
MobSplitCallbacks::register("example:observer", Observer).unwrap();
assert!(MobSplitCallbacks::has_callbacks());
```

### `MobSplitCallbacks::has_callbacks`

Whether any split callback is registered; the native split hooks stay inert while none is.

```rust
use pluto_lib::MobSplitCallbacks;
let active: bool = MobSplitCallbacks::has_callbacks();
let _ = active;
```

## Spawn placements

### `SpawnPlacementContext`

The facts of one candidate spawn position: `entity_type` and `dimension` name the spawn, `position` is the block position of the mob's feet, `reason` is the backend reason, `peaceful` reports a peaceful level, `block_below` is the block below the candidate, `fluid_at` and `fluid_below` are the fluids at and below it (`minecraft:empty` when none), `block_below_valid_spawn` says the block below is a valid spawn block for the type's base, and `dark_enough` says a monster would be dark enough to spawn there.

```rust
use pluto_lib::{Identifier, SpawnPlacementContext};
let context = SpawnPlacementContext { entity_type: Identifier::parse("example:slime").unwrap(),
    dimension: Identifier::parse("minecraft:overworld").unwrap(), position: [0, 64, 0], reason: "natural".into(),
    peaceful: false, block_below: Identifier::parse("example:grass").unwrap(),
    fluid_at: Identifier::parse("minecraft:empty").unwrap(), fluid_below: Identifier::parse("minecraft:empty").unwrap(),
    block_below_valid_spawn: true, dark_enough: true };
assert_eq!(context.position[1], 64);
```

### `SpawnPlacementRegistry`

Per-type spawn placement registration: one predicate per type, whose verdict replaces the inherited species rules for that type where the game consults placements.

```rust
use pluto_lib::SpawnPlacementRegistry;
let _registry = SpawnPlacementRegistry;
```

### `SpawnPlacementRegistry::register`

Registers a type's placement predicate; a second registration for the same type rejects.

```rust
use pluto_lib::{Identifier, SpawnPlacementRegistry};
SpawnPlacementRegistry::register(Identifier::parse("example:slime").unwrap(), |context| {
    !context.peaceful && context.block_below == Identifier::parse("example:grass").unwrap()
}).unwrap();
assert!(SpawnPlacementRegistry::is_registered(&Identifier::parse("example:slime").unwrap()));
```

### `SpawnPlacementRegistry::is_registered`

Whether a placement predicate is registered for the type.

```rust
use pluto_lib::{Identifier, SpawnPlacementRegistry};
let registered: bool = SpawnPlacementRegistry::is_registered(&Identifier::parse("example:absent").unwrap());
let _ = registered;
```

### `FishingRegistry`

The registry of fishing rods a mod casts with: which items count as rods (a hook stays alive only while its owner holds one — the game's own check names the vanilla rod, and the transport asks this registry for the rest), and how a rod stack's cast profile is computed. The game's fishing is built around its own hook, whose rod use is gated to the vanilla item; a registered rod's use goes through [`Fishing::use_rod`](#fishing) instead.

```rust
use pluto_lib::{FishingProfile, FishingRegistry, Grapple, Identifier, StackData};

FishingRegistry::register(Identifier::parse("example:war_rod").unwrap(), |stack| {
    (stack.count > 0).then_some(FishingProfile { power: 2.0, grapple: Grapple::Drill, ..FishingProfile::default() })
});
assert!(FishingRegistry::is_rod("example:war_rod"));
let stack = StackData { item: "example:war_rod".into(), count: 1, ..StackData::default() };
assert_eq!(FishingRegistry::profile(&stack).map(|profile| profile.power), Some(2.0));
```

### `FishingProfile`

The numbers a rod casts with, computed from the stack used: the `luck` and `lure` the hook fishes with, the `velocity` it flies at, the cast's `inaccuracy` spread, the `power` a hooked entity is hurt by (0 fishes without fighting), the `knockback` bonus on the retrieve pull, whether the hook `grapple`s and collects what it touches.

```rust
use pluto_lib::{FishingProfile, Grapple};

let plain = FishingProfile::default();
assert_eq!((plain.velocity, plain.power, plain.grapple), (1.0, 0.0, Grapple::None));
let war = FishingProfile { luck: 2, lure: 1, power: 2.0, grapple: Grapple::Dash, collecting: true, ..plain };
assert_eq!((war.luck, war.lure, war.collecting), (2, 1, true));
```

### `Grapple`

How a grapple hook behaves: `None` only fishes and fights, `Dash` pulls its owner toward the hook when it lands, and `Drill` also pulls when it hooks an entity.

```rust
use pluto_lib::Grapple;

assert_eq!(Grapple::default(), Grapple::None);
assert_ne!(Grapple::Dash, Grapple::Drill);
```

### `Fishing`

Fishing with a mod's rod: the calls a mod makes. `use_rod` queues a use — the game's fishing tick casts with the profile when no hook of this rod is out and retrieves when one is — and `take_wear` drains the rod wear the game side reports (a cast's and a hit's 1, a retrieve's cost), which the mod pays into its own tool at its next use, so the wear lands at most one use late. The reach is the local player's, as the game's fishing simulation is single-hook and local.

```rust
use pluto_lib::{Fishing, FishingProfile, ServerPlayer};

let player = ServerPlayer { uuid: 7, name: "Alex".into() };
Fishing::use_rod(&player, FishingProfile::default());
assert_eq!(Fishing::take_use(), Some((7, FishingProfile::default())));
Fishing::note_wear(7, 3);
assert_eq!(Fishing::take_wear(&player), 3);
assert_eq!(Fishing::take_wear(&player), 0);
```

### `combat_damage`

The damage a hooked entity takes at retrieve: the impact speed times the power times 15, rounded up to a whole tenth.

```rust
use pluto_lib::combat_damage;

assert_eq!(combat_damage(2.0, 2.0), 6.0);
assert_eq!(combat_damage(0.5, 1.0), 0.8);
assert_eq!(combat_damage(3.0, 0.0), 0.0);
```

### `pull_delta`

The velocity a hooked entity gains at retrieve: toward the owner, a tenth of the way, plus the knockback bonus `0.25 * knockback / distance` when the rod has knockback.

```rust
use pluto_lib::pull_delta;

assert_eq!(pull_delta([0.0, 64.0, 0.0], [10.0, 64.0, 0.0], 0.0), [-1.0, 0.0, 0.0]);
assert_eq!(pull_delta([0.0, 64.0, 0.0], [10.0, 64.0, 0.0], 4.0), [-2.0, 0.0, 0.0]);
```

### `grapple_delta`

The velocity the owner gains from a grapple: toward the hook, scaled by `0.58 / sqrt(distance)`.

```rust
use pluto_lib::grapple_delta;

let pull = grapple_delta([0.0, 64.0, 0.0], [16.0, 64.0, 0.0]);
assert!((pull[0] - 2.32).abs() < 1e-9);
assert_eq!(grapple_delta([0.0, 64.0, 0.0], [0.0, 64.0, 0.0]), [0.0; 3]);
```

### `retrieve_damage`

The rod wear a retrieve costs, over the game's own retrieve number: a grounded hook costs at least 2 and at most 3; anything else costs at most 3.

```rust
use pluto_lib::retrieve_damage;

assert_eq!(retrieve_damage(5, true), 3);
assert_eq!(retrieve_damage(0, true), 2);
assert_eq!(retrieve_damage(5, false), 3);
assert_eq!(retrieve_damage(1, false), 1);
```

### `DispenseRegistry::register`

Registers the behaviour for an item, replacing any earlier one for it.

```rust
use pluto_lib::{DispenseBehavior, DispenseRegistry, Identifier};

DispenseRegistry::register(
    Identifier::parse("example:ball").unwrap(),
    DispenseBehavior::Shoot { entity: Identifier::parse("example:ball").unwrap(), power: 1.1, inaccuracy: 6.0 },
);
assert!(DispenseRegistry::get("example:ball").is_some());
```

### `DispenseRegistry::get`

The behaviour registered for the item named, if any.

```rust
use pluto_lib::DispenseRegistry;

assert!(DispenseRegistry::get("example:never_registered").is_none());
```

### `FishingRegistry::register`

Registers a rod item and its cast profile computation: when the item is used, the computation reads the stack and answers the profile to cast with, or None to leave the use to the game.

```rust
use pluto_lib::{FishingProfile, FishingRegistry, Identifier};

FishingRegistry::register(Identifier::parse("example:rod").unwrap(), |_| Some(FishingProfile::default()));
assert!(FishingRegistry::is_rod("example:rod"));
```

### `FishingRegistry::register_rod`

Names an item a fishing rod for the hook's lifetime check, without a cast computation of its own.

```rust
use pluto_lib::{FishingRegistry, Identifier};

FishingRegistry::register_rod(Identifier::parse("example:plain_rod").unwrap());
assert!(FishingRegistry::is_rod("example:plain_rod"));
```

### `FishingRegistry::is_rod`

Whether the item named is a registered rod.

```rust
use pluto_lib::FishingRegistry;

assert!(!FishingRegistry::is_rod("example:not_a_rod"));
```

### `FishingRegistry::profile`

The profile a stack casts with, when its item registered a computation and the computation answers one.

```rust
use pluto_lib::{FishingRegistry, StackData};

assert_eq!(FishingRegistry::profile(&StackData::default()), None);
```

### `Fishing::use_rod`

Queues a use of a fishing rod by the player with the profile: the game's fishing tick casts with it when no hook of this rod is out, and retrieves when one is.

```rust
use pluto_lib::{Fishing, FishingProfile, ServerPlayer};

let player = ServerPlayer { uuid: 3, name: "Sam".into() };
Fishing::use_rod(&player, FishingProfile::default());
assert!(Fishing::take_use().is_some());
```

### `Fishing::take_use`

Takes the queued use, if any; the transport calls this.

```rust
use pluto_lib::Fishing;

assert_eq!(Fishing::take_use(), None);
```

### `Fishing::note_wear`

Records rod wear the game side reports; the transport calls this.

```rust
use pluto_lib::{Fishing, ServerPlayer};

Fishing::note_wear(5, 2);
let player = ServerPlayer { uuid: 5, name: "Sam".into() };
assert_eq!(Fishing::take_wear(&player), 2);
```

### `Fishing::take_wear`

The rod wear waiting for the player, drained: the total the game side has reported since the last call.

```rust
use pluto_lib::{Fishing, ServerPlayer};

let player = ServerPlayer { uuid: 6, name: "Sam".into() };
assert_eq!(Fishing::take_wear(&player), 0);
```
