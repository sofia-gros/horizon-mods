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
//! - `edible_crumb_<namespace>_<path>` (this lane): one crumb type per representative item an edible
//!   tool names, registered lazily on first use by `edible_crumb` — the port's particle spawn carries
//!   no per-spawn item, so the item's crumbs need a type of their own. Their consumer is
//!   `edible_hook`'s eating bursts.
use pluto_lib::{Identifier, ParticleFactory, ParticleTypeRegistry, StackData};
use std::collections::HashSet;
use std::sync::Mutex;

/// The crumb particle types registered lazily for edible representatives, by id: the registry
/// outlives a reload of this module's callers, so the set is the record of what is already there
/// (re-registering a name replaces its factory, so a second registration would be harmless but
/// wasteful on every burst).
static EDIBLE_CRUMBS: Mutex<HashSet<String>> = Mutex::new(HashSet::new());

/// The particle type for the crumbs of `item`, registered lazily on first use: the port's particle
/// spawn carries no per-spawn item, so each representative item gets its own crumb type, an
/// item-crumb factory of that item, named `tconstruct:edible_crumb_<namespace>_<path>`.
pub(crate) fn edible_crumb(item: &Identifier) -> Identifier {
    let name = format!("edible_crumb_{}_{}", item.namespace(), item.path().replace('/', "_"));
    let id = Identifier::new("tconstruct", &name).expect("edible crumb particle id");
    let fresh = EDIBLE_CRUMBS.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).insert(id.to_string());
    if fresh {
        let _ = ParticleTypeRegistry::register(id.clone(), ParticleFactory::ItemCrumb(StackData { item: item.to_string(), count: 1, ..Default::default() }));
    }
    id
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edible_crumb_names_the_item_and_registers_it_once() {
        let item = Identifier::new("tconstruct", "cheese_ingot").expect("valid id");
        let crumb = edible_crumb(&item);
        assert_eq!(crumb.to_string(), "tconstruct:edible_crumb_tconstruct_cheese_ingot");
        // A second call names the same type (the registration is not repeated).
        assert_eq!(edible_crumb(&item), crumb);
        let slashed = Identifier::new("example", "a/b").expect("valid id");
        assert_eq!(edible_crumb(&slashed).to_string(), "tconstruct:edible_crumb_example_a_b");
    }
}
