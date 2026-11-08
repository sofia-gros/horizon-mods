# Data attachments: values saved with a player

Attach any `serde` value to a player and read it back later, also after the game restarts (the value is stored as JSON with the player). Only players can carry attachments so far; worlds, chunks, block entities and other entities follow.

Like every `ServerPlayer` method, these work only inside a callback that carries the player (`with_ops`); elsewhere they report "nothing there" and store nothing.

### `AttachmentRegistry`

Creates attachment types.

```rust
use pluto_lib::{AttachmentRegistry, Identifier};

let visits = AttachmentRegistry::create(Identifier::new("example", "visits").unwrap(), || 0u32);
assert_eq!(visits.id().to_string(), "example:visits");
```

### `AttachmentRegistry::create`

`fn create<T: Serialize + DeserializeOwned>(id: Identifier, default: fn() -> T) -> AttachmentType<T>`. `default` is the value `get_attached` returns while nothing is stored.

```rust
use pluto_lib::{AttachmentRegistry, Identifier};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Quest { stage: u8, done: bool }

let quest = AttachmentRegistry::create(Identifier::new("example", "quest").unwrap(), || Quest { stage: 0, done: false });
let _ = quest;
```

### `AttachmentType`

A kind of data a player can carry; made by `AttachmentRegistry::create`. Keep it in a function, a `static` with `LazyLock`, or recreate it where used: it holds only an id and a default.

```rust
use pluto_lib::{AttachmentRegistry, AttachmentType, Identifier};

fn flag() -> AttachmentType<bool> {
    AttachmentRegistry::create(Identifier::new("example", "flag").unwrap(), || false)
}
assert_eq!(flag().id().path(), "flag");
```

### `AttachmentType::id`

`fn id(&self) -> &Identifier`. The id the value is stored under.

```rust
use pluto_lib::{AttachmentRegistry, Identifier};

let kind = AttachmentRegistry::create(Identifier::new("example", "n").unwrap(), || 0i64);
assert_eq!(kind.id().namespace(), "example");
```

### `ServerPlayer::set_attached`

`fn set_attached<T>(&self, attachment: &AttachmentType<T>, value: T) -> bool`. Stores the value. False if it cannot be stored or the call is outside a callback.

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

`fn get_attached<T>(&self, attachment: &AttachmentType<T>) -> T`. The stored value, or the type's default when nothing is stored (the default is not stored).

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

`fn get_attached_existing<T>(&self, attachment: &AttachmentType<T>) -> Option<T>`. The stored value, or `None` if nothing is stored (or it cannot be read back). Use it to tell "never set" from "set to the default".

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

`fn has_attached<T>(&self, attachment: &AttachmentType<T>) -> bool`. Whether a value is stored.

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
