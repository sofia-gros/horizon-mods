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

> **Use with Caution.** Native-recognized custom block names are an uncovered factory-priority case. The pinned host recognizes skull/head suffixes before asking the mod block-entity factory, so registering a custom `*_head` or `*_skull` here does not replace its native skull entity. Mod block-entity rendering requires a `Mod` entity and will not receive that native skull. A registered-custom-only priority policy is pure-tested; its predicate installation and early-factory hook are now connected in source. It preserves vanilla/unregistered fallback, calls the existing native dispatcher once for claimed custom IDs, and never retries a failed claimed factory as a native skull. The `block_entities` composition guard checks owner, package bytes and native-source authority using the existing exact-contract validator. Its actual function fingerprint remains deliberately unset: the guard rejects this route until the native attachment combined plan has been reviewed. Native compilation, save/reopen and network verification remain pending; registration alone does not prove this path works. Existing native-skull save tags are not migrated by this new-object factory.

## Client Animation

`BlockEntityType::client_ticker` declares a client-level `BlockEntityTicker`, independently of the server `ticker`. The client callback reads synchronized `data` but writes only `client_data`, the transient state of that client block entity. This separates animation from saves, menu data and server synchronization.

```rust
use pluto_lib::{BlockEntityType, Identifier, Nbt, NbtCompoundExt};
let kind = BlockEntityType::new(Identifier::parse("example:animated_machine").unwrap())
    .client_ticker(|tick| {
        let Some(powered) = tick.world.has_neighbor_signal(tick.pos) else { return };
        if powered {
            let count = tick.client_data.get_int("animation_ticks", 0) as i32;
            tick.client_data.insert("animation_ticks".into(), Nbt::Int(count.wrapping_add(1)));
        }
    });
assert_eq!(kind.block.to_string(), "example:animated_machine");
```

> **Use with Caution.** This remains a native-verification stub. State/constructor declarations, renderer snapshots and client tick dispatch are wired in source but not applied or native-compiled. The declared HEAD at the existing block-entity animation pass excludes servers, absent levels, title screens, pause and frozen ticks; its API dispatcher converts read-only saved NBT and an owned transient update, then commits only a dispatched callback. Shared entity iteration and owned leases release world/entity borrows before native signal queries without waking server tickers. Residence changes reset the client state independently of frustum/fade; stable residence preserves it. Busy snapshots and unknown power remain unavailable, not fabricated empty data or false. Replacement/unload resets are now source-wired: a retained same-level respawn preserves animation, a different or forced new level resets before replacement, and disconnect/reconfiguration clears the leaving lifetime. The host notifies old-level unload after new-level load; that late notification must not reset the new lifetime. Constructor initialization alone is insufficient, and a position-only global map is not equivalent. The composition guard deliberately rejects use until native attachment owner/constructor/function-plan review. Native save/network exclusion, cache lifecycle and live animation remain unverified.

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

### `BlockEntityClientTick`

Client callback context with `pos: BlockPos`, `data: &NbtCompound` (read-only server data), `client_data: &mut NbtCompound` (unsaved instance data) and `world: &dyn BlockView`. The callback never gets mutable access to server data. Native dispatch and lifecycle ownership remain unimplemented.

```rust
use pluto_lib::{BlockEntityClientTick, MemoryBlockView, Nbt, NbtCompound, NbtCompoundExt};
let world = MemoryBlockView::default();
let saved = NbtCompound::new();
let mut local = NbtCompound::new();
let tick = BlockEntityClientTick {
    pos: [1, 64, 2], data: &saved, client_data: &mut local, world: &world,
};
tick.client_data.insert("angle".into(), Nbt::Int(3));
assert_eq!(tick.client_data.get_int("angle", 0), 3);
assert!(tick.data.is_empty());
```

### `BlockEntityType::client_ticker`

`fn client_ticker(self, ticker: impl Fn(&mut BlockEntityClientTick<'_>) + Send + Sync + 'static) -> BlockEntityType`. Replaces this type's client callback without adding or changing its server callback. Registration currently records the callback only; the native adapter is a documented stub.

```rust
use pluto_lib::{BlockEntityType, Identifier, Nbt};
let kind = BlockEntityType::new(Identifier::parse("example:display").unwrap())
    .client_ticker(|tick| { tick.client_data.insert("visible".into(), Nbt::Byte(1)); });
assert_eq!(kind.id.to_string(), "example:display");
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

One stack in an item slot. `item` is the item, `count` how many, `damage` the damage taken (0 for a new item), `max_damage` the most damage the stack takes before it breaks when that differs from the item's (the `minecraft:max_damage` component, None for the item's own) and `custom_data` the stack's `minecraft:custom_data` (empty for none). Clone an existing slot when moving it: an opaque host-owned snapshot carries its other components (including native wire data) across slot/cursor changes, and the adapter applies edits to the public fields afterward. Use `ItemSlot::new`, not a struct literal, for a plain new stack; it must not inherit the destination's components. Changing `item` discards the captured source components. To empty a container slot, assign `None`; a committed slot with `count == 0` is also written back as `None`, matching the host's empty-slot representation. Production conversion fixtures pass; the changed native adapter has not yet been compiled or exercised in game. A tool a mod made keeps its materials and stats in `custom_data`, so a block that takes tools or tool parts can read them. `tool` is the stack's own `minecraft:tool` component: write the mining rules and speeds there and only this stack mines that way (`None` keeps the item's own). `attribute_modifiers` is the stack's own `minecraft:attribute_modifiers` component, one compound per modifier with `type`, `id`, `amount`, `operation` and `slot`: write the list there and only this stack carries those attribute modifiers (an empty list gives it none, `None` keeps the item's); the host applies the list to a player holding or wearing the stack. `enchantments` is the stack's own `minecraft:enchantments` component as enchantment id to level: write it there and only this stack carries those enchantments (an empty map removes them, `None` keeps the stack's), and the host applies them to a player holding or wearing the stack. The native mining-speed and drop rules read it, and the native item encoder/decoder already carries it between players; the conversion is source-wired and not yet compiled against the pinned host. `custom_model_data` is the stack's own `minecraft:custom_model_data` component (see `CustomModelData` on the player page): write it there and only this stack selects that item-model entry (`None` keeps the stack's); the conversion is source-wired and not yet compiled against the pinned host.

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

## Custom fluid rules and use hooks

A block whose tank capacity depends on what it is offered (a casting table that wants exactly the recipe's amount), or that trades items with the player's hand, declares the rule itself. Both hooks are pure and tested; the host adapter that routes a neighbour's fill and a player's right click to them is not written (see the caution below).

> **Use with Caution.** Source-ready only. `BlockEntityTypeRegistry::fill_fluid` and `use_block` run the hooks over data and slots the caller supplies and are tested through the in-memory worlds, but the native host does not call them yet: until its adapter reads a block's `fluid_filler` for fills (neighbour fills, faucets, buckets) and its `on_use` hook for right clicks, a block with these hooks keeps the default tank fill and the game's own use. The generated `Host` signatures can only be checked by a native build.

### `BlockEntityType::fluid_filler`

`fn fluid_filler(self, filler: impl Fn(&mut BlockEntityFill<'_>) -> u32 + Send + Sync + 'static) -> BlockEntityType`. The hook is asked how much of an offered fluid the block takes, and may change its saved data and item slots. Without one the declared tanks are filled to their capacity (Fabric: a custom `Storage<FluidVariant>.insert`; NeoForge: a custom `IFluidHandler.fill`).

```rust
use pluto_lib::{BlockEntityType, BlockEntityTypeRegistry, FluidStack, Identifier, NbtCompound};

let basin = Identifier::new("example", "basin").unwrap();
BlockEntityTypeRegistry::register(BlockEntityType::new(basin).fluid_tanks([1000]).fluid_filler(|fill| {
    // takes at most 90 millibuckets of a single fluid
    let held = fill.fluid_in(0).map_or(0, |held| held.amount);
    let taken = fill.fluid.amount.min(90 - held);
    if taken > 0 {
        fill.set_fluid(0, Some(FluidStack { fluid: fill.fluid.fluid.clone(), amount: held + taken }));
    }
    taken
}));
let lava = FluidStack { fluid: Identifier::new("minecraft", "lava").unwrap(), amount: 500 };
let (mut data, mut items) = (NbtCompound::new(), Vec::new());
assert_eq!(BlockEntityTypeRegistry::fill_fluid("example:basin", [0, 64, 0], &mut data, &mut items, lava, None, true), Some(90));
```

### `BlockEntityFill`

What a fluid filler receives: `pos`, the saved `data`, the item `items`, the `fluid` offered, the `face` it comes through (`None` when not said) and `execute`. With `execute` false the caller only asks how much would be taken and the changes are thrown away, so the hook can run the same code either way. `fluid_in(index)` and `set_fluid(index, stack)` read and write a tank.

```rust
use pluto_lib::{BlockEntityFill, FluidStack, Identifier, NbtCompound};

fn take_all(fill: &mut BlockEntityFill<'_>) -> u32 {
    let _ = (fill.pos, fill.execute, fill.items.len());
    fill.set_fluid(0, Some(fill.fluid.clone()));
    fill.fluid.amount
}
let (mut data, mut items) = (NbtCompound::new(), Vec::new());
let water = FluidStack { fluid: Identifier::new("minecraft", "water").unwrap(), amount: 250 };
let mut fill = BlockEntityFill { pos: [0, 64, 0], data: &mut data, items: &mut items, fluid: water, face: None, execute: true };
assert_eq!(take_all(&mut fill), 250);
assert_eq!(fill.fluid_in(0).unwrap().amount, 250);
```

### `BlockEntityTypeRegistry::fill_fluid`

`fn fill_fluid(block: &str, pos: BlockPos, data: &mut NbtCompound, items: &mut Vec<Option<ItemSlot>>, fluid: FluidStack, face: Option<Direction>, execute: bool) -> Option<u32>`. Offers `fluid`, coming through `face`, to the block's filler over the given data and slots: the amount taken, or `None` when the block has no filler (the host then fills the declared tanks). With `execute` false the data and slots are left unchanged.

```rust
use pluto_lib::{BlockEntityType, BlockEntityTypeRegistry, FluidStack, Identifier, NbtCompound};

BlockEntityTypeRegistry::register(BlockEntityType::new(Identifier::new("example", "sink").unwrap()).fluid_filler(|fill| fill.fluid.amount));
let (mut data, mut items) = (NbtCompound::new(), Vec::new());
let water = FluidStack { fluid: Identifier::new("minecraft", "water").unwrap(), amount: 40 };
assert_eq!(BlockEntityTypeRegistry::fill_fluid("example:sink", [0; 3], &mut data, &mut items, water.clone(), None, false), Some(40));
assert_eq!(BlockEntityTypeRegistry::fill_fluid("example:plain", [0; 3], &mut data, &mut items, water, None, false), None);
```

### `BlockEntityType::on_use`

`fn on_use(self, user: impl Fn(&mut BlockEntityUse<'_>) -> ActionResult + Send + Sync + 'static) -> BlockEntityType`. Run when a player uses the block; it may exchange items with the hand and the slots, and its answer is the use's answer (`Pass` leaves it to the game).

```rust
use pluto_lib::{ActionResult, BlockEntityType, BlockEntityTypeRegistry, Identifier, ItemSlot};

BlockEntityTypeRegistry::register(BlockEntityType::new(Identifier::new("example", "shelf").unwrap()).item_slots(1).on_use(|used| {
    // an empty hand takes the item out, a held item goes on the shelf when it is empty
    match (used.held.take(), used.items[0].take()) {
        (None, Some(stack)) => { used.give.push(stack); ActionResult::Success }
        (Some(held), None) => { used.items[0] = Some(held); ActionResult::Success }
        (held, slot) => { used.held = held; used.items[0] = slot; ActionResult::Pass }
    }
}));
let mut items = vec![None];
let hand = ItemSlot::new(Identifier::new("minecraft", "stick").unwrap(), 1);
let out = BlockEntityTypeRegistry::use_block("example:shelf", [0; 3], &mut Default::default(), &mut items, Some(hand), false).unwrap();
assert_eq!(out.result, ActionResult::Success);
assert!(items[0].is_some() && out.held.is_none());
```

### `BlockEntityUse`

What a use hook receives: `pos`, `data`, `items`, `held` (the stack in the used hand, `None` when empty; the hook may change it), `sneaking`, and `give`, the stacks the player is handed (into the inventory, dropped when it is full). `fluid(index)` reads a tank.

```rust
use pluto_lib::{BlockEntityUse, ItemSlot, Identifier, NbtCompound};

let (mut data, mut items) = (NbtCompound::new(), vec![None]);
let stick = ItemSlot::new(Identifier::new("minecraft", "stick").unwrap(), 1);
let mut used = BlockEntityUse { pos: [0; 3], data: &mut data, items: &mut items, held: None, sneaking: false, give: vec![stick] };
assert!(used.fluid(0).is_none() && used.give.len() == 1 && !used.sneaking && used.pos == [0; 3]);
```

### `BlockEntityTypeRegistry::use_block`

`fn use_block(block: &str, pos: BlockPos, data: &mut NbtCompound, items: &mut Vec<Option<ItemSlot>>, held: Option<ItemSlot>, sneaking: bool) -> Option<BlockUseOutcome>`. Runs the block's use hook for a player holding `held`; `None` when the block has none.

```rust
use pluto_lib::{BlockEntityTypeRegistry, NbtCompound};

assert!(BlockEntityTypeRegistry::use_block("example:no_hook", [0; 3], &mut NbtCompound::new(), &mut Vec::new(), None, false).is_none());
```

### `BlockUseOutcome`

What a use did: `result` (the answer), `held` (the used hand afterwards) and `give` (stacks for the player).

```rust
use pluto_lib::{ActionResult, BlockUseOutcome};

let outcome = BlockUseOutcome { result: ActionResult::Pass, held: None, give: Vec::new() };
assert_eq!(outcome.result, ActionResult::Pass);
assert!(outcome.held.is_none() && outcome.give.is_empty());
```

### `BlockEntityFill::fluid_in`

`fn fluid_in(&self, index: usize) -> Option<FluidStack>`. The fluid in tank `index` of the data being filled.

```rust
use pluto_lib::{BlockEntityFill, FluidStack, Identifier, NbtCompound};

let (mut data, mut items) = (NbtCompound::new(), Vec::new());
let water = FluidStack { fluid: Identifier::new("minecraft", "water").unwrap(), amount: 10 };
let fill = BlockEntityFill { pos: [0; 3], data: &mut data, items: &mut items, fluid: water, face: None, execute: false };
assert_eq!(fill.fluid_in(0), None);
```

### `BlockEntityFill::set_fluid`

`fn set_fluid(&mut self, index: usize, stack: Option<FluidStack>)`. Puts a fluid in tank `index`, or empties it with `None`.

```rust
use pluto_lib::{BlockEntityFill, FluidStack, Identifier, NbtCompound};

let (mut data, mut items) = (NbtCompound::new(), Vec::new());
let water = FluidStack { fluid: Identifier::new("minecraft", "water").unwrap(), amount: 10 };
let mut fill = BlockEntityFill { pos: [0; 3], data: &mut data, items: &mut items, fluid: water.clone(), face: None, execute: true };
fill.set_fluid(0, Some(water));
assert_eq!(fill.fluid_in(0).unwrap().amount, 10);
```

### `BlockEntityUse::fluid`

`fn fluid(&self, index: usize) -> Option<FluidStack>`. The fluid in tank `index`, for a use hook that depends on it (a casting table does nothing with an item while it holds fluid).

```rust
use pluto_lib::{BlockEntityUse, NbtCompound};

let (mut data, mut items) = (NbtCompound::new(), Vec::new());
let used = BlockEntityUse { pos: [0; 3], data: &mut data, items: &mut items, held: None, sneaking: false, give: Vec::new() };
assert!(used.fluid(0).is_none());
```
