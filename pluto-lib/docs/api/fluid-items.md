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

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

One kind of item that holds fluid.

> `item` is the item it describes. Build a definition with `tank`, `portion`,
> `constant` or `empty_container` — plus the tagged variants — and register it
> with `FluidItems::register`.

```rust
use pluto_lib::{FluidItemDef, Identifier};

let def = FluidItemDef::tank(Identifier::parse("example:tank").unwrap(), "tank", 4000);
assert_eq!(def.item.to_string(), "example:tank");
```

### `FluidItemDef::tank`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

A container holding any one fluid up to `capacity` millibuckets.

`fn tank(item: Identifier, key: &str, capacity: u32) -> FluidItemDef`

> The fluid is kept under `key` in the stack's custom data as
> `{FluidName, Amount}`.

```rust
use pluto_lib::{FluidItemDef, Identifier};

assert_eq!(FluidItemDef::tank(Identifier::parse("example:tank").unwrap(), "tank", 1000).key(), Some("tank"));
```

### `FluidItemDef::portion`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

A container holding exactly `amount` of one fluid, or nothing.

`fn portion(item: Identifier, key: &str, amount: u32) -> FluidItemDef`

> The fluid is kept as the fluid id under `key`. A stack of several holds
> `amount` each, so the stack's contents scale with its count.

```rust
use pluto_lib::{FluidItemDef, Identifier};

assert_eq!(FluidItemDef::portion(Identifier::parse("example:can").unwrap(), "fluid", 90).key(), Some("fluid"));
```

### `FluidItemDef::constant`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

An item that always holds `amount` of one `fluid`.

`fn constant(item: Identifier, fluid: Identifier, amount: u32, empty: Option<Identifier>) -> FluidItemDef`

> Drained, the item becomes `empty` — or vanishes when no empty item is named.
> Its contents need no stored key, so `key()` is `None` for this kind.

```rust
use pluto_lib::{FluidItemDef, Identifier};

let id = |text: &str| Identifier::parse(text).unwrap();
assert_eq!(FluidItemDef::constant(id("example:bottle"), id("example:honey"), 250, None).key(), None);
```

### `FluidItemDef::empty_container`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

An empty item that becomes `filled` when an accepted fluid goes in.

`fn empty_container(item, accepts: impl Fn(&Identifier) -> bool, amount: u32, filled: Identifier) -> FluidItemDef`

> It takes `amount` of a fluid the `accepts` predicate allows and turns into the
> `filled` item. Add further fillings of the same empty item with `fills`.

```rust
use pluto_lib::{FluidItemDef, Identifier};

let id = |text: &str| Identifier::parse(text).unwrap();
let bowl = FluidItemDef::empty_container(id("minecraft:bowl"), |fluid| fluid.path() == "soup", 250, id("example:soup_bowl"));
assert_eq!(bowl.item.to_string(), "minecraft:bowl");
```

### `FluidItemDef::fills`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

One more filling of the same empty item.

`fn fills(self, accepts, amount: u32, filled: Identifier) -> FluidItemDef`

> Each added filling is tried after the earlier ones when the empty item is
> filled, so the order of `fills` calls sets the priority between accepted
> fluids.

```rust
use pluto_lib::{FluidItemDef, Identifier};

let id = |text: &str| Identifier::parse(text).unwrap();
let bottle = FluidItemDef::empty_container(id("minecraft:glass_bottle"), |f| f.path() == "a", 250, id("example:a_bottle"))
    .fills(|f| f.path() == "b", 250, id("example:b_bottle"));
assert_eq!(bottle.item.path(), "glass_bottle");
```

### `FluidItemDef::scaled`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

A tank whose stack of n items is one tank of n times the capacity.

`fn scaled(self) -> FluidItemDef`

> Amounts are kept per item and transfers move in steps of n — the behaviour of
> Forge's `ScaledFluidTank`.

```rust
use pluto_lib::{FluidItemDef, FluidItems, FluidStack, Identifier, ItemSlot};

let id = |text: &str| Identifier::parse(text).unwrap();
FluidItems::register(FluidItemDef::tank(id("example:scaled"), "tank", 1000).scaled());
let filled = FluidItems::fill(&ItemSlot::new(id("example:scaled"), 4), &FluidStack { fluid: id("minecraft:lava"), amount: 4002, tag: None });
assert_eq!(filled.amount, 4000);
```

### `FluidItemDef::fill_stack_limit`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

A tank that takes no fluid while the stack holds more than `limit` items.

`fn fill_stack_limit(self, limit: u32) -> FluidItemDef`

> Draining is unaffected — the limit only refuses fills. It pairs with `scaled`,
> where a bigger stack would otherwise mean a bigger tank.

```rust
use pluto_lib::{FluidItemDef, FluidItems, FluidStack, Identifier, ItemSlot};

let id = |text: &str| Identifier::parse(text).unwrap();
FluidItems::register(FluidItemDef::tank(id("example:limited"), "tank", 1000).scaled().fill_stack_limit(16));
assert_eq!(FluidItems::fill(&ItemSlot::new(id("example:limited"), 17), &FluidStack { fluid: id("minecraft:lava"), amount: 17, tag: None }).amount, 0);
```

### `FluidItemDef::key`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The custom data key that holds the fluid.

`fn key(&self) -> Option<&str>`

> It answers `None` for constant and empty containers, whose contents are not
> stored under a key.

```rust
use pluto_lib::{FluidItemDef, Identifier};

assert_eq!(FluidItemDef::tank(Identifier::parse("example:t").unwrap(), "tank", 10).key(), Some("tank"));
```

### `Filled`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

What a fill did.

> `amount` is how much went in, and `container` the stack afterwards — same
> count, possibly another item, as when an empty container becomes its filled
> form.

```rust
use pluto_lib::{FluidItemDef, FluidItems, FluidStack, Identifier, ItemSlot};

let id = |text: &str| Identifier::parse(text).unwrap();
FluidItems::register(FluidItemDef::tank(id("example:f"), "tank", 100));
let filled = FluidItems::fill(&ItemSlot::new(id("example:f"), 1), &FluidStack { fluid: id("minecraft:water"), amount: 30, tag: None });
assert_eq!((filled.amount, filled.container.item.path()), (30, "f"));
```

### `Drained`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

What a drain did.

> `fluid` is the fluid that came out, and `container` the stack afterwards —
> `None` when nothing is left of the stack.

```rust
use pluto_lib::{FluidItemDef, FluidItems, Identifier, ItemSlot};

let id = |text: &str| Identifier::parse(text).unwrap();
FluidItems::register(FluidItemDef::constant(id("example:jar"), id("example:jam"), 100, None));
let drained = FluidItems::drain(&ItemSlot::new(id("example:jar"), 1), None, 100).unwrap();
assert_eq!(drained.fluid.amount, 100);
assert!(drained.container.is_none());
```

### `FluidTarget`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

A tank to trade with.

> It offers `fill` and `drain`, each with an `execute` flag; passing false turns
> the call into a question — how much would move — without moving anything. See
> the Basic Example.

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

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Which way a held stack and a tank trade.

> `Auto` tries the stack into the tank first, and otherwise the tank into the
> stack. `ToTank` and `FromTank` force the one direction.

```rust
use pluto_lib::TransferDirection;

assert_ne!(TransferDirection::ToTank, TransferDirection::FromTank);
assert_eq!(TransferDirection::Auto, TransferDirection::Auto);
```

### `Interaction`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

What a trade did.

> `held` is the hand afterwards, `give` the stacks for the player, `moved` the
> fluid that moved, and `into_tank` whether it went into the tank.

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

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The registry of fluid containers and the transfers between them.

> Register definitions with `register` and look them up with `get` and
> `registered`. Move fluid with `fill`, `drain` and `interact`; `contents` and
> `capacity` read a stack without moving anything, and `is_container` says
> whether a stack can trade at all.

```rust
use pluto_lib::{FluidItemDef, FluidItems, Identifier};

FluidItems::register(FluidItemDef::tank(Identifier::parse("example:reg").unwrap(), "tank", 10));
assert!(FluidItems::get("example:reg").is_some());
```

### `FluidItems::register`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Registers a definition, making its item a container.

`fn register(def: FluidItemDef)`

> Registering the same item again replaces the earlier definition.

```rust
use pluto_lib::{FluidItemDef, FluidItems, Identifier};

FluidItems::register(FluidItemDef::tank(Identifier::parse("example:again").unwrap(), "tank", 10));
FluidItems::register(FluidItemDef::tank(Identifier::parse("example:again").unwrap(), "tank", 20));
```

### `FluidItems::get`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The definition registered for an item id.

`fn get(item: &str) -> Option<FluidItemDef>`

> It answers `None` when the item has no definition — such an item is not a
> fluid container.

```rust
use pluto_lib::FluidItems;

assert!(FluidItems::get("example:never_registered").is_none());
```

### `FluidItems::registered`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Every definition registered so far, in registration order.

`fn registered() -> Vec<FluidItemDef>`

> The list is empty in a fresh process, before any `register` call has run.

```rust
use pluto_lib::FluidItems;

assert!(FluidItems::registered().is_empty(), "nothing is registered in a fresh process");
```
### `FluidItems::is_container`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Whether the stack's item holds or can be filled with fluid.

`fn is_container(stack: &ItemSlot) -> bool`

> Both a container already holding fluid and an empty container that can accept
> one count; an ordinary item does not.

```rust
use pluto_lib::{FluidItems, Identifier, ItemSlot};

assert!(!FluidItems::is_container(&ItemSlot::new(Identifier::parse("minecraft:stick").unwrap(), 1)));
```

### `FluidItems::contents`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

What the stack holds.

`fn contents(stack: &ItemSlot) -> Option<FluidStack>`

> For a scaled tank or a stack of portions, the amount is the whole stack's, not
> one item's. A stack holding no fluid answers `None`.

```rust
use pluto_lib::{FluidItemDef, FluidItems, Identifier, ItemSlot};

let id = |text: &str| Identifier::parse(text).unwrap();
FluidItems::register(FluidItemDef::constant(id("example:c"), id("example:oil"), 50, None));
assert_eq!(FluidItems::contents(&ItemSlot::new(id("example:c"), 1)).unwrap().amount, 50);
```

### `FluidItems::capacity`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

How much the stack holds at most.

`fn capacity(stack: &ItemSlot) -> u32`

> It answers 0 for an item that is not a container.

```rust
use pluto_lib::{FluidItemDef, FluidItems, Identifier, ItemSlot};

let id = |text: &str| Identifier::parse(text).unwrap();
FluidItems::register(FluidItemDef::tank(id("example:cap"), "tank", 4000));
assert_eq!(FluidItems::capacity(&ItemSlot::new(id("example:cap"), 1)), 4000);
assert_eq!(FluidItems::capacity(&ItemSlot::new(id("minecraft:stick"), 1)), 0);
```

### `FluidItems::fill`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Puts fluid into the stack as far as it goes.

`fn fill(stack: &ItemSlot, fluid: &FluidStack) -> Filled`

> Asking and doing are the same call: the returned `Filled` reports how much
> went in and the container stack afterwards.

```rust
use pluto_lib::{FluidItemDef, FluidItems, FluidStack, Identifier, ItemSlot};

let id = |text: &str| Identifier::parse(text).unwrap();
FluidItems::register(FluidItemDef::tank(id("example:fill"), "tank", 100));
let filled = FluidItems::fill(&ItemSlot::new(id("example:fill"), 1), &FluidStack { fluid: id("minecraft:lava"), amount: 500, tag: None });
assert_eq!(filled.amount, 100);
```

### `FluidItems::drain`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Drains the fluid the stack gives up, up to `max`.

`fn drain(stack: &ItemSlot, wanted: Option<&Identifier>, max: u32) -> Option<Drained>`

> A portion or constant gives all of it or nothing — those kinds cannot be
> partly drained. The result names the fluid and the container stack afterwards.

```rust
use pluto_lib::{FluidItemDef, FluidItems, FluidStack, Identifier, ItemSlot};

let id = |text: &str| Identifier::parse(text).unwrap();
FluidItems::register(FluidItemDef::tank(id("example:drain"), "tank", 100));
let full = FluidItems::fill(&ItemSlot::new(id("example:drain"), 1), &FluidStack { fluid: id("minecraft:lava"), amount: 100, tag: None }).container;
assert_eq!(FluidItems::drain(&full, None, 30).unwrap().fluid.amount, 30);
```

### `FluidItems::interact`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

A trade between the held stack and a tank.

`fn interact(held: Option<&ItemSlot>, target: &mut dyn FluidTarget, direction: TransferDirection) -> Option<Interaction>`

> This is `FluidUtil.interactWithFluidHandler`. A container that is not a scaled
> tank trades one at a time, splitting one off a stack and giving its result in
> `give`.

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

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

How a fluid flows.

> `tick_delay` is game ticks per step, `level_drop` the levels lost per block,
> and `slope_distance` how many blocks are searched for a way down.
> `FluidFlow::standard` gives the water-like numbers.

```rust
use pluto_lib::FluidFlow;

let flow = FluidFlow { tick_delay: 30, level_drop: 3, slope_distance: 3 };
assert_ne!(flow, FluidFlow::standard());
```

### `FluidFlow::standard`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Water-like flow: a tick delay of 5, a level drop of 1 and a slope distance of 4.

`const fn standard() -> FluidFlow`

> These are the flow numbers of water. Use them as the starting point for a
> fluid that should spread the way water does, and adjust the fields from there.

```rust
use pluto_lib::FluidFlow;

assert_eq!(FluidFlow::standard().tick_delay, 5);
```

### `FluidDef::flow`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Sets how the fluid flows in the world.

`fn flow(self, tick_delay: u16, level_drop: u8, slope_distance: u8) -> FluidDef`

> The three numbers become the fluid's `FluidFlow`: `tick_delay` game ticks per
> step, `level_drop` levels lost per block, and `slope_distance` blocks searched
> for a way down.

```rust
use pluto_lib::{FluidDef, Identifier};

let def = FluidDef::new(Identifier::parse("example:slow").unwrap()).flow(40, 2, 2);
assert_eq!(def.flow.level_drop, 2);
```

### `FluidDef::bucket_sounds`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The sound events the bucket makes with this fluid.

`fn bucket_sounds(self, fill: &str, empty: &str) -> FluidDef`

> `fill` is the sound of taking the fluid up and `empty` the sound of pouring it
> out; they are kept as the definition's `bucket_fill_sound` and
> `bucket_empty_sound`.

```rust
use pluto_lib::{FluidDef, Identifier};

let def = FluidDef::new(Identifier::parse("example:hot").unwrap()).bucket_sounds("minecraft:item.bucket.fill_lava", "minecraft:item.bucket.empty_lava");
assert_eq!(def.bucket_empty_sound.as_deref(), Some("minecraft:item.bucket.empty_lava"));
assert!(def.bucket_fill_sound.is_some());
```

### `FluidWorld`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The rules of a mod fluid in the world.

> Its functions name the fluid's block ( `<id>_fluid`) and bucket (
> `<id>_bucket`), read levels from block states, place and pick up sources, and
> run the flow tick. A fluid gets a world form through
> `FluidDef::with_world_blocks`.

```rust
use pluto_lib::{FluidWorld, Identifier};

assert_eq!(FluidWorld::bucket_id(&Identifier::parse("example:oil").unwrap()).to_string(), "example:oil_bucket");
```

### `FluidWorld::LEVEL`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The block state property that carries the level.

> Its values are `BLOCK_LEVELS`: `0` for a source, `1` to `7` for flowing
> levels, and `8` for falling.

```rust
use pluto_lib::FluidWorld;

assert_eq!(FluidWorld::LEVEL, "level");
```

### `FluidWorld::BLOCK_LEVELS`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The values of the `level` property.

> `0` is a source, `1` to `7` are flowing levels, and `8` is falling — nine
> values in all.

```rust
use pluto_lib::FluidWorld;

assert_eq!(FluidWorld::BLOCK_LEVELS.len(), 9);
```

### `FluidWorld::state`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The fluid block at a level.

`fn state(fluid: &Identifier, level: u8) -> BlockState`

> It is the block state of the fluid's own block at the given level — what a
> world query answers for a position the fluid fills to that height. Levels run
> from a full block down to the thinnest film the fluid can stand at.

```rust
use pluto_lib::{FluidWorld, Identifier};

assert_eq!(FluidWorld::state(&Identifier::parse("example:oil").unwrap(), 2).to_string(), "example:oil_fluid[level=2]");
```

### `FluidWorld::block_id`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The fluid's block id, `<id>_fluid`.

`fn block_id(fluid: &Identifier) -> Identifier`

> The `_fluid` suffix keeps a fluid and a building block of the same name from
> clashing.

```rust
use pluto_lib::{FluidWorld, Identifier};

assert_eq!(FluidWorld::block_id(&Identifier::parse("a:b").unwrap()).path(), "b_fluid");
```

### `FluidWorld::bucket_id`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The fluid's bucket item id, `<id>_bucket`.

`fn bucket_id(fluid: &Identifier) -> Identifier`

> This is the item `FluidDef::with_bucket` registers for the fluid: a 1000
> container that becomes `minecraft:bucket` when emptied.

```rust
use pluto_lib::{FluidWorld, Identifier};

assert_eq!(FluidWorld::bucket_id(&Identifier::parse("a:b").unwrap()).path(), "b_bucket");
```

### `FluidWorld::level_of`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The level of a state of the fluid's block.

`fn level_of(state: &BlockState, fluid: &Identifier) -> Option<u8>`

> It answers `None` for another block, or for a level above 8.

```rust
use pluto_lib::{BlockState, FluidWorld, Identifier};

let oil = Identifier::parse("example:oil").unwrap();
assert_eq!(FluidWorld::level_of(&BlockState::parse("example:oil_fluid[level=5]").unwrap(), &oil), Some(5));
assert_eq!(FluidWorld::level_of(&BlockState::parse("minecraft:stone").unwrap(), &oil), None);
```

### `FluidWorld::source_block`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The source block of a fluid that has a world form.

`fn source_block(fluid: &Identifier) -> Option<BlockState>`

> For water and lava the block id is the fluid's own id; for a registered fluid
> with world blocks it is its block at level 0. It answers `None` for a fluid
> with no world form, so a bucket module's world halves know the fluid cannot be
> placed. It is the forward half of the fluid's world-block form, with
> `fluid_of` as the reverse.

```rust
use pluto_lib::{FluidDef, FluidRegistry, FluidWorld, Identifier};

let oil = Identifier::parse("example:oil").unwrap();
FluidRegistry::register(FluidDef::new(oil.clone()).with_world_blocks().with_bucket());
assert_eq!(FluidWorld::source_block(&oil).map(|state| state.to_string()), Some("example:oil_fluid[level=0]".to_owned()));
assert_eq!(FluidWorld::source_block(&Identifier::parse("minecraft:water").unwrap()).map(|state| state.to_string()), Some("minecraft:water".to_owned()));
assert!(FluidWorld::source_block(&Identifier::parse("example:potion").unwrap()).is_none());
```

### `FluidWorld::fluid_of`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The fluid and level a block state holds, if it is a fluid's world form.

`fn fluid_of(state: &BlockState) -> Option<(Identifier, u8)>`

> A registered fluid's `<id>_fluid` block reports its `level` — 0 source, 1 to 7
> flowing, 8 falling — and water and lava report by their own block id, where a
> missing `level` reads as a source and a vanilla level of 8 to 15 is a falling
> cell, the port's 8. Any other block answers `None`. It is the reverse half of
> the fluid's world-block form, with `source_block` as the forward half.

```rust
use pluto_lib::{BlockState, FluidWorld, Identifier};

assert_eq!(FluidWorld::fluid_of(&BlockState::parse("minecraft:water[level=0]").unwrap()), Some((Identifier::parse("minecraft:water").unwrap(), 0)));
assert_eq!(FluidWorld::fluid_of(&BlockState::parse("minecraft:water[level=12]").unwrap()), Some((Identifier::parse("minecraft:water").unwrap(), 8)));
assert_eq!(FluidWorld::fluid_of(&BlockState::parse("minecraft:stone").unwrap()), None);
```

### `FluidWorld::is_world_fluid`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Whether `block` is the block of a defined fluid with world blocks.

`fn is_world_fluid(block: &Identifier) -> bool`

> The block must be the fluid's `<id>_fluid` block; the fluid's own item id and
> unrelated blocks answer false.

```rust
use pluto_lib::{FluidWorld, Identifier};

assert!(!FluidWorld::is_world_fluid(&Identifier::parse("minecraft:stone").unwrap()));
```

### `FluidWorld::place_source`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Puts a source of the fluid at `pos` and schedules its tick.

`fn place_source(world: &mut dyn BlockEditor, pos: BlockPos, def: &FluidDef) -> bool`

> It only places into air or a replaceable non-fluid block, and answers false
> when the position will not take the source.

```rust
use pluto_lib::{FluidDef, FluidWorld, Identifier, MemoryBlockView};

let def = FluidDef::new(Identifier::parse("example:oil").unwrap());
let mut world = MemoryBlockView::default();
assert!(FluidWorld::place_source(&mut world, [0, 64, 0], &def));
assert!(!FluidWorld::place_source(&mut world, [0, 64, 0], &def));
```

### `FluidWorld::pick_up_source`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Removes a source of this fluid.

`fn pick_up_source(world: &mut dyn BlockEditor, pos: BlockPos, def: &FluidDef) -> bool`

> A flowing cell is refused — only a true source can be picked up, which is what
> a bucket takes.

```rust
use pluto_lib::{FluidDef, FluidWorld, Identifier, MemoryBlockView};

let def = FluidDef::new(Identifier::parse("example:oil").unwrap());
let mut world = MemoryBlockView::default();
FluidWorld::place_source(&mut world, [0, 64, 0], &def);
assert!(FluidWorld::pick_up_source(&mut world, [0, 64, 0], &def));
```

### `FluidWorld::tick`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

One scheduled tick of a fluid cell.

`fn tick(world: &mut dyn BlockEditor, pos: BlockPos, def: &FluidDef)`

> This is `FlowingFluid.tick`: a flowing cell settles to what its neighbours
> make of it, then the cell spreads down or toward the nearest drop. Changed
> cells are scheduled again.

```rust
use pluto_lib::{FluidDef, FluidWorld, Identifier, MemoryBlockView};

let def = FluidDef::new(Identifier::parse("example:oil").unwrap());
let mut world = MemoryBlockView::default();
FluidWorld::place_source(&mut world, [0, 64, 0], &def);
FluidWorld::tick(&mut world, [0, 64, 0], &def);
assert!(world.blocks.contains_key(&[0, 63, 0]), "it falls");
```

### `FluidWorld::register_behaviour`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Gives the fluid's block the flow behaviour.

`fn register_behaviour(def: &FluidDef)`

> `FluidRegistry::register` calls it for a fluid with world blocks. Extend the
> registered behaviour with `BlockBehaviors::get` rather than replacing it.

```rust
use pluto_lib::{BlockBehaviors, FluidDef, FluidWorld, Identifier};

FluidWorld::register_behaviour(&FluidDef::new(Identifier::parse("example:tar").unwrap()));
assert!(BlockBehaviors::get("example:tar_fluid").is_some());
```

### `World::schedule_tick`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Asks for the block at `pos` to tick after `delay` game ticks.

`fn schedule_tick(&self, pos: BlockPos, block: &str, delay: i32) -> bool`

> It answers false outside a callback, for a delay below 1, or when the host
> cannot schedule from this callback.

```rust
use pluto_lib::World;

assert!(!World::new("minecraft:overworld").schedule_tick([0, 0, 0], "example:oil", 5), "no world in a plain test");
```
### `FluidItemDef::tagged_constant`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

An item that always holds `amount` of `fluid`, together with a tag.

`fn tagged_constant(item: Identifier, fluid: Identifier, amount: u32, empty: Option<Identifier>) -> FluidItemDef`

> The stack's own custom data is the fluid's tag, so a stack with no custom data
> holds the plain fluid. Drained, it becomes `empty` or vanishes and hands the
> tag back with the fluid.

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

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

An empty item that becomes `filled` when an accepted fluid goes in, carrying the fluid's tag.

`fn empty_container_tagged(item: Identifier, accepts: impl Fn(&Identifier) -> bool + Send + Sync + 'static, amount: u32, filled: Identifier) -> FluidItemDef`

> It takes `amount` of a fluid the `accepts` predicate allows, and the filled
> stack's own custom data is the fluid's tag.

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

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

One more filling of the same empty item, whose filled stack takes the fluid's tag.

`fn fills_tagged(self, accepts: impl Fn(&Identifier) -> bool + Send + Sync + 'static, amount: u32, filled: Identifier) -> FluidItemDef`

> The filling is tried after the earlier ones, and the filled stack's own custom
> data is the fluid's tag. It does nothing to other kinds of item.

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
