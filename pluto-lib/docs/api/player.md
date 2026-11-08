# Players: messages and items

A `ServerPlayer` is a plain handle (uuid and name). Its methods reach the game only while a PlutoLib callback that carries a player is running (a join, a command). Called anywhere else, a method does nothing and says so: it returns `false` or `0`, or hands back everything it was asked to give. A stored handle can therefore never touch the game from the wrong place.

To test code that uses a handle, run it inside `with_ops` with a `MemoryPlayerOps`.

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

What the game can do for a player. PlutoLib implements it over Horizon; a test can implement it (or use `MemoryPlayerOps`). Methods: `tell(player, line) -> bool`, `give(player, item, count) -> u32` (the number that did not fit), `attachment_get(player, id) -> Option<String>`, `attachment_set(player, id, json) -> bool`, `count_item(player, item) -> u32`, `take_item(player, item, count) -> u32`, `empty_slots(player) -> u32`.

```rust
use pluto_lib::{with_ops, PlayerOps, ServerPlayer};

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

An in-memory game for tests. `told` records every message sent, `items` holds each player's items by `(player, item id)`, `attachments` holds attachment JSON by `(player, attachment id)`, and `free_slots` is how many main slots are empty for every player.

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
