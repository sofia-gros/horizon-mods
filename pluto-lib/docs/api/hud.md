# HUD

What a mod draws over the game while no screen is up: the heads-up display around the hotbar. A mod registers one draw callback; PlutoLib calls it each frame the HUD shows, with the same canvas the menu screens draw on, the window's size, and the local player's own carrying. This page first walks through using it, then lists every item in the [Reference](#reference).

## Basic Example

An item the player wears in the legs slot carries a second item in its own data (the original's shield strap): draw that item above the offhand slot, at the hotbar's left for a right-handed player.

```rust
use pluto_lib::{HudDraw, HudRegistry, ItemSlot};

HudRegistry::register(|draw: &mut HudDraw<'_>| {
    if draw.spectator {
        return;
    }
    let Some(legs) = &draw.equipment.legs else { return };
    let Some(shield) = legs.custom_data.get("shield").and_then(pluto_lib::Nbt::as_str) else { return };
    let Ok(item) = pluto_lib::Identifier::parse(shield) else { return };
    let stack = ItemSlot::new(item, 1);
    let x = draw.size[0] / 2.0 + if draw.equipment.right_handed { -117.0 } else { 101.0 };
    draw.canvas.item(&stack, [x, draw.size[1] - 41.0], 16.0);
});
```

## What the callback receives

`size` is the window's size in GUI pixels; hotbar-relative art anchors to its centre and bottom, as the example does. `inventory` is the local player's inventory as the client last saw it synced — the hotbar first, then the main inventory, 36 slots — and `equipment` what the player wears and holds: `head`, `chest`, `legs`, `feet`, `offhand`, `main_hand`, and `right_handed` for which side the offhand slot sits on. `spectator` tells whether the local player is a spectator (the original draws none of its HUD then). `effects` is the player's active effects, each with whether it is beneficial and whether its icon shows — the original's effect shift counts exactly the ones that show. Everything is the client's own view: nothing here reaches the server, and before the first sync the inventory and equipment simply read empty.

## Reference

### `HudRegistry`

`HudRegistry::register(draw)` gives the HUD a draw callback, called each frame the HUD shows. Registering again adds another callback; each runs in registration order.

```rust
use pluto_lib::{HudDraw, HudRegistry};

HudRegistry::register(|draw: &mut HudDraw<'_>| {
    let _ = draw.size;
});
```

### `HudRegistry::register`

`fn register(draw: impl Fn(&mut HudDraw<'_>) + Send + Sync + 'static)`. Registers one HUD draw callback. Registration is the whole setup: there is no unregister, and a callback that has nothing to draw (no player, a spectator, the wrong equipment) simply returns.

```rust
use pluto_lib::{HudDraw, HudRegistry};

HudRegistry::register(|draw: &mut HudDraw<'_>| {
    if draw.spectator {
        return;
    }
    let _ = &draw.equipment;
});
```

### `HudDraw`

What a HUD draw callback receives: `canvas` (a [`ScreenCanvas`](screen.md), in GUI pixels from the window's top left), `size`, `inventory`, `equipment`, `spectator` and `effects`, as described above.

```rust
use pluto_lib::{HudDraw, HudRegistry};

HudRegistry::register(|draw: &mut HudDraw<'_>| {
    let (width, height) = (draw.size[0], draw.size[1]);
    let _ = (width, height, draw.inventory.len(), draw.spectator);
});
```

### `HudEquipment`

The local player's worn and held stacks, one `Option<ItemSlot>` per slot (`head`, `chest`, `legs`, `feet`, `offhand`, `main_hand`), plus `right_handed: bool`.

```rust
use pluto_lib::HudEquipment;

let equipment = HudEquipment { right_handed: true, ..HudEquipment::default() };
assert!(equipment.right_handed && equipment.legs.is_none() && equipment.main_hand.is_none());
```
