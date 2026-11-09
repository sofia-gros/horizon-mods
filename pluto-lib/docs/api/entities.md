# Entity events: entering and leaving worlds, damage and death

## Basic Example

Hear when an entity enters or leaves a world:

```rust
use pluto_lib::ServerEntityEvents;

ServerEntityEvents::ENTITY_LOAD.register(|entity| {
    if entity.kind == "minecraft:zombie" {
        println!("a zombie appeared in {}", entity.dimension);
    }
});
```

## Cancelling Damage

`ALLOW_DAMAGE` is asked before damage is applied. Answer `false` to cancel it; one `false` among all the callbacks is enough:

```rust
use pluto_lib::{LivingDamage, ServerLivingEntityEvents};

ServerLivingEntityEvents::ALLOW_DAMAGE.register(|damage: &LivingDamage| damage.damage_type != "minecraft:fall");
```

`attacker` and `direct` are entity ids, negative when there is none: a skeleton's arrow has the skeleton as `attacker` and the arrow as `direct`.

> **Tip.** `AFTER_DEATH` is called when a living entity's death goes ahead, just before it is removed (Horizon's death hook can still cancel it for other mods). `attacker` is the killer's id, negative when there is none; `direct` is always negative and `amount` 0 here. Players are not reported yet.

## Reference

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

### `DamageModification`

Damage about to be applied, for a callback to change. `damage` is what is known of it (its `amount` is the game's and does not change), `amount` the amount now (a `Cell`: change it with `set`) and `attacker_stack` the stack a player attacked with, with its damage, max damage and custom data; None when no player's attack caused it.

```rust
use pluto_lib::{DamageModification, LivingDamage};
use std::cell::Cell;

let damage = LivingDamage { entity: 4, kind: "minecraft:zombie".into(), damage_type: "minecraft:player_attack".into(), attacker: 1, direct: 1, amount: 3.0 };
let hit = DamageModification { damage, amount: Cell::new(3.0), attacker_stack: None };
hit.amount.set(hit.amount.get() + 2.0);
assert_eq!((hit.damage.amount, hit.amount.get()), (3.0, 5.0));
assert!(hit.attacker_stack.is_none());
```

### `ServerLivingEntityEvents::MODIFY_DAMAGE`

NeoForge: `LivingDamageEvent.Pre`. Called for damage that goes ahead, before armor and effects take their share; change the amount with `amount.set`. For a player's attack on the server it carries the stack used.

```rust
use pluto_lib::ServerLivingEntityEvents;

ServerLivingEntityEvents::MODIFY_DAMAGE.register(|hit| {
    if hit.attacker_stack.as_ref().is_some_and(|stack| stack.item == "example:sword") {
        hit.amount.set(hit.amount.get() * 2.0);
    }
});
```

### `ServerLivingEntityEvents::AFTER_DEATH`

An entity died: called when a living entity's death goes ahead, just before it is removed. Players are not reported yet.

```rust
use pluto_lib::ServerLivingEntityEvents;

ServerLivingEntityEvents::AFTER_DEATH.register(|death| {
    let _ = death.entity;
});
```
