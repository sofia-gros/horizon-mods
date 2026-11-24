# Map: ticking a map stored in a worn piece

A map stored in a worn piece does not tick on its own: the host only ticks maps a player holds or carries. PlutoLib's map-tick reach swaps the stored map into the offhand for the maps tick, lets vanilla tick it, swaps the old offhand stack back and sends a LAN guest the update packet — the stored map keeps exploring while it is worn, as Tinkers' minimap module does in the original (its inventory tick swaps the stored map into the offhand, ticks it and sends the packet). The host cannot parse a mod's storage layout, so the game asks instead of reading: once per tick, while the maps tick runs, the game hands the worn head stack to the registered handler and the handler answers the id of the stored map to tick, if any.

## Basic Example

A helmet whose minimap inventory holds a filled map answers that map's id; any other head passes with `None`, and the first `Some` wins.

```rust
use pluto_lib::{Identifier, ItemSlot, MapTickCallback, MapTickRequest};

MapTickCallback::EVENT.register(|request| {
    // A real handler reads the map id from its own storage (the helmet's minimap
    // inventory, at its selected slot); this one answers for one distinctive head.
    if request.head.item.to_string() == "example:helmet" {
        Some(7)
    } else {
        None
    }
});
let head = ItemSlot::new(Identifier::new("example", "helmet").unwrap(), 1);
assert_eq!(MapTickCallback::EVENT.invoke(&MapTickRequest { head }), Some(7));
```

### `MapTickRequest`

The worn head stack a map-tick handler is asked about. `head` is the worn head stack: the piece whose stored map may tick.

```rust
use pluto_lib::{Identifier, ItemSlot, MapTickRequest};

let head = ItemSlot::new(Identifier::new("example", "helmet").unwrap(), 1);
let request = MapTickRequest { head };
assert_eq!(request.head.item.to_string(), "example:helmet");
```

### `MapTickCallback`

The stored map's id for a worn head stack (see `MapTickRequest`): the mod that owns the storage registers one callback answering the id of the map to tick, or `None` for a head it does not own. The first `Some` wins.

```rust
use pluto_lib::{Identifier, ItemSlot, MapTickCallback, MapTickRequest};

MapTickCallback::EVENT.register(|request| {
    if request.head.item.to_string() == "example:helmet" {
        Some(7)
    } else {
        None
    }
});
let head = ItemSlot::new(Identifier::new("example", "helmet").unwrap(), 1);
assert_eq!(MapTickCallback::EVENT.invoke(&MapTickRequest { head }), Some(7));
```

### `MapTickCallback::EVENT`

The event itself. A head no handler answers for ticks no map.

```rust
use pluto_lib::{Identifier, ItemSlot, MapTickCallback, MapTickRequest};

let head = ItemSlot::new(Identifier::new("example", "plain").unwrap(), 1);
assert_eq!(MapTickCallback::EVENT.invoke(&MapTickRequest { head }), None);
```
