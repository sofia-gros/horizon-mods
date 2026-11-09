# Block entities and NBT

A block entity is data that belongs to a block position: it is saved with the chunk, ticked every game tick on the server and can be sent to clients. The block itself (its model, its recipes) comes from the mod's content; here the block gets its block entity type.

## Basic Example

A block entity gives a block a place to keep data that is saved with the chunk and updated every tick. This oven burns down a fuel counter once a tick:

```rust
use pluto_lib::{BlockEntityType, BlockEntityTypeRegistry, Identifier, Nbt, NbtCompound, NbtCompoundExt};

let mut start = NbtCompound::new();
start.insert("fuel".to_owned(), Nbt::Int(200));
BlockEntityTypeRegistry::register(
    BlockEntityType::new(Identifier::new("example", "oven").unwrap())
        .initial_data(start)
        .ticker(|tick| {
            let fuel = tick.data.get_int("fuel", 0);
            if fuel > 0 {
                tick.data.insert("fuel".to_owned(), Nbt::Int(fuel as i32 - 1));
            }
        }),
);
```

The block itself (model, name, recipes) comes from your mod's content; this only gives it a block entity.

## Saved Data

`tick.data` is an `NbtCompound`, the tagged data the game saves. Read it with the typed helpers (`get_int`, `get_double`, `get_string`, `get_compound`), which return a default for a missing key instead of failing, and write it with `insert`:

```rust
use pluto_lib::{Nbt, NbtCompound, NbtCompoundExt};

let mut data = NbtCompound::new();
data.insert("name".to_owned(), Nbt::String("oven".into()));
data.insert("heat".to_owned(), Nbt::Short(300));
assert_eq!(data.get_int("heat", 0), 300);
assert_eq!(data.get_int("cold", -1), -1);
assert_eq!(data.get_string("name"), Some("oven"));
```

## Sending Data to Clients

Nothing is sent to players unless you say so. A renderer or a screen on the client sees only the *update tag*; send everything with `sync_data()` or choose what with `update_tag`:

```rust
use pluto_lib::{BlockEntityType, Identifier, NbtCompound};

let kind = BlockEntityType::new(Identifier::new("example", "tank_display").unwrap()).update_tag(|data| {
    let mut tag = NbtCompound::new();
    if let Some(level) = data.get("level") {
        tag.insert("level".to_owned(), level.clone());
    }
    tag
});
let _ = kind;
```

## Synchronized Menu Values

A content menu declares `data_slots` in its JSON. Set those integer values from a ticker with `set_menu_data`; the game's menu synchronization delivers them to the screen's `data`, including dynamic layouts. These values are saved with the block entity, separately from its update tag; `sync_data()` is not needed for this path.

```rust
use pluto_lib::{BlockEntityType, Identifier};
let oven = BlockEntityType::new(Identifier::parse("example:oven").unwrap())
    .ticker(|tick| tick.set_menu_data(0, 200));
let _ = oven;
```

> **Use with Caution.** Each value is clamped to -32768..32767, the menu protocol's signed short. Use several data slots for a larger value. Values are shared by every viewer of the block's menu.

## Items and Fluids

Add `item_slots(n)` for slots that hoppers and menus can reach and `fluid_tanks([...])` for tanks other blocks can fill and drain; the ticker reads and changes both (see [Item slots and fluid tanks](#item-slots-and-fluid-tanks)).

> **Use with Caution.** A ticker runs on the server for every loaded block entity of the type, every tick. Keep it cheap: no searching the world, no allocating in the common case.

## Reference

## NBT

NBT is the tagged data the game saves and sends. PlutoLib has its own copy of the types so code using it does not depend on the game's.

### `Nbt`

One NBT value: `Byte`, `Short`, `Int`, `Long`, `Float`, `Double`, `ByteArray`, `String`, `List`, `Compound`, `IntArray` or `LongArray`.

```rust
use pluto_lib::Nbt;

let values = [Nbt::Byte(1), Nbt::Short(2), Nbt::Int(3), Nbt::Long(4), Nbt::Float(0.5), Nbt::Double(0.25),
              Nbt::String("text".into()), Nbt::List(vec![Nbt::Int(1)]), Nbt::IntArray(vec![1, 2])];
assert_eq!(values.len(), 9);
```

### `NbtCompound`

A named set of NBT values (`BTreeMap<String, Nbt>`), the shape of a block entity's saved data.

```rust
use pluto_lib::{Nbt, NbtCompound};

let mut data = NbtCompound::new();
data.insert("heat".to_owned(), Nbt::Int(100));
assert_eq!(data.len(), 1);
```

### `Nbt::as_i64`

`fn as_i64(&self) -> Option<i64>`. Any integer tag (`Byte`, `Short`, `Int`, `Long`) as an `i64`; `None` for other tags.

```rust
use pluto_lib::Nbt;

assert_eq!(Nbt::Short(7).as_i64(), Some(7));
assert_eq!(Nbt::Long(-1).as_i64(), Some(-1));
assert_eq!(Nbt::String("7".into()).as_i64(), None);
```

### `Nbt::as_f64`

`fn as_f64(&self) -> Option<f64>`. Any number tag as an `f64`; `None` for other tags.

```rust
use pluto_lib::Nbt;

assert_eq!(Nbt::Float(0.5).as_f64(), Some(0.5));
assert_eq!(Nbt::Int(3).as_f64(), Some(3.0));
assert_eq!(Nbt::List(vec![]).as_f64(), None);
```

### `Nbt::as_str`

`fn as_str(&self) -> Option<&str>`. The text of a `String` tag.

```rust
use pluto_lib::Nbt;

assert_eq!(Nbt::String("lava".into()).as_str(), Some("lava"));
assert_eq!(Nbt::Int(1).as_str(), None);
```

### `Nbt::as_compound`

`fn as_compound(&self) -> Option<&NbtCompound>`. The compound inside a `Compound` tag.

```rust
use pluto_lib::{Nbt, NbtCompound};

let inner = Nbt::Compound(NbtCompound::new());
assert!(inner.as_compound().is_some());
assert!(Nbt::Int(1).as_compound().is_none());
```

### `Nbt::as_list`

`fn as_list(&self) -> Option<&[Nbt]>`. The elements of a `List` tag.

```rust
use pluto_lib::Nbt;

let list = Nbt::List(vec![Nbt::Int(1), Nbt::Int(2)]);
assert_eq!(list.as_list().unwrap().len(), 2);
assert!(Nbt::Int(1).as_list().is_none());
```

### `NbtCompoundExt`

Typed reads from a compound with a default, like Minecraft's `CompoundTag.getInt`: `get_int(key, default) -> i64`, `get_double(key, default) -> f64`, `get_string(key) -> Option<&str>`, `get_compound(key) -> Option<&NbtCompound>`. A missing key, or a key of another tag kind, gives the default.

```rust
use pluto_lib::{Nbt, NbtCompound, NbtCompoundExt};

let mut data = NbtCompound::new();
data.insert("count".into(), Nbt::Short(3));
data.insert("ratio".into(), Nbt::Float(0.5));
data.insert("name".into(), Nbt::String("melter".into()));
data.insert("inner".into(), Nbt::Compound(NbtCompound::new()));
assert_eq!(data.get_int("count", 0), 3);
assert_eq!(data.get_int("missing", -1), -1);
assert_eq!(data.get_double("ratio", 0.0), 0.5);
assert_eq!(data.get_string("name"), Some("melter"));
assert!(data.get_compound("inner").is_some());
```

## Block entity types

### `BlockPos`

A block position `[x, y, z]`.

```rust
use pluto_lib::BlockPos;

let pos: BlockPos = [10, 64, -3];
assert_eq!(pos[1], 64);
```

### `BlockEntityTick`

What a ticker receives. `pos` is the block entity's position, `data` its saved data and `items` its item slots, which the ticker may change; changes are saved with the chunk.

```rust
use pluto_lib::{BlockEntityTick, Nbt, NbtCompound, NbtCompoundExt};

fn count_up(tick: &mut BlockEntityTick<'_>) {
    let n = tick.data.get_int("ticks", 0) + 1;
    tick.data.insert("ticks".to_owned(), Nbt::Int(n as i32));
    let _at = tick.pos;
}
let mut data = NbtCompound::new();
let mut items = Vec::new();
count_up(&mut BlockEntityTick { pos: [0, 64, 0], data: &mut data, items: &mut items });
assert_eq!(data.get_int("ticks", 0), 1);
```

### `BlockEntityType`

The kind of block entity a block has. `block` is the block it belongs to, `id` the type's own id (saved with the block entity; the block's id unless changed), `initial_data` the data a new block entity starts with. Build one with `BlockEntityType::new` and the builder methods, then register it.

```rust
use pluto_lib::{BlockEntityType, Identifier};

let kind = BlockEntityType::new(Identifier::new("example", "oven").unwrap());
assert_eq!(kind.block.to_string(), "example:oven");
assert_eq!(kind.id.to_string(), "example:oven");
assert!(kind.initial_data.is_empty());
```

### `BlockEntityType::new`

`fn new(block: Identifier) -> BlockEntityType`. A type for `block` with no data, no ticker and nothing sent to clients.

```rust
use pluto_lib::{BlockEntityType, Identifier};

let kind = BlockEntityType::new(Identifier::new("example", "oven").unwrap());
assert_eq!(kind.block.path(), "oven");
```

### `BlockEntityType::with_id`

`fn with_id(self, id: Identifier) -> BlockEntityType`. The same type under another id.

```rust
use pluto_lib::{BlockEntityType, Identifier};

let kind = BlockEntityType::new(Identifier::new("example", "oven").unwrap()).with_id(Identifier::new("example", "oven_entity").unwrap());
assert_eq!(kind.id.path(), "oven_entity");
```

### `BlockEntityType::initial_data`

`fn initial_data(self, data: NbtCompound) -> BlockEntityType`. New block entities start with `data`.

```rust
use pluto_lib::{BlockEntityType, Identifier, Nbt, NbtCompound};

let mut start = NbtCompound::new();
start.insert("fuel".to_owned(), Nbt::Int(0));
let kind = BlockEntityType::new(Identifier::new("example", "oven").unwrap()).initial_data(start);
assert_eq!(kind.initial_data.len(), 1);
```

### `BlockEntityType::ticker`

`fn ticker(self, ticker: impl Fn(&mut BlockEntityTick<'_>) + Send + Sync + 'static) -> BlockEntityType`. The block entity is ticked on the server every game tick by `ticker` (Fabric: `BlockEntityTicker`). Keep it short.

```rust
use pluto_lib::{BlockEntityType, Identifier, Nbt, NbtCompoundExt};

let kind = BlockEntityType::new(Identifier::new("example", "oven").unwrap()).ticker(|tick| {
    let fuel = tick.data.get_int("fuel", 0);
    if fuel > 0 {
        tick.data.insert("fuel".to_owned(), Nbt::Int(fuel as i32 - 1));
    }
});
let _ = kind;
```

### `BlockEntityType::update_tag`

`fn update_tag(self, update_tag: impl Fn(&NbtCompound) -> NbtCompound + Send + Sync + 'static) -> BlockEntityType`. Clients are sent what `update_tag` makes of the saved data whenever a chunk holding the block entity is sent or its block changes (Minecraft: `getUpdateTag`). Without it nothing is sent.

```rust
use pluto_lib::{BlockEntityType, Identifier, NbtCompound};

// clients need to draw the fuel level but not the internal bookkeeping
let kind = BlockEntityType::new(Identifier::new("example", "oven").unwrap()).update_tag(|data| {
    let mut tag = NbtCompound::new();
    if let Some(fuel) = data.get("fuel") {
        tag.insert("fuel".to_owned(), fuel.clone());
    }
    tag
});
let _ = kind;
```

### `BlockEntityType::sync_data`

`fn sync_data(self) -> BlockEntityType`. Clients are sent all of the saved data.

```rust
use pluto_lib::{BlockEntityType, Identifier};

let kind = BlockEntityType::new(Identifier::new("example", "sign").unwrap()).sync_data();
let _ = kind;
```

### `BlockEntityTypeRegistry`

Registers block entity types (Fabric: `Registry.register(Registries.BLOCK_ENTITY_TYPE, ...)`).

```rust
use pluto_lib::{BlockEntityType, BlockEntityTypeRegistry, Identifier};

BlockEntityTypeRegistry::register(BlockEntityType::new(Identifier::new("example", "chest_like").unwrap()));
```

### `BlockEntityTypeRegistry::register`

`fn register(entity_type: BlockEntityType)`. Gives the type's block its block entity. Registering the same block again replaces the earlier type. Register in your mod's `hooks()`.

```rust
use pluto_lib::{BlockEntityType, BlockEntityTypeRegistry, Identifier};

let block = Identifier::new("example", "replaced").unwrap();
BlockEntityTypeRegistry::register(BlockEntityType::new(block.clone()).with_id(Identifier::new("example", "old").unwrap()));
BlockEntityTypeRegistry::register(BlockEntityType::new(block).with_id(Identifier::new("example", "new").unwrap()));
assert_eq!(BlockEntityTypeRegistry::get("example:replaced").unwrap().id.path(), "new");
```

### `BlockEntityTypeRegistry::get`

`fn get(block: &str) -> Option<BlockEntityType>`. The type registered for a block id such as `example:oven`, or `None`.

```rust
use pluto_lib::{BlockEntityType, BlockEntityTypeRegistry, Identifier};

assert!(BlockEntityTypeRegistry::get("example:nothing_here").is_none());
BlockEntityTypeRegistry::register(BlockEntityType::new(Identifier::new("example", "found").unwrap()));
assert!(BlockEntityTypeRegistry::get("example:found").is_some());
```

## Item slots and fluid tanks

### `ItemSlot`

One stack in an item slot. `item` is the item, `count` how many, `damage` the damage taken (0 for a new item), `max_damage` the most damage the stack takes before it breaks when that differs from the item's (the `minecraft:max_damage` component, None for the item's own) and `custom_data` the stack's `minecraft:custom_data` (empty for none). A slot whose item is unchanged keeps all its other data when only `count` is changed. A tool a mod made keeps its materials and stats in `custom_data`, so a block that takes tools or tool parts can read them.

```rust
use pluto_lib::{Identifier, ItemSlot, Nbt};

let mut part = ItemSlot::new(Identifier::new("example", "pick_head").unwrap(), 1);
part.custom_data.insert("material".to_owned(), Nbt::String("example:flint".into()));
part.max_damage = Some(60);
assert_eq!(part.item.path(), "pick_head");
assert_eq!((part.count, part.damage, part.max_damage), (1, 0, Some(60)));
assert_eq!(part.custom_data["material"], Nbt::String("example:flint".into()));
```

### `ItemSlot::new`

`fn new(item: Identifier, count: u32) -> ItemSlot`. A plain stack of `count` of `item`, with no damage, no max damage and no data.

```rust
use pluto_lib::{Identifier, ItemSlot};

let slot = ItemSlot::new(Identifier::new("minecraft", "coal").unwrap(), 8);
assert_eq!(slot.count, 8);
assert_eq!(slot.item.path(), "coal");
assert!(slot.custom_data.is_empty() && slot.max_damage.is_none());
```

### `FluidStack`

Fluid in a tank. `fluid` is the fluid (`minecraft:water`, `minecraft:lava` or a mod's own), `amount` the millibuckets (a bucket is a thousand).

```rust
use pluto_lib::{FluidStack, Identifier};

let bucket = FluidStack { fluid: Identifier::new("minecraft", "water").unwrap(), amount: 1000 };
assert_eq!(bucket.amount, 1000);
assert_eq!(bucket.fluid.to_string(), "minecraft:water");
```

### `BlockEntityType::item_slots`

`fn item_slots(self, count: u32) -> BlockEntityType`. The block entity has `count` item slots (at most 256) that hoppers and the block's menu can reach. A ticker reads and changes them through `BlockEntityTick::items`.

```rust
use pluto_lib::{BlockEntityType, Identifier, ItemSlot};

let kind = BlockEntityType::new(Identifier::new("example", "smelter").unwrap())
    .item_slots(3)
    .ticker(|tick| {
        // burn one coal from slot 0 per tick
        if let Some(Some(slot)) = tick.items.get_mut(0) {
            if slot.item.path() == "coal" && slot.count > 0 {
                slot.count -= 1;
            }
        }
        if let Some(Some(slot)) = tick.items.get(0) {
            if slot.count == 0 {
                tick.items[0] = None;
            }
        }
    });
let _: Option<ItemSlot> = None;
let _ = kind;
```

### `BlockEntityType::fluid_tanks`

`fn fluid_tanks(self, capacities: impl IntoIterator<Item = u32>) -> BlockEntityType`. One fluid tank for each capacity given, in millibuckets (at most 16 tanks, each 1 to 1 000 000). The contents are saved with the block entity; a ticker reads and changes them with `BlockEntityTick::fluid` and `set_fluid`, and other blocks can fill and drain them (Fabric: `FluidStorage`).

```rust
use pluto_lib::{BlockEntityType, FluidStack, Identifier};

let kind = BlockEntityType::new(Identifier::new("example", "melter").unwrap())
    .fluid_tanks([4000, 1000])
    .ticker(|tick| {
        // move whatever is in tank 0 into tank 1 until it is full
        if let Some(stack) = tick.fluid(0) {
            let room = 1000 - tick.fluid(1).map_or(0, |f| f.amount);
            let moved = stack.amount.min(room);
            if moved > 0 {
                tick.set_fluid(1, Some(FluidStack { fluid: stack.fluid.clone(), amount: tick.fluid(1).map_or(0, |f| f.amount) + moved }));
                let left = stack.amount - moved;
                tick.set_fluid(0, (left > 0).then(|| FluidStack { fluid: stack.fluid, amount: left }));
            }
        }
    });
let _ = kind;
```

### `BlockEntityTick::set_menu_data`

`fn set_menu_data(&mut self, index: usize, value: i32)`. Sets a synchronized integer of the block's menu; declare the data slot in the content menu JSON. Indices 0..256 are supported (256 excluded); larger indices do nothing. Values are clamped to the signed 16-bit wire range and saved with the block entity.

```rust
use pluto_lib::{BlockEntityTick, NbtCompound};
let mut data = NbtCompound::new();
let mut items = Vec::new();
let mut tick = BlockEntityTick { pos: [0, 64, 0], data: &mut data, items: &mut items };
tick.set_menu_data(0, 120);
```

### `BlockEntityTick::fluid`

`fn fluid(&self, index: usize) -> Option<FluidStack>`. The fluid in tank `index`, or `None` if the tank is empty or does not exist.

```rust
use pluto_lib::{BlockEntityTick, FluidStack, Identifier, NbtCompound};

let mut data = NbtCompound::new();
let mut items = Vec::new();
let mut tick = BlockEntityTick { pos: [0, 64, 0], data: &mut data, items: &mut items };
assert_eq!(tick.fluid(0), None);
tick.set_fluid(0, Some(FluidStack { fluid: Identifier::new("minecraft", "water").unwrap(), amount: 500 }));
assert_eq!(tick.fluid(0).unwrap().amount, 500);
```

### `BlockEntityTick::set_fluid`

`fn set_fluid(&mut self, index: usize, stack: Option<FluidStack>)`. Puts `stack` in tank `index`, or empties the tank with `None`. The tanks before `index` are kept (empty if there were none yet).

```rust
use pluto_lib::{BlockEntityTick, FluidStack, Identifier, NbtCompound};

let mut data = NbtCompound::new();
let mut items = Vec::new();
let mut tick = BlockEntityTick { pos: [0, 64, 0], data: &mut data, items: &mut items };
let lava = FluidStack { fluid: Identifier::new("minecraft", "lava").unwrap(), amount: 250 };
tick.set_fluid(2, Some(lava.clone()));
assert_eq!(tick.fluid(0), None);
assert_eq!(tick.fluid(2), Some(lava));
tick.set_fluid(2, None);
assert_eq!(tick.fluid(2), None);
```
