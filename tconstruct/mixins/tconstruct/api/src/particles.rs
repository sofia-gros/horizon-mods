//! The original's particle types, declared on PlutoLib's particle-type registry (TinkerWorld,
//! TinkerCommons and TinkerTools registration; Tinkers' Construct, MIT — see LICENSE):
//!
//! - `sky_slime`, `ender_slime`, `terracube` (TinkerWorld): plain particle types whose client factory
//!   (`SlimeParticle.Factory`) is a breaking-item particle of the type's own item — TinkerCommons'
//!   slime ball of the type for the two slimes, a clay ball for the terracube. The slime classes
//!   override `getParticleType()` to these types, so here each type is declared with exactly that
//!   item-crumb factory and the entity-particle override replaces the slime type's inherited landing
//!   particle (`minecraft:item_slime`, the slime factory's) with its own type.
//! - `fluid` (TinkerCommons): the fluid particle, a billboard textured by the still texture of the
//!   fluid each spawn carries. Declared as a fluid-sprite factory; a spawn's data is the fluid stack.
//!   Its ported consumer is the fluid cannon's direct-apply shot (`crate::fluid_cannon`); the other
//!   original consumers (the use-on-hit modifier, splashing, slurping) are not ported yet. The host's
//!   sprite sets are static per type, so the per-spawn fluid sprite has no channel yet: the send is
//!   real and carries the fluid, and draws nothing until the host grows one (see PlutoLib's particle
//!   page).
//! - `hammer_attack`, `axe_attack`, `bonk` (TinkerTools): always-spawn types drawn from the sprite
//!   sets shipped in `assets/tconstruct/particles/` — eight slash frames each, played by age over the
//!   original's four-tick lifetime through the host's `horizon:behaviour` block. Their consumer, the
//!   melee module that sends one aimed particle on a fully charged hit, is not ported yet, so the
//!   types are declared and shipped but nothing sends them. What the behaviour block cannot express
//!   of the original's attack particle: its full-bright light, its random grey tint, and the size it
//!   takes from the swing's look vector.
use pluto_lib::{Identifier, ParticleFactory, ParticleTypeRegistry, StackData};

pub(crate) fn register() {
    // The slime types first: the entity overrides point at their item-crumb factories.
    for (particle, item) in [
        ("sky_slime", "tconstruct:sky_slime_ball"),
        ("ender_slime", "tconstruct:ender_slime_ball"),
        ("terracube", "minecraft:clay_ball"),
    ] {
        let id = Identifier::new("tconstruct", particle).expect("particle type id");
        ParticleTypeRegistry::register(
            id.clone(),
            ParticleFactory::ItemCrumb(StackData { item: item.into(), count: 1, ..Default::default() }),
        )
        .unwrap_or_else(|error| panic!("registering particle type tconstruct:{particle}: {error}"));
        ParticleTypeRegistry::register_entity_particle(
            Identifier::new("tconstruct", particle).expect("entity type id"),
            Identifier::new("minecraft", "item_slime").expect("inherited particle id"),
            id,
        )
        .unwrap_or_else(|error| panic!("registering the {particle} landing particle: {error}"));
    }
    ParticleTypeRegistry::register(Identifier::new("tconstruct", "fluid").expect("particle type id"), ParticleFactory::FluidSprite)
        .unwrap_or_else(|error| panic!("registering particle type tconstruct:fluid: {error}"));
    for particle in ["hammer_attack", "axe_attack", "bonk"] {
        ParticleTypeRegistry::register(
            Identifier::new("tconstruct", particle).expect("particle type id"),
            ParticleFactory::SpriteSet { always_spawn: true },
        )
        .unwrap_or_else(|error| panic!("registering particle type tconstruct:{particle}: {error}"));
    }
}
