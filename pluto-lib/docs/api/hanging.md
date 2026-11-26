# Hanging entities: item frame and armor stand variants

A mod whose items place item frames or armor stands with behaviour of their own — the shape of the original's `FancyItemFrameEntity` and `FancyArmorStandEntity` — registers a **variant** for each item: a row of behaviour data keyed by the item and by the variant index the entity saves (the original saves its enum ordinal as an int, so the index is part of the saved format). The game keeps frames and stands in its vanilla lists under the vanilla type ids; PlutoLib's game side reads the registry and runs the variant's mechanics against that state, with the variant riding in the entity's saved data. Anything unregistered keeps the vanilla behaviour exactly.

What a frame variant runs: placement from its item (survival, sounds and game events as the vanilla item), rotation through its own `set_rotation` (16 steps for the wide variants, a cap instead of a wrap for the winding-down one), its comparator output, the rotation timers (stepping every 20 ticks, winding down every 300 with the click), `setItem` resetting a rotating variant's rotation, the clear variant using the block behind it while filled and hiding its frame while filled, and the fire-immune variant ignoring explosions. What a stand variant runs: placement with its `onPlace` properties and the left-handed flag, fire immunity for every variant, the clear variant hiding while it wears anything, the break drop keeping the variant item with its custom name, dispenser placement, and `/damage` refusing fire.

Rendering is the host's: a variant frame draws with the vanilla frame model and a variant stand with the vanilla stand model, so per-variant models, textures, the manyullyn/necrotic fullbright rendering and the clear frame's larger held item are not covered by this surface.

## Basic Example

```rust
use pluto_lib::{FrameVariant, HangingRegistry, Identifier, StandVariant};

let gold = FrameVariant {
    item: Identifier::parse("mymod:gold_item_frame").unwrap(),
    entity_type: Identifier::parse("mymod:fancy_item_frame").unwrap(),
    variant: 3,
    rotation_step: 1,
    winds_down: false,
    more_rotations: false,
    fullbright: false,
    hidden_when_filled: false,
    click_through: false,
    fire_immune: false,
};
HangingRegistry::register_frame(gold).unwrap();

let bamboo = StandVariant {
    item: Identifier::parse("mymod:bamboo_armor_stand").unwrap(),
    entity_type: Identifier::parse("mymod:armor_stand").unwrap(),
    variant: 0,
    small: true,
    arms: false,
    fullbright: false,
    hidden_when_equipped: false,
};
HangingRegistry::register_stand(bamboo).unwrap();

assert!(HangingRegistry::frame_for_item(&Identifier::parse("mymod:gold_item_frame").unwrap()).is_some());
assert!(HangingRegistry::stand_for_variant(0).is_some());
```

## Reference

### `FrameVariant`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

One item frame variant's behaviour row.

> The fields are the original's `FrameType` properties: `item` is the item that places the
> variant; `entity_type` is the entity type the variant belongs to (the original's entity
> id, used where a type name is reported); `variant` is the saved index; `rotation_step`
> is the rotation change the 20-tick timer applies (gold +1, reversed gold -1, 0 for
> variants whose timer does not step); `winds_down` is diamond's behaviour (the timer
> loses one step every 300 ticks, clicks, and the rotation caps at 16 instead of
> wrapping); `more_rotations` gives 16 rotation steps and a 0-16 comparator output instead
> of 8; `fullbright` renders the frame and its item full bright (a rendering note — the
> game side does not draw variant models); `hidden_when_filled` hides the frame's own
> model while it holds an item (clear); `click_through` uses the block behind the filled
> frame when it is clicked (clear); `fire_immune` makes the frame immune to fire and
> explosions (netherite).

```rust
use pluto_lib::{FrameVariant, Identifier};

let diamond = FrameVariant {
    item: Identifier::parse("mymod:diamond_item_frame").unwrap(),
    entity_type: Identifier::parse("mymod:fancy_item_frame").unwrap(),
    variant: 1,
    rotation_step: 0,
    winds_down: true,
    more_rotations: true,
    fullbright: false,
    hidden_when_filled: false,
    click_through: false,
    fire_immune: false,
};
assert!(diamond.winds_down && diamond.more_rotations);
```

### `FrameVariant::rotates`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Whether the variant has a rotation timer (it steps or winds down), so `setItem` resets its rotation: the original's `doesRotate`.

> A variant whose rotation moves on its own — it steps or winds down on a timer — answers
> true, and that is why `setItem` resets its rotation. A variant that only turns when a
> player clicks it answers false.

```rust
use pluto_lib::{FrameVariant, Identifier};

let mut gold = FrameVariant {
    item: Identifier::parse("mymod:gold_item_frame").unwrap(),
    entity_type: Identifier::parse("mymod:fancy_item_frame").unwrap(),
    variant: 3,
    rotation_step: 1,
    winds_down: false,
    more_rotations: false,
    fullbright: false,
    hidden_when_filled: false,
    click_through: false,
    fire_immune: false,
};
assert!(gold.rotates());
gold.rotation_step = 0;
assert!(!gold.rotates());
```

### `FrameVariant::set_rotation`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The original's `setRotation` arithmetic: a 16-step variant caps at 16 when it winds down and wraps modulo 16 otherwise.

> Every other variant wraps modulo 8. Java's `%` is the remainder, so a negative input
> stays negative, as the original leaves it.

```rust
use pluto_lib::{FrameVariant, Identifier};

let mut frame = FrameVariant {
    item: Identifier::parse("mymod:manyullyn_item_frame").unwrap(),
    entity_type: Identifier::parse("mymod:fancy_item_frame").unwrap(),
    variant: 2,
    rotation_step: 0,
    winds_down: false,
    more_rotations: true,
    fullbright: true,
    hidden_when_filled: false,
    click_through: false,
    fire_immune: false,
};
assert_eq!(frame.set_rotation(17), 1);
frame.more_rotations = false;
assert_eq!(frame.set_rotation(9), 1);
```

### `FrameVariant::analog_output`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The original's `getAnalogOutput`.

> An empty frame reads 0; a 16-step variant reads its rotation (0-16, so all sixteen steps
> are distinct signals); every other variant reads `rotation % 8 + 1`.

```rust
use pluto_lib::{FrameVariant, Identifier};

let frame = FrameVariant {
    item: Identifier::parse("mymod:diamond_item_frame").unwrap(),
    entity_type: Identifier::parse("mymod:fancy_item_frame").unwrap(),
    variant: 1,
    rotation_step: 0,
    winds_down: true,
    more_rotations: true,
    fullbright: false,
    hidden_when_filled: false,
    click_through: false,
    fire_immune: false,
};
assert_eq!(frame.analog_output(false, 12), 0);
assert_eq!(frame.analog_output(true, 12), 12);
```

### `FrameVariant::tick_rotation`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Advances the variant's rotation timer by one tick.

> It returns the new timer, the rotation to set when the timer fires, and whether the
> change plays the variant's click sound (the winding-down variant's `setRotation`
> clicks). The rotation is applied through the same path as a player's rotation, which
> also resets the timer and updates the comparators. A variant that does not rotate
> returns its timer untouched.

```rust
use pluto_lib::{FrameVariant, Identifier};

let gold = FrameVariant {
    item: Identifier::parse("mymod:gold_item_frame").unwrap(),
    entity_type: Identifier::parse("mymod:fancy_item_frame").unwrap(),
    variant: 3,
    rotation_step: 1,
    winds_down: false,
    more_rotations: false,
    fullbright: false,
    hidden_when_filled: false,
    click_through: false,
    fire_immune: false,
};
assert_eq!(gold.tick_rotation(19, 7), (0, Some(0), false));
assert_eq!(gold.tick_rotation(3, 7), (4, None, false));
```

### `StandVariant`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

One armor stand variant's behaviour row.

> The fields are the original's `StandType` properties: `item` is the item that places the
> variant; `entity_type` is the entity type the variant belongs to; `variant` is the saved
> index; `small` places the stand small (bamboo); `arms` places it with arms shown (bone,
> necrotic bone); `fullbright` renders it full bright (necrotic bone — a rendering note,
> as with frames); `hidden_when_equipped` hides the stand while any equipment slot is
> filled (clear). Every variant is fire immune, at the class level, so there is no field
> for it.

```rust
use pluto_lib::{Identifier, StandVariant};

let necrotic = StandVariant {
    item: Identifier::parse("mymod:necrotic_bone_armor_stand").unwrap(),
    entity_type: Identifier::parse("mymod:armor_stand").unwrap(),
    variant: 2,
    small: false,
    arms: true,
    fullbright: true,
    hidden_when_equipped: false,
};
assert!(necrotic.arms && necrotic.fullbright);
```

### `StandVariant::hidden`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The original's `isClearHidden`.

> The variant is invisible exactly while it wears anything: equip a piece on it and the
> stand itself disappears, leaving the worn pieces showing. An empty stand shows as usual.
> It is the original's `isClearHidden`.

```rust
use pluto_lib::{Identifier, StandVariant};

let clear = StandVariant {
    item: Identifier::parse("mymod:clear_armor_stand").unwrap(),
    entity_type: Identifier::parse("mymod:armor_stand").unwrap(),
    variant: 3,
    small: false,
    arms: false,
    fullbright: false,
    hidden_when_equipped: true,
};
assert!(clear.hidden(true));
assert!(!clear.hidden(false));
```

### `HangingRegistry`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The hanging variant registry.

> One variant per item and one per saved variant index within each kind (frame, stand) —
> the index is written into the entity's saved data, so two registrations may not share
> one.

```rust
use pluto_lib::{HangingRegistry, Identifier};

assert!(HangingRegistry::frame_for_item(&Identifier::parse("mymod:unregistered").unwrap()).is_none());
assert!(HangingRegistry::stand_for_variant(99).is_none());
```

### `HangingRegistry::register_frame`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Registers one item frame variant.

> A second registration for the same item or the same variant index rejects.

```rust
use pluto_lib::{FrameVariant, HangingRegistry, Identifier};

let gold = FrameVariant {
    item: Identifier::parse("mymod:gold_item_frame").unwrap(),
    entity_type: Identifier::parse("mymod:fancy_item_frame").unwrap(),
    variant: 3,
    rotation_step: 1,
    winds_down: false,
    more_rotations: false,
    fullbright: false,
    hidden_when_filled: false,
    click_through: false,
    fire_immune: false,
};
HangingRegistry::register_frame(gold).unwrap();
assert!(HangingRegistry::register_frame(FrameVariant {
    item: Identifier::parse("mymod:other_item_frame").unwrap(),
    entity_type: Identifier::parse("mymod:fancy_item_frame").unwrap(),
    variant: 3,
    rotation_step: 0,
    winds_down: false,
    more_rotations: false,
    fullbright: false,
    hidden_when_filled: false,
    click_through: false,
    fire_immune: false,
}).is_err());
```

### `HangingRegistry::register_stand`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Registers one armor stand variant.

> A second registration for the same item or the same variant index rejects.

```rust
use pluto_lib::{HangingRegistry, Identifier, StandVariant};

let bamboo = StandVariant {
    item: Identifier::parse("mymod:bamboo_armor_stand").unwrap(),
    entity_type: Identifier::parse("mymod:armor_stand").unwrap(),
    variant: 0,
    small: true,
    arms: false,
    fullbright: false,
    hidden_when_equipped: false,
};
HangingRegistry::register_stand(bamboo).unwrap();
assert!(HangingRegistry::register_stand(StandVariant {
    item: Identifier::parse("mymod:bamboo_armor_stand").unwrap(),
    entity_type: Identifier::parse("mymod:armor_stand").unwrap(),
    variant: 1,
    small: false,
    arms: false,
    fullbright: false,
    hidden_when_equipped: false,
}).is_err());
```

### `HangingRegistry::frame_for_item`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The frame variant an item places, if any.

> The game side asks this when the item is used on a block.

```rust
use pluto_lib::{FrameVariant, HangingRegistry, Identifier};

HangingRegistry::register_frame(FrameVariant {
    item: Identifier::parse("mymod:gold_item_frame").unwrap(),
    entity_type: Identifier::parse("mymod:fancy_item_frame").unwrap(),
    variant: 3,
    rotation_step: 1,
    winds_down: false,
    more_rotations: false,
    fullbright: false,
    hidden_when_filled: false,
    click_through: false,
    fire_immune: false,
}).unwrap();
let found = HangingRegistry::frame_for_item(&Identifier::parse("mymod:gold_item_frame").unwrap()).unwrap();
assert_eq!(found.variant, 3);
```

### `HangingRegistry::frame_for_variant`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The frame variant a saved variant index names, if any.

> The game side asks this for a frame already in the world.

```rust
use pluto_lib::{FrameVariant, HangingRegistry, Identifier};

HangingRegistry::register_frame(FrameVariant {
    item: Identifier::parse("mymod:reversed_gold_item_frame").unwrap(),
    entity_type: Identifier::parse("mymod:fancy_item_frame").unwrap(),
    variant: 0,
    rotation_step: -1,
    winds_down: false,
    more_rotations: false,
    fullbright: false,
    hidden_when_filled: false,
    click_through: false,
    fire_immune: false,
}).unwrap();
assert_eq!(HangingRegistry::frame_for_variant(0).unwrap().rotation_step, -1);
```

### `HangingRegistry::stand_for_item`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The stand variant an item places, if any.

> The game side asks this when the item is used on a block or dispensed.

```rust
use pluto_lib::{HangingRegistry, Identifier, StandVariant};

HangingRegistry::register_stand(StandVariant {
    item: Identifier::parse("mymod:bone_armor_stand").unwrap(),
    entity_type: Identifier::parse("mymod:armor_stand").unwrap(),
    variant: 1,
    small: false,
    arms: true,
    fullbright: false,
    hidden_when_equipped: false,
}).unwrap();
assert!(HangingRegistry::stand_for_item(&Identifier::parse("mymod:bone_armor_stand").unwrap()).unwrap().arms);
```

### `HangingRegistry::stand_for_variant`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The stand variant a saved variant index names, if any.

> The game side asks this for a stand already in the world.

```rust
use pluto_lib::{HangingRegistry, Identifier, StandVariant};

HangingRegistry::register_stand(StandVariant {
    item: Identifier::parse("mymod:clear_armor_stand").unwrap(),
    entity_type: Identifier::parse("mymod:armor_stand").unwrap(),
    variant: 3,
    small: false,
    arms: false,
    fullbright: false,
    hidden_when_equipped: true,
}).unwrap();
assert!(HangingRegistry::stand_for_variant(3).unwrap().hidden(true));
```
