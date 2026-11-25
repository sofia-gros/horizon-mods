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

A projectile that has found what it meets. `dimension` is the dimension id (also `world()`), `projectile` the projectile's entity id, `kind` its type (`minecraft:snowball` or a mod's thrown type), `owner` the entity that threw it (negative if none), `entity` the entity it hit, `entity_kind` that entity's type (empty for a block), `entity_effects` the mob effects the entity it hit carries, by id (empty for a block, or when the host could not reach the target), `block` the block it hit, `face` the face as a direction vector (`[0, 1, 0]` is the top) and `at` where it hit. `motion` is how fast it was moving when it hit, in blocks a tick, and `data` the projectile's saved data — what its spawn data and the stack that fired it left on it, so a launcher that puts its modifiers in the fired stack's data reads them back here. `piercing_ignored` is the entities a piercing arrow has already gone through, by id, as the game reports them — the entity this hit is on is already in it — and `ignore_entities` is where a callback writes the entities the hit has passed through, for the game to add to the arrow's list before it flies again, so a piercing arrow does not meet the same entity twice. `pierced_and_killed` is the entities the arrow has pierced and killed so far, by id — the list the crossbow-kill criterion is fed from, which the game keeps no home for, so PlutoLib keeps it per projectile and a kill a hit settles joins it from the next hit on. `shot_punch` is the punch level the shot was fired with, as the game reports it: the shooter's bow bonus, recorded on the projectile at the shot, which a shooter holding an enchanted bow contributes even when no weapon is saved on the arrow (a skeleton's shot) — a callback re-dealing an arrow's hit reads the punch off the saved weapon when the save names one and falls back to this. Two outcomes a callback settles on the projectile itself: set `discard` to have the game remove it as the callbacks return, or set `motion_after` to the motion it flies on with — with a cancelled hit, a bounce. And `actions` is the queue of actions for the hit's parties (`Victim` the entity it hit, `Attacker` the thrower), which the game does after the callbacks answer.

```rust
use pluto_lib::ProjectileImpact;

let hit = ProjectileImpact {
    dimension: "minecraft:overworld".into(),
    projectile: 5,
    kind: "minecraft:snowball".into(),
    owner: 1,
    entity: None,
    entity_kind: String::new(),
    entity_effects: Vec::new(),
    block: [3, 64, 3],
    face: [0, 1, 0],
    at: [3.5, 65.0, 3.5],
    motion: [0.0, -1.5, 0.25],
    data: Default::default(),
    piercing_ignored: Default::default(),
    pierced_and_killed: Default::default(),
    shot_punch: Default::default(),
    discard: Default::default(),
    motion_after: Default::default(),
    ignore_entities: Default::default(),
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
    entity_kind: String::new(), entity_effects: Vec::new(), block: [0; 3], face: [0, 1, 0], at: [0.0; 3], motion: [0.0; 3], data: Default::default(),
    piercing_ignored: Default::default(), pierced_and_killed: Default::default(), shot_punch: Default::default(), discard: Default::default(), motion_after: Default::default(), ignore_entities: Default::default(), actions: Default::default(),
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
    entity_kind: String::new(), entity_effects: Vec::new(), block: [0; 3], face: [0, 1, 0], at: [0.0; 3], motion: [0.0; 3], data: Default::default(),
    piercing_ignored: Default::default(), pierced_and_killed: Default::default(), shot_punch: Default::default(), discard: Default::default(), motion_after: Default::default(), ignore_entities: Default::default(), actions: Default::default(),
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
).unwrap().throw_sound(Identifier::parse("example:throw.shuriken").unwrap());
ThrownTypeRegistry::register(shuriken).unwrap();
assert_eq!(ThrownTypeRegistry::get(&Identifier::parse("example:shuriken_thrown").unwrap()).unwrap().damage, 4.0);
assert_eq!(ThrownTypeRegistry::throw_sound(&Identifier::parse("example:shuriken_thrown").unwrap()).map(|sound| sound.to_string()), Some("example:throw.shuriken".to_owned()));
```

A hand throw of a thrown type plays the game's snowball throw unless the type carries a throw sound: `.throw_sound(...)` registers the sound the throw plays instead. The game's volume (0.5) and pitch (`0.4 / (random * 0.4 + 0.8)`) already match Java's. Two residues: Java plays at the thrower's feet and files the sound on the NEUTRAL source, but the play position stays the game's eye (the game plays its own throw at the eye) and the filing stays MASTER (where the game files a modded event) — no weave shape reaches either value, as the manifest's throw-sound block documents; the follow-up design is a pending-throw bridge. A type with no throw sound keeps the game's own sound, position and source.

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

A thrown item type: `id` (never a `minecraft:` id — a mod's type must not shadow a vanilla one), `item`, `damage` and `throw_sound`, the sound a hand throw plays instead of the game's snowball throw (`None` keeps the game's own). `ThrownType::new` validates as the game validates its content form: the damage must be finite and not negative.

```rust
use pluto_lib::{Identifier, ThrownType};

let thrown = ThrownType::new(Identifier::parse("example:rock_thrown").unwrap(), Identifier::parse("example:rock").unwrap(), 2.0).unwrap();
assert_eq!(thrown.id.to_string(), "example:rock_thrown");
assert_eq!(thrown.item.to_string(), "example:rock");
assert_eq!(thrown.damage, 2.0);
assert_eq!(thrown.throw_sound, None);
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

### `note_launch_stack`

`fn note_launch_stack(projectile: u32, stack: StackData)`. Remembers the stack the projectile with this entity id (one a spawn handed back) was launched from, for the experience drop of a mob it goes on to kill (see `ExperienceDrop` in [entities](entities.md)): the drop reads the projectile's recorded experience float from this stack instead of the credited player's held tool. The game names the launcher's stack itself for the launches it makes — a shot from a bow, a dispenser's shot, a mob's shot; a launch a mod makes with a stack the game never held at the launch — a tool thrown from the hand, whose hand is already empty when the launch is reported — names it here, with the stack as launched (a thrown tool: the one stack thrown).

```rust
use pluto_lib::{StackData, note_launch_stack};

note_launch_stack(12, StackData { item: "example:throwing_knife".into(), count: 1, ..StackData::default() });
```

### `throw_sound_for`

`fn throw_sound_for(id: &str) -> Option<&'static str>`. The sound a thrown type throws with, for the host's throw-sound sink: the registered type's throw sound, if it carries one. None keeps the game's own snowball throw.

```rust
use pluto_lib::{Identifier, ThrownType, ThrownTypeRegistry, throw_sound_for};

assert_eq!(throw_sound_for("example:unregistered_thrown"), None);
let id = Identifier::parse("example:chime_thrown").unwrap();
ThrownTypeRegistry::register(ThrownType::new(id.clone(), Identifier::parse("example:chime").unwrap(), 0.0).unwrap()
    .throw_sound(Identifier::parse("example:throw.chime").unwrap())).unwrap();
assert_eq!(throw_sound_for("example:chime_thrown"), Some("example:throw.chime"));
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

### `ThrownType::throw_sound`

`fn throw_sound(self, sound: Identifier) -> ThrownType`. The same thrown type, throwing with `sound` instead of the vanilla snowball throw.

```rust
use pluto_lib::{Identifier, ThrownType};

let thrown = ThrownType::new(Identifier::parse("example:ball_thrown").unwrap(), Identifier::parse("example:ball").unwrap(), 0.0).unwrap()
    .throw_sound(Identifier::parse("example:throw.throwball").unwrap());
assert_eq!(thrown.throw_sound.unwrap().to_string(), "example:throw.throwball");
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

### `ThrownTypeRegistry::throw_sound`

`fn throw_sound(id: &Identifier) -> Option<Identifier>`. The throw sound registered for `id`, if the type carries one.

```rust
use pluto_lib::{Identifier, ThrownType, ThrownTypeRegistry};

let id = Identifier::parse("example:shard_thrown").unwrap();
assert_eq!(ThrownTypeRegistry::throw_sound(&id), None);
ThrownTypeRegistry::register(ThrownType::new(id.clone(), Identifier::parse("example:shard").unwrap(), 1.0).unwrap()
    .throw_sound(Identifier::parse("example:throw.shard").unwrap())).unwrap();
assert_eq!(ThrownTypeRegistry::throw_sound(&id).map(|sound| sound.to_string()), Some("example:throw.shard".to_owned()));
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

### `ProjectileEvents::SPAWN`

A projectile joined the flying list, before its first tick: the set-up pass. A callback may still set the shot up — replace its `motion_after`, give an arrow a `pierce` level, set an arrow's pickup status (`pickup`), lift its gravity (`no_gravity`), or name the `pickup_stack` a stuck arrow hands back. Heard for every projectile the game adds on a server, a callback's own summons included; the projectile's id may still be 0 here (it is numbered as it joins), so bind per-projectile state at `LAUNCH` or on the first `FLIGHT`. Server only.

```rust
use pluto_lib::ProjectileEvents;

ProjectileEvents::SPAWN.register(|spawn| {
    if spawn.kind == "minecraft:arrow" {
        spawn.pierce.set(Some(2));
    }
});
```

### `ProjectileSpawned`

A projectile at the moment it joins the flying list. `dimension` is the dimension id, `projectile` its entity id (0 when not yet numbered), `kind` its type, `owner` the entity that threw or shot it (negative if none), `position` where it enters and `motion` how fast it is moving, in blocks a tick. `arrow` holds the arrow facts when it is an arrow, `stack` the projectile's own stack as data (an arrow's pickup stack, a thrown type's item) and `data` its saved data, as on `ProjectileImpact`. The outcomes a callback settles: `motion_after` replaces the motion of its first tick, `pierce` gives an arrow its pierce level, `pickup` sets an arrow's pickup status (creative-only for an arrow its launcher supplied), `no_gravity` lifts its gravity, and `pickup_stack` names the stack a stuck arrow hands back when picked up.

```rust
use pluto_lib::{ArrowFacts, ArrowPickup, ProjectileSpawned};
use std::cell::{Cell, RefCell};

let spawn = ProjectileSpawned {
    dimension: "minecraft:overworld".into(), projectile: 0, kind: "minecraft:arrow".into(), owner: 1,
    position: [0.5, 64.0, 0.5], motion: [0.0, 0.2, -2.0],
    arrow: Some(ArrowFacts { pierce: 0, in_ground: false, in_ground_time: 0, dealt_damage: false }),
    stack: None, data: Default::default(),
    motion_after: RefCell::new(None), pierce: Cell::new(None), pickup: Cell::new(None), no_gravity: Cell::new(None), pickup_stack: RefCell::new(None),
};
spawn.pierce.set(Some(3));
assert_eq!(spawn.pierce.get(), Some(3));
assert_eq!(spawn.arrow.unwrap().pierce, 0);
spawn.pickup.set(Some(ArrowPickup::CreativeOnly));
assert_eq!(spawn.pickup.get(), Some(ArrowPickup::CreativeOnly));
```

### `ArrowFacts`

What a flying projectile's arrow half keeps: `pierce` how many entities it passes through, `in_ground` whether it rests stuck in a block, `in_ground_time` how many ticks it has been stuck, and `dealt_damage` whether it has dealt its damage (a trident body's flag: it hits no more, and a loyal flight comes home).

```rust
use pluto_lib::ArrowFacts;

let facts = ArrowFacts { pierce: 2, in_ground: true, in_ground_time: 5, dealt_damage: false };
assert!(facts.in_ground && facts.in_ground_time > 4 && !facts.dealt_damage);
```

### `ArrowPickup`

An arrow's pickup status (`AbstractArrow.Pickup`): who may pick a stuck arrow up. `Disallowed` is picked up by no player, `Allowed` by any player (who is handed the arrow's pickup stack), and `CreativeOnly` only by a player in creative, who takes no item for it — an arrow its shooter did not spend, such as one a launcher's modifier supplied. The codes are the host's saved byte: 0, 1, 2.

```rust
use pluto_lib::ArrowPickup;

assert_eq!(ArrowPickup::default(), ArrowPickup::Disallowed);
assert_eq!((ArrowPickup::Disallowed as u8, ArrowPickup::Allowed as u8, ArrowPickup::CreativeOnly as u8), (0, 1, 2));
```

### `ProjectileEvents::FLIGHT`

A projectile ended a tick it flew (or rested stuck) in: the steering pass. Heard once a tick for every projectile the game flies on a server, stuck arrows included, for as long as it stays in the flying list. A callback may set the motion it flies the next tick with, lift or restore its gravity, set an arrow's pickup status, free a stuck arrow, discard it, and queue `FlightAction`s around it. A fuse is a callback counting `age`; a magnet is one queueing `FlightAction::PullItems` while its tool rests stuck. Server only.

```rust
use pluto_lib::ProjectileEvents;

ProjectileEvents::FLIGHT.register(|flight| {
    if flight.age >= 200 {
        flight.discard.set(true);
    }
});
```

### `ProjectileFlight`

A projectile at the end of a tick of flight. `dimension` is the dimension id, `projectile` its entity id, `kind` its type and `owner` the entity that threw or shot it (negative if none); `owner_position` is where the owner stands, when the game could reach it — how a callback steers a flight home. `age` is how many ticks it has flown, `position` where it ended the tick and `motion` how fast it is moving, in blocks a tick: the motion it will fly the next tick with, before drag and gravity. `in_ground` says it rests stuck in a block, `no_gravity` whether its flight is without gravity, `arrow` the arrow facts when it is an arrow, and `data` its saved data, as on `ProjectileImpact`. The outcomes a callback settles: `motion_after` replaces the next tick's motion, `discard` removes it as the tick settles, `no_gravity_after` lifts or restores its gravity from the next tick, `pickup_after` sets an arrow's pickup status from the next tick, `release_from_ground` frees a stuck arrow so the motion it is given can fly it, and `actions` is the queue of `FlightAction`s to do around it.

```rust
use pluto_lib::{ArrowFacts, FlightAction, ProjectileFlight};
use std::cell::{Cell, RefCell};

let flight = ProjectileFlight {
    dimension: "minecraft:overworld".into(), projectile: 9, kind: "minecraft:arrow".into(), owner: 1,
    owner_position: Some([0.5, 64.0, 3.5]), age: 25, position: [0.5, 64.0, 0.5], motion: [0.0; 3],
    in_ground: true, no_gravity: false,
    arrow: Some(ArrowFacts { pierce: 0, in_ground: true, in_ground_time: 5, dealt_damage: true }),
    data: Default::default(),
    motion_after: RefCell::new(None), discard: Cell::new(false), no_gravity_after: Cell::new(None), pickup_after: Cell::new(None),
    release_from_ground: Cell::new(false), actions: RefCell::new(Vec::new()),
};
if flight.in_ground && flight.arrow.is_some_and(|a| a.dealt_damage) {
    flight.release_from_ground.set(true);
    flight.actions.borrow_mut().push(FlightAction::PullItems { center: flight.position, radius: 3.0, strength: 0.05 });
}
assert!(flight.release_from_ground.get());
assert_eq!(flight.actions.borrow().len(), 1);
```

### `FlightAction`

One thing a flight callback asks the game to do around a flying projectile, in plain data, done as the tick settles in the order queued: `Explode` sets off a blast, `Particle` spawns a particle, `PushEntity` adds a motion to an entity, `PullItems` pulls the item entities near a point toward it, `PullEntities` pulls the mobs near a point toward it, `DropStack` drops a stack as an item entity, and `GiveStack` hands a stack to the projectile's owner (into a player owner's inventory, or dropped at their feet when it does not fit or the owner is no player).

```rust
use pluto_lib::{FlightAction, StackData, ExplosionInteraction};

let boom = FlightAction::Explode { center: [0.5, 64.0, 0.5], radius: 2.0, fire: false, interaction: ExplosionInteraction::Block };
let back = FlightAction::GiveStack { stack: StackData { item: "example:thrown_tool".into(), count: 1, ..Default::default() } };
assert!(matches!(boom, FlightAction::Explode { radius: 2.0, .. }));
assert!(matches!(back, FlightAction::GiveStack { .. }));
```
