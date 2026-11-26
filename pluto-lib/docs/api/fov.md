# Field of view

A mod reshapes the local player's field of view: goggles that widen it, a helmet that narrows it, a
scope that narrows it further the longer it is drawn. The game computes a field-of-view modifier
from the player's state each tick (flying, speed, the item in use); a callback registered with
`FovRegistry` answers the factor its own modifiers contribute, and the game multiplies its
modifier by the product of every callback's factors — the shape of the original's FOV modifier
data, whose per-modifier multipliers the FOV event multiplies together.

```rust
use pluto_lib::fov_api_v1::{FovFacts, FovRegistry};

// Widen the view a little while flying; leave it alone otherwise.
FovRegistry::register(|facts: &FovFacts<'_>| {
    if facts.flying { Some(1.1) } else { None }
});
```

Every ask hands the callback the facts of the computation: the modifier the game already worked
out, the player's state, the equipment stacks the player holds and wears (a tool's modifiers and
persistent data ride in their custom data), and how long the item in use has been used — a zoom
reads that count against its draw time. A callback that owns nothing in the facts answers None.

> **Use with Caution.** The registry and the product are pure-tested. The transport rides the
> game's own field-of-view modifier hook — the native seam the game calls on itself each tick —
> so no injection carries it, and the equipment a callback reads is the local player's own, read
> through the host on the client (native-unvalidated).

## Reference

### `FovFacts`

![Horizon: absent](badges/horizon-absent.svg) ![Sunrise: works](badges/sunrise-works.svg)

The facts of one field-of-view computation.

> `base` is the field-of-view modifier the game computed from the player's state before
> callbacks (the value a callback's factor multiplies), `flying` whether the player is
> flying, `walking_speed` the player's walking speed attribute, `movement_speed` the
> player's movement speed attribute, `stacks` the player's equipment stacks (each naming
> its slot; an empty slot is absent), and `using_ticks` how many ticks the player has been
> using the item in use, or None when the player is not using anything. `keybind_zoom`
> says whether the worn-helmet keybind is held (a zoom module answers it with its keybind
> factor).

```rust
use pluto_lib::{Identifier, ItemSlot};
use pluto_lib::fov_api_v1::{FovFacts, FovSlot, FovStack};

let stack = ItemSlot::new(Identifier::new("example", "goggles").unwrap(), 1);
let stacks = [FovStack { slot: FovSlot::Head, stack: &stack }];
let facts = FovFacts { base: 1.0, flying: false, walking_speed: 0.1, movement_speed: 0.1, stacks: &stacks, using_ticks: Some(12), keybind_zoom: false };
assert_eq!(facts.base, 1.0);
assert_eq!(facts.stacks.len(), 1);
assert_eq!(facts.using_ticks, Some(12));
```

### `FovSlot`

![Horizon: absent](badges/horizon-absent.svg) ![Sunrise: works](badges/sunrise-works.svg)

One equipment slot of the local player, as a `FovStack` names it.

> The variants are `Feet`, `Legs`, `Chest`, `Head`, `MainHand` and `OffHand` — the four
> armour slots and the two hands. A `FovStack` pairs one of these with the stack sitting
> in it.

```rust
use pluto_lib::fov_api_v1::FovSlot;

assert_eq!(FovSlot::Head, FovSlot::Head);
assert_ne!(FovSlot::MainHand, FovSlot::OffHand);
assert_ne!(FovSlot::Feet, FovSlot::Legs);
assert_ne!(FovSlot::Chest, FovSlot::Head);
```

### `FovStack`

![Horizon: absent](badges/horizon-absent.svg) ![Sunrise: works](badges/sunrise-works.svg)

One of the local player's equipment stacks.

> `slot` is the slot it sits in and `stack` the stack itself, with its custom data.

```rust
use pluto_lib::{Identifier, ItemSlot};
use pluto_lib::fov_api_v1::{FovSlot, FovStack};

let stack = ItemSlot::new(Identifier::new("example", "scope").unwrap(), 1);
let fov_stack = FovStack { slot: FovSlot::MainHand, stack: &stack };
assert_eq!(fov_stack.slot, FovSlot::MainHand);
assert_eq!(fov_stack.stack.count, 1);
```

### `FovCallback`

![Horizon: absent](badges/horizon-absent.svg) ![Sunrise: works](badges/sunrise-works.svg)

Answers the factor one mod's modifiers contribute to the field of view, or None when they contribute nothing this tick.

`type FovCallback = fn(&FovFacts<'_>) -> Option<f32>`

> The registry asks every registered callback on each field-of-view computation and
> multiplies the game's own modifier by the product of the factors answered. Answer `None`
> and your callback contributes nothing to that product this tick.

```rust
use pluto_lib::fov_api_v1::{FovCallback, FovFacts};

fn steady(_: &FovFacts<'_>) -> Option<f32> { Some(1.05) }
let callback: FovCallback = steady;
assert_eq!(callback(&FovFacts { base: 1.0, flying: false, walking_speed: 0.1, movement_speed: 0.1, stacks: &[], using_ticks: None }), Some(1.05));
```

### `FovRegistry`

![Horizon: absent](badges/horizon-absent.svg) ![Sunrise: works](badges/sunrise-works.svg)

The registry of field-of-view callbacks.

> Register once during startup; the transport asks every registered callback on each
> field-of-view computation and multiplies the game's modifier by the product of the
> factors they answer.

```rust
use pluto_lib::fov_api_v1::{FovRegistry, fov_multiplier, FovFacts};

FovRegistry::register(|_| Some(1.05));
let facts = FovFacts { base: 1.0, flying: false, walking_speed: 0.1, movement_speed: 0.1, stacks: &[], using_ticks: None };
assert_eq!(fov_multiplier(&facts), Some(1.05));
```

### `FovRegistry::register`

![Horizon: absent](badges/horizon-absent.svg) ![Sunrise: works](badges/sunrise-works.svg)

Registers a field-of-view callback.

`fn register(callback: FovCallback)`

> Callbacks are plain functions (`FovCallback`), so the same function registered twice is
> recognised and held once. Every registered callback is asked on each field-of-view
> computation, and the factors they answer are multiplied together.

```rust
use pluto_lib::fov_api_v1::{FovRegistry, fov_multiplier, FovFacts};

fn widen(_: &FovFacts<'_>) -> Option<f32> { Some(1.05) }
FovRegistry::register(widen);
FovRegistry::register(widen);
let facts = FovFacts { base: 1.0, flying: false, walking_speed: 0.1, movement_speed: 0.1, stacks: &[], using_ticks: None };
assert_eq!(fov_multiplier(&facts), Some(1.05), "registered once, applied once");
```

### `fov_multiplier`

![Horizon: absent](badges/horizon-absent.svg) ![Sunrise: works](badges/sunrise-works.svg)

The product of every registered callback's factor over `facts`, or None when no callback contributes.

`fn fov_multiplier(facts: &FovFacts<'_>) -> Option<f32>`

> The transport multiplies the game's own modifier by this, and leaves it alone when None.

```rust
use pluto_lib::fov_api_v1::{FovRegistry, fov_multiplier, FovFacts};

let facts = FovFacts { base: 1.0, flying: false, walking_speed: 0.1, movement_speed: 0.1, stacks: &[], using_ticks: None };
FovRegistry::register(|_| None);
assert_eq!(fov_multiplier(&facts), None, "a callback that never speaks changes nothing");
FovRegistry::register(|_| Some(1.05));
FovRegistry::register(|_| Some(0.5));
assert_eq!(fov_multiplier(&facts), Some(1.05 * 0.5));
```

### `FovRegistry::set_keybind_zoom`

![Horizon: absent](badges/horizon-absent.svg) ![Sunrise: works](badges/sunrise-works.svg)

Records the helmet key as held (`true`) or released (`false`).

`fn set_keybind_zoom(held: bool)`

> The keybind zoom's half of the state, written by the armor interact hook's key tracking
> and read into `FovFacts::keybind_zoom` on each field-of-view computation.

```rust
use pluto_lib::fov_api_v1::{FovRegistry, keybind_zoom};

FovRegistry::set_keybind_zoom(true);
assert!(keybind_zoom());
FovRegistry::set_keybind_zoom(false);
assert!(!keybind_zoom());
```

### `keybind_zoom`

![Horizon: absent](badges/horizon-absent.svg) ![Sunrise: works](badges/sunrise-works.svg)

Whether the helmet key is held now, as the armor interact hook last reported it.

`fn keybind_zoom() -> bool`

> The transport reads it into the facts on each field-of-view computation (see
> `FovRegistry::set_keybind_zoom`).

```rust
use pluto_lib::fov_api_v1::{FovRegistry, keybind_zoom};

FovRegistry::set_keybind_zoom(false);
assert!(!keybind_zoom());
```
