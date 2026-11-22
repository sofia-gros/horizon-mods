# Projectiles

Be told when a projectile hits something: an arrow, a snowball, a thrown item of a mod. A callback sees what flew, who threw it and what it met, can change the world where it landed with [world](world.md), and can cancel the hit. This page first walks through using it, then lists every item in the [Reference](#reference).

A callback hears a projectile when it enters a level and when it lands, and nowhere between: the game's entity-tick telling covers mobs, item entities, minecarts and boats, not projectiles, so nothing here can follow one in flight. What acts at the two ends is all here — the impact reports the motion and the saved data the projectile arrived with and lets a callback discard it or send it flying on with a new motion, which is what pierce, bounce and an explosion where it lands are made of. Flight behaviours — a projectile that falls faster, a burning fuse, one that pulls mobs toward it or lights its path — stay unimplementable until the game tells projectile ticks.

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

A projectile that has found what it meets. `dimension` is the dimension id (also `world()`), `projectile` the projectile's entity id, `kind` its type (`minecraft:snowball` or a mod's thrown type), `owner` the entity that threw it (negative if none), `entity` the entity it hit, `entity_kind` that entity's type (empty for a block), `block` the block it hit, `face` the face as a direction vector (`[0, 1, 0]` is the top) and `at` where it hit. `motion` is how fast it was moving when it hit, in blocks a tick, and `data` the projectile's saved data — what its spawn data and the stack that fired it left on it, so a launcher that puts its modifiers in the fired stack's data reads them back here. Two outcomes a callback settles on the projectile itself: set `discard` to have the game remove it as the callbacks return, or set `motion_after` to the motion it flies on with — with a cancelled hit, a bounce. And `actions` is the queue of actions for the hit's parties (`Victim` the entity it hit, `Attacker` the thrower), which the game does after the callbacks answer.

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
    motion: [0.0, -1.5, 0.25],
    data: Default::default(),
    discard: Default::default(),
    motion_after: Default::default(),
    actions: Default::default(),
};
assert_eq!(hit.block[1] + hit.face[1], 65);
assert_eq!(hit.at[1], 65.0);
assert_eq!((hit.projectile, hit.owner), (5, 1));
assert!(hit.entity.is_none() && hit.entity_kind.is_empty());
assert_eq!(hit.kind, "minecraft:snowball");
assert_eq!(hit.motion[1], -1.5);
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
    entity_kind: String::new(), block: [0; 3], face: [0, 1, 0], at: [0.0; 3], motion: [0.0; 3], data: Default::default(),
    discard: Default::default(), motion_after: Default::default(), actions: Default::default(),
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

A projectile hit something, before the hit acts. Answer `false` to cancel the hit; one `false` is enough. Heard for the projectiles the game flies: arrows, tridents, snowballs and mod thrown types, eggs, pearls, potions, fireballs, wither skulls, wind charges, llama spit and shulker bullets. While callbacks run, the entities around the hit are in reach (`EntityReach`), and the actions a callback queues on the hit's `actions` are done to its parties afterwards — the entity it hit as the victim, the thrower as the attacker — whether the hit is cancelled or not.

```rust
use pluto_lib::{ProjectileEvents, ProjectileImpact};

ProjectileEvents::IMPACT.register(|hit| hit.kind != "minecraft:egg");
let egg = ProjectileImpact {
    dimension: "minecraft:overworld".into(), projectile: 2, kind: "minecraft:egg".into(), owner: 1, entity: None,
    entity_kind: String::new(), block: [0; 3], face: [0, 1, 0], at: [0.0; 3], motion: [0.0; 3], data: Default::default(),
    discard: Default::default(), motion_after: Default::default(), actions: Default::default(),
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

A projectile that entered a level. `player` is the launching player's uuid (`None` for a dispenser's or a mob's), `name` their name (empty without a player), `projectile` the projectile's entity id and `kind` its type. `motion` is how fast it is moving as it enters, in blocks a tick, and `data` its saved data, as on `ProjectileImpact`.

```rust
use pluto_lib::ProjectileLaunch;

let launch = ProjectileLaunch { player: Some(7), name: "Alex".into(), projectile: 12, kind: "minecraft:arrow".into(), motion: [0.0, 0.5, -3.0], data: Default::default() };
assert_eq!((launch.player, launch.projectile), (Some(7), 12));
assert_eq!((launch.name.as_str(), launch.kind.as_str()), ("Alex", "minecraft:arrow"));
assert_eq!(launch.motion[2], -3.0);
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

## Registering a thrown type

A thrown type is a projectile that flies as an item — a copy of the snowball that deals damage: its id is the projectile's entity kind, `item` is what it flies as and drops, and `damage` is what a hit hurts for. Register the types a mod throws once, while it loads:

```rust
use pluto_lib::{Identifier, ThrownType, ThrownTypeRegistry};

let shuriken = ThrownType::new(
    Identifier::parse("example:shuriken_thrown").unwrap(),
    Identifier::parse("example:shuriken").unwrap(),
    4.0,
).unwrap();
ThrownTypeRegistry::register(shuriken).unwrap();
assert_eq!(ThrownTypeRegistry::get(&Identifier::parse("example:shuriken_thrown").unwrap()).unwrap().damage, 4.0);
```

> **Use with Caution.** Registration is the trait-mod source of truth, validated as the game validates the content form; delivery to the game's own registry is a wall. The game fills its thrown-type registry once, at startup, from content mods' entity types alone, and no entry or provider seam reaches it for a trait mod, so a type registered here must also be declared in the mod's content entity types to fly in the game. `ThrownTypeRegistry::declarations()` hands over exactly what that declaration needs: each type's id, its `item` and its `damage`, on the `minecraft:snowball` template.

## Spawning projectiles

Spawning goes through the world's summon (see [world](world.md)): any projectile kind loads from its saved data, a registered thrown type included. `spawn_projectile` assembles the data — the kind's own data with `Motion` and `Owner` written from the spawn's fields — and answers the new entity's id, the id its hit is later reported under in `ProjectileEvents::IMPACT`:

```rust
use pluto_lib::{Identifier, MemoryWorldOps, ProjectileSpawn, World, spawn_projectile, with_world};

let world = World::new("minecraft:overworld");
let spawn = ProjectileSpawn::new(Identifier::parse("minecraft:snowball").unwrap(), [0.0, 64.0, 0.0], [1.0, 1.0, 0.0]);
assert_eq!(spawn_projectile(&world, &spawn), None, "outside a world scope");
let mut ops = MemoryWorldOps::default();
with_world(&mut ops, || assert_eq!(spawn_projectile(&world, &spawn), Some(1)));
assert_eq!(ops.summons[0].kind, "minecraft:snowball");
```

> **Use with Caution.** Host adapter written, not yet compiled or game-verified. `HostWorld` in `hooks.rs` summons through the host's `/summon` call; a kind the game cannot load, an unloaded chunk or an out-of-bounds position answers `None`.

### `ThrownType`

A thrown item type: `id` (never a `minecraft:` id — a mod's type must not shadow a vanilla one), `item` and `damage`. `ThrownType::new` validates as the game validates its content form: the damage must be finite and not negative.

```rust
use pluto_lib::{Identifier, ThrownType};

let thrown = ThrownType::new(Identifier::parse("example:rock_thrown").unwrap(), Identifier::parse("example:rock").unwrap(), 2.0).unwrap();
assert_eq!(thrown.id.to_string(), "example:rock_thrown");
assert_eq!(thrown.item.to_string(), "example:rock");
assert_eq!(thrown.damage, 2.0);
```

### `ThrownTypeRegistry`

The registry of thrown types. `register` fails, keeping the earlier registration, for a duplicate id or after the declarations have been collected. `get(id)` returns the registered type. `declarations()` returns every registered type, by id, and freezes the registry, as the game's own set-once registry does.

```rust
use pluto_lib::{Identifier, ThrownType, ThrownTypeRegistry};

let id = Identifier::parse("example:pebble_thrown").unwrap();
ThrownTypeRegistry::register(ThrownType::new(id.clone(), Identifier::parse("example:pebble").unwrap(), 1.0).unwrap()).unwrap();
assert_eq!(ThrownTypeRegistry::get(&id).unwrap().damage, 1.0);
assert!(ThrownTypeRegistry::get(&Identifier::parse("example:missing_thrown").unwrap()).is_none());
```

### `ProjectileSpawn`

A projectile to spawn: `kind`, `position`, `motion` (blocks per tick), `owner` (the throwing entity's uuid, if any) and `data`, the kind's own saved data. `ProjectileSpawn::new(kind, position, motion)` starts with no owner and no data; `.owner(uuid)` and `.data(nbt)` add them. `to_nbt()` is the saved data the spawn loads from: the kind's data with `Motion` (a list of three doubles) and `Owner` (a uuid as four ints, most significant first) written from the fields, replacing any in the data.

```rust
use pluto_lib::{Identifier, Nbt, ProjectileSpawn};

let spawn = ProjectileSpawn::new(Identifier::parse("minecraft:arrow").unwrap(), [0.0; 3], [0.0, -0.5, 2.0])
    .owner(0x0011_2233_4455_6677_8899_AABB_CCDD_EEFF);
let nbt = spawn.to_nbt();
assert_eq!(nbt.get("Motion"), Some(&Nbt::List(vec![Nbt::Double(0.0), Nbt::Double(-0.5), Nbt::Double(2.0)])));
assert_eq!(nbt.get("Owner"), Some(&Nbt::IntArray(vec![0x0011_2233, 0x4455_6677, -0x7766_5545, -0x3322_1101])));
```

### `spawn_projectile`

`fn spawn_projectile(world: &World, spawn: &ProjectileSpawn) -> Option<u32>`. Spawns the projectile in `world` and returns the new entity's id; `None` when the world cannot summon there or outside a callback that carries a world.

```rust
use pluto_lib::{Identifier, MemoryWorldOps, ProjectileSpawn, World, spawn_projectile, with_world};

let world = World::new("minecraft:overworld");
let spawn = ProjectileSpawn::new(Identifier::parse("minecraft:snowball").unwrap(), [0.0, 64.0, 0.0], [1.0, 1.0, 0.0]);
assert_eq!(spawn_projectile(&world, &spawn), None, "outside a world scope");
let mut ops = MemoryWorldOps::default();
with_world(&mut ops, || assert_eq!(spawn_projectile(&world, &spawn), Some(1)));
assert_eq!(ops.summons[0].kind, "minecraft:snowball");
```

### `spawn_thrown`

`fn spawn_thrown(world: &World, thrown: &Identifier, position: [f64; 3], motion: [f64; 3], owner: Option<u128>, data: NbtCompound) -> Option<u32>`. Spawns a registered thrown type by its id; `None` when no type is registered under `thrown`.

```rust
use pluto_lib::{Identifier, MemoryWorldOps, NbtCompound, ThrownType, ThrownTypeRegistry, World, spawn_thrown, with_world};

let id = Identifier::parse("example:dart_thrown").unwrap();
ThrownTypeRegistry::register(ThrownType::new(id.clone(), Identifier::parse("example:dart").unwrap(), 3.0).unwrap()).unwrap();
let world = World::new("minecraft:overworld");
let mut ops = MemoryWorldOps::default();
with_world(&mut ops, || assert_eq!(spawn_thrown(&world, &id, [0.0, 64.0, 0.0], [0.0, 0.5, 1.0], None, NbtCompound::new()), Some(1)));
assert_eq!(ops.summons[0].kind, "example:dart_thrown");
assert_eq!(spawn_thrown(&world, &Identifier::parse("example:unregistered_thrown").unwrap(), [0.0; 3], [0.0; 3], None, NbtCompound::new()), None);
```

### `uuid_int_array`

`fn uuid_int_array(uuid: u128) -> [i32; 4]`. A uuid as the four ints entity data holds it in (`UUIDUtil.uuidToIntArray`), most significant first.

```rust
use pluto_lib::uuid_int_array;

assert_eq!(uuid_int_array(0x0011_2233_4455_6677_8899_AABB_CCDD_EEFF), [0x0011_2233, 0x4455_6677, -0x7766_5545, -0x3322_1101]);
```

### `ThrownType::new`

`fn new(id: Identifier, item: Identifier, damage: f32) -> Result<ThrownType, String>`. A thrown type validated as the game validates its content form: a `minecraft:` id and a non-finite or negative `damage` are errors.

```rust
use pluto_lib::{Identifier, ThrownType};

assert!(ThrownType::new(Identifier::parse("example:ok_thrown").unwrap(), Identifier::parse("example:ok").unwrap(), 0.0).is_ok());
assert!(ThrownType::new(Identifier::parse("minecraft:arrow").unwrap(), Identifier::parse("minecraft:arrow").unwrap(), 1.0).is_err());
assert!(ThrownType::new(Identifier::parse("example:bad_thrown").unwrap(), Identifier::parse("example:bad").unwrap(), -1.0).is_err());
assert!(ThrownType::new(Identifier::parse("example:bad_thrown").unwrap(), Identifier::parse("example:bad").unwrap(), f32::NAN).is_err());
```

### `ThrownTypeRegistry::register`

`fn register(thrown: ThrownType) -> Result<(), String>`. Registers a thrown type; a duplicate id or registration after the declarations have been collected fails, keeping the earlier registration.

```rust
use pluto_lib::{Identifier, ThrownType, ThrownTypeRegistry};

let thrown = ThrownType::new(Identifier::parse("example:stone_thrown").unwrap(), Identifier::parse("example:stone").unwrap(), 2.0).unwrap();
ThrownTypeRegistry::register(thrown.clone()).unwrap();
assert!(ThrownTypeRegistry::register(thrown).is_err(), "duplicate id");
```

### `ThrownTypeRegistry::get`

`fn get(id: &Identifier) -> Option<ThrownType>`. The thrown type registered under `id`, if any.

```rust
use pluto_lib::{Identifier, ThrownType, ThrownTypeRegistry};

let id = Identifier::parse("example:orb_thrown").unwrap();
assert!(ThrownTypeRegistry::get(&id).is_none());
ThrownTypeRegistry::register(ThrownType::new(id.clone(), Identifier::parse("example:orb").unwrap(), 5.0).unwrap()).unwrap();
assert_eq!(ThrownTypeRegistry::get(&id).unwrap().item.to_string(), "example:orb");
```

### `ThrownTypeRegistry::declarations`

`fn declarations() -> Result<Vec<ThrownType>, String>`. Every registered thrown type, by id; collecting them freezes the registry, as the game's own set-once registry does.

```rust
use pluto_lib::{Identifier, ThrownType, ThrownTypeRegistry};

ThrownTypeRegistry::register(ThrownType::new(Identifier::parse("example:gem_thrown").unwrap(), Identifier::parse("example:gem").unwrap(), 4.0).unwrap()).unwrap();
let declarations = ThrownTypeRegistry::declarations().unwrap();
assert_eq!(declarations.len(), 1);
assert_eq!(declarations[0].id.to_string(), "example:gem_thrown");
assert!(ThrownTypeRegistry::register(ThrownType::new(Identifier::parse("example:late_thrown").unwrap(), Identifier::parse("example:late").unwrap(), 1.0).unwrap()).is_err(), "frozen after collection");
```

### `ProjectileSpawn::new`

`fn new(kind: Identifier, position: [f64; 3], motion: [f64; 3]) -> ProjectileSpawn`. A projectile of `kind` at `position` with `motion`, with no owner and no extra data.

```rust
use pluto_lib::{Identifier, ProjectileSpawn};

let spawn = ProjectileSpawn::new(Identifier::parse("minecraft:arrow").unwrap(), [1.0, 64.0, 1.0], [0.0, 0.5, -2.0]);
assert_eq!(spawn.position, [1.0, 64.0, 1.0]);
assert_eq!(spawn.motion, [0.0, 0.5, -2.0]);
assert!(spawn.owner.is_none() && spawn.data.is_empty());
```

### `ProjectileSpawn::owner`

`fn owner(self, owner: u128) -> ProjectileSpawn`. The same spawn, thrown by the entity `owner`.

```rust
use pluto_lib::{Identifier, ProjectileSpawn};

let spawn = ProjectileSpawn::new(Identifier::parse("minecraft:snowball").unwrap(), [0.0; 3], [1.0, 1.0, 0.0]).owner(42);
assert_eq!(spawn.owner, Some(42));
```

### `ProjectileSpawn::data`

`fn data(self, data: NbtCompound) -> ProjectileSpawn`. The same spawn, with the kind's own saved `data`.

```rust
use pluto_lib::{Identifier, Nbt, NbtCompound, ProjectileSpawn};

let mut data = NbtCompound::new();
data.insert("damage".to_owned(), Nbt::Double(2.5));
let spawn = ProjectileSpawn::new(Identifier::parse("minecraft:arrow").unwrap(), [0.0; 3], [0.0; 3]).data(data);
assert_eq!(spawn.data.get("damage"), Some(&Nbt::Double(2.5)));
```

### `ProjectileSpawn::to_nbt`

`fn to_nbt(&self) -> NbtCompound`. The saved data the spawn loads from: the kind's data with `Motion` (a list of three doubles) and `Owner` (a uuid as four ints, most significant first) written from the fields, replacing any in the data.

```rust
use pluto_lib::{Identifier, Nbt, ProjectileSpawn};

let spawn = ProjectileSpawn::new(Identifier::parse("minecraft:arrow").unwrap(), [0.0; 3], [0.0, -0.5, 2.0]).owner(0x0011_2233_4455_6677_8899_AABB_CCDD_EEFF);
let nbt = spawn.to_nbt();
assert_eq!(nbt.get("Motion"), Some(&Nbt::List(vec![Nbt::Double(0.0), Nbt::Double(-0.5), Nbt::Double(2.0)])));
assert_eq!(nbt.get("Owner"), Some(&Nbt::IntArray(vec![0x0011_2233, 0x4455_6677, -0x7766_5545, -0x3322_1101])));
```

### `AmmoRequest`

A living entity is about to shoot and the game asks what projectile it fires. `player` is the shooting player's uuid (`None` for a mob), `name` their name (empty for a mob), `entity` the shooting mob's entity id (`None` for a player), `kind` the shooter's type, `weapon` the weapon being shot, `vanilla` the projectile stack the game picked itself, `carried` every stack the shooter carries, `supplied` the projectile stack to fire instead of `vanilla` (which a callback may set; a count of 0 means no projectile) and `edits` the whole new stacks for places the shooter carries, which callbacks add to in order to consume the source of a supplied stack.

```rust
use pluto_lib::{AmmoRequest, StackData};
use std::cell::RefCell;

let request = AmmoRequest {
    player: Some(7), name: "Alex".into(), entity: None, kind: "minecraft:player".into(),
    weapon: StackData { item: "minecraft:bow".into(), count: 1, ..Default::default() },
    vanilla: None, carried: Vec::new(), supplied: RefCell::new(None), edits: RefCell::new(Vec::new()),
};
assert_eq!(request.player, Some(7));
assert_eq!(request.name, "Alex");
assert!(request.entity.is_none());
assert_eq!(request.kind, "minecraft:player");
assert_eq!(request.weapon.item, "minecraft:bow");
assert!(request.vanilla.is_none() && request.carried.is_empty());
assert!(request.supplied.borrow().is_none() && request.edits.borrow().is_empty());
```

### `ProjectileEvents::AMMO`

A living entity is about to shoot a weapon and the game asks what projectile it fires (NeoForge: `LivingGetProjectileEvent`), heard before the shot. Leave a request's `supplied` alone to change nothing; set it to take over firing that stack, consuming its source yourself through its `edits`. Server only.

```rust
use pluto_lib::{AmmoRequest, Identifier, ItemSlot, ProjectileEvents, StackData};
use std::cell::RefCell;

ProjectileEvents::AMMO.register(|request| {
    if request.name == "archer" {
        request.supplied.borrow_mut().replace(ItemSlot::new(Identifier::parse("minecraft:arrow").unwrap(), 1));
    }
});
let request = AmmoRequest {
    player: Some(9), name: "archer".into(), entity: None, kind: "minecraft:player".into(),
    weapon: StackData { item: "minecraft:bow".into(), count: 1, ..Default::default() },
    vanilla: None, carried: Vec::new(), supplied: RefCell::new(None), edits: RefCell::new(Vec::new()),
};
ProjectileEvents::AMMO.invoke(&request);
assert_eq!(request.supplied.borrow().as_ref().unwrap().item.to_string(), "minecraft:arrow");
```
