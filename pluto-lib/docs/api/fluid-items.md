# Fluid in items and in the world

Two things a mod fluid needs besides tanks: fluid inside item stacks (buckets, bottles, cans, tank items) and the fluid as blocks in the world. Fluid in an item lives in the stack's own custom data, so it needs no host feature; every transfer is a function from a stack to the stack afterwards. The world fluid is one block of the mod with a `level` state, flowed by PlutoLib through the block behaviour hooks.

## Basic Example

A copper can that holds exactly one ingot of any fluid, filled from a tank and emptied into one:

```rust
use pluto_lib::{FluidItemDef, FluidItems, FluidStack, FluidTarget, Identifier, ItemSlot, TransferDirection};

struct Pool(Option<FluidStack>);
impl FluidTarget for Pool {
    fn fill(&mut self, fluid: &FluidStack, execute: bool) -> u32 {
        if self.0.is_some() { return 0; }
        if execute { self.0 = Some(fluid.clone()); }
        fluid.amount
    }
    fn drain(&mut self, wanted: Option<&Identifier>, amount: u32, execute: bool) -> Option<FluidStack> {
        let held = self.0.as_ref()?;
        if wanted.is_some_and(|wanted| wanted != &held.fluid) { return None; }
        let out = FluidStack { fluid: held.fluid.clone(), amount: held.amount.min(amount), tag: held.tag.clone() };
        if execute { self.0 = None; }
        Some(out)
    }
}

let id = |text: &str| Identifier::parse(text).unwrap();
FluidItems::register(FluidItemDef::portion(id("example:can"), "fluid", 90));
let mut pool = Pool(Some(FluidStack { fluid: id("example:molten_iron"), amount: 90, tag: None }));
let done = FluidItems::interact(Some(&ItemSlot::new(id("example:can"), 1)), &mut pool, TransferDirection::Auto).unwrap();
assert!(!done.into_tank, "the empty can filled from the pool");
assert_eq!(FluidItems::contents(done.held.as_ref().unwrap()).unwrap().amount, 90);
assert!(pool.0.is_none());
```

> **Use with Caution.** Source-ready and pure-tested only; the native host adapter is not compiled. The stack's fluid is plain custom data, so a host that keeps custom data keeps the fluid. A per-stack maximum stack size (a filled tank stacks to 16, an empty one to 64) is an item behaviour, not a container property: register it with `ItemBehavior::max_stack_size` (see items.md); menu stacking honours it, and the game's own stacking awaits a host seam (see the support matrix).

## Kinds of container

`FluidItemDef::tank` holds any one fluid up to a capacity (`scaled` makes a stack of n one tank of n times the capacity, as Forge's `ScaledFluidTank`); `portion` holds exactly one fixed amount (a can); `constant` always holds one fluid (a bottle of venom, emptied into its glass bottle); `empty_container` is the empty counterpart that becomes a filled item when a matching fluid goes in.

```rust
use pluto_lib::{FluidItemDef, FluidItems, FluidStack, Identifier, ItemSlot};

let id = |text: &str| Identifier::parse(text).unwrap();
FluidItems::register(FluidItemDef::constant(id("example:venom_bottle"), id("example:venom"), 250, Some(id("minecraft:glass_bottle"))));
FluidItems::register(FluidItemDef::empty_container(id("minecraft:glass_bottle"), |fluid| fluid.path() == "venom", 250, id("example:venom_bottle")));
let filled = FluidItems::fill(&ItemSlot::new(id("minecraft:glass_bottle"), 1), &FluidStack { fluid: id("example:venom"), amount: 1000, tag: None });
assert_eq!((filled.amount, filled.container.item.to_string()), (250, "example:venom_bottle".to_owned()));
```

## Buckets and world fluids

`FluidDef::with_bucket` registers `<id>_bucket` as a 1000 container that becomes `minecraft:bucket`, and the empty bucket fills with it. With `with_world_blocks` the fluid's block `<id>_fluid` (declared by the mod with the `level` state `0` to `8`, a shape only on the source, no collision) flows by the rules of `FlowingFluid` and the buckets place and pick up the source.

```rust
use pluto_lib::{BlockBehaviors, FluidDef, FluidRegistry, FluidWorld, Identifier, MemoryBlockView};

let id = Identifier::parse("example:molten_gold").unwrap();
FluidRegistry::register(FluidDef::new(id.clone()).temperature(1000).with_world_blocks().with_bucket().flow(30, 3, 3));
assert!(BlockBehaviors::get("example:molten_gold_fluid").is_some());

let def = FluidRegistry::get("example:molten_gold").unwrap();
let mut world = MemoryBlockView::default();
world.blocks.insert([0, 0, 0], pluto_lib::BlockState::parse("minecraft:stone").unwrap());
assert!(FluidWorld::place_source(&mut world, [0, 1, 0], &def));
FluidWorld::tick(&mut world, [0, 1, 0], &def);
assert_eq!(world.blocks[&[1, 1, 0]].property("level"), Some("3"), "a drop of 3 leaves amount 5 beside the source");
```

> **Use with Caution.** Pure-tested flow, buckets and container rules; no native build or game run. Not modelled: swimming, buoyancy, fog and overlays, entity contact effects (needs a hook that can act on the entity inside), mixing with water or lava, infinite sources, and waterlogging. A source the bucket places asks for its first tick with `World::schedule_tick`, which needs a host adapter; until then it starts flowing at its first random tick or when a neighbour changes. An empty bucket reaches a source because the source block has a shape. A bucket placed over a replaceable block only replaces air and the common plants.

## Reference

### `FluidItemDef`

One kind of item that holds fluid. `item` is the item it describes.

```rust
use pluto_lib::{FluidItemDef, Identifier};

let def = FluidItemDef::tank(Identifier::parse("example:tank").unwrap(), "tank", 4000);
assert_eq!(def.item.to_string(), "example:tank");
```

### `FluidItemDef::tank`

`fn tank(item: Identifier, key: &str, capacity: u32) -> FluidItemDef`. Any one fluid up to `capacity` millibuckets, kept under `key` in the stack's custom data as `{FluidName, Amount}`.

```rust
use pluto_lib::{FluidItemDef, Identifier};

assert_eq!(FluidItemDef::tank(Identifier::parse("example:tank").unwrap(), "tank", 1000).key(), Some("tank"));
```

### `FluidItemDef::portion`

`fn portion(item: Identifier, key: &str, amount: u32) -> FluidItemDef`. Exactly `amount` of one fluid or nothing, kept as the fluid id under `key`; a stack of several holds `amount` each.

```rust
use pluto_lib::{FluidItemDef, Identifier};

assert_eq!(FluidItemDef::portion(Identifier::parse("example:can").unwrap(), "fluid", 90).key(), Some("fluid"));
```

### `FluidItemDef::constant`

`fn constant(item: Identifier, fluid: Identifier, amount: u32, empty: Option<Identifier>) -> FluidItemDef`. An item that always holds `amount` of `fluid`; drained, it becomes `empty` or vanishes.

```rust
use pluto_lib::{FluidItemDef, Identifier};

let id = |text: &str| Identifier::parse(text).unwrap();
assert_eq!(FluidItemDef::constant(id("example:bottle"), id("example:honey"), 250, None).key(), None);
```

### `FluidItemDef::empty_container`

`fn empty_container(item, accepts: impl Fn(&Identifier) -> bool, amount: u32, filled: Identifier) -> FluidItemDef`. An empty item that becomes `filled` when `amount` of an accepted fluid goes in.

```rust
use pluto_lib::{FluidItemDef, Identifier};

let id = |text: &str| Identifier::parse(text).unwrap();
let bowl = FluidItemDef::empty_container(id("minecraft:bowl"), |fluid| fluid.path() == "soup", 250, id("example:soup_bowl"));
assert_eq!(bowl.item.to_string(), "minecraft:bowl");
```

### `FluidItemDef::fills`

`fn fills(self, accepts, amount: u32, filled: Identifier) -> FluidItemDef`. One more filling of the same empty item, tried after the earlier ones.

```rust
use pluto_lib::{FluidItemDef, Identifier};

let id = |text: &str| Identifier::parse(text).unwrap();
let bottle = FluidItemDef::empty_container(id("minecraft:glass_bottle"), |f| f.path() == "a", 250, id("example:a_bottle"))
    .fills(|f| f.path() == "b", 250, id("example:b_bottle"));
assert_eq!(bottle.item.path(), "glass_bottle");
```

### `FluidItemDef::scaled`

`fn scaled(self) -> FluidItemDef`. A tank whose stack of n items is one tank of n times the capacity; amounts are kept per item and transfers move in steps of n.

```rust
use pluto_lib::{FluidItemDef, FluidItems, FluidStack, Identifier, ItemSlot};

let id = |text: &str| Identifier::parse(text).unwrap();
FluidItems::register(FluidItemDef::tank(id("example:scaled"), "tank", 1000).scaled());
let filled = FluidItems::fill(&ItemSlot::new(id("example:scaled"), 4), &FluidStack { fluid: id("minecraft:lava"), amount: 4002, tag: None });
assert_eq!(filled.amount, 4000);
```

### `FluidItemDef::fill_stack_limit`

`fn fill_stack_limit(self, limit: u32) -> FluidItemDef`. A tank that takes no fluid while the stack holds more than `limit` items; draining is unaffected.

```rust
use pluto_lib::{FluidItemDef, FluidItems, FluidStack, Identifier, ItemSlot};

let id = |text: &str| Identifier::parse(text).unwrap();
FluidItems::register(FluidItemDef::tank(id("example:limited"), "tank", 1000).scaled().fill_stack_limit(16));
assert_eq!(FluidItems::fill(&ItemSlot::new(id("example:limited"), 17), &FluidStack { fluid: id("minecraft:lava"), amount: 17, tag: None }).amount, 0);
```

### `FluidItemDef::key`

`fn key(&self) -> Option<&str>`. The custom data key that holds the fluid; `None` for constant and empty containers.

```rust
use pluto_lib::{FluidItemDef, Identifier};

assert_eq!(FluidItemDef::tank(Identifier::parse("example:t").unwrap(), "tank", 10).key(), Some("tank"));
```

### `Filled`

What a fill did: `amount` that went in and the `container` stack afterwards (same count, possibly another item).

```rust
use pluto_lib::{FluidItemDef, FluidItems, FluidStack, Identifier, ItemSlot};

let id = |text: &str| Identifier::parse(text).unwrap();
FluidItems::register(FluidItemDef::tank(id("example:f"), "tank", 100));
let filled = FluidItems::fill(&ItemSlot::new(id("example:f"), 1), &FluidStack { fluid: id("minecraft:water"), amount: 30, tag: None });
assert_eq!((filled.amount, filled.container.item.path()), (30, "f"));
```

### `Drained`

What a drain did: the `fluid` that came out and the `container` afterwards (`None` when nothing is left of the stack).

```rust
use pluto_lib::{FluidItemDef, FluidItems, Identifier, ItemSlot};

let id = |text: &str| Identifier::parse(text).unwrap();
FluidItems::register(FluidItemDef::constant(id("example:jar"), id("example:jam"), 100, None));
let drained = FluidItems::drain(&ItemSlot::new(id("example:jar"), 1), None, 100).unwrap();
assert_eq!(drained.fluid.amount, 100);
assert!(drained.container.is_none());
```

### `FluidTarget`

A tank to trade with: `fill` and `drain`, each with an `execute` flag that false turns into a question. See the Basic Example.

```rust
use pluto_lib::{FluidStack, FluidTarget, Identifier};

struct Nothing;
impl FluidTarget for Nothing {
    fn fill(&mut self, _: &FluidStack, _: bool) -> u32 { 0 }
    fn drain(&mut self, _: Option<&Identifier>, _: u32, _: bool) -> Option<FluidStack> { None }
}
assert_eq!(Nothing.fill(&FluidStack { fluid: Identifier::parse("minecraft:water").unwrap(), amount: 1, tag: None }, false), 0);
```

### `TransferDirection`

Which way a held stack and a tank trade: `Auto` (stack into tank first, otherwise tank into stack), `ToTank`, `FromTank`.

```rust
use pluto_lib::TransferDirection;

assert_ne!(TransferDirection::ToTank, TransferDirection::FromTank);
assert_eq!(TransferDirection::Auto, TransferDirection::Auto);
```

### `Interaction`

What a trade did: `held` (the hand afterwards), `give` (stacks for the player), `moved` (the fluid) and `into_tank` (whether it went into the tank).

```rust
use pluto_lib::{FluidItemDef, FluidItems, FluidStack, FluidTarget, Identifier, ItemSlot, TransferDirection};

struct Sink;
impl FluidTarget for Sink {
    fn fill(&mut self, fluid: &FluidStack, _: bool) -> u32 { fluid.amount }
    fn drain(&mut self, _: Option<&Identifier>, _: u32, _: bool) -> Option<FluidStack> { None }
}
let id = |text: &str| Identifier::parse(text).unwrap();
FluidItems::register(FluidItemDef::constant(id("example:flask"), id("example:ink"), 250, Some(id("example:empty_flask"))));
let done = FluidItems::interact(Some(&ItemSlot::new(id("example:flask"), 1)), &mut Sink, TransferDirection::Auto).unwrap();
assert!(done.into_tank && done.give.is_empty());
assert_eq!((done.moved.amount, done.held.unwrap().item.path().to_owned()), (250, "empty_flask".to_owned()));
```

### `FluidItems`

The registry and the transfers.

```rust
use pluto_lib::{FluidItemDef, FluidItems, Identifier};

FluidItems::register(FluidItemDef::tank(Identifier::parse("example:reg").unwrap(), "tank", 10));
assert!(FluidItems::get("example:reg").is_some());
```

### `FluidItems::register`

`fn register(def: FluidItemDef)`. Makes the item a container; the same item again replaces the earlier definition.

```rust
use pluto_lib::{FluidItemDef, FluidItems, Identifier};

FluidItems::register(FluidItemDef::tank(Identifier::parse("example:again").unwrap(), "tank", 10));
FluidItems::register(FluidItemDef::tank(Identifier::parse("example:again").unwrap(), "tank", 20));
```

### `FluidItems::get`

`fn get(item: &str) -> Option<FluidItemDef>`. The definition registered for an item id.

```rust
use pluto_lib::FluidItems;

assert!(FluidItems::get("example:never_registered").is_none());
```

### `FluidItems::is_container`

`fn is_container(stack: &ItemSlot) -> bool`. Whether the stack's item holds or can be filled with fluid.

```rust
use pluto_lib::{FluidItems, Identifier, ItemSlot};

assert!(!FluidItems::is_container(&ItemSlot::new(Identifier::parse("minecraft:stick").unwrap(), 1)));
```

### `FluidItems::contents`

`fn contents(stack: &ItemSlot) -> Option<FluidStack>`. What the stack holds, the whole stack's amount for a scaled tank or a stack of portions.

```rust
use pluto_lib::{FluidItemDef, FluidItems, Identifier, ItemSlot};

let id = |text: &str| Identifier::parse(text).unwrap();
FluidItems::register(FluidItemDef::constant(id("example:c"), id("example:oil"), 50, None));
assert_eq!(FluidItems::contents(&ItemSlot::new(id("example:c"), 1)).unwrap().amount, 50);
```

### `FluidItems::capacity`

`fn capacity(stack: &ItemSlot) -> u32`. How much the stack holds at most; 0 for an item that is not a container.

```rust
use pluto_lib::{FluidItemDef, FluidItems, Identifier, ItemSlot};

let id = |text: &str| Identifier::parse(text).unwrap();
FluidItems::register(FluidItemDef::tank(id("example:cap"), "tank", 4000));
assert_eq!(FluidItems::capacity(&ItemSlot::new(id("example:cap"), 1)), 4000);
assert_eq!(FluidItems::capacity(&ItemSlot::new(id("minecraft:stick"), 1)), 0);
```

### `FluidItems::fill`

`fn fill(stack: &ItemSlot, fluid: &FluidStack) -> Filled`. The stack with fluid put in as far as it goes; asking and doing are the same call.

```rust
use pluto_lib::{FluidItemDef, FluidItems, FluidStack, Identifier, ItemSlot};

let id = |text: &str| Identifier::parse(text).unwrap();
FluidItems::register(FluidItemDef::tank(id("example:fill"), "tank", 100));
let filled = FluidItems::fill(&ItemSlot::new(id("example:fill"), 1), &FluidStack { fluid: id("minecraft:lava"), amount: 500, tag: None });
assert_eq!(filled.amount, 100);
```

### `FluidItems::drain`

`fn drain(stack: &ItemSlot, wanted: Option<&Identifier>, max: u32) -> Option<Drained>`. The fluid the stack gives up to `max`; a portion or constant gives all of it or nothing.

```rust
use pluto_lib::{FluidItemDef, FluidItems, FluidStack, Identifier, ItemSlot};

let id = |text: &str| Identifier::parse(text).unwrap();
FluidItems::register(FluidItemDef::tank(id("example:drain"), "tank", 100));
let full = FluidItems::fill(&ItemSlot::new(id("example:drain"), 1), &FluidStack { fluid: id("minecraft:lava"), amount: 100, tag: None }).container;
assert_eq!(FluidItems::drain(&full, None, 30).unwrap().fluid.amount, 30);
```

### `FluidItems::interact`

`fn interact(held: Option<&ItemSlot>, target: &mut dyn FluidTarget, direction: TransferDirection) -> Option<Interaction>`. A trade between the held stack and a tank (`FluidUtil.interactWithFluidHandler`). A container that is not a scaled tank trades one at a time, splitting one off a stack and giving its result in `give`.

```rust
use pluto_lib::{FluidItems, FluidStack, FluidTarget, Identifier, TransferDirection};

struct Empty;
impl FluidTarget for Empty {
    fn fill(&mut self, _: &FluidStack, _: bool) -> u32 { 0 }
    fn drain(&mut self, _: Option<&Identifier>, _: u32, _: bool) -> Option<FluidStack> { None }
}
assert!(FluidItems::interact(None, &mut Empty, TransferDirection::Auto).is_none());
```

### `FluidFlow`

How a fluid flows: `tick_delay` game ticks per step, `level_drop` levels lost per block, `slope_distance` blocks searched for a way down.

```rust
use pluto_lib::FluidFlow;

let flow = FluidFlow { tick_delay: 30, level_drop: 3, slope_distance: 3 };
assert_ne!(flow, FluidFlow::standard());
```

### `FluidFlow::standard`

`const fn standard() -> FluidFlow`. Water-like flow: 5, 1 and 4.

```rust
use pluto_lib::FluidFlow;

assert_eq!(FluidFlow::standard().tick_delay, 5);
```

### `FluidDef::flow`

`fn flow(self, tick_delay: u16, level_drop: u8, slope_distance: u8) -> FluidDef`. How the fluid flows in the world.

```rust
use pluto_lib::{FluidDef, Identifier};

let def = FluidDef::new(Identifier::parse("example:slow").unwrap()).flow(40, 2, 2);
assert_eq!(def.flow.level_drop, 2);
```

### `FluidDef::bucket_sounds`

`fn bucket_sounds(self, fill: &str, empty: &str) -> FluidDef`. The sound events the bucket makes taking the fluid up and pouring it out.

```rust
use pluto_lib::{FluidDef, Identifier};

let def = FluidDef::new(Identifier::parse("example:hot").unwrap()).bucket_sounds("minecraft:item.bucket.fill_lava", "minecraft:item.bucket.empty_lava");
assert_eq!(def.bucket_empty_sound.as_deref(), Some("minecraft:item.bucket.empty_lava"));
assert!(def.bucket_fill_sound.is_some());
```

### `FluidWorld`

The rules of a mod fluid in the world.

```rust
use pluto_lib::{FluidWorld, Identifier};

assert_eq!(FluidWorld::bucket_id(&Identifier::parse("example:oil").unwrap()).to_string(), "example:oil_bucket");
```

### `FluidWorld::LEVEL`

The block state property that carries the level.

```rust
use pluto_lib::FluidWorld;

assert_eq!(FluidWorld::LEVEL, "level");
```

### `FluidWorld::BLOCK_LEVELS`

The values of the property: `0` source, `1` to `7` flowing, `8` falling.

```rust
use pluto_lib::FluidWorld;

assert_eq!(FluidWorld::BLOCK_LEVELS.len(), 9);
```

### `FluidWorld::state`

`fn state(fluid: &Identifier, level: u8) -> BlockState`. The fluid block at a level.

```rust
use pluto_lib::{FluidWorld, Identifier};

assert_eq!(FluidWorld::state(&Identifier::parse("example:oil").unwrap(), 2).to_string(), "example:oil_fluid[level=2]");
```

### `FluidWorld::block_id`

`fn block_id(fluid: &Identifier) -> Identifier`. The fluid's block id, `<id>_fluid` (so a fluid and a building block of the same name do not clash).

```rust
use pluto_lib::{FluidWorld, Identifier};

assert_eq!(FluidWorld::block_id(&Identifier::parse("a:b").unwrap()).path(), "b_fluid");
```

### `FluidWorld::bucket_id`

`fn bucket_id(fluid: &Identifier) -> Identifier`. The fluid's bucket item id, `<id>_bucket`.

```rust
use pluto_lib::{FluidWorld, Identifier};

assert_eq!(FluidWorld::bucket_id(&Identifier::parse("a:b").unwrap()).path(), "b_bucket");
```

### `FluidWorld::level_of`

`fn level_of(state: &BlockState, fluid: &Identifier) -> Option<u8>`. The level of a state of the fluid's block; `None` for another block or a level above 8.

```rust
use pluto_lib::{BlockState, FluidWorld, Identifier};

let oil = Identifier::parse("example:oil").unwrap();
assert_eq!(FluidWorld::level_of(&BlockState::parse("example:oil_fluid[level=5]").unwrap(), &oil), Some(5));
assert_eq!(FluidWorld::level_of(&BlockState::parse("minecraft:stone").unwrap(), &oil), None);
```

### `FluidWorld::is_world_fluid`

`fn is_world_fluid(block: &Identifier) -> bool`. Whether `block` is the block (`<id>_fluid`) of a defined fluid with world blocks.

```rust
use pluto_lib::{FluidWorld, Identifier};

assert!(!FluidWorld::is_world_fluid(&Identifier::parse("minecraft:stone").unwrap()));
```

### `FluidWorld::place_source`

`fn place_source(world: &mut dyn BlockEditor, pos: BlockPos, def: &FluidDef) -> bool`. Puts a source at `pos` and schedules its tick; only into air or a replaceable non-fluid block.

```rust
use pluto_lib::{FluidDef, FluidWorld, Identifier, MemoryBlockView};

let def = FluidDef::new(Identifier::parse("example:oil").unwrap());
let mut world = MemoryBlockView::default();
assert!(FluidWorld::place_source(&mut world, [0, 64, 0], &def));
assert!(!FluidWorld::place_source(&mut world, [0, 64, 0], &def));
```

### `FluidWorld::pick_up_source`

`fn pick_up_source(world: &mut dyn BlockEditor, pos: BlockPos, def: &FluidDef) -> bool`. Removes a source of this fluid; a flowing cell is refused.

```rust
use pluto_lib::{FluidDef, FluidWorld, Identifier, MemoryBlockView};

let def = FluidDef::new(Identifier::parse("example:oil").unwrap());
let mut world = MemoryBlockView::default();
FluidWorld::place_source(&mut world, [0, 64, 0], &def);
assert!(FluidWorld::pick_up_source(&mut world, [0, 64, 0], &def));
```

### `FluidWorld::tick`

`fn tick(world: &mut dyn BlockEditor, pos: BlockPos, def: &FluidDef)`. One scheduled tick of a cell (`FlowingFluid.tick`): a flowing cell settles to what its neighbours make of it, then the cell spreads down or toward the nearest drop; changed cells are scheduled again.

```rust
use pluto_lib::{FluidDef, FluidWorld, Identifier, MemoryBlockView};

let def = FluidDef::new(Identifier::parse("example:oil").unwrap());
let mut world = MemoryBlockView::default();
FluidWorld::place_source(&mut world, [0, 64, 0], &def);
FluidWorld::tick(&mut world, [0, 64, 0], &def);
assert!(world.blocks.contains_key(&[0, 63, 0]), "it falls");
```

### `FluidWorld::register_behaviour`

`fn register_behaviour(def: &FluidDef)`. Gives the fluid's block the flow behaviour; `FluidRegistry::register` calls it for a fluid with world blocks. Extend the registered behaviour with `BlockBehaviors::get` rather than replacing it.

```rust
use pluto_lib::{BlockBehaviors, FluidDef, FluidWorld, Identifier};

FluidWorld::register_behaviour(&FluidDef::new(Identifier::parse("example:tar").unwrap()));
assert!(BlockBehaviors::get("example:tar_fluid").is_some());
```

### `World::schedule_tick`

`fn schedule_tick(&self, pos: BlockPos, block: &str, delay: i32) -> bool`. Asks for the block at `pos` to tick after `delay` game ticks; false outside a callback, for a delay below 1, or when the host cannot schedule from this callback.

```rust
use pluto_lib::World;

assert!(!World::new("minecraft:overworld").schedule_tick([0, 0, 0], "example:oil", 5), "no world in a plain test");
```
### `FluidItemDef::tagged_constant`

`fn tagged_constant(item: Identifier, fluid: Identifier, amount: u32, empty: Option<Identifier>) -> FluidItemDef`. An item that always holds `amount` of `fluid` together with a tag: the stack's own custom data is the fluid's tag, so a stack with no custom data holds the plain fluid. Drained, it becomes `empty` or vanishes and hands the tag back with the fluid.

```rust
use pluto_lib::{FluidItemDef, FluidItems, Identifier, ItemSlot, Nbt, NbtCompound};

let id = |text: &str| Identifier::parse(text).unwrap();
FluidItems::register(FluidItemDef::tagged_constant(id("example:potion_bucket"), id("example:potion"), 1000, Some(id("minecraft:bucket"))));
let mut stack = ItemSlot::new(id("example:potion_bucket"), 1);
let mut tag = NbtCompound::new();
tag.insert("Potion".to_owned(), Nbt::String("minecraft:healing".to_owned()));
stack.custom_data = tag.clone();
let contents = FluidItems::contents(&stack).unwrap();
assert_eq!(contents.amount, 1000);
assert_eq!(contents.tag, Some(tag));
```

### `FluidItemDef::empty_container_tagged`

`fn empty_container_tagged(item: Identifier, accepts: impl Fn(&Identifier) -> bool + Send + Sync + 'static, amount: u32, filled: Identifier) -> FluidItemDef`. An empty item that becomes `filled` when `amount` of an accepted fluid goes in, taking the fluid's tag as the filled stack's own custom data.

```rust
use pluto_lib::{FluidItemDef, FluidItems, FluidStack, Identifier, ItemSlot, Nbt, NbtCompound};

let id = |text: &str| Identifier::parse(text).unwrap();
FluidItems::register(FluidItemDef::empty_container_tagged(id("example:empty_bucket"), |fluid| fluid.path() == "potion", 1000, id("example:potion_bucket")));
let mut tag = NbtCompound::new();
tag.insert("Potion".to_owned(), Nbt::String("minecraft:healing".to_owned()));
let filled = FluidItems::fill(&ItemSlot::new(id("example:empty_bucket"), 1), &FluidStack { fluid: id("example:potion"), amount: 1000, tag: Some(tag.clone()) });
assert_eq!(filled.amount, 1000);
assert_eq!(filled.container.item.to_string(), "example:potion_bucket");
assert_eq!(filled.container.custom_data, tag);
```

### `FluidItemDef::fills_tagged`

`fn fills_tagged(self, accepts: impl Fn(&Identifier) -> bool + Send + Sync + 'static, amount: u32, filled: Identifier) -> FluidItemDef`. One more filling of the same empty item, tried after the earlier ones, whose filled stack takes the fluid's tag as its own custom data. Does nothing to other kinds of item.

```rust
use pluto_lib::{FluidItemDef, FluidItems, FluidStack, Identifier, ItemSlot, Nbt, NbtCompound};

let id = |text: &str| Identifier::parse(text).unwrap();
FluidItems::register(FluidItemDef::empty_container(id("example:vial"), |f| f.path() == "water_like", 250, id("example:water_vial"))
    .fills_tagged(|f| f.path() == "potion", 250, id("example:potion_vial")));
let mut tag = NbtCompound::new();
tag.insert("Potion".to_owned(), Nbt::String("minecraft:swiftness".to_owned()));
let filled = FluidItems::fill(&ItemSlot::new(id("example:vial"), 1), &FluidStack { fluid: id("example:potion"), amount: 250, tag: Some(tag.clone()) });
assert_eq!(filled.container.item.to_string(), "example:potion_vial");
assert_eq!(filled.container.custom_data, tag);
```
