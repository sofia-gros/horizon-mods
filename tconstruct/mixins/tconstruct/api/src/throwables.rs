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
//!   drop is summoned here. On an entity hit the game already deals the type's damage and
//!   pushes the target with the projectile's motion; Java's exact knockback strength
//!   (flint 0.6, quartz 0.4, `LivingEntity.knockback`) has no velocity surface to set it
//!   through, so the game's push stands — a documented reduction.
//! - EFLN: `EFLNEntity.onHit` detonates an `EFLNExplosion` (size 4, damage 8, knockback 1,
//!   sphere falloff, block interaction DESTROY). No surface in this PlutoLib base can cause an
//!   explosion: the host API only hears explosions (`explosion_detonate`/`explosion_knockback`
//!   hooks) and PlutoLib dispatches neither, and there is no explode helper on the world or
//!   entity reach. The ball therefore flies and hits as a 0-damage thrown type and does not
//!   explode — a documented wall, with the Java numbers kept in [`THROWABLES`]' notes and in
//!   this header for the lane that builds an explosion surface.
//! - Sounds: Java plays `tconstruct:throw.throwball` / `tconstruct:throw.shuriken` (Sounds.java,
//!   NEUTRAL, 0.5 volume, pitch `0.4F / (random * 0.4F + 0.8F)`); the game picks the throw sound
//!   by projectile kind and plays the vanilla snowball throw for thrown types, so the custom
//!   sounds are not heard — a documented reduction.
//! - Dispensers: Java registers a `ShootProjectileDispenserBehavior` (power 1.1, inaccuracy
//!   6.0) for all four items. The host has a `dispense_item` hook (API 67, `DispenseItemEvent`
//!   → `Option<NativeDispensed>`), but this PlutoLib base dispatches no dispenser hook, so a
//!   dispenser treats the four as ordinary items — a documented wall for the foundation lane
//!   that wires the dispenser hook into PlutoLib.
//!
//! There are no recipes for the four in the original (`gadgets/data/GadgetRecipeProvider.java`
//! has none; the items are deprecated), so none are converted.
use pluto_lib::{BlockState, Direction, Identifier, ItemTooltipCallback, Nbt, NbtCompound, ProjectileEvents, ProjectileImpact, ThrownType, ThrownTypeRegistry, TooltipLine, World};

/// One throwable of TinkerGadgets, with its entity's numbers from the entity classes:
/// `damage`/`knockback` are ShurikenEntityBase's `getDamage()`/`getKnockback()` (flint 3.0/0.6,
/// quartz 5.0/0.4); the balls deal no damage (GlowballEntity and EFLNEntity override no
/// `onHitEntity` damage; the EFLN's harm is its explosion, size 4.0, damage 8.0, knockback 1).
pub(crate) struct Thrown {
    pub name: &'static str,
    pub damage: f32,
    pub knockback: f32,
}

pub(crate) const THROWABLES: [Thrown; 4] = [
    Thrown { name: "glow_ball", damage: 0.0, knockback: 0.0 },
    Thrown { name: "efln_ball", damage: 0.0, knockback: 0.0 },
    Thrown { name: "flint_shuriken", damage: 3.0, knockback: 0.6 },
    Thrown { name: "quartz_shuriken", damage: 5.0, knockback: 0.4 },
];

fn id(name: &str) -> Identifier {
    Identifier::new("tconstruct", name).expect("valid item id")
}

pub(crate) fn register() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        for thrown in &THROWABLES {
            let item = id(thrown.name);
            ThrownTypeRegistry::register(ThrownType::new(item.clone(), item, thrown.damage).expect("a valid thrown type"))
                .expect("Tinkers thrown type registration failed");
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

/// The impact half of the throwables. Always `true`: the game's own hit (the type's damage,
/// the break burst, the discard) is Java's `super.onHit` and must still run.
fn on_impact(hit: &ProjectileImpact) -> bool {
    match hit.kind.as_str() {
        "tconstruct:glow_ball" => {
            let (pos, facing) = glow_target(hit);
            place_glow(&hit.world(), pos, facing);
        }
        "tconstruct:flint_shuriken" | "tconstruct:quartz_shuriken" => {
            // Java drops the item only on a block hit (onHitBlock); an entity hit keeps it flying into the hit.
            if hit.entity.is_none() {
                drop_thrown_item(&hit.world(), hit.at, &hit.kind);
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
            block,
            face,
            at: [block[0] as f64 + 0.5, block[1] as f64 + 0.5, block[2] as f64 + 0.5],
        }
    }

    #[test]
    fn the_thrown_types_carry_the_entity_classes_damage() {
        for thrown in &THROWABLES {
            let item = id(thrown.name);
            let made = ThrownType::new(item.clone(), item, thrown.damage).expect("the row validates");
            let _ = ThrownTypeRegistry::register(made);
            assert_eq!(ThrownTypeRegistry::get(&id(thrown.name)).map(|t| t.damage), Some(thrown.damage), "{}", thrown.name);
        }
        assert_eq!(THROWABLES[2].knockback, 0.6, "flint knockback (documented, not settable)");
        assert_eq!(THROWABLES[3].knockback, 0.4, "quartz knockback (documented, not settable)");
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
}
