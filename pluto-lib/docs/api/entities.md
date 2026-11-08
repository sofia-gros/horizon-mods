# Entity events: entering and leaving worlds, damage and death

## Data

### `EntityInfo`

An entity entering or leaving a world. `id` is the entity id, `kind` its type id such as `minecraft:zombie`, `dimension` the dimension id.

```rust
use pluto_lib::{EntityInfo, ServerEntityEvents};

ServerEntityEvents::ENTITY_LOAD.register(|entity: &EntityInfo| {
    if entity.kind == "minecraft:zombie" && entity.dimension == "minecraft:overworld" {
        let _id = entity.id;
    }
});
ServerEntityEvents::ENTITY_LOAD.invoke(&EntityInfo { id: 5, kind: "minecraft:zombie".into(), dimension: "minecraft:overworld".into() });
```

### `LivingDamage`

Damage about to be applied to a living entity. `entity` is the victim's id and `kind` its type. `damage_type` is the damage type id such as `minecraft:player_attack`. `attacker` is the id of the entity that caused it (negative if none) and `direct` the entity that dealt it directly, a projectile for example (negative if none). `amount` is the damage.

```rust
use pluto_lib::{LivingDamage, ServerLivingEntityEvents};

ServerLivingEntityEvents::ALLOW_DAMAGE.register(|damage: &LivingDamage| {
    // no fall damage for anyone
    damage.damage_type != "minecraft:fall"
});
let fall = LivingDamage {
    entity: 1, kind: "minecraft:player".into(), damage_type: "minecraft:fall".into(), attacker: -1, direct: -1, amount: 4.0,
};
assert!(!ServerLivingEntityEvents::ALLOW_DAMAGE.invoke(&fall));
```

## Events

### `ServerEntityEvents`

Entity load and unload events.

```rust
use pluto_lib::ServerEntityEvents;

ServerEntityEvents::ENTITY_LOAD.register(|_| {});
ServerEntityEvents::ENTITY_UNLOAD.register(|_| {});
```

### `ServerEntityEvents::ENTITY_LOAD`

An entity joined a world (spawned, or loaded with its chunk).

```rust
use pluto_lib::{EntityInfo, ServerEntityEvents};

ServerEntityEvents::ENTITY_LOAD.register(|e| println!("{} {} loaded", e.kind, e.id));
ServerEntityEvents::ENTITY_LOAD.invoke(&EntityInfo { id: 9, kind: "minecraft:cow".into(), dimension: "minecraft:overworld".into() });
```

### `ServerEntityEvents::ENTITY_UNLOAD`

An entity left a world (died, was removed, or its chunk unloaded).

```rust
use pluto_lib::{EntityInfo, ServerEntityEvents};

ServerEntityEvents::ENTITY_UNLOAD.register(|e| println!("{} {} unloaded", e.kind, e.id));
ServerEntityEvents::ENTITY_UNLOAD.invoke(&EntityInfo { id: 9, kind: "minecraft:cow".into(), dimension: "minecraft:overworld".into() });
```

### `ServerLivingEntityEvents`

Events for living entities.

```rust
use pluto_lib::ServerLivingEntityEvents;

ServerLivingEntityEvents::ALLOW_DAMAGE.register(|_| true);
```

### `ServerLivingEntityEvents::ALLOW_DAMAGE`

Damage is about to be applied. Answer `false` to cancel it; any `false` cancels.

```rust
use pluto_lib::{LivingDamage, ServerLivingEntityEvents};

ServerLivingEntityEvents::ALLOW_DAMAGE.register(|d| d.amount < 100.0);
let hit = LivingDamage { entity: 2, kind: "minecraft:cow".into(), damage_type: "minecraft:generic".into(), attacker: -1, direct: -1, amount: 3.0 };
assert!(ServerLivingEntityEvents::ALLOW_DAMAGE.invoke(&hit));
```

### `ServerLivingEntityEvents::AFTER_DEATH`

Stub: an entity died. Accepted, not invoked yet (support matrix).

```rust
use pluto_lib::ServerLivingEntityEvents;

ServerLivingEntityEvents::AFTER_DEATH.register(|death| {
    let _ = death.entity;
});
```
