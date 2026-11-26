# Particles

A mod declares its particle types — a name plus what one particle of the type is made of — and sends
particles into the world from its logic. The original's types are the model: a slime's landing burst is
a fragment of its slime ball, a weapon's swing draws a short animated slash from a sprite set, and a
fluid effect draws the fluid itself.

```rust
use pluto_lib::{Identifier, ParticleFactory, ParticleTypeRegistry, StackData};

// The type is a fragment of an item stack (the standard BreakingItemParticle constructor).
ParticleTypeRegistry::register(
    Identifier::parse("example:sky_slime").unwrap(),
    ParticleFactory::ItemCrumb(StackData { item: "example:sky_slime_ball".into(), count: 1, ..Default::default() }),
).unwrap();
// The type is a textured billboard from the sprite set shipped in the mod's assets.
ParticleTypeRegistry::register(
    Identifier::parse("example:slash").unwrap(),
    ParticleFactory::SpriteSet { always_spawn: true },
).unwrap();
// The type draws the fluid each spawn carries.
ParticleTypeRegistry::register(
    Identifier::parse("example:fluid").unwrap(),
    ParticleFactory::FluidSprite,
).unwrap();
```

Sending is part of the world handle: [`World::send_particles`](#worldsend_particles) takes the type's
name, the spawn's data and the vanilla send shape (centre, Gaussian spread and speed, count — with 0
meaning one aimed particle). A client draws a mod's type from the sprite set in the mod's
`assets/<namespace>/particles/<path>.json`; the file may carry the host's `horizon:behaviour` block
(animation, lifetime, gravity, friction, size and alpha ramps) to shape the billboard.

> **Use with Caution.** Declarations, validation and the memory host are pure-tested. The send's host
> backend is source-declared: it resolves a vanilla name to the host's own type and sends any other
> namespaced name by name through the level bridge (the host's documented named-particle path), but
> that call has not been natively attached yet. A fluid-sprite spawn is sent by name like any other:
> the host's sprite sets are static per type, so it has no channel for a sprite chosen from the spawn's
> fluid, and the spawn draws nothing until the host grows one. Item-crumb factories ride the
> item-factory registry's existing transports (the named-request bridge and the cube-landing override).

## Reference

### `ParticleFactory`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: inert](https://img.shields.io/badge/Sunrise-inert-yellow)

What one particle of a registered type is made of: the type's client factory, in the original's
terms. `ItemCrumb` particles are fragments of the held stack; `SpriteSet` particles are billboards
from the sprite set the mod ships under the type's name; `FluidSprite` particles are billboards
textured by the still texture of the fluid each spawn carries, tinted with the fluid's colour.

```rust
use pluto_lib::{ParticleFactory, StackData};

let crumb = ParticleFactory::ItemCrumb(StackData { item: "minecraft:clay_ball".into(), count: 1, ..Default::default() });
let slash = ParticleFactory::SpriteSet { always_spawn: true };
let fluid = ParticleFactory::FluidSprite;
assert_ne!(crumb, slash);
assert_ne!(slash, fluid);
```

### `ParticleData`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: inert](https://img.shields.io/badge/Sunrise-inert-yellow)

The per-spawn data a particle type's factory reads, beyond the type itself. Most types take `Plain`;
a fluid-sprite type takes the fluid stack the spawn draws — fluid, amount and tag together, as the
original's fluid particle data carries them.

```rust
use pluto_lib::{FluidStack, Identifier, ParticleData};

let plain = ParticleData::Plain;
let fluid = ParticleData::Fluid(FluidStack::new(Identifier::parse("example:molten_glass").unwrap(), 250));
assert_ne!(plain, fluid);
```

### `ParticleTypeRegistry`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: inert](https://img.shields.io/badge/Sunrise-inert-yellow)

The declared particle types of every mod, by name. Registration is process-lifetime and
re-registering a name replaces its factory. The registry is the declaration store only: an item-crumb
type's stack is held by the item-factory registry (whose transports the host consults), and a
sprite-set type's sprites ship as data in the mod's assets.

```rust
use pluto_lib::{Identifier, ParticleFactory, ParticleTypeRegistry};

let id = Identifier::parse("example:puff").unwrap();
assert_eq!(ParticleTypeRegistry::factory(&id), None);
ParticleTypeRegistry::register(id.clone(), ParticleFactory::SpriteSet { always_spawn: false }).unwrap();
assert!(ParticleTypeRegistry::registered().contains(&"example:puff".to_owned()));
```

### `ParticleTypeRegistry::register`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: inert](https://img.shields.io/badge/Sunrise-inert-yellow)

Declares the particle type with its factory. An item-crumb factory is validated and stored by the
item-factory registry first — a valid item id, a count in range, a positive durability override — so
a rejected stack leaves any previous declaration of the type, and of its factory, intact.

```rust
use pluto_lib::{Identifier, ParticleFactory, ParticleTypeRegistry, StackData};

let id = Identifier::parse("example:crumb").unwrap();
ParticleTypeRegistry::register(id.clone(), ParticleFactory::ItemCrumb(StackData {
    item: "minecraft:clay_ball".into(), count: 1, ..Default::default()
})).unwrap();
let mut bad = StackData { item: "minecraft:clay_ball".into(), count: 1, ..Default::default() };
bad.count = 0;
assert!(ParticleTypeRegistry::register(id.clone(), ParticleFactory::ItemCrumb(bad)).is_err());
```

### `ParticleTypeRegistry::factory`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: inert](https://img.shields.io/badge/Sunrise-inert-yellow)

The declared factory of a type, or None when no mod declared it. An item-crumb factory is read back
from the item-factory registry, so a stack replaced there is the factory seen here.

```rust
use pluto_lib::{Identifier, ParticleFactory, ParticleTypeRegistry};

let id = Identifier::parse("example:mist").unwrap();
ParticleTypeRegistry::register(id.clone(), ParticleFactory::FluidSprite).unwrap();
assert_eq!(ParticleTypeRegistry::factory(&id), Some(ParticleFactory::FluidSprite));
```

### `ParticleTypeRegistry::registered`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: inert](https://img.shields.io/badge/Sunrise-inert-yellow)

Every declared particle type's name, sorted: the registry's own census, for tests and tools.

```rust
use pluto_lib::{Identifier, ParticleFactory, ParticleTypeRegistry};

ParticleTypeRegistry::register(
    Identifier::parse("example:census").unwrap(),
    ParticleFactory::SpriteSet { always_spawn: false },
).unwrap();
assert!(ParticleTypeRegistry::registered().iter().any(|name| name == "example:census"));
```

### `ParticleTypeRegistry::register_entity_particle`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: inert](https://img.shields.io/badge/Sunrise-inert-yellow)

Replaces one entity type's inherited particle with a registered item-crumb type — the cube landing
override, by which a mod's cube bursts fragments of its own item where the vanilla cube bursts the
vanilla slime item. The replacement must be registered as an item crumb first, and its namespace must
not be `minecraft`.

```rust
use pluto_lib::{Identifier, ParticleFactory, ParticleTypeRegistry, StackData};

let replacement = Identifier::parse("example:landing_crumb").unwrap();
ParticleTypeRegistry::register(replacement.clone(), ParticleFactory::ItemCrumb(StackData {
    item: "minecraft:clay_ball".into(), count: 1, ..Default::default()
})).unwrap();
ParticleTypeRegistry::register_entity_particle(
    Identifier::parse("example:landing_cube").unwrap(),
    Identifier::parse("minecraft:item_slime").unwrap(),
    replacement,
).unwrap();
```

### `World::send_particles`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

Sends particles of a type to the players near a point (Minecraft: `ServerLevel.sendParticles`):
`count` of them, each offset by a Gaussian times the spread and moving at a Gaussian times the speed;
a count of 0 sends one particle moving along the spread scaled by the speed, which is how the
original aims a single attack particle. The spawn's data rides along — the fluid for a fluid-sprite
type, `ParticleData::Plain` for the rest. True means the game accepted the send, not that every
client drew it: a client without a mod type's sprite set draws nothing, and a vanilla client is not
sent a mod's type at all. False outside a callback, for a position, spread or speed the game would
refuse, for a negative count, or when the host cannot send.

```rust
use pluto_lib::{MemoryWorldOps, ParticleData, World, with_world};

let world = World::new("minecraft:overworld");
let mut ops = MemoryWorldOps::default();
with_world(&mut ops, || {
    assert!(world.send_particles("minecraft:flame", &ParticleData::Plain, [0.5, 64.0, 0.5], [0.1, 0.2, 0.1], 0.2, 10));
    assert!(!world.send_particles("minecraft:flame", &ParticleData::Plain, [0.5, 64.0, 0.5], [0.1, 0.2, 0.1], 0.2, -1));
});
assert_eq!(ops.particles.len(), 1);
```

### `ParticleRecord`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

One particle send a `MemoryWorldOps` was asked for, as it was asked: the `dimension` it was sent in,
the `particle` type's name, the spawn's `data`, the centre `at`, the Gaussian `spread` scale per
axis, the `speed` scale, and the `count` (0 is one particle moving along the spread).

```rust
use pluto_lib::{MemoryWorldOps, ParticleData, ParticleRecord, World, with_world};

let world = World::new("minecraft:overworld");
let mut ops = MemoryWorldOps::default();
with_world(&mut ops, || {
    assert!(world.send_particles("example:slash", &ParticleData::Plain, [1.5, 64.0, -2.5], [0.1, 0.2, 0.1], 0.2, 10));
});
assert_eq!(ops.particles, vec![ParticleRecord {
    dimension: "minecraft:overworld".into(),
    particle: "example:slash".into(),
    data: ParticleData::Plain,
    at: [1.5, 64.0, -2.5],
    spread: [0.1, 0.2, 0.1],
    speed: 0.2,
    count: 10,
}]);
```
