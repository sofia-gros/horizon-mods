//! Domain checks for particles.rs: the original's seven particle types are declared with the
//! original's factories, and the slime types' crumb stacks are the original's items.
use pluto_lib::{Identifier, ParticleFactory, ParticleFactoryRegistry, ParticleTypeRegistry};

fn crumb_item(particle: &str) -> String {
    let id = Identifier::parse(particle).unwrap();
    ParticleFactoryRegistry::item(&id).unwrap_or_else(|| panic!("{particle} has an item-crumb factory")).item
}

#[test]
fn the_originals_particle_types_are_declared() {
    crate::particles::register();
    let all = ParticleTypeRegistry::registered();
    let declared: Vec<&str> = all.iter().map(String::as_str).filter(|name| name.starts_with("tconstruct:")).collect();
    // The original declares exactly these seven (TinkerWorld's three slimes, TinkerCommons'
    // fluid, TinkerTools' three attack types); the registry is shared with the edible tests,
    // whose bursts lazily register one `edible_crumb_<namespace>_<path>` type per representative
    // item (particles.rs: the port's own invention, the original has no such types), so the
    // assertion is the seven plus only those crumbs — never any other stray type.
    let original = [
        "tconstruct:axe_attack",
        "tconstruct:bonk",
        "tconstruct:ender_slime",
        "tconstruct:fluid",
        "tconstruct:hammer_attack",
        "tconstruct:sky_slime",
        "tconstruct:terracube",
    ];
    for name in original {
        assert!(declared.contains(&name), "the original's {name} is declared: {declared:?}");
    }
    let extra: Vec<&&str> = declared.iter().filter(|name| !original.contains(name)).collect();
    assert!(
        extra.iter().all(|name| name.starts_with("tconstruct:edible_crumb_")),
        "only the port's lazy edible-crumb types may join the original's seven: {extra:?}"
    );
    // The slime factories are breaking-item particles of the type's own item (SlimeParticle.Factory).
    assert_eq!(crumb_item("tconstruct:sky_slime"), "tconstruct:sky_slime_ball");
    assert_eq!(crumb_item("tconstruct:ender_slime"), "tconstruct:ender_slime_ball");
    assert_eq!(crumb_item("tconstruct:terracube"), "minecraft:clay_ball");
    // The fluid type draws the fluid each spawn carries; the attack types are always-spawn sprite sets.
    let fluid = Identifier::parse("tconstruct:fluid").unwrap();
    assert_eq!(ParticleTypeRegistry::factory(&fluid), Some(ParticleFactory::FluidSprite));
    for attack in ["tconstruct:hammer_attack", "tconstruct:axe_attack", "tconstruct:bonk"] {
        let id = Identifier::parse(attack).unwrap();
        assert_eq!(ParticleTypeRegistry::factory(&id), Some(ParticleFactory::SpriteSet { always_spawn: true }), "{attack}");
    }
}
