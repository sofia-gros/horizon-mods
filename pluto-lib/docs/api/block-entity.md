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

### `Nbt::from_binary`

`fn from_binary(bytes: &[u8]) -> Result<NbtCompound, String>`. Reads a binary NBT document (a named root compound) into the model, inflating it first when it is gzip-wrapped, as structure templates are stored. The root's own name is dropped; trailing bytes after the root are refused, so a corrupt file fails instead of reading as a shorter one. Java's byte arrays are signed there and bit patterns here, so a byte array round-trips exactly.

```rust
use pluto_lib::Nbt;

// A root compound holding one int, DataVersion 3465, written out by hand.
let bytes: &[u8] = &[
    10, 0, 0, // compound, empty root name
    3, 0, 11, b'D', b'a', b't', b'a', b'V', b'e', b'r', b's', b'i', b'o', b'n', 0, 0, 13, 137,
    0, // end
];
let root = Nbt::from_binary(bytes).unwrap();
assert_eq!(root.get("DataVersion"), Some(&Nbt::Int(3465)));
assert!(Nbt::from_binary(&[3, 0, 0]).is_err(), "an int root is not a compound");
```

### `gunzip`

`fn gunzip(bytes: &[u8]) -> Result<Vec<u8>, String>`. Inflates a gzip stream to the bytes it wraps (stored and Huffman deflate blocks alike; the pure crate takes no compression dependency, so the inflater is PlutoLib's own). The stream's CRC32 and size trailer are not verified: the deflate stream's own final block marks the data's end.

```rust
use pluto_lib::gunzip;

// A stored (uncompressed) gzip stream around the bytes 1, 2, 3.
let mut stream = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 3, 1, 3, 0, 0xfc, 0xff, 1, 2, 3];
stream.extend_from_slice(&[0; 8]);
assert_eq!(gunzip(&stream).unwrap(), [1, 2, 3]);
assert!(gunzip(&[0x1f, 0x8b, 7]).is_err(), "a non-deflate gzip is refused");
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

One stack in an item slot. `item` is the item, `count` how many, `damage` the damage taken (0 for a new item), `max_damage` the most damage the stack takes before it breaks when that differs from the item's (the `minecraft:max_damage` component, None for the item's own) and `custom_data` the stack's `minecraft:custom_data` (empty for none). Clone an existing slot when moving it: an opaque host-owned snapshot carries its other components (including native wire data) across slot/cursor changes, and the adapter applies edits to the public fields afterward. Use `ItemSlot::new`, not a struct literal, for a plain new stack; it must not inherit the destination's components. Changing `item` discards the captured source components. To empty a container slot, assign `None`; a committed slot with `count == 0` is also written back as `None`, matching the host's empty-slot representation. Production conversion fixtures pass; the changed native adapter has not yet been compiled or exercised in game. A tool a mod made keeps its materials and stats in `custom_data`, so a block that takes tools or tool parts can read them. `tool` is the stack's own `minecraft:tool` component: write the mining rules and speeds there and only this stack mines that way (`None` keeps the item's own). `attribute_modifiers` is the stack's own `minecraft:attribute_modifiers` component, one compound per modifier with `type`, `id`, `amount`, `operation` and `slot`: write the list there and only this stack carries those attribute modifiers (an empty list gives it none, `None` keeps the item's); the host applies the list to a player holding or wearing the stack. `enchantments` is the stack's own `minecraft:enchantments` component as enchantment id to level: write it there and only this stack carries those enchantments (an empty map removes them, `None` keeps the stack's), and the host applies them to a player holding or wearing the stack. The native mining-speed and drop rules read it, and the native item encoder/decoder already carries it between players; the conversion is source-wired and not yet compiled against the pinned host. `custom_model_data` is the stack's own `minecraft:custom_model_data` component (see `CustomModelData` on the player page): write it there and only this stack selects that item-model entry (`None` keeps the stack's); the conversion is source-wired and not yet compiled against the pinned host. `profile` is the stack's own `minecraft:profile` component (a player head's owner: `name`, `id` as the uuid's four ints, and `properties`): write it there and only this stack carries that owner, so a head a beheading drops keeps the player's skin (`None` keeps the stack's); the conversion is source-wired and not yet compiled against the pinned host. `map_id` is the stack's own `minecraft:map_id` component (a filled map's id): write it there and only this stack carries that map id, so a map a minimap inventory stores keeps its explored data (`None` keeps the stack's); the conversion is source-wired and not yet compiled against the pinned host. `can_place_on` is the stack's own `minecraft:can_place_on` component (the adventure-mode placement predicates: a list of block predicates) as NBT: write it there and only this stack carries those predicates (`None` keeps the stack's); the conversion is source-wired and not yet compiled against the pinned host.

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

let bucket = FluidStack { fluid: Identifier::new("minecraft", "water").unwrap(), amount: 1000, tag: None };
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
                tick.set_fluid(1, Some(FluidStack { fluid: stack.fluid.clone(), amount: tick.fluid(1).map_or(0, |f| f.amount) + moved, tag: stack.tag.clone() }));
                let left = stack.amount - moved;
                tick.set_fluid(0, (left > 0).then(|| FluidStack { fluid: stack.fluid, amount: left, tag: stack.tag }));
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
tick.set_fluid(0, Some(FluidStack { fluid: Identifier::new("minecraft", "water").unwrap(), amount: 500, tag: None }));
assert_eq!(tick.fluid(0).unwrap().amount, 500);
```

### `BlockEntityTick::set_fluid`

`fn set_fluid(&mut self, index: usize, stack: Option<FluidStack>)`. Puts `stack` in tank `index`, or empties the tank with `None`. The tanks before `index` are kept (empty if there were none yet).

```rust
use pluto_lib::{BlockEntityTick, FluidStack, Identifier, NbtCompound};

let mut data = NbtCompound::new();
let mut items = Vec::new();
let mut tick = BlockEntityTick { pos: [0, 64, 0], data: &mut data, items: &mut items };
let lava = FluidStack { fluid: Identifier::new("minecraft", "lava").unwrap(), amount: 250, tag: None };
tick.set_fluid(2, Some(lava.clone()));
assert_eq!(tick.fluid(0), None);
assert_eq!(tick.fluid(2), Some(lava));
tick.set_fluid(2, None);
assert_eq!(tick.fluid(2), None);
```

## Custom fluid rules and use hooks

A block whose tank capacity depends on what it is offered (a casting table that wants exactly the recipe's amount), or that trades items with the player's hand, declares the rule itself. Both hooks are pure and tested; the host adapter in `hooks.rs` routes fills through the host's custom fluid hooks (a block with a `fluid_filler` opts in with `custom_block_fluids`; `custom_fluid_tanks` reads its declared tanks, `custom_fluid_fill` runs the filler, simulating on a copy) and right clicks through `right_click_block_v2` (see the caution below).

> **Use with Caution.** Host adapter written, not yet compiled or game-verified. A right click on a block whose type has an `on_use` hook runs it before the block's own use and the item's: any answer but `Pass` cancels both (the hand does not swing, because the host cannot swing a player's arm), the hand afterwards is written back to the used hand and the stacks it gives go to the inventory, the rest dropped at the player's feet. A block with a `fluid_filler` takes fluid only through that hook, and nothing is drained out of it by another block (`custom_fluid_drain` answers none). Items given or left in the hand keep their components only for stacks the hook did not rebuild.

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
        fill.set_fluid(0, Some(FluidStack { fluid: fill.fluid.fluid.clone(), amount: held + taken, tag: fill.fluid.tag.clone() }));
    }
    taken
}));
let lava = FluidStack { fluid: Identifier::new("minecraft", "lava").unwrap(), amount: 500, tag: None };
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
let water = FluidStack { fluid: Identifier::new("minecraft", "water").unwrap(), amount: 250, tag: None };
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
let water = FluidStack { fluid: Identifier::new("minecraft", "water").unwrap(), amount: 40, tag: None };
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
let water = FluidStack { fluid: Identifier::new("minecraft", "water").unwrap(), amount: 10, tag: None };
let fill = BlockEntityFill { pos: [0; 3], data: &mut data, items: &mut items, fluid: water, face: None, execute: false };
assert_eq!(fill.fluid_in(0), None);
```

### `BlockEntityFill::set_fluid`

`fn set_fluid(&mut self, index: usize, stack: Option<FluidStack>)`. Puts a fluid in tank `index`, or empties it with `None`.

```rust
use pluto_lib::{BlockEntityFill, FluidStack, Identifier, NbtCompound};

let (mut data, mut items) = (NbtCompound::new(), Vec::new());
let water = FluidStack { fluid: Identifier::new("minecraft", "water").unwrap(), amount: 10, tag: None };
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

### `BlockEntityUse::set_fluid`

`fn set_fluid(&mut self, index: usize, stack: Option<FluidStack>)`. Puts `stack` in tank `index`, or empties it with `None`: a use hook that trades fluid with the held container (see [fluid-items](fluid-items.md)) writes the tank back with it.

```rust
use pluto_lib::{BlockEntityUse, FluidStack, Identifier, NbtCompound};

let (mut data, mut items) = (NbtCompound::new(), Vec::new());
let mut used = BlockEntityUse { pos: [0; 3], data: &mut data, items: &mut items, held: None, sneaking: false, give: Vec::new() };
used.set_fluid(0, Some(FluidStack { fluid: Identifier::parse("minecraft:lava").unwrap(), amount: 250, tag: None }));
assert_eq!(used.fluid(0).unwrap().amount, 250);
used.set_fluid(0, None);
assert!(used.fluid(0).is_none());
```

## Forwarding and custom fluid storage

A block that is only a door to another block's tanks (a drain in a smeltery wall), or that keeps its fluid in its own shape (one tank shared by several fluids), declares how its storage is reached. The pure rules are in [fluid-storage](fluid-storage.md), which answers a neighbour's query over these hooks.

> **Use with Caution.** Source-ready only. `BlockEntityTypeRegistry::forward_of`, `read_tanks` and `drain_fluid` run the hooks over data the caller supplies and are tested through `FluidStorage` and its in-memory world, but the native host does not call them yet: until its adapter answers a neighbour's tank, fill and drain query through them (the same adapter `fluid_filler` waits for), a block with these hooks keeps the default declared-tank answers of the game.

### `FluidFilter`

What passes through a forwarding block: `Any`, `Only(fluid)` (fills and drains of other fluids are refused; only tanks that are empty or hold the fluid are listed) or `Closed` (nothing moves; only the last tank is listed, and only while it is empty).

```rust
use pluto_lib::{FluidFilter, Identifier};

let lava = FluidFilter::Only(Identifier::parse("minecraft:lava").unwrap());
assert_ne!(lava, FluidFilter::Any);
assert_ne!(FluidFilter::Closed, FluidFilter::Any);
```

### `FluidForward`

Where a forwarding block's storage really is: `target`, the position of the block entity that answers, and `filter`, a [`FluidFilter`](#fluidfilter).

```rust
use pluto_lib::{FluidFilter, FluidForward};

let forward = FluidForward { target: [0, 64, 0], filter: FluidFilter::Any };
assert_eq!((forward.target, forward.filter), ([0, 64, 0], FluidFilter::Any));
```

### `BlockEntityType::fluid_forward`

`fn fluid_forward(self, forward: impl Fn(&NbtCompound, &[Option<ItemSlot>]) -> Option<FluidForward> + Send + Sync + 'static) -> BlockEntityType`. The block's fluid storage is another block entity's: the hook reads the saved data and item slots and names the target and a filter, or `None` when it points nowhere (then the block has no storage). Fabric: a `FluidStorage.SIDED` lookup that returns the storage of another position.

```rust
use pluto_lib::{BlockEntityType, FluidFilter, FluidForward, Identifier, Nbt};

// a drain remembers its controller's position in its data
let drain = BlockEntityType::new(Identifier::parse("example:drain").unwrap()).fluid_forward(|data, _items| match data.get("master") {
    Some(Nbt::IntArray(at)) if at.len() == 3 => Some(FluidForward { target: [at[0], at[1], at[2]], filter: FluidFilter::Any }),
    _ => None,
});
let _ = drain;
```

### `BlockEntityType::fluid_reader`

`fn fluid_reader(self, reader: impl Fn(&NbtCompound) -> Vec<NeighbourTank> + Send + Sync + 'static) -> BlockEntityType`. How the block lists its tanks, from its saved data (Fabric: `Storage.iterator`; NeoForge: `IFluidHandler.getFluidInTank`). Without one the declared tanks are listed.

```rust
use pluto_lib::{BlockEntityType, Identifier, NeighbourTank};

let vat = BlockEntityType::new(Identifier::parse("example:vat").unwrap()).fluid_reader(|_data| vec![NeighbourTank { fluid: None, capacity: 4000 }]);
let _ = vat;
```

### `BlockEntityDrain`

What a drainer receives: `pos`, the saved `data` and `items` (changes are kept only when `execute` is true), `wanted` (the fluid asked for, `None` for whatever comes first), `amount` (the most millibuckets) and `execute`. `fluid_in(index)` and `set_fluid(index, stack)` read and write a declared tank.

```rust
use pluto_lib::{BlockEntityDrain, FluidStack, Identifier, NbtCompound};

let (mut data, mut items) = (NbtCompound::new(), Vec::new());
let mut drain = BlockEntityDrain { pos: [0; 3], data: &mut data, items: &mut items, wanted: None, amount: 90, execute: true };
drain.set_fluid(0, Some(FluidStack { fluid: Identifier::parse("minecraft:lava").unwrap(), amount: 500, tag: None }));
assert_eq!((drain.fluid_in(0).unwrap().amount, drain.amount, drain.wanted.is_none(), drain.pos, drain.execute, drain.items.len(), drain.data.len()), (500, 90, true, [0; 3], true, 0, 1));
```

### `BlockEntityDrain::fluid_in`

`fn fluid_in(&self, index: usize) -> Option<FluidStack>`. The fluid in declared tank `index` of the data being drained.

```rust
use pluto_lib::{BlockEntityDrain, NbtCompound};

let (mut data, mut items) = (NbtCompound::new(), Vec::new());
let drain = BlockEntityDrain { pos: [0; 3], data: &mut data, items: &mut items, wanted: None, amount: 1, execute: false };
assert_eq!(drain.fluid_in(0), None);
```

### `BlockEntityDrain::set_fluid`

`fn set_fluid(&mut self, index: usize, stack: Option<FluidStack>)`. Puts a fluid in declared tank `index`, or empties it with `None`.

```rust
use pluto_lib::{BlockEntityDrain, FluidStack, Identifier, NbtCompound};

let (mut data, mut items) = (NbtCompound::new(), Vec::new());
let mut drain = BlockEntityDrain { pos: [0; 3], data: &mut data, items: &mut items, wanted: None, amount: 1, execute: true };
drain.set_fluid(0, Some(FluidStack { fluid: Identifier::parse("minecraft:water").unwrap(), amount: 10, tag: None }));
drain.set_fluid(0, None);
assert_eq!(drain.fluid_in(0), None);
```

### `BlockEntityType::fluid_drainer`

`fn fluid_drainer(self, drainer: impl Fn(&mut BlockEntityDrain<'_>) -> Option<FluidStack> + Send + Sync + 'static) -> BlockEntityType`. The block's own rule for fluid taken out (Fabric: `Storage.extract`; NeoForge: `IFluidHandler.drain`). Without one the first declared tank that holds fluid, or the one named, is drained.

```rust
use pluto_lib::{BlockEntityType, Identifier};

let vat = BlockEntityType::new(Identifier::parse("example:vat").unwrap()).fluid_drainer(|drain| {
    let _ = (drain.wanted.as_ref(), drain.amount);
    None
});
let _ = vat;
```

### `BlockEntityTypeRegistry::forward_of`

`fn forward_of(block: &str, data: &NbtCompound, items: &[Option<ItemSlot>]) -> Option<FluidForward>`. Where the block's fluid storage really is, from its `fluid_forward` hook; `None` without a hook or when it points nowhere.

```rust
use pluto_lib::{BlockEntityTypeRegistry, NbtCompound};

assert!(BlockEntityTypeRegistry::forward_of("example:no_such_block", &NbtCompound::new(), &[]).is_none());
```

### `BlockEntityTypeRegistry::forwards`

`fn forwards(block: &str) -> bool`. Whether the block has a `fluid_forward` hook, so that a `None` from `forward_of` means "no storage" and not "the block's own storage".

```rust
use pluto_lib::BlockEntityTypeRegistry;

assert!(!BlockEntityTypeRegistry::forwards("example:no_such_block"));
```

### `BlockEntityTypeRegistry::read_tanks`

`fn read_tanks(block: &str, data: &NbtCompound) -> Option<Vec<NeighbourTank>>`. The tanks as the block's `fluid_reader` lists them, or the declared tanks with what the saved data holds; `None` when the block has neither.

```rust
use pluto_lib::{BlockEntityType, BlockEntityTypeRegistry, Identifier, NbtCompound};

BlockEntityTypeRegistry::register(BlockEntityType::new(Identifier::parse("example:cistern").unwrap()).fluid_tanks([4000]));
let tanks = BlockEntityTypeRegistry::read_tanks("example:cistern", &NbtCompound::new()).unwrap();
assert_eq!((tanks.len(), tanks[0].capacity, tanks[0].fluid.is_none()), (1, 4000, true));
```

### `BlockEntityTypeRegistry::drain_fluid`

`fn drain_fluid(block: &str, pos: BlockPos, data: &mut NbtCompound, items: &mut Vec<Option<ItemSlot>>, wanted: Option<Identifier>, amount: u32, execute: bool) -> Option<Option<FluidStack>>`. Takes up to `amount` millibuckets out over the given data and slots: through the block's `fluid_drainer`, or from the first declared tank that holds fluid (or `wanted`). `None` when the block has no tanks, `Some(None)` when nothing came out; with `execute` false the data and slots are left unchanged.

```rust
use pluto_lib::{BlockEntityTick, BlockEntityType, BlockEntityTypeRegistry, FluidStack, Identifier, NbtCompound};

BlockEntityTypeRegistry::register(BlockEntityType::new(Identifier::parse("example:reservoir").unwrap()).fluid_tanks([4000]));
let (mut data, mut items) = (NbtCompound::new(), Vec::new());
let lava = FluidStack { fluid: Identifier::parse("minecraft:lava").unwrap(), amount: 500, tag: None };
BlockEntityTick { pos: [0; 3], data: &mut data, items: &mut items }.set_fluid(0, Some(lava));
let taken = BlockEntityTypeRegistry::drain_fluid("example:reservoir", [0; 3], &mut data, &mut items, None, 200, true);
assert_eq!(taken.unwrap().unwrap().amount, 200);
```

## Placement, signals and item automation

### `FluidStack::new`

`fn new(fluid: Identifier, amount: u32) -> FluidStack`. A plain stack of `amount` millibuckets of `fluid`, with no tag (`tag` is `None`).

```rust
use pluto_lib::{FluidStack, Identifier};

let stack = FluidStack::new(Identifier::parse("minecraft:water").unwrap(), 250);
assert_eq!(stack.amount, 250);
assert_eq!(stack.fluid.to_string(), "minecraft:water");
assert!(stack.tag.is_none());
```

### `FluidStack::same_fluid`

`fn same_fluid(&self, other: &FluidStack) -> bool`. Whether both stacks hold the same fluid with the same tag; the amounts may differ, so a tagged stack and a plain stack of one fluid are not the same fluid.

```rust
use pluto_lib::{FluidStack, Identifier, NbtCompound};

let water = Identifier::parse("minecraft:water").unwrap();
let plain = FluidStack::new(water.clone(), 100);
assert!(plain.same_fluid(&FluidStack::new(water.clone(), 900)));
let tagged = FluidStack { fluid: water, amount: 100, tag: Some(NbtCompound::new()) };
assert!(!plain.same_fluid(&tagged));
```

### `BlockEntityPlace`

What a placement hook receives, once, right after the block entity exists. `pos` is its position, `stack` the stack that was placed (its count is the hand's, not yet shrunk), `player` the player who placed it (`None` when the game names none), `data` the saved data it starts with and `items` its item slots; changes to `data` and `items` are saved with the chunk. A block placed by the game itself is not heard.

```rust
use pluto_lib::{BlockEntityPlace, Identifier, ItemSlot, Nbt, NbtCompound};

let (mut data, mut items) = (NbtCompound::new(), vec![None]);
let mut placed = BlockEntityPlace {
    pos: [1, 64, 2],
    stack: ItemSlot::new(Identifier::parse("example:tank").unwrap(), 1),
    player: None,
    data: &mut data,
    items: &mut items,
};
assert_eq!(placed.pos, [1, 64, 2]);
assert!(placed.player.is_none());
assert_eq!(placed.stack.item.to_string(), "example:tank");
placed.data.insert("seeded".to_owned(), Nbt::Byte(1));
assert_eq!(placed.data.len(), 1);
assert_eq!(placed.items.len(), 1);
```

### `BlockEntityPlace::fluid`

`fn fluid(&self, index: usize) -> Option<FluidStack>`. The fluid in tank `index` of the data the new block entity starts with, or `None` if that tank is empty.

```rust
use pluto_lib::{BlockEntityPlace, FluidStack, Identifier, ItemSlot, NbtCompound};

let (mut data, mut items) = (NbtCompound::new(), Vec::new());
let mut placed = BlockEntityPlace {
    pos: [0; 3],
    stack: ItemSlot::new(Identifier::parse("example:tank").unwrap(), 1),
    player: None,
    data: &mut data,
    items: &mut items,
};
assert_eq!(placed.fluid(0), None);
placed.set_fluid(0, Some(FluidStack::new(Identifier::parse("minecraft:lava").unwrap(), 125)));
assert_eq!(placed.fluid(0).unwrap().amount, 125);
```

### `BlockEntityPlace::set_fluid`

`fn set_fluid(&mut self, index: usize, stack: Option<FluidStack>)`. Puts `stack` in tank `index`, or empties the tank with `None`: a placed tank item's fluid seeding the tank is the usual use.

```rust
use pluto_lib::{BlockEntityPlace, FluidStack, Identifier, ItemSlot, NbtCompound};

let (mut data, mut items) = (NbtCompound::new(), Vec::new());
let mut placed = BlockEntityPlace {
    pos: [0; 3],
    stack: ItemSlot::new(Identifier::parse("example:tank").unwrap(), 1),
    player: None,
    data: &mut data,
    items: &mut items,
};
placed.set_fluid(0, Some(FluidStack::new(Identifier::parse("minecraft:water").unwrap(), 500)));
assert_eq!(placed.fluid(0).unwrap().amount, 500);
placed.set_fluid(0, None);
assert_eq!(placed.fluid(0), None);
```

### `BlockEntitySignal`

What a signal hook receives when the redstone signal reaching the block entity changes. `pos` is its position, `data` its saved data, `items` its item slots, `strength` the signal strength now (0 to 15, the strongest reaching the block from its six neighbours) and `powered` whether any signal reaches it (`strength` above 0). Changes to `data` and `items` are saved with the chunk.

```rust
use pluto_lib::{BlockEntitySignal, Nbt, NbtCompound};

let (mut data, mut items) = (NbtCompound::new(), Vec::new());
let mut signal = BlockEntitySignal { pos: [0, 64, 0], data: &mut data, items: &mut items, strength: 15, powered: true };
assert_eq!(signal.pos, [0, 64, 0]);
assert_eq!(signal.strength, 15);
assert!(signal.powered);
signal.data.insert("last_signal".to_owned(), Nbt::Int(i32::from(signal.strength)));
assert_eq!(signal.data.len(), 1);
assert!(signal.items.is_empty());
```

### `BlockEntitySignal::fluid`

`fn fluid(&self, index: usize) -> Option<FluidStack>`. The fluid in tank `index`, or `None` if that tank is empty.

```rust
use pluto_lib::{BlockEntitySignal, FluidStack, Identifier, NbtCompound};

let (mut data, mut items) = (NbtCompound::new(), Vec::new());
let mut signal = BlockEntitySignal { pos: [0; 3], data: &mut data, items: &mut items, strength: 0, powered: false };
assert_eq!(signal.fluid(0), None);
signal.set_fluid(0, Some(FluidStack::new(Identifier::parse("minecraft:water").unwrap(), 75)));
assert_eq!(signal.fluid(0).unwrap().amount, 75);
```

### `BlockEntitySignal::set_fluid`

`fn set_fluid(&mut self, index: usize, stack: Option<FluidStack>)`. Puts `stack` in tank `index`, or empties the tank with `None`.

```rust
use pluto_lib::{BlockEntitySignal, FluidStack, Identifier, NbtCompound};

let (mut data, mut items) = (NbtCompound::new(), Vec::new());
let mut signal = BlockEntitySignal { pos: [0; 3], data: &mut data, items: &mut items, strength: 3, powered: true };
signal.set_fluid(0, Some(FluidStack::new(Identifier::parse("minecraft:lava").unwrap(), 250)));
assert_eq!(signal.fluid(0).unwrap().amount, 250);
signal.set_fluid(0, None);
assert_eq!(signal.fluid(0), None);
```

### `ItemFilter`

Which items a block that forwards its item storage lets through: `Any` lets every item the target holds or takes through, `Only(item)` refuses inserts of another item and extractions of another item (only slots holding it, or empty ones for listing, are listed), and `Closed` lets nothing move and lists no slots.

```rust
use pluto_lib::{Identifier, ItemFilter};

let buckets = ItemFilter::Only(Identifier::parse("minecraft:bucket").unwrap());
assert_ne!(buckets, ItemFilter::Any);
assert_ne!(ItemFilter::Closed, ItemFilter::Any);
```

### `ItemForward`

Where a forwarding block's item storage really is, and what passes through it: `target` is the position of the block entity whose item slots answer for this block, and `filter` is the `ItemFilter` that narrows what passes.

```rust
use pluto_lib::{ItemFilter, ItemForward};

let forward = ItemForward { target: [0, 64, 0], filter: ItemFilter::Any };
assert_eq!((forward.target, forward.filter), ([0, 64, 0], ItemFilter::Any));
```

### `SidedAccess`

The slots one face of a block opens to automation: `insert` lists the slots that take items through the face and `take` lists the slots that give items through it. The default is two empty lists.

```rust
use pluto_lib::SidedAccess;

let top = SidedAccess { insert: vec![0], take: vec![] };
assert_eq!(top.insert, vec![0]);
assert!(top.take.is_empty());
assert_eq!(SidedAccess::default(), SidedAccess { insert: Vec::new(), take: Vec::new() });
```

### `TransferMode`

Which way an item transfer moves, for `BlockEntityTransfer`: `Insert` moves into the block entity (a hopper above, another block pushing) and `Take` moves out of it (a hopper below, another block pulling).

```rust
use pluto_lib::TransferMode;

assert_ne!(TransferMode::Insert, TransferMode::Take);
let mode = TransferMode::Insert;
assert_eq!(mode, TransferMode::Insert);
```

### `BlockEntityTransfer`

An item transfer automation wants to make, as a transfer rule is asked about it. `pos` is the block entity's position, `data` its saved data (read-only; the rule decides with it), `items` its item slots (read-only), `slot` the slot the stack would move through, `face` the face the transfer comes through (`None` when the caller names none), `mode` which way the stack moves and `stack` the stack that would move: the incoming one for an insert, the stored one for a take.

```rust
use pluto_lib::{BlockEntityTransfer, Direction, Identifier, ItemSlot, NbtCompound, TransferMode};

let data = NbtCompound::new();
let items: Vec<Option<ItemSlot>> = vec![None];
let stack = ItemSlot::new(Identifier::parse("minecraft:bucket").unwrap(), 1);
let transfer = BlockEntityTransfer {
    pos: [0, 64, 0],
    data: &data,
    items: &items,
    slot: 0,
    face: Some(Direction::Up),
    mode: TransferMode::Insert,
    stack,
};
assert_eq!(transfer.pos, [0, 64, 0]);
assert_eq!(transfer.slot, 0);
assert_eq!(transfer.face, Some(Direction::Up));
assert_eq!(transfer.mode, TransferMode::Insert);
assert_eq!(transfer.stack.item.to_string(), "minecraft:bucket");
assert!(transfer.data.is_empty() && transfer.items.len() == 1);
```

### `BlockEntityType::on_place`

`fn on_place(self, placer: impl Fn(&mut BlockEntityPlace<'_>) + Send + Sync + 'static) -> BlockEntityType`. The same type, with `placer` run once when a player places the block: it seeds the new block entity from the placed stack (a tank item's fluid, a custom name, a retexture). Without one a placed block entity starts with the type's initial data alone, and what the stack carried is lost.

```rust
use pluto_lib::{BlockEntityType, FluidStack, Identifier, NbtCompoundExt};

let kind = BlockEntityType::new(Identifier::parse("example:placed_tank").unwrap())
    .item_slots(1)
    .fluid_tanks([1000])
    .on_place(|placed| {
        // a tank item keeps its fluid amount in its custom data; the placed tank starts with it
        let amount = placed.stack.custom_data.get_int("amount", 0);
        if amount > 0 {
            placed.set_fluid(0, Some(FluidStack::new(Identifier::parse("minecraft:lava").unwrap(), amount as u32)));
        }
    });
let _ = kind;
```

### `BlockEntityType::on_signal`

`fn on_signal(self, signaller: impl Fn(&mut BlockEntitySignal<'_>) + Send + Sync + 'static) -> BlockEntityType`. The same type, with `signaller` run when the redstone signal reaching the block changes (see `BlockEntitySignal`). The type must also have a ticker: the change is noticed on the block entity's own tick.

```rust
use pluto_lib::{BlockEntityType, Identifier, Nbt};

let kind = BlockEntityType::new(Identifier::parse("example:swapping_tank").unwrap())
    .item_slots(2)
    .ticker(|_tick| {})
    .on_signal(|signal| {
        signal.data.insert("last_signal".to_owned(), Nbt::Int(i32::from(signal.strength)));
        if signal.powered && signal.items[0].is_some() && signal.items[1].is_none() {
            signal.items[1] = signal.items[0].take();
        }
    });
let _ = kind;
```

### `BlockEntityType::item_forward`

`fn item_forward(self, forward: impl Fn(&NbtCompound, &[Option<ItemSlot>]) -> Option<ItemForward> + Send + Sync + 'static) -> BlockEntityType`. The same type, whose item storage is another block entity's (a chute in a smeltery wall answers with the controller's inventory): `forward` is asked with the saved data and item slots, and `None` means this block has no storage of its own either. The `ItemFilter` narrows what passes.

```rust
use pluto_lib::{BlockEntityType, Identifier, ItemFilter, ItemForward, Nbt};

// a chute remembers its controller's position in its data
let chute = BlockEntityType::new(Identifier::parse("example:chute").unwrap()).item_forward(|data, _items| match data.get("master") {
    Some(Nbt::IntArray(at)) if at.len() == 3 => Some(ItemForward { target: [at[0], at[1], at[2]], filter: ItemFilter::Any }),
    _ => None,
});
let _ = chute;
```

### `BlockEntityType::sided_access`

`fn sided_access(self, access: impl Fn(Direction) -> SidedAccess + Send + Sync + 'static) -> BlockEntityType`. The same type, with the slots each face of the block opens to automation (see `SidedAccess`): `access` answers for a face which slots take items and which give them through it. Without one every slot takes and gives through every face.

```rust
use pluto_lib::{BlockEntityType, Direction, Identifier, SidedAccess};

let kind = BlockEntityType::new(Identifier::parse("example:casting_table").unwrap())
    .item_slots(2)
    .sided_access(|face| match face {
        Direction::Up => SidedAccess { insert: vec![0], take: vec![] },
        Direction::Down => SidedAccess { insert: vec![], take: vec![1] },
        _ => SidedAccess::default(),
    });
let _ = kind;
```

### `BlockEntityType::item_transfer_rule`

`fn item_transfer_rule(self, rule: impl Fn(&BlockEntityTransfer<'_>) -> bool + Send + Sync + 'static) -> BlockEntityType`. The same type, with its own rule for the item transfers automation makes with it (see `BlockEntityTransfer`): `rule` is asked for each stack a hopper or another block would move in or out, with the saved data and slots at hand, and a stack it refuses does not move. Without one every transfer the faces open is allowed.

```rust
use pluto_lib::{BlockEntityType, Identifier, TransferMode};

let kind = BlockEntityType::new(Identifier::parse("example:casting_table").unwrap())
    .item_slots(2)
    .item_transfer_rule(|transfer| {
        // nothing goes in while a cast is running, and only fluid containers go in at all
        if transfer.mode == TransferMode::Insert && transfer.data.contains_key("casting") {
            return false;
        }
        transfer.mode == TransferMode::Take || transfer.stack.item.to_string().ends_with("bucket")
    });
let _ = kind;
```

### `BlockEntityTypeRegistry::item_forward_of`

`fn item_forward_of(block: &str, data: &NbtCompound, items: &[Option<ItemSlot>]) -> Option<ItemForward>`. Where the item storage of the block entity of `block` really is, from its `item_forward` hook; `None` when the block has no hook or the hook says it forwards nowhere.

```rust
use pluto_lib::{BlockEntityType, BlockEntityTypeRegistry, Identifier, ItemFilter, ItemForward, NbtCompound};

BlockEntityTypeRegistry::register(BlockEntityType::new(Identifier::parse("example:item_chute").unwrap())
    .item_forward(|_data, _items| Some(ItemForward { target: [0, 64, 0], filter: ItemFilter::Any })));
let forward = BlockEntityTypeRegistry::item_forward_of("example:item_chute", &NbtCompound::new(), &[]).unwrap();
assert_eq!(forward.target, [0, 64, 0]);
assert_eq!(forward.filter, ItemFilter::Any);
assert!(BlockEntityTypeRegistry::item_forward_of("example:no_such_block", &NbtCompound::new(), &[]).is_none());
```

### `BlockEntityTypeRegistry::item_forwards`

`fn item_forwards(block: &str) -> bool`. Whether the block has an item forwarding hook, so that a `None` from `item_forward_of` means "no storage" and not "the block's own storage".

```rust
use pluto_lib::{BlockEntityType, BlockEntityTypeRegistry, Identifier, ItemFilter, ItemForward};

BlockEntityTypeRegistry::register(BlockEntityType::new(Identifier::parse("example:forwards_items").unwrap())
    .item_forward(|_data, _items| Some(ItemForward { target: [1, 64, 1], filter: ItemFilter::Any })));
assert!(BlockEntityTypeRegistry::item_forwards("example:forwards_items"));
assert!(!BlockEntityTypeRegistry::item_forwards("example:no_such_block"));
```

### `BlockEntityTypeRegistry::sided_access`

`fn sided_access(block: &str, face: Direction) -> Option<SidedAccess>`. The slots `face` of the block entity of `block` opens to automation, from its `sided_access` hook; `None` when the block has no hook (every slot then opens both ways).

```rust
use pluto_lib::{BlockEntityType, BlockEntityTypeRegistry, Direction, Identifier, SidedAccess};

BlockEntityTypeRegistry::register(BlockEntityType::new(Identifier::parse("example:sided").unwrap())
    .item_slots(2)
    .sided_access(|face| match face {
        Direction::Up => SidedAccess { insert: vec![0], take: vec![] },
        Direction::Down => SidedAccess { insert: vec![], take: vec![1] },
        _ => SidedAccess::default(),
    }));
assert_eq!(BlockEntityTypeRegistry::sided_access("example:sided", Direction::Up), Some(SidedAccess { insert: vec![0], take: vec![] }));
assert_eq!(BlockEntityTypeRegistry::sided_access("example:sided", Direction::Down), Some(SidedAccess { insert: vec![], take: vec![1] }));
assert!(BlockEntityTypeRegistry::sided_access("example:no_such_block", Direction::Up).is_none());
```

### `BlockEntityTypeRegistry::item_transfer_allowed`

`fn item_transfer_allowed(block: &str, pos: BlockPos, data: &NbtCompound, items: &[Option<ItemSlot>], slot: usize, face: Option<Direction>, mode: TransferMode, stack: ItemSlot) -> bool`. Whether the `item_transfer_rule` of `block` allows the transfer described by the arguments; true when the block has no rule.

```rust
use pluto_lib::{BlockEntityType, BlockEntityTypeRegistry, Direction, Identifier, ItemSlot, Nbt, NbtCompound, TransferMode};

BlockEntityTypeRegistry::register(BlockEntityType::new(Identifier::parse("example:guarded_table").unwrap())
    .item_slots(2)
    .item_transfer_rule(|transfer| {
        if transfer.mode == TransferMode::Insert && transfer.data.contains_key("casting") {
            return false;
        }
        transfer.mode == TransferMode::Take || transfer.stack.item.to_string().ends_with("bucket")
    }));
let items: Vec<Option<ItemSlot>> = vec![None, None];
let mut data = NbtCompound::new();
let bucket = ItemSlot::new(Identifier::parse("minecraft:bucket").unwrap(), 1);
let stick = ItemSlot::new(Identifier::parse("minecraft:stick").unwrap(), 1);
assert!(BlockEntityTypeRegistry::item_transfer_allowed("example:guarded_table", [0; 3], &data, &items, 0, Some(Direction::Up), TransferMode::Insert, bucket.clone()));
assert!(!BlockEntityTypeRegistry::item_transfer_allowed("example:guarded_table", [0; 3], &data, &items, 0, Some(Direction::Up), TransferMode::Insert, stick));
data.insert("casting".to_owned(), Nbt::Byte(1));
assert!(!BlockEntityTypeRegistry::item_transfer_allowed("example:guarded_table", [0; 3], &data, &items, 0, Some(Direction::Up), TransferMode::Insert, bucket));
assert!(BlockEntityTypeRegistry::item_transfer_allowed("example:unregistered", [0; 3], &data, &items, 0, None, TransferMode::Insert, ItemSlot::new(Identifier::parse("minecraft:stone").unwrap(), 1)));
```
