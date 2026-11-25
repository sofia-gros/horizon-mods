//! TinkerGadgets' throwables (SlimeKnights, MIT; see LICENSE): the four deprecated-but-registered
//! throwable items of `gadgets/TinkerGadgets.java` — `glow_ball` (GlowBallItem), `efln_ball`
//! (EFLNItem), `flint_shuriken` and `quartz_shuriken` (ShurikenItem) — and their projectile
//! entities, which are registered under the same ids.
//!
//! How a throw works here: the game itself throws a declared thrown type. A player's use of an
//! item whose entity type is declared in the content (`data/tconstruct/horizon/entity_type/`)
//! goes through the game's own `throw_item` (pinned Horizon `game/use_hand.rs` →
//! `game/creatures.rs::throw_item`): it spawns the type at the eye, shoots it with velocity 1.5
//! and inaccuracy 1.0 (exactly the Java items' `shootFromRotation(player, xRot, yRot, 0.0F,
//! 1.5F, 1.0F)`), awards `ITEM_USED`, applies the stack's `use_cooldown` component and shrinks
//! the held stack by one for non-creative players. So no use callback is ported: the four types
//! are registered in PlutoLib's [`ThrownTypeRegistry`] below (the trait-mod source of truth,
//! kept in step with the content declarations by the domain tests) and the items are declared
//! on the `minecraft:snowball` template with `max_stack_size` 16 (`Item.Properties().stacksTo(16)`);
//! the shurikens' 4-tick use cooldown (`player.getCooldowns().addCooldown(item, 4)` in
//! ShurikenItem.use) is the items' `minecraft:use_cooldown` component patch of 0.2 seconds.
//! Like the original, none of the four is in the creative tab (TinkerGadgets.addTabItems omits
//! them); they only carry the optional Mantle tooltip the Java items add.
//!
//! What the game's thrown hit does not do, this module adds from `ProjectileEvents::IMPACT`
//! (returning `true` throughout, so the game's own hit — damage, burst, discard — still runs):
//! - Glow ball: `GlowballEntity.onHit` places a glow block (see [`place_glow`]); the block is
//!   `crate::glow_block`.
//! - Shurikens: `ShurikenEntityBase.onHitBlock` drops the thrown item (`spawnAtLocation(
//!   getDefaultItem())`); the game bursts and discards the projectile without dropping, so the
//!   drop is summoned here. On an entity hit the game deals the type's damage and pushes the
//!   target as its own hit knockback; Java then knocks the target again with the type's exact
//!   strength (flint 0.6, quartz 0.4) along the flight, which is queued here as a
//!   [`pluto_lib::EntityAction::Knockback`] with the vector direction (the negated motion, as
//!   `LivingEntity.knockback` reads its arguments). The action's push shapes the vertical part
//!   as the game's own knockback does rather than Java's half-motion blend — the one residue.
//! - EFLN: `EFLNEntity.onHit` detonates where the ball lands and the ball is spent, through
//!   PlutoLib's [`World::explode`] at the original's size 4 with the DESTROY block interaction
//!   (the port's `ExplosionInteraction::Tnt`). What does not carry is `EFLNExplosion` itself:
//!   the detonation is the game's own explosion, so its damage and push follow the game's
//!   falloff rather than the custom sphere's flat damage 8 and knockback 1 — the same
//!   documented deviation the fluid explosion effect carries.
//! - Sounds: Java plays `tconstruct:throw.throwball` / `tconstruct:throw.shuriken` (Sounds.java,
//!   NEUTRAL, 0.5 volume, pitch `0.4F / (random * 0.4F + 0.8F)`) at the thrower's feet; the game
//!   picks the throw sound by projectile kind and plays the vanilla snowball throw for thrown
//!   types, at the eye and filed under MASTER, so each type registers its throw sound on its
//!   [`ThrownType`] and PlutoLib's throw reaches (`pluto-throw-sound`, a MODIFY_VARIABLE on the
//!   game's `throw_item`, with `pluto-throw-sound-position` and `pluto-throw-source`) play that
//!   sound at the feet on NEUTRAL instead — the volume and pitch the game already plays are
//!   Java's.
//! - Dispensers: Java registers a `ShootProjectileDispenserBehavior` (power 1.1, inaccuracy
//!   6.0) for all four items, and they are registered the same way below, in PlutoLib's
//!   [`DispenseRegistry`]: the host's `dispense_item` hook (API 67) asks its mods before it
//!   ejects, and PlutoLib answers for a registered item by summoning its projectile at the
//!   dispenser's mouth with the shot's motion, consuming one item and playing the projectile
//!   launch sound. One reduction: the dispense callback is handed no random source, so the
//!   inaccuracy spread is not rolled — the shot flies true (PlutoLib's `dispense_shot`).
//!   The tool items' dispenser behaviours (the modifiable arrow and shuriken behaviours) are
//!   a separate matter: they read tool stats at dispense time, which this registration does
//!   not do.
//!
//! There are no recipes for the four in the original (`gadgets/data/GadgetRecipeProvider.java`
//! has none; the items are deprecated), so none are converted.
use pluto_lib::{ActionTarget, BlockState, Direction, DispenseBehavior, DispenseRegistry, EntityAction, ExplosionInteraction, Identifier, ItemTooltipCallback, KnockbackDirection, Nbt, NbtCompound, ProjectileEvents, ProjectileImpact, ThrownType, ThrownTypeRegistry, TooltipLine, World};


/// The dispenser numbers of `ShootProjectileDispenserBehavior` as `TinkerGadgets` registers it for all
/// four throwables: power 1.1, inaccuracy 6.0.
pub(crate) const DISPENSE_POWER: f32 = 1.1;
pub(crate) const DISPENSE_INACCURACY: f32 = 6.0;

/// One throwable of TinkerGadgets, with its entity's numbers from the entity classes:
/// `damage`/`knockback` are ShurikenEntityBase's `getDamage()`/`getKnockback()` (flint 3.0/0.6,
/// quartz 5.0/0.4); the balls deal no damage (GlowballEntity and EFLNEntity override no
/// `onHitEntity` damage; the EFLN's harm is its explosion, size 4.0, damage 8.0, knockback 1).
/// `throw_sound` is the Java throw sound: `throw.throwball` for the balls (GlowBallItem.use,
/// EFLNItem.use), `throw.shuriken` for the shurikens (ShurikenItem.use).
pub(crate) struct Thrown {
    pub name: &'static str,
    pub damage: f32,
    pub knockback: f32,
    /// The sound a hand throw of this type plays, instead of the vanilla snowball throw.
    pub throw_sound: &'static str,
}

pub(crate) const THROWABLES: [Thrown; 4] = [
    Thrown { name: "glow_ball", damage: 0.0, knockback: 0.0, throw_sound: "tconstruct:throw.throwball" },
    Thrown { name: "efln_ball", damage: 0.0, knockback: 0.0, throw_sound: "tconstruct:throw.throwball" },
    Thrown { name: "flint_shuriken", damage: 3.0, knockback: 0.6, throw_sound: "tconstruct:throw.shuriken" },
    Thrown { name: "quartz_shuriken", damage: 5.0, knockback: 0.4, throw_sound: "tconstruct:throw.shuriken" },
];

fn id(name: &str) -> Identifier {
    Identifier::new("tconstruct", name).expect("valid item id")
}

pub(crate) fn register() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        for thrown in &THROWABLES {
            let item = id(thrown.name);
            ThrownTypeRegistry::register(
                ThrownType::new(item.clone(), item.clone(), thrown.damage)
                    .expect("a valid thrown type")
                    .throw_sound(Identifier::parse(thrown.throw_sound).expect("a valid throw sound")),
            )
            .expect("Tinkers thrown type registration failed");
            // The item's own entity type is its projectile, under the same id (TinkerGadgets registers
            // the ShootProjectileDispenserBehavior with the item's entity for each of the four).
            DispenseRegistry::register(item.clone(), DispenseBehavior::Shoot { entity: item, power: DISPENSE_POWER, inaccuracy: DISPENSE_INACCURACY });
        }
        ProjectileEvents::IMPACT.register(on_impact);
        // The optional Mantle tooltip of all four items: `item.tconstruct.<name>.tooltip` in
        // the shipped lang files (the piggybackpack registers its own in crate::piggyback).
        ItemTooltipCallback::EVENT.register(|tip| {
            if let Some(name @ ("glow_ball" | "efln_ball" | "flint_shuriken" | "quartz_shuriken")) =
                tip.item.strip_prefix("tconstruct:")
            {
                tip.added.borrow_mut().push(TooltipLine::translatable(format!("item.tconstruct.{name}.tooltip")));
            }
        });
    });
}

/// The radius of EFLNEntity's detonation (`EFLNExplosion`'s size 4).
pub(crate) const EFLN_RADIUS: f32 = 4.0;

/// The impact half of the throwables. Always `true`: the game's own hit (the type's damage,
/// the break burst, the discard) is Java's `super.onHit` and must still run.
fn on_impact(hit: &ProjectileImpact) -> bool {
    match hit.kind.as_str() {
        "tconstruct:glow_ball" => {
            let (pos, facing) = glow_target(hit);
            place_glow(&hit.world(), pos, facing);
        }
        "tconstruct:efln_ball" => {
            // EFLNEntity.onHit: the ball detonates where it lands and is spent (the module header
            // records how the game's explosion stands in for EFLNExplosion).
            hit.world().explode(hit.at, EFLN_RADIUS, false, ExplosionInteraction::Tnt);
            hit.discard.set(true);
        }
        "tconstruct:flint_shuriken" | "tconstruct:quartz_shuriken" => {
            if hit.entity.is_none() {
                // Java drops the item only on a block hit (onHitBlock); an entity hit keeps it flying into the hit.
                drop_thrown_item(&hit.world(), hit.at, &hit.kind);
            } else if let Some(thrown) = THROWABLES.iter().find(|thrown| hit.kind == format!("tconstruct:{}", thrown.name)) {
                // ShurikenEntityBase.onHitEntity: after the hurt, the target is knocked with the
                // type's strength along the flight — `living.knockback(strength, -motion.x,
                // -motion.z)` with the motion normalized, which the vector direction reproduces.
                hit.actions.borrow_mut().push(EntityAction::Knockback {
                    target: ActionTarget::Victim,
                    strength: f64::from(thrown.knockback),
                    direction: KnockbackDirection::Vector { x: -hit.motion[0], z: -hit.motion[2] },
                });
            }
        }
        _ => {}
    }
    true
}

/// Where a glow ball's glow goes and which way it faces (GlowballEntity.onHit +
/// GlowBlock.addGlow): an entity hit puts it at the hit entity's block position facing down;
/// a block hit puts it in the cell on the hit face's side, facing back toward the hit block.
pub(crate) fn glow_target(hit: &ProjectileImpact) -> ([i32; 3], Direction) {
    if hit.entity.is_some() {
        return (hit.block, Direction::Down);
    }
    let face = Direction::from_offset(hit.face).unwrap_or(Direction::Up);
    (face.relative(hit.block), face.opposite())
}

/// Places the glow block as `GlowBlock.addGlow` does, as far as the world reach allows: the
/// cell must not already hold a glow block, and a liquid cell never takes one (canSurvive's
/// LiquidBlock clause). Java then tries the preferred facing and every other direction whose
/// support face is sturdy (`Block.isFaceSturdy` on the block the glow would hang from); the
/// world reach in this base has no sturdy/replaceable query (those live on the block
/// behaviours' `BlockView`, which an impact callback does not carry), so the preferred facing
/// is placed and survival is enforced a tick later by the glow block's own scheduled-tick
/// self-check (crate::glow_block), with its registered `can_survive` covering placement and
/// neighbour changes through the game's own paths. A glow Java could only have hung on a
/// side support is therefore not placed — a documented reduction.
pub(crate) fn place_glow(world: &World, pos: [i32; 3], facing: Direction) {
    let Some(current) = world.get_block_state(pos) else { return };
    if current.block == id("glow") || current.block == Identifier::new("minecraft", "water").expect("valid id") || current.block == Identifier::new("minecraft", "lava").expect("valid id") {
        return;
    }
    let state = BlockState::new(id("glow")).with("facing", facing.name());
    if world.set_block_state(pos, &state) {
        world.schedule_tick(pos, "tconstruct:glow", 1);
    }
}

/// Drops one of the thrown item where the projectile landed (ShurikenEntityBase's
/// `spawnAtLocation(getDefaultItem())`), through the world's `/summon` reach with the item
/// entity data in the game's own layout (`{Item: {id, count}}`, as PlutoLib's host adapter
/// writes it for an inventory overflow drop).
pub(crate) fn drop_thrown_item(world: &World, at: [f64; 3], item: &str) {
    let mut stack = NbtCompound::new();
    stack.insert("id".to_owned(), Nbt::String(item.to_owned()));
    stack.insert("count".to_owned(), Nbt::Int(1));
    let mut nbt = NbtCompound::new();
    nbt.insert("Item".to_owned(), Nbt::Compound(stack));
    world.summon("minecraft:item", at, &nbt);
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::{with_world, MemoryWorldOps};

    fn hit(kind: &str, entity: Option<u32>, block: [i32; 3], face: [i32; 3]) -> ProjectileImpact {
        ProjectileImpact {
            dimension: "minecraft:overworld".into(),
            projectile: 7,
            kind: kind.into(),
            owner: 1,
            entity,
            entity_kind: if entity.is_some() { "minecraft:zombie".into() } else { String::new() },
            entity_effects: Vec::new(),
            block,
            face,
            at: [block[0] as f64 + 0.5, block[1] as f64 + 0.5, block[2] as f64 + 0.5],
            motion: [0.0, 0.0, 0.0],
            data: NbtCompound::new(),
            piercing_ignored: Vec::new(),
            pierced_and_killed: Vec::new(),
            shot_punch: 0,
            discard: std::cell::Cell::new(false),
            motion_after: std::cell::RefCell::new(None),
            ignore_entities: std::cell::RefCell::new(Vec::new()),
            actions: std::cell::RefCell::new(Vec::new()),
        }
    }

    #[test]
    fn the_thrown_types_carry_the_entity_classes_damage_and_throw_sound() {
        for thrown in &THROWABLES {
            let item = id(thrown.name);
            let made = ThrownType::new(item.clone(), item, thrown.damage)
                .expect("the row validates")
                .throw_sound(Identifier::parse(thrown.throw_sound).expect("the row's sound validates"));
            let _ = ThrownTypeRegistry::register(made);
            let registered = ThrownTypeRegistry::get(&id(thrown.name)).expect("the type is registered");
            assert_eq!(registered.damage, thrown.damage, "{}", thrown.name);
            let expected = if thrown.name.ends_with("shuriken") { "tconstruct:throw.shuriken" } else { "tconstruct:throw.throwball" };
            assert_eq!(
                registered.throw_sound.as_ref().map(|sound| sound.to_string()),
                Some(expected.to_owned()),
                "{}",
                thrown.name
            );
        }
        assert_eq!(THROWABLES[2].knockback, 0.6, "flint knockback");
        assert_eq!(THROWABLES[3].knockback, 0.4, "quartz knockback");
    }

    #[test]
    fn a_dispenser_shoots_each_throwable_as_its_own_projectile() {
        // register() makes exactly these registrations (it is not called here: its thrown-type
        // registrations are one-shot and other tests own them).
        for thrown in &THROWABLES {
            let item = id(thrown.name);
            DispenseRegistry::register(item.clone(), DispenseBehavior::Shoot { entity: item, power: DISPENSE_POWER, inaccuracy: DISPENSE_INACCURACY });
        }
        for thrown in &THROWABLES {
            let behavior = DispenseRegistry::get(&id(thrown.name).to_string()).expect("the behaviour is registered");
            let DispenseBehavior::Shoot { entity, power, inaccuracy } = behavior else { panic!("a shot, not another behaviour") };
            assert_eq!(entity, id(thrown.name), "{} shoots its own entity type", thrown.name);
            assert_eq!((power, inaccuracy), (1.1, 6.0), "ShootProjectileDispenserBehavior's numbers");
        }
    }

    #[test]
    fn a_glow_ball_lands_where_java_puts_it() {
        // Block hit on the top face: the glow sits on the hit block, facing down at it.
        assert_eq!(glow_target(&hit("tconstruct:glow_ball", None, [4, 63, 4], [0, 1, 0])), ([4, 64, 4], Direction::Down));
        // Block hit on a wall's east face: the glow sits in the cell east of the wall, facing west back
        // at its support (GlowballEntity passes the hit direction's opposite to addGlow, which sets
        // FACING to that direction directly; FACING points from the glow toward the block it hangs on).
        assert_eq!(glow_target(&hit("tconstruct:glow_ball", None, [4, 64, 4], [1, 0, 0])), ([5, 64, 4], Direction::West));
        // Entity hit: the hit entity's block position, facing down.
        assert_eq!(glow_target(&hit("tconstruct:glow_ball", Some(9), [4, 64, 4], [0, 1, 0])), ([4, 64, 4], Direction::Down));
    }

    #[test]
    fn a_glow_ball_places_a_glow_block_and_never_in_a_liquid_or_on_a_glow() {
        let mut ops = MemoryWorldOps::default();
        with_world(&mut ops, || {
            let world = World::new("minecraft:overworld");
            world.set_block_state([4, 64, 4], &BlockState::new(Identifier::parse("minecraft:air").unwrap()));
            place_glow(&world, [4, 64, 4], Direction::Down);
            let placed = world.get_block_state([4, 64, 4]).expect("the glow was placed");
            assert_eq!(placed.block, id("glow"));
            assert_eq!(placed.property("facing"), Some("down"));
            // A second ball on the same cell does nothing (addGlow's not-already-glow clause).
            place_glow(&world, [4, 64, 4], Direction::Up);
            assert_eq!(world.get_block_state([4, 64, 4]).unwrap().property("facing"), Some("down"));
            // A water cell never takes one.
            world.set_block_state([5, 64, 4], &BlockState::new(Identifier::parse("minecraft:water").unwrap()));
            place_glow(&world, [5, 64, 4], Direction::Down);
            assert_eq!(world.get_block_state([5, 64, 4]).unwrap().block, Identifier::parse("minecraft:water").unwrap());
        });
    }

    #[test]
    fn a_shuriken_block_hit_drops_the_item_and_an_entity_hit_does_not() {
        let mut ops = MemoryWorldOps::default();
        let block_hit = hit("tconstruct:flint_shuriken", None, [1, 64, 1], [0, 1, 0]);
        with_world(&mut ops, || assert!(on_impact(&block_hit)));
        assert_eq!(ops.summons.len(), 1);
        assert_eq!(ops.summons[0].kind, "minecraft:item");
        let item = ops.summons[0].nbt.get("Item").expect("the dropped stack");
        let Nbt::Compound(stack) = item else { panic!("the drop is a compound") };
        assert_eq!(stack.get("id"), Some(&Nbt::String("tconstruct:flint_shuriken".into())));
        assert_eq!(stack.get("count"), Some(&Nbt::Int(1)));

        let mut ops = MemoryWorldOps::default();
        let entity_hit = hit("tconstruct:quartz_shuriken", Some(3), [1, 64, 1], [0, 1, 0]);
        with_world(&mut ops, || assert!(on_impact(&entity_hit)));
        assert!(ops.summons.is_empty(), "an entity hit drops nothing");
    }

    #[test]
    fn a_shuriken_entity_hit_knocks_the_target_along_its_flight_with_the_types_strength() {
        let mut flying = hit("tconstruct:flint_shuriken", Some(3), [1, 64, 1], [0, 1, 0]);
        flying.motion = [0.6, -0.2, -0.8];
        assert!(on_impact(&flying));
        assert_eq!(
            flying.actions.borrow().as_slice(),
            [EntityAction::Knockback { target: ActionTarget::Victim, strength: f64::from(0.6f32), direction: KnockbackDirection::Vector { x: -0.6, z: 0.8 } }],
            "flint: strength 0.6 along the negated motion, as LivingEntity.knockback reads it"
        );
        let mut flying = hit("tconstruct:quartz_shuriken", Some(3), [1, 64, 1], [0, 1, 0]);
        flying.motion = [0.0, 0.0, 1.5];
        assert!(on_impact(&flying));
        assert_eq!(
            flying.actions.borrow().as_slice(),
            [EntityAction::Knockback { target: ActionTarget::Victim, strength: f64::from(0.4f32), direction: KnockbackDirection::Vector { x: 0.0, z: -1.5 } }],
            "quartz: strength 0.4"
        );
        // A block hit queues no knockback (and drops the item instead, covered above).
        let block_hit = hit("tconstruct:flint_shuriken", None, [1, 64, 1], [0, 1, 0]);
        let mut ops = MemoryWorldOps::default();
        with_world(&mut ops, || assert!(on_impact(&block_hit)));
        assert!(block_hit.actions.borrow().is_empty());
    }

    #[test]
    fn an_efln_ball_detonates_where_it_lands_and_is_spent() {
        let mut ops = MemoryWorldOps::default();
        let landed = hit("tconstruct:efln_ball", None, [2, 64, 2], [0, 1, 0]);
        with_world(&mut ops, || assert!(on_impact(&landed)));
        assert!(landed.discard.get(), "the ball is spent by its detonation");
        assert_eq!(ops.explosions.len(), 1);
        let explosion = &ops.explosions[0];
        assert_eq!(explosion.center, [2.5, 64.5, 2.5], "at the hit position");
        assert_eq!(explosion.radius, EFLN_RADIUS, "EFLNExplosion's size 4");
        assert!(!explosion.fire, "EFLNExplosion sets no fire");
        assert_eq!(explosion.interaction, ExplosionInteraction::Tnt, "NeoForge's DESTROY block interaction");
        // An entity hit detonates too (EFLNEntity.onHit covers both).
        let mut ops = MemoryWorldOps::default();
        let struck = hit("tconstruct:efln_ball", Some(5), [2, 64, 2], [0, 1, 0]);
        with_world(&mut ops, || assert!(on_impact(&struck)));
        assert_eq!(ops.explosions.len(), 1);
    }
}
