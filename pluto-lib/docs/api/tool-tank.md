# A fluid tank in a stack's data

A tank that lives in an item stack's own data rather than in a block: one fluid stack kept under a key of a data compound, in the `{FluidName, Amount, Tag?}` shape the fluid item tanks use. The capacity is not stored — it is a fact of the stack (for a tool, its tank capacity stat) that changes as the stack changes, so every call takes the current capacity. Setting, filling and draining follow the tool tank rules of Tinkers' Construct (`ToolTankHelper`, `TankModule`): setting clamps and an empty set removes the key, filling is first come first served, and draining takes up to what is asked. Asking and doing are the same calls with an `execute` flag, as with the fluid storage API.

> **Use with Caution.** Source-ready and pure-tested only; the native host adapter is not compiled. The stored fluid is plain stack data, so a host that keeps stack data keeps the tank.

### `read`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The fluid stored under a key, if any.

> A stored amount of 0 or less, a missing name or an unparsable fluid id all read as an
> empty tank.

```rust
use pluto_lib::{tool_tank_api_v1, FluidStack, Identifier, NbtCompound};

let mut data = NbtCompound::new();
assert_eq!(tool_tank_api_v1::read(&data, "tank"), None);
let lava = FluidStack::new(Identifier::parse("minecraft:lava").unwrap(), 500);
tool_tank_api_v1::write(&mut data, "tank", Some(&lava), 1000);
assert_eq!(tool_tank_api_v1::read(&data, "tank"), Some(lava));
```

### `write`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Stores a fluid under a key, clamped to the capacity.

> An empty stack — or one clamped to nothing — removes the key instead of storing an empty
> entry. The call returns what is stored afterwards, so you can read the result of the
> clamp or the removal without looking the key up again.

```rust
use pluto_lib::{tool_tank_api_v1, FluidStack, Identifier, NbtCompound};

let mut data = NbtCompound::new();
let lava = FluidStack::new(Identifier::parse("minecraft:lava").unwrap(), 1500);
let stored = tool_tank_api_v1::write(&mut data, "tank", Some(&lava), 1000).unwrap();
assert_eq!(stored.amount, 1000, "the write clamps to the capacity");
assert_eq!(tool_tank_api_v1::write(&mut data, "tank", None, 1000), None);
assert_eq!(tool_tank_api_v1::read(&data, "tank"), None, "an empty write removes the key");
```

### `fill`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

How much of an offered fluid the tank takes, out of a capacity.

> An empty tank takes the offered fluid up to the capacity; a tank holding the same fluid
> (same id and same tag) tops up; any other fluid is refused whole. With `execute` the
> taken fluid is stored; a simulation changes nothing.

```rust
use pluto_lib::{tool_tank_api_v1, FluidStack, Identifier, NbtCompound};

let mut data = NbtCompound::new();
let lava = FluidStack::new(Identifier::parse("minecraft:lava").unwrap(), 1400);
assert_eq!(tool_tank_api_v1::fill(&mut data, "tank", 1000, &lava, false), 1000);
assert_eq!(tool_tank_api_v1::read(&data, "tank"), None, "a simulation stores nothing");
assert_eq!(tool_tank_api_v1::fill(&mut data, "tank", 1000, &lava, true), 1000);
let water = FluidStack::new(Identifier::parse("minecraft:water").unwrap(), 10);
assert_eq!(tool_tank_api_v1::fill(&mut data, "tank", 1000, &water, true), 0, "another fluid is refused");
```

### `drain`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Takes up to the wanted amount of the stored fluid, which must be the same fluid as the wanted one (id and tag).

> The wanted fluid must match the stored one in both id and tag; a tank holding anything
> else gives nothing, as does an empty tank. To take whatever the tank holds without
> naming a fluid, use `drain_amount`.

```rust
use pluto_lib::{tool_tank_api_v1, FluidStack, Identifier, NbtCompound};

let mut data = NbtCompound::new();
let lava = FluidStack::new(Identifier::parse("minecraft:lava").unwrap(), 500);
tool_tank_api_v1::write(&mut data, "tank", Some(&lava), 1000);
let wanted = FluidStack::new(Identifier::parse("minecraft:lava").unwrap(), 200);
assert_eq!(tool_tank_api_v1::drain(&mut data, "tank", &wanted, true), Some(wanted));
assert_eq!(tool_tank_api_v1::read(&data, "tank").unwrap().amount, 300);
let water = FluidStack::new(Identifier::parse("minecraft:water").unwrap(), 10);
assert_eq!(tool_tank_api_v1::drain(&mut data, "tank", &water, true), None);
```

### `drain_amount`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Takes up to a millibucket amount of whatever the tank holds.

> Unlike `drain`, which only releases the fluid you name, this takes whatever the tank
> currently holds, up to the amount asked. An empty tank gives nothing.

```rust
use pluto_lib::{tool_tank_api_v1, FluidStack, Identifier, NbtCompound};

let mut data = NbtCompound::new();
let lava = FluidStack::new(Identifier::parse("minecraft:lava").unwrap(), 500);
tool_tank_api_v1::write(&mut data, "tank", Some(&lava), 1000);
assert_eq!(tool_tank_api_v1::drain_amount(&mut data, "tank", 700, true).unwrap().amount, 500);
assert_eq!(tool_tank_api_v1::read(&data, "tank"), None, "a full drain empties the tank");
```

### `validate`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Clamps the stored amount to the capacity, keeping the fluid itself.

> Run it when a stack's capacity may have shrunk under its content — the stored amount is
> cut down to the new capacity, never the fluid swapped. It returns whether the stored
> stack changed.

```rust
use pluto_lib::{tool_tank_api_v1, FluidStack, Identifier, NbtCompound};

let mut data = NbtCompound::new();
let lava = FluidStack::new(Identifier::parse("minecraft:lava").unwrap(), 900);
tool_tank_api_v1::write(&mut data, "tank", Some(&lava), 1000);
assert!(!tool_tank_api_v1::validate(&mut data, "tank", 1000));
assert!(tool_tank_api_v1::validate(&mut data, "tank", 300), "the capacity shrank under the content");
assert_eq!(tool_tank_api_v1::read(&data, "tank").unwrap().amount, 300);
```
