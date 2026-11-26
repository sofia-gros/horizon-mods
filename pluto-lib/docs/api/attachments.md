# Player and entity attachments

Attach any `serde` value to a player and read it back later, also after the game restarts (the value is stored as JSON with the player). Entities also expose typed attachments through `EntityInfo` in a host-scoped callback; worlds, chunks and block entities are not exposed here yet.

Like every `ServerPlayer` method, these work only inside a callback that carries the player (`with_ops`); elsewhere they report "nothing there" and store nothing.

## Basic Example

An attachment is a value that belongs to a player and is saved with them. Create its type once, then read and write it from any callback that carries the player. Here a counter remembers how many times a player has joined:

```rust
use pluto_lib::{AttachmentRegistry, Identifier, ServerPlayConnectionEvents};

ServerPlayConnectionEvents::JOIN.register(|join| {
    let visits = AttachmentRegistry::create(Identifier::new("example", "visits").unwrap(), || 0u32);
    let before = join.player.get_attached(&visits);
    join.player.set_attached(&visits, before + 1);
});
```

## Structured Values

The value can be any type that `serde` can save, so a whole record fits in one attachment:

```rust
use pluto_lib::{with_ops, AttachmentRegistry, Identifier, MemoryPlayerOps, ServerPlayer};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Default)]
struct Quest {
    stage: u8,
    notes: Vec<String>,
}

let quest = AttachmentRegistry::create(Identifier::new("example", "quest").unwrap(), Quest::default);
let player = ServerPlayer { uuid: 1, name: "Alex".into() };
let mut game = MemoryPlayerOps::default();
with_ops(&mut game, || {
    player.set_attached(&quest, Quest { stage: 2, notes: vec!["found the key".into()] });
    assert_eq!(player.get_attached(&quest).stage, 2);
});
```

## Never Set, or Set to the Default?

`get_attached` hides the difference between "nothing stored" and "stored the default". When it matters (a book that is given once), ask `has_attached` or `get_attached_existing`:

```rust
use pluto_lib::{with_ops, AttachmentRegistry, Identifier, MemoryPlayerOps, ServerPlayer};

let given = AttachmentRegistry::create(Identifier::new("example", "book_given").unwrap(), || false);
let player = ServerPlayer { uuid: 1, name: "Alex".into() };
let mut game = MemoryPlayerOps::default();
with_ops(&mut game, || {
    assert!(!player.has_attached(&given));
    player.set_attached(&given, false);
    assert!(player.has_attached(&given), "stored, even though it is the default value");
});
```

> **Use with Caution.** The methods work only inside a callback that carries the player (a join, a command). Elsewhere they report "nothing there" and store nothing, so a handle kept in a `static` cannot write to a player at the wrong moment. Entities also expose typed attachments through `EntityInfo` in a host-scoped callback; worlds, chunks and block entities are not exposed here yet.

## Entity State and Synchronization

Use the same typed attachment on the current callback's `EntityInfo`. `create_synced` requests persistence plus native delivery to clients tracking the holder. Native entity storage is UUID-owned; PlutoLib does not add an ID-keyed entity-state map. The memory fixture uses explicit dimension/ID keys and does not simulate native holder lifetimes.

```rust
use pluto_lib::{with_ops, AttachmentRegistry, EntityInfo, Identifier, MemoryPlayerOps};
let metal = AttachmentRegistry::create_synced(Identifier::parse("example:metal").unwrap(), || false);
let entity = EntityInfo { id: 7, kind: "example:slime".into(), dimension: "minecraft:overworld".into() };
let mut game = MemoryPlayerOps::default();
with_ops(&mut game, || {
    assert!(!entity.has_attached(&metal));
    assert!(entity.set_attached(&metal, true));
    assert_eq!(entity.get_attached_existing(&metal), Some(true));
});
assert!(!entity.set_attached(&metal, false)); // no host scope
```

> **Use with Caution.** The API is source-wired to existing native attachment storage/codec/sync, not tested in a running game. JSON snapshot tests do not prove save/reopen or LAN delivery. Keep one immutable sync policy per ID; conflicting synced/unsynced access fails. Entity IDs are current-world callback IDs, not persistent entity references: do not reuse an old EntityInfo after removal or in another callback. Missing native holders/wrong dimensions refuse access; unload callbacks may already have no holder. Clients changing their own native copy do not authoritatively change server state. Native state-to-attribute/render integration is separate.

## Reference

### `AttachmentRegistry::create_synced`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Like `create`, with persistent JSON and tracking-client sync requested.

`fn create_synced<T: Serialize + DeserializeOwned>(id: Identifier, default: fn() -> T) -> AttachmentType<T>`

> One ID cannot mix sync policies. Custom hosts without policy support refuse rather than
> silently storing unsynced data.

```rust
use pluto_lib::{AttachmentRegistry, Identifier};
let state = AttachmentRegistry::create_synced(Identifier::parse("example:synced_state").unwrap(), || 0u32);
assert_eq!(state.id().to_string(), "example:synced_state");
```

### `EntityInfo::has_attached`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

True only for existing decodable data in the current host scope.

`fn has_attached<T: Serialize + DeserializeOwned>(&self, attachment: &AttachmentType<T>) -> bool`

> Stored data that cannot be decoded, or that sits outside the current scope, counts as
> absent: the answer is about usable data, not about whether anything was ever written.
> The check creates nothing — pair it with `get_attached_existing` when you need the value
> itself.

```rust
use pluto_lib::{AttachmentRegistry, EntityInfo, Identifier};
let value = AttachmentRegistry::create(Identifier::parse("example:entity_has").unwrap(), || false);
let entity = EntityInfo { id: 1, kind: "example:mob".into(), dimension: "minecraft:overworld".into() };
assert!(!entity.has_attached(&value));
```

### `EntityInfo::get_attached_existing`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Does not create data.

`fn get_attached_existing<T: Serialize + DeserializeOwned>(&self, attachment: &AttachmentType<T>) -> Option<T>`

> Missing, invalid, unsupported and out-of-scope access returns None.

```rust
use pluto_lib::{AttachmentRegistry, EntityInfo, Identifier};
let value = AttachmentRegistry::create(Identifier::parse("example:entity_read").unwrap(), || 0u32);
let entity = EntityInfo { id: 1, kind: "example:mob".into(), dimension: "minecraft:overworld".into() };
assert_eq!(entity.get_attached_existing(&value), None);
```

### `EntityInfo::get_attached`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Returns stored data or an unstored default.

`fn get_attached<T: Serialize + DeserializeOwned>(&self, attachment: &AttachmentType<T>) -> T`

> Unlike `get_attached_existing`, which answers `None` when nothing is stored, this always
> answers a value: the stored data when there is some, and the attachment type's default
> when there is not. Use it when the default is a sensible stand-in and you do not need to
> tell “never set” apart.

```rust
use pluto_lib::{AttachmentRegistry, EntityInfo, Identifier};
let value = AttachmentRegistry::create(Identifier::parse("example:entity_default").unwrap(), || 12u32);
let entity = EntityInfo { id: 1, kind: "example:mob".into(), dimension: "minecraft:overworld".into() };
assert_eq!(entity.get_attached(&value), 12);
```

### `EntityInfo::set_attached`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Stores JSON through the current native holder.

`fn set_attached<T: Serialize + DeserializeOwned>(&self, attachment: &AttachmentType<T>, value: T) -> bool`

> Unsupported/out-of-scope, policy-conflicting or unserializable writes return false.

```rust
use pluto_lib::{with_ops, AttachmentRegistry, EntityInfo, Identifier, MemoryPlayerOps};
let value = AttachmentRegistry::create_synced(Identifier::parse("example:entity_write").unwrap(), || false);
let entity = EntityInfo { id: 1, kind: "example:mob".into(), dimension: "minecraft:overworld".into() };
with_ops(&mut MemoryPlayerOps::default(), || assert!(entity.set_attached(&value, true)));
```

### `AttachmentRegistry`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Creates attachment types.

> The factory for attachment types. A mod creates each kind of data it wants to carry on
> players or entities once, keeps the returned AttachmentType, and uses that handle for
> every read and write of the data from then on.

```rust
use pluto_lib::{AttachmentRegistry, Identifier};

let visits = AttachmentRegistry::create(Identifier::new("example", "visits").unwrap(), || 0u32);
assert_eq!(visits.id().to_string(), "example:visits");
```

### `AttachmentRegistry::create`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Makes a new attachment type from an id and a default-value function.

`fn create<T: Serialize + DeserializeOwned>(id: Identifier, default: fn() -> T) -> AttachmentType<T>`

> `default` is the value `get_attached` returns while nothing is stored, so it runs
> whenever a player with no stored data is read. `T` is the stored value's type, and its
> serde bounds in the signature are what let stored values be kept and read back.

```rust
use pluto_lib::{AttachmentRegistry, Identifier};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Quest { stage: u8, done: bool }

let quest = AttachmentRegistry::create(Identifier::new("example", "quest").unwrap(), || Quest { stage: 0, done: false });
let _ = quest;
```

### `AttachmentType`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

A kind of data a player can carry; made by `AttachmentRegistry::create`.

> Keep it in a function, a `static` with `LazyLock`, or recreate it where used: it holds
> only an id and a default.

```rust
use pluto_lib::{AttachmentRegistry, AttachmentType, Identifier};

fn flag() -> AttachmentType<bool> {
    AttachmentRegistry::create(Identifier::new("example", "flag").unwrap(), || false)
}
assert_eq!(flag().id().path(), "flag");
```

### `AttachmentType::id`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The id the value is stored under.

`fn id(&self) -> &Identifier`

> Every read and write of this attachment goes under this id, so it is the identity of the
> stored value. It is the `Identifier` the type was created with, through
> `AttachmentRegistry::create` or `create_synced`.

```rust
use pluto_lib::{AttachmentRegistry, Identifier};

let kind = AttachmentRegistry::create(Identifier::new("example", "n").unwrap(), || 0i64);
assert_eq!(kind.id().namespace(), "example");
```

### `ServerPlayer::set_attached`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Stores the value.

`fn set_attached<T>(&self, attachment: &AttachmentType<T>, value: T) -> bool`

> False if it cannot be stored or the call is outside a callback.

```rust
use pluto_lib::{with_ops, AttachmentRegistry, Identifier, MemoryPlayerOps, ServerPlayer};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
let score = AttachmentRegistry::create(Identifier::new("example", "score").unwrap(), || 0u32);
assert!(!player.set_attached(&score, 10)); // outside a callback: nothing stored

let mut game = MemoryPlayerOps::default();
with_ops(&mut game, || assert!(player.set_attached(&score, 10)));
assert_eq!(game.attachments.len(), 1);
```

### `ServerPlayer::get_attached`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The stored value, or the type's default when nothing is stored (the default is not stored).

`fn get_attached<T>(&self, attachment: &AttachmentType<T>) -> T`

> Because the default is not stored, reading a player who was never written changes
> nothing, and a later `get_attached_existing` still answers `None`. Use that call instead
> when you must tell “never set” from “set to the default”.

```rust
use pluto_lib::{with_ops, AttachmentRegistry, Identifier, MemoryPlayerOps, ServerPlayer};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
let score = AttachmentRegistry::create(Identifier::new("example", "score").unwrap(), || 5u32);
let mut game = MemoryPlayerOps::default();
with_ops(&mut game, || {
    assert_eq!(player.get_attached(&score), 5); // the default
    player.set_attached(&score, 9);
    assert_eq!(player.get_attached(&score), 9);
});
```

### `ServerPlayer::get_attached_existing`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The stored value, or `None` if nothing is stored (or it cannot be read back).

`fn get_attached_existing<T>(&self, attachment: &AttachmentType<T>) -> Option<T>`

> Use it to tell “never set” from “set to the default” — the distinction `get_attached`
> erases by answering the default. Data that cannot be read back also answers `None`, so
> `None` means no usable stored value.

```rust
use pluto_lib::{with_ops, AttachmentRegistry, Identifier, MemoryPlayerOps, ServerPlayer};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
let given = AttachmentRegistry::create(Identifier::new("example", "given").unwrap(), || false);
let mut game = MemoryPlayerOps::default();
with_ops(&mut game, || {
    assert_eq!(player.get_attached_existing(&given), None);
    player.set_attached(&given, false);
    assert_eq!(player.get_attached_existing(&given), Some(false));
});
```

### `ServerPlayer::has_attached`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Whether a value is stored.

`fn has_attached<T>(&self, attachment: &AttachmentType<T>) -> bool`

> Whether a value is currently stored for this player under the given attachment type. It
> answers false when nothing was ever set, which makes it the check to make before
> get_attached when absence is a normal case.

```rust
use pluto_lib::{with_ops, AttachmentRegistry, Identifier, MemoryPlayerOps, ServerPlayer};

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
let seen = AttachmentRegistry::create(Identifier::new("example", "seen").unwrap(), || false);
let mut game = MemoryPlayerOps::default();
with_ops(&mut game, || {
    assert!(!player.has_attached(&seen));
    player.set_attached(&seen, true);
    assert!(player.has_attached(&seen));
});
```
