//! Projectiles a mod brings into the world: registering thrown item types and spawning projectiles with a
//! motion, an owner and their own data. Being told about a hit is the other half and already lives in
//! `projectile_events_v1` (`ProjectileEvents::IMPACT`, host-wired); a spawned projectile is reported there
//! under its kind with the entity id [`spawn_projectile`] returns, so a mod can follow its own shots.
//!
//! A thrown type is the game's "a copy of the snowball that flies as its item": the type's id is the
//! projectile's entity kind, `item` is what it flies as and drops, and `damage` is what a hit hurts for.
//! Registration here is the trait-mod source of truth, validated the way the game validates the content
//! form. Delivery to the game's own registry is the one wall: the game fills its thrown-type registry
//! once, at startup, from content mods' `entity-types.json` alone (`projectiles::register_thrown_types`,
//! called only by `game::mods::register_entity_types`), and no entry or provider seam reaches it for a
//! trait mod, so a type registered here must also be declared in the mod's content entity types to fly
//! in the game. [`ThrownTypeRegistry::declarations`] hands over exactly what that declaration needs.
//! Spawning has no such wall: it goes through [`World::summon`], the game's `/summon`, which loads any
//! entity kind — a registered thrown type included — from its data.

use std::collections::BTreeMap;
use std::sync::Mutex;

use super::base::{Identifier, Nbt, NbtCompound};
use super::world_api_v1::World;

/// A thrown item type (the game's `ThrownType`): a projectile that flies as `item`.
#[derive(Clone, Debug, PartialEq)]
pub struct ThrownType {
    /// The projectile's entity kind, such as `tconstruct:shuriken_thrown`. Never a `minecraft:` id:
    /// a mod's type must not shadow a vanilla one.
    pub id: Identifier,
    /// The item the projectile flies as and drops on a block hit.
    pub item: Identifier,
    /// What a hit hurts for; finite and not negative, as the game requires of the content form.
    pub damage: f32,
    /// The sound a hand throw of this type plays (`tconstruct:throw.shuriken`, say), instead of the
    /// vanilla snowball throw the game plays for every thrown type. None throws with the game's own
    /// sound. The host's thrown-type form carries no sound, so this stays on the trait-mod side and
    /// reaches the game's throw through the throw-sound sink. A throw with a registered sound also
    /// plays as Java plays it — at the thrower's feet, where the game plays its own throw at the
    /// eye, and on the NEUTRAL source, where the game files a modded event under MASTER — through
    /// the throw-position and throw-source reaches.
    pub throw_sound: Option<Identifier>,
}

impl ThrownType {
    /// A thrown type, validated as the game validates its content form.
    pub fn new(id: Identifier, item: Identifier, damage: f32) -> Result<Self, String> {
        if id.namespace() == "minecraft" {
            return Err(format!("thrown type {id} would shadow a vanilla entity type"));
        }
        if !damage.is_finite() || damage < 0.0 {
            return Err(format!("thrown type {id} damage must be finite and not negative"));
        }
        Ok(Self { id, item, damage, throw_sound: None })
    }

    /// The same thrown type, throwing with `sound` instead of the vanilla snowball throw.
    pub fn throw_sound(mut self, sound: Identifier) -> Self {
        self.throw_sound = Some(sound);
        self
    }
}

#[derive(Default)]
struct Registry {
    frozen: bool,
    types: BTreeMap<String, ThrownType>,
}

impl Registry {
    fn register(&mut self, thrown: ThrownType) -> Result<(), String> {
        if self.frozen {
            return Err("thrown types have already been collected".into());
        }
        let key = thrown.id.to_string();
        if self.types.contains_key(&key) {
            return Err(format!("thrown type {key} already registered"));
        }
        self.types.insert(key, thrown);
        Ok(())
    }

    fn get(&self, id: &Identifier) -> Option<ThrownType> {
        self.types.get(&id.to_string()).cloned()
    }

    fn snapshot(&mut self) -> Vec<ThrownType> {
        self.frozen = true;
        self.types.values().cloned().collect()
    }
}

static TYPES: Mutex<Registry> = Mutex::new(Registry { frozen: false, types: BTreeMap::new() });

/// The registry of thrown item types. Registration happens once, while mods load; the declarations are
/// collected (and the registry frozen) when the mod's content entity types are written from them.
pub struct ThrownTypeRegistry;

impl ThrownTypeRegistry {
    /// Registers a thrown type. Fails, keeping the earlier registration, for a duplicate id or after
    /// the declarations have been collected.
    pub fn register(thrown: ThrownType) -> Result<(), String> {
        TYPES.lock().map_err(|_| "thrown type registry poisoned")?.register(thrown)
    }

    /// The thrown type registered under `id`, if any.
    pub fn get(id: &Identifier) -> Option<ThrownType> {
        TYPES.lock().ok()?.get(id)
    }

    /// The throw sound registered for `id`, if the type carries one.
    pub fn throw_sound(id: &Identifier) -> Option<Identifier> {
        Self::get(id)?.throw_sound
    }

    /// Whether `event` is the throw sound of some registered thrown type. The game picks the
    /// source a played sound belongs to from its event, and a throw plays on NEUTRAL, so the
    /// throw-source reach asks by event rather than by type.
    pub fn is_throw_sound(event: &str) -> bool {
        TYPES.lock().is_ok_and(|registry| registry.types.values().any(|thrown| thrown.throw_sound.as_ref().is_some_and(|sound| sound.to_string() == event)))
    }

    /// Every registered thrown type, by id; collecting them freezes the registry, as the game's own
    /// set-once registry does. A content entity-types declaration is written from these rows:
    /// each type's id, its `item` and its `damage`, on the `minecraft:snowball` template.
    pub fn declarations() -> Result<Vec<ThrownType>, String> {
        Ok(TYPES.lock().map_err(|_| "thrown type registry poisoned")?.snapshot())
    }
}

/// The throw sounds handed to the host, leaked once per thrown type id: the host's throw reach asks for a
/// `&'static str`, and the registry holds the sound as an owned id, so the first throw of a type leaks its
/// sound once (as the game leaks a thrown type's own id and item strings when it registers them).
static THROW_SOUNDS: Mutex<BTreeMap<String, &'static str>> = Mutex::new(BTreeMap::new());

/// The sound a thrown type throws with, for the host's throw-sound sink: the registered type's throw sound,
/// if it carries one. None keeps the game's own snowball throw. A poisoned cache or an unparsable id
/// answers None rather than inventing a sound.
pub fn throw_sound_for(id: &str) -> Option<&'static str> {
    let sound = ThrownTypeRegistry::throw_sound(&Identifier::parse(id).ok()?)?.to_string();
    let mut cached = THROW_SOUNDS.lock().ok()?;
    Some(*cached.entry(id.to_owned()).or_insert_with(|| Box::leak(sound.into_boxed_str())))
}

/// Whether `event` is a registered thrown type's throw sound, for the host's throw-source
/// sink: the game asks by event when it files a played sound under a source, and a throw
/// belongs to NEUTRAL. Installed beside [`throw_sound_for`].
pub fn is_throw_sound_event(event: &str) -> bool {
    ThrownTypeRegistry::is_throw_sound(event)
}

/// A projectile to spawn: its kind, where it starts, how it moves, whose it is and its own data.
/// The data is the entity's saved data as `/summon` takes it, minus the position (the spawn point
/// is separate) and minus `Motion` and `Owner`, which the fields below write — a `Motion` or `Owner`
/// in `data` is replaced by the field, so what flies is what the fields say.
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectileSpawn {
    /// The projectile's entity kind, such as `minecraft:arrow` or a registered thrown type's id.
    pub kind: Identifier,
    /// Where it starts, in the world's dimension.
    pub position: [f64; 3],
    /// Its motion in blocks per tick (the entity data's `Motion`).
    pub motion: [f64; 3],
    /// The throwing entity's uuid, if any (the entity data's `Owner`).
    pub owner: Option<u128>,
    /// The kind's own saved data (an arrow's `damage`, a thrown type's tags, ...).
    pub data: NbtCompound,
}

impl ProjectileSpawn {
    /// A projectile of `kind` at `position` with `motion` and no owner and no extra data.
    pub fn new(kind: Identifier, position: [f64; 3], motion: [f64; 3]) -> Self {
        Self { kind, position, motion, owner: None, data: NbtCompound::new() }
    }

    /// The same spawn, thrown by the entity `owner`.
    pub fn owner(mut self, owner: u128) -> Self {
        self.owner = Some(owner);
        self
    }

    /// The same spawn, with the kind's own saved data.
    pub fn data(mut self, data: NbtCompound) -> Self {
        self.data = data;
        self
    }

    /// The saved data the spawn loads from: the kind's data with `Motion` and `Owner` written from
    /// the fields, in the game's own layout (`Motion` a list of three doubles, `Owner` a uuid as
    /// four ints, most significant first — `UUIDUtil.uuidToIntArray`).
    pub fn to_nbt(&self) -> NbtCompound {
        let mut nbt = self.data.clone();
        nbt.insert("Motion".into(), Nbt::List(self.motion.iter().map(|v| Nbt::Double(*v)).collect()));
        if let Some(owner) = self.owner {
            nbt.insert("Owner".into(), Nbt::IntArray(uuid_int_array(owner).to_vec()));
        }
        nbt
    }
}

/// A uuid as the four ints entity data holds it in (`UUIDUtil.uuidToIntArray`).
pub fn uuid_int_array(uuid: u128) -> [i32; 4] {
    [(uuid >> 96) as i32, (uuid >> 64) as i32, (uuid >> 32) as i32, uuid as i32]
}

/// Spawns the projectile in `world` and returns the new entity's id, the id its hit is later reported
/// under in `projectile_events_v1`. None when the world cannot summon there (the chunk is not loaded,
/// the position is out of bounds, the kind is one the game cannot load) or outside a callback that
/// carries a world.
pub fn spawn_projectile(world: &World, spawn: &ProjectileSpawn) -> Option<u32> {
    world.summon(&spawn.kind.to_string(), spawn.position, &spawn.to_nbt())
}

/// Spawns a registered thrown type: the same as [`spawn_projectile`] with the type's id as the kind.
/// None when no type is registered under `thrown` (or when the world cannot summon there).
pub fn spawn_thrown(world: &World, thrown: &Identifier, position: [f64; 3], motion: [f64; 3], owner: Option<u128>, data: NbtCompound) -> Option<u32> {
    let thrown = ThrownTypeRegistry::get(thrown)?;
    let mut spawn = ProjectileSpawn::new(thrown.id, position, motion).data(data);
    spawn.owner = owner;
    spawn_projectile(world, &spawn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pure::world_api_v1::{MemoryWorldOps, with_world};

    fn thrown(id: &str) -> ThrownType {
        ThrownType::new(Identifier::parse(id).unwrap(), Identifier::parse("tconstruct:shuriken").unwrap(), 4.0).unwrap()
    }

    #[test]
    fn thrown_types_validate_like_the_content_form_and_freeze_on_collection() {
        assert!(ThrownType::new(Identifier::parse("minecraft:arrow").unwrap(), Identifier::parse("minecraft:arrow").unwrap(), 1.0).is_err(), "no shadowing vanilla");
        assert!(ThrownType::new(Identifier::parse("test:bad").unwrap(), Identifier::parse("test:bad").unwrap(), f32::NAN).is_err());
        assert!(ThrownType::new(Identifier::parse("test:bad").unwrap(), Identifier::parse("test:bad").unwrap(), -1.0).is_err());
        let mut registry = Registry::default();
        registry.register(thrown("test:shuriken_thrown")).unwrap();
        assert!(registry.register(thrown("test:shuriken_thrown")).is_err(), "duplicate");
        assert_eq!(registry.get(&Identifier::parse("test:shuriken_thrown").unwrap()).unwrap().damage, 4.0);
        assert_eq!(registry.snapshot().len(), 1);
        assert!(registry.register(thrown("test:late")).is_err(), "frozen after collection");
    }

    #[test]
    fn a_thrown_type_throws_with_its_registered_sound() {
        let id = Identifier::parse("test:throw_sound_ball").unwrap();
        assert_eq!(ThrownTypeRegistry::throw_sound(&id), None);
        assert_eq!(throw_sound_for("test:throw_sound_ball"), None);
        let thrown = ThrownType::new(id.clone(), Identifier::parse("test:ball").unwrap(), 0.0)
            .unwrap()
            .throw_sound(Identifier::parse("test:throw.throwball").unwrap());
        assert_eq!(thrown.throw_sound.as_ref().map(|sound| sound.to_string()), Some("test:throw.throwball".to_owned()));
        ThrownTypeRegistry::register(thrown).unwrap();
        assert_eq!(ThrownTypeRegistry::throw_sound(&id).map(|sound| sound.to_string()), Some("test:throw.throwball".to_owned()));
        assert_eq!(throw_sound_for("test:throw_sound_ball"), Some("test:throw.throwball"));
        assert!(is_throw_sound_event("test:throw.throwball"), "a registered type's sound is a throw sound by event");
        assert!(!is_throw_sound_event("test:throw.throwball2"));
        assert!(!is_throw_sound_event("minecraft:entity.snowball.throw"), "the game's own throw is no modded throw sound");
        let plain = ThrownType::new(Identifier::parse("test:throw_sound_plain").unwrap(), Identifier::parse("test:plain").unwrap(), 0.0).unwrap();
        assert!(plain.throw_sound.is_none());
        ThrownTypeRegistry::register(plain).unwrap();
        assert_eq!(throw_sound_for("test:throw_sound_plain"), None);
    }

    #[test]
    fn a_spawn_writes_motion_and_owner_in_the_games_layout_over_its_data() {
        let mut data = NbtCompound::new();
        data.insert("damage".into(), Nbt::Double(2.5));
        data.insert("Motion".into(), Nbt::List(vec![Nbt::Double(9.0)]));
        let spawn = ProjectileSpawn::new(Identifier::parse("minecraft:arrow").unwrap(), [1.0, 64.0, 1.0], [0.0, -0.5, 2.0])
            .owner(0x0011_2233_4455_6677_8899_AABB_CCDD_EEFF)
            .data(data);
        let nbt = spawn.to_nbt();
        assert_eq!(nbt.get("damage"), Some(&Nbt::Double(2.5)), "the kind's own data is kept");
        assert_eq!(
            nbt.get("Motion"),
            Some(&Nbt::List(vec![Nbt::Double(0.0), Nbt::Double(-0.5), Nbt::Double(2.0)])),
            "the field's motion replaces the data's"
        );
        assert_eq!(nbt.get("Owner"), Some(&Nbt::IntArray(vec![0x0011_2233, 0x4455_6677, -0x7766_5545, -0x3322_1101])));
        assert_eq!(uuid_int_array(0), [0, 0, 0, 0]);
    }

    #[test]
    fn spawning_goes_through_the_worlds_summon_and_reports_the_new_id() {
        let world = World::new("minecraft:overworld");
        let spawn = ProjectileSpawn::new(Identifier::parse("minecraft:snowball").unwrap(), [0.0, 64.0, 0.0], [1.0, 1.0, 0.0]);
        assert_eq!(spawn_projectile(&world, &spawn), None, "outside a scope nothing spawns");
        let mut ops = MemoryWorldOps::default();
        with_world(&mut ops, || {
            assert_eq!(spawn_projectile(&world, &spawn), Some(1));
            assert_eq!(spawn_projectile(&world, &spawn), Some(2));
        });
        assert_eq!(ops.summons.len(), 2);
        assert_eq!(ops.summons[0].kind, "minecraft:snowball");
        assert_eq!(ops.summons[0].dimension, "minecraft:overworld");
        assert_eq!(ops.summons[0].pos, [0.0, 64.0, 0.0]);
        assert!(ops.summons[0].nbt.contains_key("Motion"));
    }

    #[test]
    fn spawn_thrown_needs_a_registered_type() {
        let world = World::new("minecraft:overworld");
        let id = Identifier::parse("test:unregistered_thrown").unwrap();
        let mut ops = MemoryWorldOps::default();
        with_world(&mut ops, || {
            assert_eq!(spawn_thrown(&world, &id, [0.0; 3], [0.0; 3], None, NbtCompound::new()), None);
        });
        assert!(ops.summons.is_empty());
    }
}
