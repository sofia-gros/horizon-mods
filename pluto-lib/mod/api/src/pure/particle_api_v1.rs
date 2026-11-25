//! Particle types a mod declares, and the data a particle spawn carries (Minecraft: `ParticleType`,
//! `ParticleOptions`; Fabric: `FabricParticleTypes`).
//!
//! A particle type is a name plus a factory: what one particle of the type is made of. The host has no
//! registry a mod can add a type to — its vanilla type table is closed — so PlutoLib keeps the
//! declarations itself and answers the host's particle questions from them:
//!
//! - an **item-crumb** factory ([`ParticleFactory::ItemCrumb`]) is the standard `BreakingItemParticle`
//!   constructor: the particle is a fragment of the factory's item stack, simulated by the host. The
//!   stack itself is held by [`ParticleFactoryRegistry`](super::particle_factory_registry::ParticleFactoryRegistry),
//!   whose named-request bridge and cube-landing override the host already consults; registering the
//!   factory here registers it there too, so a type declared once emits everywhere the host asks.
//! - a **sprite-set** factory ([`ParticleFactory::SpriteSet`]) is a textured billboard: the sprite set
//!   (and an optional `horizon:behaviour` block: animation, lifetime, gravity, size and alpha ramps)
//!   ships as data in the mod's `assets/<namespace>/particles/<path>.json`, and the host's engine loads
//!   it from there and draws spawns of the type by name. Registration here is the declaration that the
//!   file exists; nothing is drawn for a type whose file a client does not have.
//! - a **fluid-sprite** factory ([`ParticleFactory::FluidSprite`]) is a billboard textured by the still
//!   texture of the fluid each spawn carries (the original's `FluidParticle`): the spawn's
//!   [`ParticleData::Fluid`] names the fluid. The host's sprite sets are static per type, so it has no
//!   channel for a sprite chosen at spawn time: a fluid spawn is sent by name like a sprite-set spawn
//!   and draws nothing until the host grows one. The declaration and the spawn data are exact, so the
//!   consumers are finished the day the channel exists.
//!
//! Spawning itself is [`World::send_particles`](super::world_api_v1::World::send_particles)
//! (Minecraft: `ServerLevel.sendParticles`); this module only declares the types and the spawn data.

use std::collections::BTreeMap;
use std::sync::RwLock;

use super::base::Identifier;
use super::block_entity_api_v1::FluidStack;
use super::particle_factory_registry::ParticleFactoryRegistry;
use super::player_api::StackData;

/// What one particle of a registered type is made of (the type's client factory, in the original's
/// terms). See the module documentation for how the host answers each kind.
#[derive(Clone, Debug, PartialEq)]
pub enum ParticleFactory {
    /// A fragment of this item stack (the standard `BreakingItemParticle` constructor; the original's
    /// slime particles are this shape, one slime ball — or clay ball — per type). The stack is held by
    /// the item-factory registry, so the host's named-request bridge resolves it from there.
    ItemCrumb(StackData),
    /// A textured billboard from the sprite set the mod ships in
    /// `assets/<namespace>/particles/<path>.json`. `always_spawn` is the type's declared
    /// `SimpleParticleType` flag (spawned regardless of distance); the host's named-particle path does
    /// not consume the flag yet, so it is declaration data, as an entity type's tracking range is.
    SpriteSet {
        /// Spawned regardless of distance (the original's attack particles declare this).
        always_spawn: bool,
    },
    /// A billboard textured by the still texture of the fluid each spawn carries as
    /// [`ParticleData::Fluid`], tinted with the fluid's colour (the original's `FluidParticle`).
    FluidSprite,
}

/// The per-spawn data a particle type's factory reads (Minecraft: the `ParticleOptions` beyond the
/// type itself). Most types take [`ParticleData::Plain`]; a fluid-sprite type takes the fluid.
#[derive(Clone, Debug, PartialEq)]
pub enum ParticleData {
    /// No data: the factory decides everything.
    Plain,
    /// The fluid a fluid-sprite particle draws (the original's `FluidParticleData`: the type's own id
    /// rides on the spawn's particle name, the stack — fluid, amount and tag — is the data).
    Fluid(FluidStack),
}

/// How a type was declared, without the item stack (which the item-factory registry holds).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Record {
    ItemCrumb,
    SpriteSet { always_spawn: bool },
    FluidSprite,
}

static TYPES: RwLock<BTreeMap<String, Record>> = RwLock::new(BTreeMap::new());

/// The declared particle types of every mod, by name. Registration is process-lifetime and
/// re-registering a name replaces its factory, as the item-factory registry replaces stacks.
pub struct ParticleTypeRegistry;

impl ParticleTypeRegistry {
    /// Declares the particle type `particle` with its `factory`. An item-crumb factory is validated
    /// and stored by the item-factory registry first (its rules apply: a valid item id, a count in
    /// range, a positive durability override), so a rejected stack leaves any previous declaration of
    /// this type — and of its factory — intact.
    pub fn register(particle: Identifier, factory: ParticleFactory) -> Result<(), String> {
        let record = match &factory {
            ParticleFactory::ItemCrumb(stack) => {
                ParticleFactoryRegistry::register_item(particle.clone(), stack.clone())?;
                Record::ItemCrumb
            }
            ParticleFactory::SpriteSet { always_spawn } => Record::SpriteSet { always_spawn: *always_spawn },
            ParticleFactory::FluidSprite => Record::FluidSprite,
        };
        let mut types = TYPES.write().map_err(|_| "particle type registry poisoned")?;
        types.insert(particle.to_string(), record);
        Ok(())
    }

    /// The declared factory of `particle`, or None when no mod declared it. An item-crumb factory is
    /// read back from the item-factory registry, so a stack replaced there is the factory seen here.
    pub fn factory(particle: &Identifier) -> Option<ParticleFactory> {
        let record = TYPES.read().ok()?.get(&particle.to_string()).copied()?;
        match record {
            Record::ItemCrumb => ParticleFactoryRegistry::item(particle).map(ParticleFactory::ItemCrumb),
            Record::SpriteSet { always_spawn } => Some(ParticleFactory::SpriteSet { always_spawn }),
            Record::FluidSprite => Some(ParticleFactory::FluidSprite),
        }
    }

    /// Every declared particle type's name, sorted (the registry's own census, for tests and tools).
    pub fn registered() -> Vec<String> {
        TYPES.read().map(|types| types.keys().cloned().collect()).unwrap_or_default()
    }

    /// Replaces one entity type's inherited particle with a registered item-crumb type (the cube
    /// landing override): the same rule as
    /// [`ParticleFactoryRegistry::register_entity_override`], named here because the replacement is a
    /// declared particle type — it must be registered (as an item crumb) first, and its namespace must
    /// not be `minecraft`.
    pub fn register_entity_particle(entity: Identifier, inherited: Identifier, replacement: Identifier) -> Result<(), String> {
        ParticleFactoryRegistry::register_entity_override(entity, inherited, replacement)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(item: &str) -> StackData {
        StackData { item: item.into(), count: 1, damage: 0, max_damage: None, custom_data: Default::default(), custom_model_data: None, can_place_on: None, enchantments: Default::default() }
    }

    #[test]
    fn declared_factories_read_back_and_replacements_replace() {
        let crumb = Identifier::parse("example:declared_crumb").unwrap();
        assert_eq!(ParticleTypeRegistry::factory(&crumb), None);
        ParticleTypeRegistry::register(crumb.clone(), ParticleFactory::ItemCrumb(stack("minecraft:clay_ball"))).unwrap();
        assert_eq!(ParticleTypeRegistry::factory(&crumb), Some(ParticleFactory::ItemCrumb(stack("minecraft:clay_ball"))));
        // The stack lives in the item-factory registry: replacing it there replaces the factory here.
        ParticleFactoryRegistry::register_item(crumb.clone(), stack("example:painted_ball")).unwrap();
        assert_eq!(ParticleTypeRegistry::factory(&crumb), Some(ParticleFactory::ItemCrumb(stack("example:painted_ball"))));
        // Re-declaring the type with another kind replaces the declaration whole.
        ParticleTypeRegistry::register(crumb.clone(), ParticleFactory::SpriteSet { always_spawn: true }).unwrap();
        assert_eq!(ParticleTypeRegistry::factory(&crumb), Some(ParticleFactory::SpriteSet { always_spawn: true }));

        let set = Identifier::parse("example:declared_set").unwrap();
        ParticleTypeRegistry::register(set.clone(), ParticleFactory::SpriteSet { always_spawn: false }).unwrap();
        assert_eq!(ParticleTypeRegistry::factory(&set), Some(ParticleFactory::SpriteSet { always_spawn: false }));

        let fluid = Identifier::parse("example:declared_fluid").unwrap();
        ParticleTypeRegistry::register(fluid.clone(), ParticleFactory::FluidSprite).unwrap();
        assert_eq!(ParticleTypeRegistry::factory(&fluid), Some(ParticleFactory::FluidSprite));

        let names = ParticleTypeRegistry::registered();
        for name in ["example:declared_crumb", "example:declared_fluid", "example:declared_set"] {
            assert!(names.contains(&name.to_owned()), "{name} is declared");
        }
    }

    #[test]
    fn an_invalid_crumb_stack_leaves_the_previous_declaration_intact() {
        let id = Identifier::parse("example:guarded_crumb").unwrap();
        ParticleTypeRegistry::register(id.clone(), ParticleFactory::ItemCrumb(stack("minecraft:clay_ball"))).unwrap();
        let mut bad = stack("minecraft:clay_ball");
        bad.count = 0;
        assert!(ParticleTypeRegistry::register(id.clone(), ParticleFactory::ItemCrumb(bad)).is_err());
        assert_eq!(ParticleTypeRegistry::factory(&id), Some(ParticleFactory::ItemCrumb(stack("minecraft:clay_ball"))));
        let bad = stack("not an item");
        assert!(ParticleTypeRegistry::register(id.clone(), ParticleFactory::ItemCrumb(bad)).is_err());
        assert_eq!(ParticleTypeRegistry::factory(&id), Some(ParticleFactory::ItemCrumb(stack("minecraft:clay_ball"))));
    }

    #[test]
    fn entity_particles_delegate_to_the_item_factory_override_rules() {
        let replacement = Identifier::parse("example:landing_crumb").unwrap();
        ParticleTypeRegistry::register(replacement.clone(), ParticleFactory::ItemCrumb(stack("minecraft:clay_ball"))).unwrap();
        let entity = Identifier::parse("example:landing_cube").unwrap();
        let inherited = Identifier::parse("minecraft:item_slime").unwrap();
        ParticleTypeRegistry::register_entity_particle(entity.clone(), inherited.clone(), replacement.clone()).unwrap();
        // An unregistered replacement and a Minecraft replacement are both refused, as the factory registry refuses them.
        assert!(ParticleTypeRegistry::register_entity_particle(entity.clone(), inherited.clone(), Identifier::parse("example:never_declared").unwrap()).is_err());
        assert!(ParticleTypeRegistry::register_entity_particle(entity, inherited, Identifier::parse("minecraft:item_slime").unwrap()).is_err());
    }

    #[test]
    fn spawn_data_keeps_the_fluid_stack_whole() {
        let fluid = FluidStack::new(Identifier::parse("example:molten_glass").unwrap(), 250);
        let data = ParticleData::Fluid(fluid.clone());
        assert_eq!(data, ParticleData::Fluid(fluid));
        assert_ne!(data, ParticleData::Plain);
    }
}
