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

> **Use with Caution.** `add_food` uses vanilla `FoodData.eat` arithmetic: its second argument is a saturation modifier, not a number of saturation points. It preserves exhaustion and caps saturation at the resulting food level. Non-finite inputs/calculations fail without changing the player; outside a player callback it returns `false`.

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

### `ServerPlayer::count_in_tag`

`fn count_in_tag(&self, tag: &str) -> u32`. How many items of the item tag `tag` the player carries in the main inventory. 0 outside a callback.

```rust
use pluto_lib::{with_ops, MemoryPlayerOps, ServerPlayer};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
let mut game = MemoryPlayerOps::default();
game.item_tags.insert("minecraft:planks".into(), vec!["minecraft:oak_planks".into(), "minecraft:birch_planks".into()]);
with_ops(&mut game, || {
    player.give_item("minecraft:oak_planks", 3);
    player.give_item("minecraft:birch_planks", 2);
    assert_eq!(player.count_in_tag("minecraft:planks"), 5);
});
```

### `ServerPlayer::take_in_tag`

`fn take_in_tag(&self, tag: &str, count: u32) -> u32`. Takes up to `count` items of the item tag `tag` and returns how many were taken. 0 outside a callback.

```rust
use pluto_lib::{with_ops, MemoryPlayerOps, ServerPlayer};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
let mut game = MemoryPlayerOps::default();
game.item_tags.insert("minecraft:planks".into(), vec!["minecraft:oak_planks".into(), "minecraft:birch_planks".into()]);
with_ops(&mut game, || {
    player.give_item("minecraft:oak_planks", 3);
    player.give_item("minecraft:birch_planks", 2);
    assert_eq!(player.take_in_tag("minecraft:planks", 4), 4);
    assert_eq!(player.count_in_tag("minecraft:planks"), 1);
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

A status effect on a player. Fields: `id: String` (such as `minecraft:speed`), `amplifier: u8` (0 is level I), `duration: i32` (ticks; -1 never ends), `ambient: bool` (beacon-style), `particles: bool` and `show_icon: bool` (whether its icon shows in the GUI; the read's flag, published by PlutoLib's effect-icon reach at the head of every HUD draw — giving an effect always shows its icon, as the host's add path carries no flag).

```rust
use pluto_lib::EffectInstance;

let plain = EffectInstance::new("minecraft:regeneration", 100);
assert_eq!(plain.amplifier, 0);
assert!(plain.particles && !plain.ambient && plain.show_icon);
let custom = EffectInstance { ambient: true, ..EffectInstance::new("minecraft:haste", -1) };
assert_eq!(custom.duration, -1);
```

### `EffectInstance::new`

`fn new(id: &str, duration: i32) -> EffectInstance`. An effect of level I lasting `duration` ticks, with particles and its icon showing.

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

`fn add_food(&self, nutrition: i32, saturation: f32) -> bool`. Add food as vanilla `FoodData.eat`: add `nutrition` to the food level and clamp to 0–20; add `nutrition * saturation * 2` to saturation and clamp to 0–the new food level. Exhaustion is unchanged. Returns `false` for an unavailable player, outside a player scope, an unsupported backend, or a non-finite calculation, without writing anything.

```rust
use pluto_lib::{with_ops, FoodStatus, MemoryPlayerOps, ServerPlayer};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
let mut game = MemoryPlayerOps { food: Some(FoodStatus { level: 12, saturation: 2.0, exhaustion: 3.0 }), ..Default::default() };
with_ops(&mut game, || {
    assert!(player.add_food(4, 0.5));
    assert_eq!(player.food(), Some(FoodStatus { level: 16, saturation: 6.0, exhaustion: 3.0 }));
    assert!(!player.add_food(4, f32::NAN));
});
assert!(!player.add_food(4, 0.5));
```

> **Use with Caution.** As in vanilla, finite negative values can reduce food/saturation, and integer addition follows Java's wrapping arithmetic before clamping. This is not a direct setter. The native adapter uses the pinned host's existing player-status setter; no extra Mixin is needed.

For custom `PlayerOps` backends:

`fn add_food(&mut self, player: u128, nutrition: i32, saturation: f32) -> bool`. Backend operation for the same feeding semantics. `player` is the UUID; `saturation` is a modifier. The default implementation returns `false`, keeping existing custom backends compatible. `MemoryPlayerOps` implements it for tests.

```rust
use pluto_lib::{FoodStatus, MemoryPlayerOps, PlayerOps};
let mut backend = MemoryPlayerOps { food: Some(FoodStatus { level: 10, saturation: 0.0, exhaustion: 1.0 }), ..Default::default() };
assert!(backend.add_food(1, 2, 0.5));
assert_eq!(backend.food(1).unwrap().saturation, 2.0);
```

### `ServerPlayer::swing_hand`

`fn swing_hand(&self, hand: Hand) -> bool`. Swings the player's arm — the place animation of a mod that places a block for them, the attack animation of one that hits. True when the swing was queued; it plays at the acting player's turn (inside their break, or at their next tick). False outside a callback.

```rust
use pluto_lib::{Hand, ServerPlayer};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
assert!(!player.swing_hand(Hand::Off)); // no running game: nothing happens
```

### `ServerPlayer::broadcast_item_break`

`fn broadcast_item_break(&self, slot: StackSlot, stack: &StackData) -> bool`. Broadcasts that the stack in `slot` broke — the break sound and crumbs where the player stands, and the item-break event for the mods listening. Use it only when the mod itself wore the stack to breaking (a `damage_slot` that returned `broke`); the host's own wear already broadcasts, so calling it there would sound the break twice. True when the broadcast was queued. False outside a callback.

```rust
use pluto_lib::{NbtCompound, ServerPlayer, StackData, StackSlot};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
let stack = StackData { item: "minecraft:iron_pickaxe".into(), count: 1, damage: 250, max_damage: Some(250), custom_data: NbtCompound::new(), custom_model_data: None, can_place_on: None, enchantments: Default::default() };
assert!(!player.broadcast_item_break(StackSlot::MainHand, &stack)); // no running game: nothing happens
```

### `PlayerItemBreakEvents`

`PlayerItemBreakEvents::EVENT` hears a player's stack break: the player, the item id, and the slot it broke in. The host reports it when its own wear breaks a stack; a mod that wears a stack itself reports it by calling `ServerPlayer::broadcast_item_break`, which is also what plays the sound and crumbs. There is no body or saddle slot on a player, so those never arrive.

```rust
use pluto_lib::{ItemBreak, PlayerItemBreakEvents, StackSlot};

PlayerItemBreakEvents::EVENT.register(|broken: &ItemBreak| {
    if broken.slot == StackSlot::MainHand {
        let _player = &broken.player;
    }
});
PlayerItemBreakEvents::EVENT.invoke(&ItemBreak { player: pluto_lib::ServerPlayer { uuid: 1, name: "Alex".into() }, item: "minecraft:iron_pickaxe".into(), slot: StackSlot::MainHand });
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

### `StackData`

An item stack as a mod sees it. `item` is the item id, `count` how many, `damage` the damage taken (0 for a new item), `max_damage` the most damage this stack takes before it breaks when that differs from the item's (None for the item's own; a tool whose durability depends on its materials sets it), and `custom_data` the compound a mod keeps on the stack (Minecraft: the `minecraft:custom_data` component, which the stack keeps when saved and sent, and which `/give` can set too). A tool mod keeps its materials and stats here. `custom_model_data` is the stack's `minecraft:custom_model_data` component (see `CustomModelData`), None for none: an item model definition selects its sprite from it, and the game reads and writes it with the stack. `can_place_on` is the stack's `minecraft:can_place_on` component (the adventure-mode placement predicates: a list of block predicates) as NBT, None for none: the game reads it when an adventure-mode player places with the stack, and a fluid effect that places blocks reads it for its place gate.

```rust
use pluto_lib::{Nbt, StackData};

let tool = StackData { item: "example:pickaxe".into(), count: 1, damage: 0, max_damage: Some(60), custom_data: [("durability".to_owned(), Nbt::Int(60))].into(), custom_model_data: None, can_place_on: None, enchantments: Default::default() };
assert_eq!(tool.custom_data["durability"], Nbt::Int(60));
assert_eq!((tool.count, tool.damage), (1, 0));
```

### `CustomModelData`

What an item model can select on (Minecraft: `minecraft:custom_model_data`). `strings` are matched by a `select` in the item definition (`index` picks the entry), `floats` feed a `range_dispatch`, `flags` a `condition` and `colors` (0xRRGGBB) a tint source. Set it on a `StackData` or `ItemSlot` and the game stores it as the component, so a mod can give each stack its own sprite, such as one per material, without a model per combination. The game cannot look inside `custom_data`, so a sprite choice must be copied here. Keep it in step with the data it comes from every time that data changes.

```rust
use pluto_lib::{CustomModelData, StackData};

let head = StackData {
    item: "example:pick_head".into(), count: 1,
    custom_model_data: Some(CustomModelData { strings: vec!["wood".into()], ..Default::default() }),
    ..Default::default()
};
assert_eq!(head.custom_model_data.unwrap().strings[0], "wood");
```

### `CustomModelData::with_strings`

`fn with_strings(strings) -> CustomModelData`. Data with only `strings`.

```rust
use pluto_lib::CustomModelData;

assert_eq!(CustomModelData::with_strings(["wood", ""]).strings, ["wood", ""]);
```

### `CustomModelData::is_empty`

`fn is_empty(&self) -> bool`. True when all four lists are empty; empty data is not stored on a stack.

```rust
use pluto_lib::CustomModelData;

assert!(CustomModelData::default().is_empty());
assert!(!CustomModelData::with_strings(["wood"]).is_empty());
```

### `CustomModelData::to_nbt`

`fn to_nbt(&self) -> NbtCompound`. The saved form of the component: `strings`, `floats`, `flags` (bytes 0 or 1) and `colors` lists, each left out when empty. The host adapters write this as `minecraft:custom_model_data`.

```rust
use pluto_lib::{CustomModelData, Nbt};

let saved = CustomModelData::with_strings(["wood"]).to_nbt();
assert_eq!(saved["strings"], Nbt::List(vec![Nbt::String("wood".into())]));
assert!(!saved.contains_key("floats"));
```

### `CustomModelData::from_nbt`

`fn from_nbt(nbt: &NbtCompound) -> CustomModelData`. Reads the saved form back, also accepting int and byte arrays, any non-zero number as a flag and three floats from 0 to 1 as a colour. Entries of another type are skipped.

```rust
use pluto_lib::CustomModelData;

let data = CustomModelData { strings: vec!["wood".into()], flags: vec![true], ..Default::default() };
assert_eq!(CustomModelData::from_nbt(&data.to_nbt()), data);
```

### `ServerPlayer::stack`

`fn stack(&self, slot: i32) -> Option<StackData>`. The stack in main inventory slot `slot` (0 to 35, the hotbar first) with its custom data; None for an empty slot, and outside a callback that carries the player.

```rust
use pluto_lib::{with_ops, MemoryPlayerOps, Nbt, ServerPlayer, StackData};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
let mut game = MemoryPlayerOps::default();
let tool = StackData { item: "example:pickaxe".into(), count: 1, damage: 3, max_damage: None, custom_data: [("level".to_owned(), Nbt::Int(2))].into(), custom_model_data: None, can_place_on: None, enchantments: Default::default() };
game.stacks.insert(1, vec![Some(tool)]);
with_ops(&mut game, || {
    assert_eq!(player.stack(0).unwrap().custom_data["level"], Nbt::Int(2));
    assert!(player.stack(1).is_none());
});
```

### `ServerPlayer::give_stack`

`fn give_stack(&self, stack: StackData) -> Option<StackData>`. Gives the player a stack with its damage, max damage and custom data, such as a tool a mod has just made. Returns what did not fit (None if all of it did); outside a callback that carries the player, the whole stack comes back.

```rust
use pluto_lib::{with_ops, MemoryPlayerOps, Nbt, ServerPlayer, StackData};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
let mut game = MemoryPlayerOps::default();
let tool = StackData { item: "example:pickaxe".into(), count: 1, damage: 0, max_damage: Some(60), custom_data: [("level".to_owned(), Nbt::Int(1))].into(), custom_model_data: None, can_place_on: None, enchantments: Default::default() };
with_ops(&mut game, || {
    assert_eq!(player.give_stack(tool), None);
    assert_eq!(player.stack(0).unwrap().max_damage, Some(60));
});
```

### `ServerPlayer::update_stack`

`fn update_stack(&self, slot: i32, new: &StackData) -> bool`. Writes count, damage and custom data of the stack in `slot` back, if it still holds the item `new.item`. A count of 0 empties the slot. True if it was written; false for an empty slot, another item, a slot that does not exist, or outside a callback.

```rust
use pluto_lib::{with_ops, MemoryPlayerOps, ServerPlayer, StackData};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
let mut game = MemoryPlayerOps::default();
game.stacks.insert(1, vec![Some(StackData { item: "example:pickaxe".into(), count: 1, damage: 0, ..Default::default() })]);
with_ops(&mut game, || {
    let mut tool = player.stack(0).unwrap();
    tool.damage += 1; // one use
    assert!(player.update_stack(0, &tool));
    assert_eq!(player.stack(0).unwrap().damage, 1);
});
```

### `ServerPlayer::set_stack`

`fn set_stack(&self, slot: i32, stack: Option<&StackData>) -> bool`. Sets the stack carried in a slot named by the carried numbering (feet 100, legs 101, chest 102, head 103, offhand -106), creating it in an empty slot, or clearing it with `None`. Unlike `update_stack`, this creates: the host's carried read names occupied slots only, but its equipment write (`LivingEntity.setItemSlot`) creates and clears, which is what an item that equips a copy of itself (a piggyback pack on the chest) needs. The host answers only for the equipment slots; any other slot, and outside a callback, is false. The memory backend keeps no such restriction.

```rust
use pluto_lib::{with_ops, MemoryPlayerOps, ServerPlayer, StackData};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
let mut game = MemoryPlayerOps::default();
let pack = StackData { item: "example:pack".into(), count: 1, ..Default::default() };
with_ops(&mut game, || {
    assert!(player.set_stack(102, Some(&pack)), "the empty chest slot takes the pack");
    assert!(player.set_stack(102, None), "and gives it back");
});
assert!(game.carried.is_empty());
```

### `ServerPlayer::set_equipment`

`fn set_equipment(&self, slot: StackSlot, stack: Option<&StackData>) -> bool`. The `set_stack` form that names the slot as a `StackSlot` (from `entity_events_v1`): feet, legs, chest, head and offhand map to their carried numbers. The main hand has no equipment write and answers false; use `update_stack` there.

```rust
use pluto_lib::{with_ops, MemoryPlayerOps, ServerPlayer, StackData, StackSlot};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
let mut game = MemoryPlayerOps::default();
let pack = StackData { item: "example:pack".into(), count: 2, ..Default::default() };
with_ops(&mut game, || {
    assert!(player.set_equipment(StackSlot::Chest, Some(&pack)));
    assert!(!player.set_equipment(StackSlot::MainHand, Some(&pack)));
});
assert_eq!(game.carried.get(&(1, 102)).map(|stack| stack.count), Some(2));
```

### `PlayerOps`

What the game can do for a player. PlutoLib implements it over Horizon; a test can implement it (or use `MemoryPlayerOps`). Methods: `tell(player, line) -> bool`, `give(player, item, count) -> u32` (the number that did not fit), `attachment_get(player, id) -> Option<String>`, `attachment_set(player, id, json) -> bool`, `count_item(player, item) -> u32`, `take_item(player, item, count) -> u32`, `empty_slots(player) -> u32`, `language(player) -> Option<String>`, `add_effect(player, &EffectInstance) -> bool`, `remove_effect(player, id) -> bool`, `effects(player) -> Option<Vec<EffectInstance>>`, `food(player) -> Option<FoodStatus>`, `is_item_on_cooldown(player, item) -> bool`, `start_item_cooldown(player, item, ticks)`, and, with defaults that do nothing, `stack(player, slot) -> Option<StackData>`, `give_stack(player, StackData) -> Option<StackData>` (what did not fit; the default gives nothing), `update_stack(player, slot, expected_item, &StackData) -> bool` and `set_stack(player, slot, Option<&StackData>) -> bool` (creates in an empty carried slot; the default sets nothing), `release_use(player) -> bool` and `stop_use(player) -> bool` (the release and the plain end of the player's item use in progress; the defaults end nothing), and `attribute_value(player, attribute) -> Option<f64>` (the value of one of the player's host attributes with every modifier on it; None when the game cannot answer).

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

**Policy-aware attachment methods on `PlayerOps`:**

**`attachment_get_with_policy`**

`fn attachment_get_with_policy(&mut self, player: u128, id: &str, sync: bool) -> Option<String>` reads existing player JSON with the declared sync policy. The default delegates unsynced access to `attachment_get`; synchronized access is unsupported unless overridden.

**`attachment_set_with_policy`**

`fn attachment_set_with_policy(&mut self, player: u128, id: &str, json: &str, sync: bool) -> bool` stores player JSON with the declared policy. The default refuses synchronized writes.

**`entity_attachment_get`**

`fn entity_attachment_get(&mut self, entity: u32, dimension: &str, id: &str, sync: bool) -> Option<String>` reads current-world entity JSON; the default is unavailable. Native forwarding rejects the wrong host dimension and delegates missing-holder/UUID rules to the host.

**`entity_attachment_set`**

`fn entity_attachment_set(&mut self, entity: u32, dimension: &str, id: &str, json: &str, sync: bool) -> bool` stores entity JSON; the default refuses. This backend-level example exercises all four policy-aware methods without claiming native transport:

```rust
use pluto_lib::{MemoryPlayerOps, PlayerOps};
let mut game = MemoryPlayerOps::default();
assert!(game.attachment_set_with_policy(1, "example:player_state", "true", true));
assert_eq!(game.attachment_get_with_policy(1, "example:player_state", true).as_deref(), Some("true"));
assert!(game.entity_attachment_set(7, "minecraft:overworld", "example:entity_state", "3", true));
assert_eq!(game.entity_attachment_get(7, "minecraft:overworld", "example:entity_state", true).as_deref(), Some("3"));
assert!(!game.entity_attachment_set(7, "minecraft:overworld", "example:entity_state", "4", false));
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

`entity_attachments` stores JSON by `(dimension, callback-local entity ID, attachment ID)`; tests simulating holder removal/ID reuse must clear the old holder themselves. `attachment_policies` records the immutable sync flag per attachment ID across holder kinds. These collections test routing and values, not native UUID ownership, save files or network delivery.

An in-memory game for tests. `told` records every message sent, `items` holds each player's items by `(player, item id)`, `item_tags` the members of each item tag, `attachments` holds attachment JSON by `(player, attachment id)`, `free_slots` is how many main slots are empty for every player and `language` the language option of every player's client (empty means `en_us`), `effects` the status effects by `(player, effect id)`, `food` the hunger every player has (None means full) and `stacks` the main inventory slots of each player, `carried` the stacks the stack operations set by `(player, carried slot)`, `selected_slot` the selected hotbar slot of every player (0 to 8), and `cooldowns` the item cooldowns started for each player by `(player, item id)` to ticks left (the memory backend does not tick them down), `released_uses` every player whose use a `release_use` ended and `stopped_uses` every player whose use a `stop_use` ended, both in order, and `attributes` the host attribute values set for a player by `(player, attribute)`. `opened_item_menus` records every item menu a callback opened, in the order they opened.

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


### `StackData::enchantment_level`

`fn enchantment_level(&self, id: &str) -> i32`. The level of enchantment `id` on this stack, from its own `minecraft:enchantments` component; 0 when it has none. Enchantments the item itself grants through its data are not listed here.

```rust
use pluto_lib::StackData;

let mut stack = StackData { item: "minecraft:iron_pickaxe".into(), count: 1, ..Default::default() };
stack.enchantments.insert("minecraft:fortune".into(), 3);
assert_eq!(stack.enchantment_level("minecraft:fortune"), 3);
assert_eq!(stack.enchantment_level("minecraft:efficiency"), 0);
```

### `ServerPlayer::selected_slot`

`fn selected_slot(&self) -> Option<i32>`. The hotbar slot the player has selected (0 to 8); None outside a callback that carries this player.

```rust
use pluto_lib::{with_ops, MemoryPlayerOps, ServerPlayer};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
assert_eq!(player.selected_slot(), None);
let mut game = MemoryPlayerOps { selected_slot: 3, ..Default::default() };
with_ops(&mut game, || assert_eq!(player.selected_slot(), Some(3)));
```

### `ServerPlayer::is_item_on_cooldown`

`fn is_item_on_cooldown(&self, item: &str) -> bool`. Whether `item` is on the player's item cooldown (the item's own `use_cooldown` component, else the item's default). False for an item with no cooldown, or outside a callback that carries this player.

```rust
use pluto_lib::{with_ops, MemoryPlayerOps, ServerPlayer};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
assert!(!player.is_item_on_cooldown("minecraft:ender_pearl"));
let mut game = MemoryPlayerOps::default();
with_ops(&mut game, || {
    player.start_item_cooldown("minecraft:ender_pearl", 10);
    assert!(player.is_item_on_cooldown("minecraft:ender_pearl"));
});
```

### `ServerPlayer::start_item_cooldown`

`fn start_item_cooldown(&self, item: &str, ticks: i32)`. Starts the player's cooldown for `item` for `ticks` ticks (the item's own `use_cooldown` component, else the item's default, names the group). Nothing happens outside a callback that carries this player, or for an item with no cooldown.

```rust
use pluto_lib::{with_ops, MemoryPlayerOps, ServerPlayer};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
let mut game = MemoryPlayerOps::default();
with_ops(&mut game, || player.start_item_cooldown("minecraft:ender_pearl", 10));
assert_eq!(game.cooldowns[&(1, "minecraft:ender_pearl".to_owned())], 10);
```

### `ServerPlayer::main_hand_stack`

`fn main_hand_stack(&self) -> Option<StackData>`. The stack in the player's selected hotbar slot (their main hand); None for an empty slot or outside a callback that carries this player.

```rust
use pluto_lib::{with_ops, MemoryPlayerOps, ServerPlayer, StackData};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
assert!(player.main_hand_stack().is_none());
let mut game = MemoryPlayerOps { selected_slot: 1, ..Default::default() };
game.stacks.insert(1, vec![None, Some(StackData { item: "minecraft:torch".into(), count: 12, ..Default::default() })]);
with_ops(&mut game, || {
    let held = player.main_hand_stack().unwrap();
    assert_eq!((held.item.as_str(), held.count), ("minecraft:torch", 12));
});
```

### `ServerPlayer::release_use`

`fn release_use(&self) -> bool`. Ends the player's item use in progress as `LivingEntity.releaseUsingItem` does: the use's own release runs first (a callback's Stop may skip that part, never the end itself), then the use is over — what letting go of the use button does, asked for from inside a callback such as a use tick. The game applies the end at its next tick seam, so a use that already ended is untouched, and an end ordered during one use never lands on a later use of a mod's. False outside a callback that carries the player.

```rust
use pluto_lib::{with_ops, MemoryPlayerOps, ServerPlayer};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
assert!(!player.release_use()); // outside a callback
let mut game = MemoryPlayerOps::default();
with_ops(&mut game, || assert!(player.release_use()));
assert_eq!(game.released_uses, vec![1]);
```

### `ServerPlayer::stop_use`

`fn stop_use(&self) -> bool`. Ends the player's item use in progress as `LivingEntity.stopUsingItem` does: the use is simply over, with no release run — what an item breaking in the hand does to its use. Applied at the game's next tick seam like `release_use`. False outside a callback that carries the player.

```rust
use pluto_lib::{with_ops, MemoryPlayerOps, ServerPlayer};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
assert!(!player.stop_use()); // outside a callback
let mut game = MemoryPlayerOps::default();
with_ops(&mut game, || assert!(player.stop_use()));
assert_eq!(game.stopped_uses, vec![1]);
```

### `ServerPlayer::attribute_value`

`fn attribute_value(&self, attribute: &str) -> Option<f64>`. The value of the host attribute `attribute` for this player with every modifier on it (armour, effects, `/attribute`), as `LivingEntity.getAttributeValue` answers it: `minecraft:block_interaction_range` is the range a use that aims traces over, `minecraft:max_health` the health cap. None outside a callback that carries the player, when the attribute is not one players have, or when the player is not reached or between bodies (just respawned, not yet ticked) — the caller falls back to the attribute's base value then, as the game's own range reads do.

```rust
use pluto_lib::{with_ops, MemoryPlayerOps, ServerPlayer};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
assert_eq!(player.attribute_value("minecraft:block_interaction_range"), None); // outside a callback
let mut game = MemoryPlayerOps::default();
game.attributes.insert((1, "minecraft:block_interaction_range".into()), 6.0);
with_ops(&mut game, || assert_eq!(player.attribute_value("minecraft:block_interaction_range"), Some(6.0)));
```

### `KeyPress`

A key of the player's own keyboard or mouse changed state, heard on the client for the local player only. `player` is the local player (None when the game names none), `key` is the game's own name for the key (the names key bindings use, such as `key.keyboard.left.shift` or `key.mouse.left`) and `pressed` says which way it moved: true when the key went down, false when it came up. Repeats are not reported.

```rust
use pluto_lib::{KeyPress, ServerPlayer};

let press = KeyPress { player: Some(ServerPlayer { uuid: 1, name: "Alex".into() }), key: "key.keyboard.left.shift".into(), pressed: true };
assert_eq!(press.key, "key.keyboard.left.shift");
assert!(press.pressed && press.player.is_some());
let release = KeyPress { player: None, key: "key.mouse.left".into(), pressed: false };
assert!(!release.pressed && release.player.is_none());
```

### `PlayerKeyEvents`

Key presses of the local player (see `KeyPress`). No instance constructor; use `EVENT`.

```rust
use pluto_lib::{KeyPress, PlayerKeyEvents};

PlayerKeyEvents::EVENT.register(|press: &KeyPress| {
    let _ = (&press.key, press.pressed);
});
```

### `PlayerKeyEvents::EVENT`

The event for a local player's key press: every callback runs, in registration order. A callback that needs the server to act sends its own packet with the networking API, as a client key press can only ever be a hint to the server.

```rust
use pluto_lib::{KeyPress, PlayerKeyEvents};

PlayerKeyEvents::EVENT.register(|press: &KeyPress| {
    if press.key == "key.keyboard.r" && press.pressed {
        let _player = &press.player;
    }
});
PlayerKeyEvents::EVENT.invoke(&KeyPress { player: None, key: "key.keyboard.r".into(), pressed: true });
```

### `InventoryTick`

One inventory slot of a player ticked, heard on a server once per tick for each carried slot the game ticks. `player` is the player carrying the slot (None when the game names none), `slot` is the slot's own number in the player's inventory in the game's numbering, `selected` says whether the slot is the selected hotbar slot, and `stack` is the stack in the slot, with its damage, max damage, custom data and enchantments (None for an empty slot, or a stack the game did not read).

```rust
use pluto_lib::{InventoryTick, ServerPlayer, StackData};

let tick = InventoryTick {
    player: Some(ServerPlayer { uuid: 1, name: "Alex".into() }),
    slot: 3,
    selected: true,
    stack: Some(StackData { item: "minecraft:torch".into(), count: 12, ..Default::default() }),
};
assert_eq!(tick.slot, 3);
assert!(tick.selected);
assert_eq!(tick.stack.unwrap().count, 12);
let empty = InventoryTick { player: None, slot: 100, selected: false, stack: None };
assert!(empty.stack.is_none() && !empty.selected);
```

### `PlayerInventoryEvents`

Inventory slot ticks of players (see `InventoryTick`). No instance constructor; use `TICK`.

```rust
use pluto_lib::{InventoryTick, PlayerInventoryEvents};

PlayerInventoryEvents::TICK.register(|tick: &InventoryTick| {
    let _ = (tick.slot, tick.selected);
});
```

### `PlayerInventoryEvents::TICK`

The event for a player's inventory slot tick: every callback runs, in registration order. Callbacks that need to change the world act through the scopes the host opens around the event (the entity and world reaches).

```rust
use pluto_lib::{InventoryTick, PlayerInventoryEvents, StackData};

PlayerInventoryEvents::TICK.register(|tick: &InventoryTick| {
    if tick.selected {
        let _item = tick.stack.as_ref().map(|stack| stack.item.as_str());
    }
});
PlayerInventoryEvents::TICK.invoke(&InventoryTick {
    player: None, slot: 3, selected: true,
    stack: Some(StackData { item: "minecraft:torch".into(), count: 12, ..Default::default() }),
});
```

### `aim_of`

Where the player `uuid` is and looks as of their last tick, for a keybind shot to fire with: the dimension, their feet position, the direction they look and whether they are in creative mode. `None` when no tick has heard them.

```rust
use pluto_lib::entity_events_v1::aim_of;

assert!(aim_of(0).is_none());
```

### `PlayerAim`

Where a player is and where they look, as of their last tick: `dimension` the dimension they are in, `position` their feet position, `look` the direction they look (a unit vector) and `creative` whether they are in creative mode.

```rust
use pluto_lib::entity_events_v1::PlayerAim;

let aim = PlayerAim { dimension: "minecraft:overworld".into(), position: [0.0, 64.0, 0.0], look: [0.0, 0.0, 1.0], creative: false };
assert_eq!((aim.dimension.as_str(), aim.position[1], aim.look[2], aim.creative), ("minecraft:overworld", 64.0, 1.0, false));
```

### `PlayerTick`

A server player's tick: `player` is the uuid, `entity` the entity id, `dimension` the dimension the player is in, `position` their feet position, `look` the direction the player faces (a unit vector), `creative` whether they are in creative mode, `game_mode` their game mode as the game numbers it (0 survival, 1 creative, 2 adventure, 3 spectator), `on_ground` whether they stand on the ground, `sneaking` whether they hold sneak, `gliding` whether they glide, `spectator` whether they spectate, `in_water` whether their feet block holds water, `on_climbable` whether on a climbable block (a gliding player passing a block it can glide through never counts), `armor` the pieces worn that carry custom data (as (slot, stack)), `hands` the stacks held that carry custom data (as (slot, stack): `mainhand`, `offhand`), `block_light`, `sky_light` and `sky_darken` the light where the player stands, sampled at their position raised by half a block, `effects` the player's effects as (effect id, amplifier, ticks left), `remove_effects` the ids callbacks push to take off, `attribute_additions` the (attribute id, amount) pairs callbacks push to add to the player's attribute base values for as long as they push them (amounts for one attribute add up; one no callback pushes any more is taken off again), and `actions` the [`EntityAction`](#entityaction)s callbacks push to be done to the ticking player (as `Attacker`; `Victim` is no one) at tick's end: a player the host can name by entity id is reached, any other is skipped. The tick is heard once more after the last piece with custom data is put away, so an equipment-tied amount is let go, and while the player carries a mob effect, so an effect's per-tick behaviour runs even with no gear. The host gives no transient modifier call, so the amount moves the saved base value: exact for an addition, taken off again when the player leaves (best effort), and a crash while it is applied leaves it in the save.

```rust
use pluto_lib::PlayerTick;
use std::cell::RefCell;

let tick = PlayerTick { player: 1, entity: 4, dimension: "minecraft:overworld".into(), position: [0.0; 3], look: [0.0, 0.0, 1.0], creative: false, game_mode: 0, on_ground: true, sneaking: false, gliding: false, spectator: false, in_water: false, on_climbable: false, armor: Vec::new(), hands: Vec::new(), block_light: 0, sky_light: 15, sky_darken: 0, effects: vec![("minecraft:poison".into(), 0, 100)], remove_effects: RefCell::new(Vec::new()), attribute_additions: RefCell::new(Vec::new()), actions: RefCell::new(Vec::new()) };
tick.remove_effects.borrow_mut().push("minecraft:poison".into());
tick.attribute_additions.borrow_mut().push(("minecraft:knockback_resistance".into(), 0.1));
assert_eq!((tick.effects.len(), tick.remove_effects.borrow().len()), (1, 1));
```

### `MovementInput`

The local player's movement input, after the game read it and before it applies it (client only). `entity` is the local player's entity id, or -1 when the game names no acting player; `forward` and `strafe` are the impulses now (each a `Cell`: change it with `set`); `jump` and `sneak` whether the keys are held (each a `Cell`); `using_item` whether the player is using an item; `using` the stack being used, the held (main-hand) stack, `None` when nothing is used or no player is named — an off-hand use reads as the main hand's stack, as the native hook names no hand. A callback may scale the impulses (an item's use slowdown) or read the jump key state (a double jump); it may not reach the player themselves, so a motion change is deferred to the player's next tick.

```rust
use pluto_lib::MovementInput;
use std::cell::Cell;

let input = MovementInput { entity: 4, forward: Cell::new(1.0), strafe: Cell::new(0.4), jump: Cell::new(false), sneak: Cell::new(false), using_item: true, using: None };
input.forward.set(input.forward.get() * 0.5);
assert_eq!((input.forward.get(), input.using.is_none()), (0.5, true));
```

### `ServerLivingEntityEvents::PLAYER_TICK`

A server player's tick, heard while they wear armor or hold a tool carrying custom data (a mod's own armor keeps its state there). It carries the armor, the held stacks, the light where the player stands and the effects on the player; take effects off them with `remove_effects`. It is how a mod grants immunity to an effect the game gives no call to refuse: the effect is gone by the next tick.

```rust
use pluto_lib::ServerLivingEntityEvents;

ServerLivingEntityEvents::PLAYER_TICK.register(|tick| {
    if tick.armor.iter().any(|(_, piece)| piece.item == "example:antidote_helmet") && tick.effects.iter().any(|(id, _, _)| id == "minecraft:poison") {
        tick.remove_effects.borrow_mut().push("minecraft:poison".into());
    }
});
```

### `ServerPlayer::stack_named`

`fn stack_named(&self, slot: i32) -> Option<StackData>`. The stack the player carries in `slot` (the carried numbering, as in `set_stack`), with its custom data; None for an empty slot or outside a callback. Unlike `stack`, this names the player, so it answers in a packet receiver too, where the player's own turn is not open.

```rust
use pluto_lib::{MemoryPlayerOps, Nbt, ServerPlayer, StackData, with_ops};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
let mut game = MemoryPlayerOps::default();
let tool = StackData { item: "example:pickaxe".into(), count: 1, damage: 3, max_damage: None, custom_data: [("level".to_owned(), Nbt::Int(2))].into(), custom_model_data: None, can_place_on: None, enchantments: Default::default() };
game.stacks.insert(1, vec![Some(tool)]);
with_ops(&mut game, || {
    assert_eq!(player.stack_named(0).unwrap().custom_data["level"], Nbt::Int(2));
    assert!(player.stack_named(1).is_none());
});
```

### `ServerPlayer::open_item_menu`

`fn open_item_menu(&self, menu: &str, title: &str, items: Vec<Option<ItemSlot>>, extra: Vec<u8>) -> Option<i32>`. Opens a mod menu for the player's item (a content menu type, such as a tool's container) with `items` in its slots and `extra` as its menu-extra data, titled `title`; the window id, or None outside a callback, for an unknown player or menu type, or inside the player's own turn (the original opened such a menu only from a command or a packet handler).

```rust
use pluto_lib::{MemoryPlayerOps, ServerPlayer, with_ops};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
let mut game = MemoryPlayerOps::default();
with_ops(&mut game, || {
    assert_eq!(player.open_item_menu("tconstruct:tool_container_3", "Tool", Vec::new(), Vec::new()), Some(1));
});
assert_eq!(game.opened_item_menus.len(), 1);
assert_eq!(game.opened_item_menus[0].menu, "tconstruct:tool_container_3");
assert_eq!(game.opened_item_menus[0].player, 1);
```

### `ItemBreak`

A player's item broke (NeoForge's `PlayerDestroyItemEvent`), heard through `PlayerItemBreakEvents::EVENT` when the game's own wear breaks a player's stack: `player` is the player whose item broke, `item` the item that broke, and `slot` the equipment slot it broke in.

```rust
use pluto_lib::{ItemBreak, ServerPlayer, StackSlot};

let broken = ItemBreak { player: ServerPlayer { uuid: 1, name: "Alex".into() }, item: "tconstruct:pickaxe".into(), slot: StackSlot::MainHand };
assert_eq!(broken.item, "tconstruct:pickaxe");
assert_eq!(broken.slot, StackSlot::MainHand);
```

### `PlayerItemBreakEvents::EVENT`

The `ItemBreak` event: every callback runs, in registration order.

```rust
use pluto_lib::{ItemBreak, PlayerItemBreakEvents, ServerPlayer, StackSlot};

PlayerItemBreakEvents::EVENT.register(|broken| println!("{} broke", broken.item));
PlayerItemBreakEvents::EVENT.invoke(&ItemBreak { player: ServerPlayer { uuid: 1, name: "Alex".into() }, item: "tconstruct:pickaxe".into(), slot: StackSlot::MainHand });
```

### `OpenedItemMenu`

An item menu opened through the memory backend, as `MemoryPlayerOps::opened_item_menus` records it: `player` is the player it opened for, `menu` the menu type, `title` the title shown, `items` the items in its slots, in order, and `extra` its menu-extra data.

```rust
use pluto_lib::OpenedItemMenu;

let opened = OpenedItemMenu { player: 7, menu: "tconstruct:tool_container_3".into(), title: "Tool".into(), items: Vec::new(), extra: Vec::new() };
assert_eq!(opened.player, 7);
assert_eq!(opened.menu, "tconstruct:tool_container_3");
```
