# Players: messages, items, effects and food

A `ServerPlayer` is a plain handle (uuid and name). Its methods reach the game only while a PlutoLib callback that carries a player is running (a join, a command). Called anywhere else, a method does nothing and says so: it returns `false` or `0`, or hands back everything it was asked to give. A stored handle can therefore never touch the game from the wrong place.

To test code that uses a handle, run it inside `with_ops` with a `MemoryPlayerOps`.

## Basic Example

Send a message and give an item to a player from a callback that carries one:

```rust
use pluto_lib::{with_ops, MemoryPlayerOps, ServerPlayer};

fn welcome(player: &ServerPlayer) {
    player.send_message("Welcome!");
    let left = player.give_item("minecraft:bread", 3);
    assert_eq!(left, 0, "everything fitted");
}

// the page uses an in-memory game; in a real callback the game is already there
let player = ServerPlayer { uuid: 1, name: "Alex".into() };
let mut game = MemoryPlayerOps::default();
with_ops(&mut game, || welcome(&player));
assert_eq!(game.told.len(), 1);
```

## Outside a Callback

A `ServerPlayer` is plain data. Its methods reach the game only while a callback that carries a player runs, so you can keep a handle without any risk of using it at the wrong time. Outside, they do nothing and say so: `send_message` returns `false`, `give_item` returns the whole count, the counters return 0:

```rust
use pluto_lib::ServerPlayer;

let stored = ServerPlayer { uuid: 1, name: "Alex".into() };
assert!(!stored.send_message("late"));
assert_eq!(stored.give_item("minecraft:stick", 4), 4);
```

## Taking Items Safely

Check before you consume: a command that needs a flint, two sticks and room for three parts should look first and change nothing if any is missing:

```rust
use pluto_lib::{with_ops, MemoryPlayerOps, ServerPlayer};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
let mut game = MemoryPlayerOps { free_slots: 5, ..Default::default() };
with_ops(&mut game, || {
    player.give_item("minecraft:flint", 1);
    player.give_item("minecraft:stick", 2);
    let can = player.count_item("minecraft:flint") >= 1 && player.count_item("minecraft:stick") >= 2 && player.empty_slots() >= 3;
    assert!(can);
    player.take_item("minecraft:flint", 1);
    player.take_item("minecraft:stick", 2);
});
```

## Effects and Food

A status effect is an `EffectInstance`: which effect, how strong, how long. `add_effect` gives one to the player, `effects` lists what the player has and `remove_effect` takes one off. A food that heals or protects (a cake, a stew) is an item use that gives an effect.

```rust
use pluto_lib::{with_ops, EffectInstance, MemoryPlayerOps, ServerPlayer};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
let mut game = MemoryPlayerOps::default();
with_ops(&mut game, || {
    // ten seconds of Speed II
    assert!(player.add_effect(&EffectInstance::new("minecraft:speed", 200).amplifier(1)));
    assert_eq!(player.effects().len(), 1);
    assert!(player.remove_effect("minecraft:speed"));
});
```

> **Tip.** A duration is in ticks: 20 are one second. `-1` never ends.

> **Use with Caution.** `add_food` is a documented stub: Horizon lets a mod read a player's food level but not change it, so `add_food` does nothing and returns `false` until it does. Give an effect such as `minecraft:saturation` instead.

## Testing

`MemoryPlayerOps` is a small game that keeps messages, items and attachments in plain collections. Run your code inside `with_ops` and look at what it recorded.

> **Tip.** Set `free_slots` to a small number to test a full inventory, and `language` to test a message in another language.

## Reference

### `ServerPlayer`

A connected player. `uuid` is the player's UUID as a number, `name` the player's name.

```rust
use pluto_lib::ServerPlayer;

let player = ServerPlayer { uuid: 0x1234, name: "Alex".to_owned() };
assert_eq!(player.name, "Alex");
assert_eq!(player.uuid, 0x1234);
```

### `ServerPlayer::send_message`

`fn send_message(&self, text: &str) -> bool`. Sends a system chat line to the player. False if the player is gone or the call is made outside a callback that carries the player.

```rust
use pluto_lib::{with_ops, MemoryPlayerOps, ServerPlayer};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
assert!(!player.send_message("too early")); // no running game: nothing happens

let mut game = MemoryPlayerOps::default();
with_ops(&mut game, || assert!(player.send_message("Welcome!")));
assert_eq!(game.told, [(1, "Welcome!".to_owned())]);
```

### `ServerPlayer::give_item`

`fn give_item(&self, item: &str, count: u32) -> u32`. Gives `count` of `item` and returns how many did **not** fit (0 means all were given). Outside a callback it returns `count`.

```rust
use pluto_lib::{with_ops, MemoryPlayerOps, ServerPlayer};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
assert_eq!(player.give_item("minecraft:bread", 3), 3); // outside: nothing given, all 3 returned

let mut game = MemoryPlayerOps::default();
with_ops(&mut game, || assert_eq!(player.give_item("minecraft:bread", 3), 0));
assert_eq!(game.items[&(1, "minecraft:bread".to_owned())], 3);
```

### `ServerPlayer::count_item`

`fn count_item(&self, item: &str) -> u32`. How many of `item` the player carries in the main inventory. 0 outside a callback.

```rust
use pluto_lib::{with_ops, MemoryPlayerOps, ServerPlayer};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
let mut game = MemoryPlayerOps::default();
with_ops(&mut game, || {
    player.give_item("minecraft:stick", 4);
    assert_eq!(player.count_item("minecraft:stick"), 4);
    assert_eq!(player.count_item("minecraft:flint"), 0);
});
```

### `ServerPlayer::take_item`

`fn take_item(&self, item: &str, count: u32) -> u32`. Takes up to `count` of `item` from the player and returns how many were taken. 0 outside a callback.

```rust
use pluto_lib::{with_ops, MemoryPlayerOps, ServerPlayer};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
let mut game = MemoryPlayerOps::default();
with_ops(&mut game, || {
    player.give_item("minecraft:stick", 2);
    assert_eq!(player.take_item("minecraft:stick", 5), 2); // only two were there
    assert_eq!(player.count_item("minecraft:stick"), 0);
});
```

### `ServerPlayer::language`

`fn language(&self) -> String`. The language option of the player's client (`en_us`, `ja_jp`, ...); `en_us` outside a callback that carries this player. Use it to write a message in the player's language with `I18n::translate_in` (see [i18n](i18n.md)).

```rust
use pluto_lib::{with_ops, MemoryPlayerOps, ServerPlayer};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
assert_eq!(player.language(), "en_us");
let mut game = MemoryPlayerOps { language: "ja_jp".to_owned(), ..Default::default() };
with_ops(&mut game, || assert_eq!(player.language(), "ja_jp"));
```

### `EffectInstance`

A status effect on a player. Fields: `id: String` (such as `minecraft:speed`), `amplifier: u8` (0 is level I), `duration: i32` (ticks; -1 never ends), `ambient: bool` (beacon-style) and `particles: bool`.

```rust
use pluto_lib::EffectInstance;

let plain = EffectInstance::new("minecraft:regeneration", 100);
assert_eq!(plain.amplifier, 0);
assert!(plain.particles && !plain.ambient);
let custom = EffectInstance { ambient: true, ..EffectInstance::new("minecraft:haste", -1) };
assert_eq!(custom.duration, -1);
```

### `EffectInstance::new`

`fn new(id: &str, duration: i32) -> EffectInstance`. An effect of level I lasting `duration` ticks, with particles.

```rust
use pluto_lib::EffectInstance;

let effect = EffectInstance::new("minecraft:night_vision", 600);
assert_eq!(effect.id, "minecraft:night_vision");
assert_eq!(effect.duration, 600);
```

### `EffectInstance::amplifier`

`fn amplifier(self, amplifier: u8) -> EffectInstance`. The same effect at another level (0 is level I).

```rust
use pluto_lib::EffectInstance;

assert_eq!(EffectInstance::new("minecraft:strength", 100).amplifier(2).amplifier, 2);
```

### `EffectInstance::without_particles`

`fn without_particles(self) -> EffectInstance`. The same effect without particles.

```rust
use pluto_lib::EffectInstance;

assert!(!EffectInstance::new("minecraft:speed", 100).without_particles().particles);
```

### `FoodStatus`

A player's hunger. Fields: `level: i32` (0 to 20), `saturation: f32` and `exhaustion: f32`.

```rust
use pluto_lib::FoodStatus;

let hungry = FoodStatus { level: 6, saturation: 0.0, exhaustion: 1.5 };
assert!(hungry.level < 20);
```

### `ServerPlayer::add_effect`

`fn add_effect(&self, effect: &EffectInstance) -> bool`. Gives the player a status effect; it replaces a weaker or shorter one of the same kind. False if nothing changed (an unknown effect, a duration of 0, a dead player) or outside a callback that carries the player.

```rust
use pluto_lib::{with_ops, EffectInstance, MemoryPlayerOps, ServerPlayer};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
let effect = EffectInstance::new("minecraft:speed", 200);
assert!(!player.add_effect(&effect)); // no running game
let mut game = MemoryPlayerOps::default();
with_ops(&mut game, || {
    assert!(player.add_effect(&effect));
    assert!(!player.add_effect(&EffectInstance::new("minecraft:speed", 0))); // nothing to give
});
```

### `ServerPlayer::remove_effect`

`fn remove_effect(&self, id: &str) -> bool`. Takes the effect `id` off the player; true if they had it.

```rust
use pluto_lib::{with_ops, EffectInstance, MemoryPlayerOps, ServerPlayer};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
let mut game = MemoryPlayerOps::default();
with_ops(&mut game, || {
    player.add_effect(&EffectInstance::new("minecraft:speed", 200));
    assert!(player.remove_effect("minecraft:speed"));
    assert!(!player.remove_effect("minecraft:speed")); // already gone
});
```

### `ServerPlayer::effects`

`fn effects(&self) -> Vec<EffectInstance>`. The status effects the player has; empty outside a callback that carries the player.

```rust
use pluto_lib::{with_ops, EffectInstance, MemoryPlayerOps, ServerPlayer};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
assert!(player.effects().is_empty());
let mut game = MemoryPlayerOps::default();
with_ops(&mut game, || {
    player.add_effect(&EffectInstance::new("minecraft:haste", 100));
    assert_eq!(player.effects()[0].id, "minecraft:haste");
});
```

### `ServerPlayer::food`

`fn food(&self) -> Option<FoodStatus>`. The player's hunger; None outside a callback that carries the player.

```rust
use pluto_lib::{with_ops, FoodStatus, MemoryPlayerOps, ServerPlayer};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
assert_eq!(player.food(), None);
let hungry = FoodStatus { level: 4, saturation: 0.0, exhaustion: 0.0 };
let mut game = MemoryPlayerOps { food: Some(hungry), ..Default::default() };
with_ops(&mut game, || assert_eq!(player.food().unwrap().level, 4));
```

### `ServerPlayer::add_food`

`fn add_food(&self, nutrition: i32, saturation: f32) -> bool`. Stub: Horizon offers no way to change a player's food level yet, so this does nothing and returns `false`, inside a callback too.

```rust
use pluto_lib::{with_ops, MemoryPlayerOps, ServerPlayer};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
let mut game = MemoryPlayerOps::default();
with_ops(&mut game, || assert!(!player.add_food(4, 0.6)));
```

### `ServerPlayer::empty_slots`

`fn empty_slots(&self) -> u32`. How many of the 36 main inventory slots are empty. Check it before an action that needs room, so nothing is consumed and then lost. 0 outside a callback.

```rust
use pluto_lib::{with_ops, MemoryPlayerOps, ServerPlayer};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
let mut game = MemoryPlayerOps { free_slots: 2, ..Default::default() };
with_ops(&mut game, || {
    let enough_room = player.empty_slots() >= 3;
    assert!(!enough_room); // only two free slots: do not start
});
```

## For hosts and tests

### `PlayerOps`

What the game can do for a player. PlutoLib implements it over Horizon; a test can implement it (or use `MemoryPlayerOps`). Methods: `tell(player, line) -> bool`, `give(player, item, count) -> u32` (the number that did not fit), `attachment_get(player, id) -> Option<String>`, `attachment_set(player, id, json) -> bool`, `count_item(player, item) -> u32`, `take_item(player, item, count) -> u32`, `empty_slots(player) -> u32`, `language(player) -> Option<String>`, `add_effect(player, &EffectInstance) -> bool`, `remove_effect(player, id) -> bool`, `effects(player) -> Option<Vec<EffectInstance>>`, `food(player) -> Option<FoodStatus>`.

```rust
use pluto_lib::{with_ops, EffectInstance, FoodStatus, PlayerOps, ServerPlayer};

/// A game where nobody can receive anything.
struct FullWorld;
impl PlayerOps for FullWorld {
    fn tell(&mut self, _: u128, _: &str) -> bool { false }
    fn give(&mut self, _: u128, _: &str, count: u32) -> u32 { count }
    fn attachment_get(&mut self, _: u128, _: &str) -> Option<String> { None }
    fn attachment_set(&mut self, _: u128, _: &str, _: &str) -> bool { false }
    fn count_item(&mut self, _: u128, _: &str) -> u32 { 0 }
    fn take_item(&mut self, _: u128, _: &str, _: u32) -> u32 { 0 }
    fn empty_slots(&mut self, _: u128) -> u32 { 0 }
    fn language(&mut self, _: u128) -> Option<String> { None }
    fn add_effect(&mut self, _: u128, _: &EffectInstance) -> bool { false }
    fn remove_effect(&mut self, _: u128, _: &str) -> bool { false }
    fn effects(&mut self, _: u128) -> Option<Vec<EffectInstance>> { None }
    fn food(&mut self, _: u128) -> Option<FoodStatus> { None }
}

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
with_ops(&mut FullWorld, || assert_eq!(player.give_item("minecraft:bread", 2), 2));
```

### `with_ops`

`fn with_ops<R>(ops: &mut dyn PlayerOps, f: impl FnOnce() -> R) -> R`. Runs `f` with `ops` available to every `ServerPlayer` method called on this thread during `f`. The previous scope is restored when `f` returns, also when it panics.

```rust
use pluto_lib::{with_ops, MemoryPlayerOps, ServerPlayer};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
let mut outer = MemoryPlayerOps::default();
let mut inner = MemoryPlayerOps::default();
with_ops(&mut outer, || {
    with_ops(&mut inner, || { player.send_message("inner"); });
    player.send_message("outer");
});
assert_eq!(inner.told.len(), 1);
assert_eq!(outer.told.len(), 1);
assert!(!player.send_message("after")); // the scope is closed
```

### `MemoryPlayerOps`

An in-memory game for tests. `told` records every message sent, `items` holds each player's items by `(player, item id)`, `attachments` holds attachment JSON by `(player, attachment id)`, `free_slots` is how many main slots are empty for every player and `language` the language option of every player's client (empty means `en_us`), `effects` the status effects by `(player, effect id)` and `food` the hunger every player has (None means full).

```rust
use pluto_lib::{with_ops, MemoryPlayerOps, ServerPlayer};

let player = ServerPlayer { uuid: 7, name: "Sam".into() };
let mut game = MemoryPlayerOps { free_slots: 36, ..Default::default() };
with_ops(&mut game, || {
    player.send_message("hi");
    player.give_item("minecraft:apple", 1);
});
assert_eq!(game.told.len(), 1);
assert_eq!(game.items.len(), 1);
assert!(game.attachments.is_empty());
assert_eq!(game.free_slots, 36);
```
