# Projectiles

Be told when a projectile hits something: an arrow, a snowball, a thrown item of a mod. A callback sees what flew, who threw it and what it met, can change the world where it landed with [world](world.md), and can cancel the hit. This page first walks through using it, then lists every item in the [Reference](#reference).

## Basic Example

A thrown item that lights up the spot where it lands, like a glow ball:

```rust
use pluto_lib::{BlockState, Identifier, ProjectileEvents};

ProjectileEvents::IMPACT.register(|hit| {
    if hit.kind == "example:glow_ball" {
        let at = [hit.block[0] + hit.face[0], hit.block[1] + hit.face[1], hit.block[2] + hit.face[2]];
        hit.world().set_block_state(at, &BlockState::new(Identifier::new("minecraft", "light").unwrap()));
    }
    true // let the hit happen as usual
});
```

`hit.block` is the block it hit and `hit.face` the face, so adding them gives the air block in front of that face.

## Cancelling a Hit

Answer `false` and the hit does not happen; the projectile flies on. One `false` among all the callbacks is enough:

```rust
use pluto_lib::ProjectileEvents;

// arrows pass through the open air of a protected area
ProjectileEvents::IMPACT.register(|hit| !(hit.kind == "minecraft:arrow" && hit.block[1] > 200));
```

## Telling Targets Apart

`hit.entity` is the entity it hit (and `entity_kind` its type); for a hit on a block they are `None` and empty:

```rust
use pluto_lib::ProjectileEvents;

ProjectileEvents::IMPACT.register(|hit| {
    if let Some(id) = hit.entity {
        println!("a {} hit entity {id} ({})", hit.kind, hit.entity_kind);
    }
    true
});
```

> **Use with Caution.** The game only keeps the copies of projectiles that this needs while some mod listens, so listening costs something every tick even for projectiles you ignore. Register a listener only if you need it, and return quickly for projectile types you do not care about. A dragon part or End crystal is reported by the id of the dragon or crystal; a breeze's deflection and fishing bobbers are not reported.

## Reference

### `ProjectileImpact`

A projectile that has found what it meets. `dimension` is the dimension id (also `world()`), `projectile` the projectile's entity id, `kind` its type (`minecraft:snowball` or a mod's thrown type), `owner` the entity that threw it (negative if none), `entity` the entity it hit, `entity_kind` that entity's type (empty for a block), `block` the block it hit, `face` the face as a direction vector (`[0, 1, 0]` is the top) and `at` where it hit.

```rust
use pluto_lib::ProjectileImpact;

let hit = ProjectileImpact {
    dimension: "minecraft:overworld".into(),
    projectile: 5,
    kind: "minecraft:snowball".into(),
    owner: 1,
    entity: None,
    entity_kind: String::new(),
    block: [3, 64, 3],
    face: [0, 1, 0],
    at: [3.5, 65.0, 3.5],
};
assert_eq!(hit.block[1] + hit.face[1], 65);
assert_eq!(hit.at[1], 65.0);
assert_eq!((hit.projectile, hit.owner), (5, 1));
assert!(hit.entity.is_none() && hit.entity_kind.is_empty());
assert_eq!(hit.kind, "minecraft:snowball");
```

### `ProjectileImpact::world`

`fn world(&self) -> World`. The world the projectile is in, to read and change blocks with.

```rust
use pluto_lib::ProjectileImpact;

fn dimension_of(hit: &ProjectileImpact) -> String {
    hit.world().dimension
}
let hit = ProjectileImpact {
    dimension: "minecraft:the_nether".into(), projectile: 1, kind: "minecraft:arrow".into(), owner: 0, entity: None,
    entity_kind: String::new(), block: [0; 3], face: [0, 1, 0], at: [0.0; 3],
};
assert_eq!(dimension_of(&hit), "minecraft:the_nether");
```

### `ProjectileEvents`

Events about projectiles.

```rust
use pluto_lib::ProjectileEvents;

ProjectileEvents::IMPACT.register(|_| true);
```

### `ProjectileEvents::IMPACT`

A projectile hit something, before the hit acts. Answer `false` to cancel the hit; one `false` is enough. Heard for the projectiles the game flies: arrows, tridents, snowballs and mod thrown types, eggs, pearls, potions, fireballs, wither skulls, wind charges, llama spit and shulker bullets.

```rust
use pluto_lib::{ProjectileEvents, ProjectileImpact};

ProjectileEvents::IMPACT.register(|hit| hit.kind != "minecraft:egg");
let egg = ProjectileImpact {
    dimension: "minecraft:overworld".into(), projectile: 2, kind: "minecraft:egg".into(), owner: 1, entity: None,
    entity_kind: String::new(), block: [0; 3], face: [0, 1, 0], at: [0.0; 3],
};
assert!(!ProjectileEvents::IMPACT.invoke(&egg));
```

### `ProjectileEvents::LAUNCH`

A projectile entered a server level (Bukkit: `ProjectileLaunchEvent`; Fabric has no counterpart), heard after it exists, so nothing here can stop it. It is heard for every projectile the game adds, a bow's arrow included, whoever launched it: `launch.player` is set only when a player did.

```rust
use pluto_lib::ProjectileEvents;

ProjectileEvents::LAUNCH.register(|launch| {
    if launch.player.is_some() && launch.kind == "minecraft:snowball" {
        println!("{} threw snowball {}", launch.name, launch.projectile);
    }
});
```

### `ProjectileLaunch`

A projectile that entered a level. `player` is the launching player's uuid (`None` for a dispenser's or a mob's), `name` their name (empty without a player), `projectile` the projectile's entity id and `kind` its type.

```rust
use pluto_lib::ProjectileLaunch;

let launch = ProjectileLaunch { player: Some(7), name: "Alex".into(), projectile: 12, kind: "minecraft:arrow".into() };
assert_eq!((launch.player, launch.projectile), (Some(7), 12));
assert_eq!((launch.name.as_str(), launch.kind.as_str()), ("Alex", "minecraft:arrow"));
```

### `ProjectileEvents::SHOOT_BOW`

A player's bow or crossbow shot an arrow, heard on a server after the arrow entered the level, once for each arrow (a multishot crossbow reports three). Push `EntityAction`s for the player onto `shot.actions` (`ActionTarget::Attacker`), for example a status effect for the shooter; the bow is in `shot.main_hand` or `shot.off_hand`.

```rust
use pluto_lib::{ActionTarget, EntityAction, ProjectileEvents};

ProjectileEvents::SHOOT_BOW.register(|shot| {
    if shot.force >= 1.0 && shot.main_hand.as_ref().is_some_and(|bow| bow.item == "example:longbow") {
        shot.actions.borrow_mut().push(EntityAction::AddEffect {
            target: ActionTarget::Attacker, effect: "minecraft:speed".into(), amplifier: 0, duration: 60,
        });
    }
});
```

### `BowShot`

An arrow shot by a player's bow or crossbow. `player` is the shooting player's uuid and `name` their name, `projectile` the arrow's entity id, `force` the arrow's launch speed as the game reports it (a fully drawn bow gives 3.0, a weaker draw less, a crossbow its fixed power), `main_hand` and `off_hand` the stacks in the player's hands as the arrow left (damage, max damage and custom data; `None` for an empty hand) and `actions` the `EntityAction`s to do to the player.

```rust
use pluto_lib::{BowShot, ProjectileEvents};

fn full_draw(shot: &BowShot) -> bool {
    shot.force >= 1.0
}
ProjectileEvents::SHOOT_BOW.register(|shot| {
    let _ = (shot.player, &shot.name, shot.projectile, &shot.off_hand);
    assert!(shot.actions.borrow().is_empty() || full_draw(shot));
});
```
