//! The entities of a world, reached from a callback that carries them (a block entity ticker): what stands inside a
//! box, hurting it, taking the items it dropped, who rides whom, and pushing it (Minecraft: `Level.getEntitiesOfClass`,
//! `LivingEntity.hurt`, `ItemEntity.setItem` and `discard`, `Entity.startRiding` and `stopRiding`,
//! `LivingEntity.knockback` and `Entity.push`). Like
//! [`World`](super::world_api_v1::World), the reach only exists while a host-installed scope is open; outside it
//! every query answers "nothing there", never a guess.
use super::{base::Identifier, block_entity_api_v1::ItemSlot};
use std::{cell::Cell, marker::PhantomData};

/// A living entity as the reach reports it: the facts a caller decides with, read by the host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LivingFacts {
    /// It cannot be hurt by fire damage (`Entity.fireImmune`); callers hurt it another way (magic, in the original).
    pub fire_immune: bool,
    /// It cannot be hurt at all right now: its invulnerable flag, or a player whose abilities make them invulnerable
    /// (creative and spectator game modes).
    pub invulnerable: bool,
    /// It has the fire resistance effect (`MobEffects.FIRE_RESISTANCE`).
    pub fire_resistant: bool,
}

/// One entity inside a box.
#[derive(Clone, Debug, PartialEq)]
pub struct WorldEntity {
    /// The entity's id in its level.
    pub id: u64,
    /// Its type, such as `minecraft:zombie` or `minecraft:item`.
    pub kind: Identifier,
    /// The stack of an item entity; `None` for any other kind.
    pub item: Option<ItemSlot>,
    /// A living entity's facts; `None` for item entities and for anything that is not alive.
    pub living: Option<LivingFacts>,
}

/// What the game can do about entities. The host implements it; [`MemoryEntityOps`] is a small world for tests.
pub trait EntityOps {
    /// Every living entity and item entity whose position is inside the box (min corner, max corner), in the level's
    /// order. Anything else (a painting, a falling block) is not listed.
    fn entities_in(&mut self, min: [f64; 3], max: [f64; 3]) -> Vec<WorldEntity>;
    /// Removes the item entity `id`, which was taken whole; false when the host cannot (no such entity, or not an
    /// item entity).
    fn discard(&mut self, id: u64) -> bool;
    /// Leaves `stack` in the item entity `id` (what did not fit where it was taken to); false when there is no such
    /// item entity.
    fn set_item(&mut self, id: u64, stack: &ItemSlot) -> bool;
    /// Hurts entity `id` by `amount` of the damage type `damage_type`, with no attacker; whether it took any (the
    /// host refuses an invulnerable, dead or creative target, as `LivingEntity.hurt` does).
    fn hurt(&mut self, id: u64, amount: f32, damage_type: &str) -> bool;
    /// Where entity `id` stands (its feet position, `Entity.position`); None when the host cannot reach it.
    fn position_of(&mut self, _id: u64) -> Option<[f64; 3]> {
        None
    }
    /// The mob effects entity `id` carries (its id, amplifier and ticks left, in that order), as the
    /// effect list of a mob or the effects of a player. Empty when it carries none, or when the host
    /// cannot say (a host that predates the call answers empty).
    fn effects_of(&mut self, _id: u64) -> Vec<(String, i32, i32)> {
        Vec::new()
    }
    /// The velocity entity `id` moves with (`Entity.getDeltaMovement`); None when the host cannot
    /// reach it (a player moves on their client: the host reads the motion their client last
    /// reported). Hosts that predate the call answer None.
    fn velocity_of(&mut self, _id: u64) -> Option<[f64; 3]> {
        None
    }
    /// The two corners of entity `id`'s bounding box (min, max), as `Entity.getBoundingBox` gives
    /// them; None when the host cannot reach it. Hosts that predate the call answer None.
    fn bounds_of(&mut self, _id: u64) -> Option<[[f64; 3]; 2]> {
        None
    }
    /// Sets entity `id`'s velocity to `velocity` (`Entity.setDeltaMovement`): the motion is replaced,
    /// not added. A player is not set (its client moves it); an item entity or a mob is. False when
    /// the host cannot (no such entity, a player, an entity the game does not simulate, or a value
    /// that is not finite). Hosts that predate the call answer false.
    fn set_velocity(&mut self, _id: u64, _velocity: [f64; 3]) -> bool {
        false
    }
    /// Moves entity `id` to `pos` (`Entity.teleportTo`): a mob's navigation stops and its fall distance
    /// resets, as the game's own teleport does; another entity keeps the way it faces. A player is moved
    /// by the host's own player teleport (the `/tp` path: they leave their vehicle, lose their motion
    /// and fall, and their client is told), keeping the way they face; a player that call cannot reach is
    /// queued through the player teleport transport and applied at the level's rider synchronisation by
    /// the game's own server teleport, with the game's own bounds (a living player on the server, the
    /// `/tp` position bounds) the last word. False when the host cannot move it.
    fn teleport(&mut self, _id: u64, _pos: [f64; 3]) -> bool {
        false
    }
    /// The direct passengers of `vehicle`, in the order they mounted (Minecraft: `Entity.getPassengers`). Empty
    /// when the entity carries nobody — or when the host cannot say. Hosts that predate passenger support answer
    /// empty.
    fn passengers_of(&mut self, vehicle: u64) -> Vec<u64> {
        let _ = vehicle;
        Vec::new()
    }
    /// The vehicle `passenger` rides, if it rides one (Minecraft: `Entity.getVehicle`). `None` when it rides
    /// nothing — or when the host cannot say.
    fn vehicle_of(&mut self, passenger: u64) -> Option<u64> {
        let _ = passenger;
        None
    }
    /// Puts `passenger` on `vehicle` (Minecraft: `Entity.startRiding`): true when the mount was accepted. The
    /// host refuses what the game refuses — the same entity twice, a passenger that already rides something, a
    /// cycle (the vehicle already rides the passenger, directly or through a chain), an entity the game will not
    /// seat, and any mount its own mount event cancels. A host may apply an accepted mount at its next rider
    /// synchronisation rather than instantly; the result is then observable through [`EntityOps::vehicle_of`] from
    /// the next tick, and a refusal there is final.
    fn start_riding(&mut self, passenger: u64, vehicle: u64) -> bool {
        let _ = (passenger, vehicle);
        false
    }
    /// Takes `passenger` off whatever it rides (Minecraft: `Entity.stopRiding`): true when it was riding and the
    /// dismount was accepted; the host may likewise apply it at its next rider synchronisation.
    fn stop_riding(&mut self, passenger: u64) -> bool {
        let _ = passenger;
        false
    }
    /// The entity id of the player whose uuid is `uuid`, in the level the scope runs in (the host resolves a
    /// uuid through its entity-by-uuid call, which names players); None when no such player is there. Hosts
    /// that predate the call answer None, as does the memory backend, whose world holds no players.
    fn id_of_uuid(&mut self, _uuid: u128) -> Option<u64> {
        None
    }
    /// The type of the entity `id` (its registry id, such as `minecraft:player`); None when there is no such
    /// entity. Hosts that predate the call answer None.
    fn kind_of(&mut self, _id: u64) -> Option<Identifier> {
        None
    }
    /// Knocks entity `id` back with `strength` along the horizontal direction (`x`, `z`), as
    /// `LivingEntity.knockback(strength, x, z)` does: the entity's own knockback resistance scales the
    /// strength, its horizontal motion is halved against the push, and it rises by the strength (at most
    /// 0.4) when it stands on the ground. A player is knocked back the same way, and their client is told
    /// the new motion, as the game's own knockback of a player tells it. Only a living entity or a player
    /// can be knocked back. A host may apply an accepted knockback at its next rider synchronisation
    /// rather than instantly; the motion is then observable from the next tick, and a refusal there (the
    /// entity is gone, or not living) is final. Hosts that predate the call answer false.
    fn knockback(&mut self, _id: u64, _strength: f64, _x: f64, _z: f64) -> bool {
        false
    }
    /// Adds `delta` to entity `id`'s velocity, as `Entity.push` does: no resistance and no halving, the
    /// motion simply grows by the vector. A player is pushed the same way, and their client is told. Only
    /// a living entity or a player is pushed. A host may apply an accepted push at its next rider
    /// synchronisation rather than instantly, as [`EntityOps::knockback`] describes. Hosts that predate
    /// the call answer false.
    fn push(&mut self, _id: u64, _delta: [f64; 3]) -> bool {
        false
    }
    /// Lifts the player `id` off the ground for a launch, as `Entity.move(MoverType.SELF, (0, 1.3, 0))`
    /// does at the head of one: standing on the ground, they rise by 1.3, or to the first ceiling the
    /// move would stop at. Only a player is lifted (the move's caller in the original launches the tool's
    /// user), and only while they stand on the ground. A host may apply an accepted lift at its next
    /// rider synchronisation rather than instantly, as [`EntityOps::knockback`] describes. Hosts that
    /// predate the call answer false.
    fn leave_ground(&mut self, _id: u64) -> bool {
        false
    }
}

thread_local! {
    static OPS: Cell<Option<*mut dyn EntityOps>> = const { Cell::new(None) };
}

struct Scope<'a> {
    previous: Option<*mut dyn EntityOps>,
    _borrow: PhantomData<&'a mut dyn EntityOps>,
}

impl Drop for Scope<'_> {
    fn drop(&mut self) {
        OPS.with(|slot| slot.set(self.previous));
    }
}

/// Runs `f` with `ops` answering every [`EntityReach`] query made (on this thread) during `f`.
pub fn with_entities<R>(ops: &mut dyn EntityOps, f: impl FnOnce() -> R) -> R {
    // SAFETY: the pointer is only dereferenced while `f` runs, inside the lifetime of `ops`; `Scope` clears it on every exit.
    let raw: *mut dyn EntityOps = unsafe { std::mem::transmute::<&mut dyn EntityOps, *mut dyn EntityOps>(ops) };
    let previous = OPS.with(|slot| slot.replace(Some(raw)));
    let _scope = Scope { previous, _borrow: PhantomData };
    f()
}

fn with_current<R>(default: R, f: impl FnOnce(&mut dyn EntityOps) -> R) -> R {
    match OPS.with(Cell::get) {
        // SAFETY: see `with_entities`: the pointer is valid for as long as it is stored.
        Some(raw) => f(unsafe { &mut *raw }),
        None => default,
    }
}

/// Queries on the entities of the world. Every method answers "nothing there" outside a scope.
pub struct EntityReach;

impl EntityReach {
    /// Every living entity and item entity inside the box (min corner, max corner); empty outside a scope.
    pub fn in_box(min: [f64; 3], max: [f64; 3]) -> Vec<WorldEntity> {
        with_current(Vec::new(), |ops| ops.entities_in(min, max))
    }

    /// Removes the item entity `id`; false outside a scope or when the host cannot.
    pub fn discard(id: u64) -> bool {
        with_current(false, |ops| ops.discard(id))
    }

    /// Leaves `stack` in the item entity `id`; false outside a scope or where there is no such item entity.
    pub fn set_item(id: u64, stack: &ItemSlot) -> bool {
        with_current(false, |ops| ops.set_item(id, stack))
    }

    /// Hurts entity `id` by `amount` of `damage_type` with no attacker; whether it took any.
    pub fn hurt(id: u64, amount: f32, damage_type: &str) -> bool {
        with_current(false, |ops| ops.hurt(id, amount, damage_type))
    }

    /// Where entity `id` stands (its feet position); None outside a scope or when the host cannot reach it.
    pub fn position_of(id: u64) -> Option<[f64; 3]> {
        with_current(None, |ops| ops.position_of(id))
    }

    /// The mob effects entity `id` carries (id, amplifier, ticks left); empty outside a scope or when
    /// it carries none.
    pub fn effects_of(id: u64) -> Vec<(String, i32, i32)> {
        with_current(Vec::new(), |ops| ops.effects_of(id))
    }

    /// The velocity entity `id` moves with; None outside a scope or when the host cannot reach it.
    pub fn velocity_of(id: u64) -> Option<[f64; 3]> {
        with_current(None, |ops| ops.velocity_of(id))
    }

    /// The two corners of entity `id`'s bounding box (min, max); None outside a scope or when the host
    /// cannot reach it.
    pub fn bounds_of(id: u64) -> Option<[[f64; 3]; 2]> {
        with_current(None, |ops| ops.bounds_of(id))
    }

    /// Sets entity `id`'s velocity to `velocity` (`Entity.setDeltaMovement`); false outside a scope
    /// or when the host cannot.
    pub fn set_velocity(id: u64, velocity: [f64; 3]) -> bool {
        with_current(false, |ops| ops.set_velocity(id, velocity))
    }

    /// Moves entity `id` to `pos` (`Entity.teleportTo`); false outside a scope or when the host cannot
    /// by the host's own player teleport (the `/tp` path: they leave their vehicle, lose their motion
    /// and fall, and their client is told), keeping the way they face; a player that call cannot reach is
    /// queued through the player teleport transport and applied at the level's rider synchronisation by
    /// the game's own server teleport, with the game's own bounds (a living player on the server, the
    /// `/tp` position bounds) the last word. False when the host cannot move it.
    pub fn teleport(id: u64, pos: [f64; 3]) -> bool {
        with_current(false, |ops| ops.teleport(id, pos))
    }

    /// The direct passengers of `vehicle`, in the order they mounted; empty outside a scope.
    pub fn passengers_of(vehicle: u64) -> Vec<u64> {
        with_current(Vec::new(), |ops| ops.passengers_of(vehicle))
    }

    /// The vehicle `passenger` rides; `None` outside a scope or when it rides nothing.
    pub fn vehicle_of(passenger: u64) -> Option<u64> {
        with_current(None, |ops| ops.vehicle_of(passenger))
    }

    /// Puts `passenger` on `vehicle`; whether the mount was accepted (see [`EntityOps::start_riding`] for what a
    /// host may refuse, and when an accepted mount becomes observable). False outside a scope.
    pub fn start_riding(passenger: u64, vehicle: u64) -> bool {
        with_current(false, |ops| ops.start_riding(passenger, vehicle))
    }

    /// Takes `passenger` off whatever it rides; whether it was riding and the dismount was accepted. False
    /// outside a scope.
    pub fn stop_riding(passenger: u64) -> bool {
        with_current(false, |ops| ops.stop_riding(passenger))
    }

    /// The entity id of the player whose uuid is `uuid`; None outside a scope or when no such player is there.
    pub fn id_of_uuid(uuid: u128) -> Option<u64> {
        with_current(None, |ops| ops.id_of_uuid(uuid))
    }

    /// The type of the entity `id`; None outside a scope or when there is no such entity.
    pub fn kind_of(id: u64) -> Option<Identifier> {
        with_current(None, |ops| ops.kind_of(id))
    }

    /// Knocks entity `id` back with `strength` along the horizontal direction (`x`, `z`), as
    /// `LivingEntity.knockback` does (see [`EntityOps::knockback`] for exactly what lands, and when);
    /// whether the knockback was accepted. False outside a scope.
    pub fn knockback(id: u64, strength: f64, x: f64, z: f64) -> bool {
        with_current(false, |ops| ops.knockback(id, strength, x, z))
    }

    /// Adds `delta` to entity `id`'s velocity, as `Entity.push` does (see [`EntityOps::push`]); whether
    /// the push was accepted. False outside a scope.
    pub fn push(id: u64, delta: [f64; 3]) -> bool {
        with_current(false, |ops| ops.push(id, delta))
    }

    /// Lifts the player `id` off the ground for a launch (see [`EntityOps::leave_ground`]); whether the
    /// lift was accepted. False outside a scope.
    pub fn leave_ground(id: u64) -> bool {
        with_current(false, |ops| ops.leave_ground(id))
    }
}

/// A small world of entities, for tests: the entities stand at fixed positions, and what was done to them is kept.
pub struct MemoryEntityOps {
    /// The entities, each at its position.
    pub entities: Vec<([f64; 3], WorldEntity)>,
    /// The item entities removed, in order.
    pub discarded: Vec<u64>,
    /// What was left in an item entity: its id and the stack.
    pub left: Vec<(u64, ItemSlot)>,
    /// The hurts asked for: the id, the amount and the damage type.
    pub hurt: Vec<(u64, f32, String)>,
    /// Whether hurts land; the host refuses some (an invulnerable or dead target), a test can say so here.
    pub hurts_land: bool,
    /// Who rides whom: `(passenger, vehicle)` pairs, in the order they mounted.
    pub mounts: Vec<(u64, u64)>,
    /// The knockbacks asked for: the id, the strength and the horizontal direction.
    pub knocked: Vec<(u64, f64, f64, f64)>,
    /// The pushes asked for: the id and the velocity added.
    pub pushed: Vec<(u64, [f64; 3])>,
    /// The players lifted off the ground, in order.
    pub lifted: Vec<u64>,
    /// The mob effects each entity carries: its id and the (effect id, amplifier, ticks left) list.
    pub effect_lists: Vec<(u64, Vec<(String, i32, i32)>)>,
    /// The velocity each entity moves with: its id and the motion.
    pub velocities: Vec<(u64, [f64; 3])>,
    /// The bounding box of each entity: its id and the (min, max) corners.
    pub bounding_boxes: Vec<(u64, [[f64; 3]; 2])>,
}

impl Default for MemoryEntityOps {
    fn default() -> Self {
        Self { entities: Vec::new(), discarded: Vec::new(), left: Vec::new(), hurt: Vec::new(), hurts_land: true, mounts: Vec::new(), knocked: Vec::new(), pushed: Vec::new(), lifted: Vec::new(), effect_lists: Vec::new(), velocities: Vec::new(), bounding_boxes: Vec::new() }
    }
}

impl EntityOps for MemoryEntityOps {
    fn entities_in(&mut self, min: [f64; 3], max: [f64; 3]) -> Vec<WorldEntity> {
        self.entities.iter()
            .filter(|(pos, _)| (0..3).all(|axis| min[axis] <= pos[axis] && pos[axis] <= max[axis]))
            .map(|(_, entity)| entity.clone())
            .collect()
    }

    fn discard(&mut self, id: u64) -> bool {
        let Some(index) = self.entities.iter().position(|(_, entity)| entity.id == id && entity.item.is_some()) else { return false };
        self.entities.remove(index);
        self.discarded.push(id);
        true
    }

    fn set_item(&mut self, id: u64, stack: &ItemSlot) -> bool {
        let Some((_, entity)) = self.entities.iter_mut().find(|(_, entity)| entity.id == id && entity.item.is_some()) else { return false };
        entity.item = Some(stack.clone());
        self.left.push((id, stack.clone()));
        true
    }

    fn hurt(&mut self, id: u64, amount: f32, damage_type: &str) -> bool {
        if !self.entities.iter().any(|(_, entity)| entity.id == id && entity.living.is_some()) {
            return false;
        }
        self.hurt.push((id, amount, damage_type.to_owned()));
        self.hurts_land
    }

    fn position_of(&mut self, id: u64) -> Option<[f64; 3]> {
        self.entities.iter().find(|(_, entity)| entity.id == id).map(|(pos, _)| *pos)
    }

    fn effects_of(&mut self, id: u64) -> Vec<(String, i32, i32)> {
        self.effect_lists.iter().find(|(held, _)| *held == id).map(|(_, effects)| effects.clone()).unwrap_or_default()
    }

    fn velocity_of(&mut self, id: u64) -> Option<[f64; 3]> {
        self.velocities.iter().find(|(held, _)| *held == id).map(|(_, velocity)| *velocity)
    }

    fn bounds_of(&mut self, id: u64) -> Option<[[f64; 3]; 2]> {
        self.bounding_boxes.iter().find(|(held, _)| *held == id).map(|(_, bounds)| *bounds)
    }

    fn set_velocity(&mut self, id: u64, velocity: [f64; 3]) -> bool {
        if !velocity.iter().all(|v| v.is_finite()) || !self.entities.iter().any(|(_, entity)| entity.id == id) {
            return false;
        }
        match self.velocities.iter_mut().find(|(held, _)| *held == id) {
            Some((_, held)) => *held = velocity,
            None => self.velocities.push((id, velocity)),
        }
        true
    }

    fn teleport(&mut self, id: u64, pos: [f64; 3]) -> bool {
        let Some((held, _)) = self.entities.iter_mut().find(|(_, entity)| entity.id == id) else { return false };
        *held = pos;
        true
    }

    fn passengers_of(&mut self, vehicle: u64) -> Vec<u64> {
        self.mounts.iter().filter(|(_, carrier)| *carrier == vehicle).map(|(passenger, _)| *passenger).collect()
    }

    fn vehicle_of(&mut self, passenger: u64) -> Option<u64> {
        self.mounts.iter().find(|(rider, _)| *rider == passenger).map(|(_, vehicle)| *vehicle)
    }

    fn start_riding(&mut self, passenger: u64, vehicle: u64) -> bool {
        let known = |id: u64| self.entities.iter().any(|(_, entity)| entity.id == id);
        if passenger == vehicle || !known(passenger) || !known(vehicle) || self.vehicle_of(passenger).is_some() {
            return false;
        }
        // A cycle: the vehicle already rides the passenger, directly or through a chain of vehicles.
        let mut carrier = vehicle;
        while let Some(next) = self.vehicle_of(carrier) {
            if next == passenger {
                return false;
            }
            carrier = next;
        }
        self.mounts.push((passenger, vehicle));
        true
    }

    fn stop_riding(&mut self, passenger: u64) -> bool {
        let Some(index) = self.mounts.iter().position(|(rider, _)| *rider == passenger) else { return false };
        self.mounts.remove(index);
        true
    }

    fn kind_of(&mut self, id: u64) -> Option<Identifier> {
        self.entities.iter().find(|(_, entity)| entity.id == id).map(|(_, entity)| entity.kind.clone())
    }

    fn knockback(&mut self, id: u64, strength: f64, x: f64, z: f64) -> bool {
        if !self.entities.iter().any(|(_, entity)| entity.id == id && entity.living.is_some()) {
            return false;
        }
        self.knocked.push((id, strength, x, z));
        true
    }

    fn push(&mut self, id: u64, delta: [f64; 3]) -> bool {
        if !self.entities.iter().any(|(_, entity)| entity.id == id && entity.living.is_some()) {
            return false;
        }
        self.pushed.push((id, delta));
        true
    }

    fn leave_ground(&mut self, id: u64) -> bool {
        if !self.entities.iter().any(|(_, entity)| entity.id == id && entity.living.is_some()) {
            return false;
        }
        self.lifted.push(id);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }

    fn world() -> MemoryEntityOps {
        let mut ops = MemoryEntityOps::default();
        ops.entities = vec![
            ([1.5, 64.5, 1.5], WorldEntity { id: 1, kind: id("minecraft:item"), item: Some(ItemSlot::new(id("minecraft:iron_ingot"), 3)), living: None }),
            ([2.5, 64.5, 2.5], WorldEntity { id: 2, kind: id("minecraft:zombie"), item: None, living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }) }),
            ([9.5, 64.5, 9.5], WorldEntity { id: 3, kind: id("minecraft:cow"), item: None, living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }) }),
        ];
        ops
    }

    #[test]
    fn outside_a_scope_the_reach_answers_nothing() {
        assert!(EntityReach::in_box([0.0, 0.0, 0.0], [4.0, 70.0, 4.0]).is_empty());
        assert!(!EntityReach::discard(1));
        assert!(!EntityReach::hurt(2, 2.0, "minecraft:in_fire"));
        assert!(!EntityReach::set_item(1, &ItemSlot::new(id("minecraft:iron_ingot"), 1)));
        assert_eq!(EntityReach::position_of(2), None);
        assert!(!EntityReach::teleport(2, [0.0, 64.0, 0.0]));
        assert!(!EntityReach::knockback(2, 1.0, 1.0, 0.0));
        assert!(!EntityReach::push(2, [0.0, 1.0, 0.0]));
        assert!(!EntityReach::leave_ground(2));
    }

    #[test]
    fn a_position_is_read_and_a_teleport_moves_the_entity_that_stands_there() {
        let mut ops = world();
        with_entities(&mut ops, || {
            assert_eq!(EntityReach::position_of(2), Some([2.5, 64.5, 2.5]));
            assert_eq!(EntityReach::position_of(99), None, "no such entity");
            assert!(EntityReach::teleport(2, [7.5, 70.0, -3.5]));
            assert_eq!(EntityReach::position_of(2), Some([7.5, 70.0, -3.5]), "the zombie stands where it was moved to");
            assert!(!EntityReach::teleport(99, [0.0, 64.0, 0.0]), "no such entity to move");
            // The box query sees the move: the zombie left the first box and stands in the second.
            assert_eq!(EntityReach::in_box([0.0, 60.0, 0.0], [4.0, 70.0, 4.0]).iter().map(|entity| entity.id).collect::<Vec<_>>(), [1]);
            assert_eq!(EntityReach::in_box([7.0, 69.0, -4.0], [8.0, 71.0, -3.0]).iter().map(|entity| entity.id).collect::<Vec<_>>(), [2]);
        });
        assert!(EntityReach::passengers_of(2).is_empty());
        assert_eq!(EntityReach::vehicle_of(2), None);
        assert!(!EntityReach::start_riding(2, 3));
        assert!(!EntityReach::stop_riding(2));
    }

    #[test]
    fn a_scope_lists_the_box_and_records_what_was_done() {
        let mut ops = world();
        with_entities(&mut ops, || {
            let found = EntityReach::in_box([0.0, 60.0, 0.0], [4.0, 70.0, 4.0]);
            assert_eq!(found.iter().map(|entity| entity.id).collect::<Vec<_>>(), [1, 2], "the cow stands outside the box");
            assert_eq!(found[0].item.as_ref().unwrap().count, 3);
            assert!(EntityReach::hurt(2, 5.0, "tconstruct:smeltery_heat"));
            assert!(!EntityReach::hurt(1, 5.0, "tconstruct:smeltery_heat"), "an item entity cannot be hurt");
            assert!(EntityReach::set_item(1, &ItemSlot::new(id("minecraft:iron_ingot"), 1)));
            assert!(EntityReach::discard(1));
            assert!(!EntityReach::discard(1), "already gone");
        });
        assert_eq!(ops.hurt, vec![(2, 5.0, "tconstruct:smeltery_heat".to_owned())]);
        assert_eq!(ops.left.len(), 1);
        assert_eq!(ops.discarded, vec![1]);
        assert_eq!(ops.entities.len(), 2, "the taken item entity is gone, the hurt zombie stays");
        assert!(EntityReach::in_box([0.0, 0.0, 0.0], [4.0, 70.0, 4.0]).is_empty(), "the scope is closed again");
    }

    #[test]
    fn a_hurt_the_host_refuses_is_reported_and_still_asked_once() {
        let mut ops = world();
        ops.hurts_land = false;
        with_entities(&mut ops, || assert!(!EntityReach::hurt(2, 2.0, "minecraft:magic")));
        assert_eq!(ops.hurt.len(), 1);
    }

    #[test]
    fn a_kind_is_named_and_a_uuid_names_nobody_in_the_memory_world() {
        let mut ops = world();
        with_entities(&mut ops, || {
            assert_eq!(EntityReach::kind_of(2), Some(id("minecraft:zombie")));
            assert_eq!(EntityReach::kind_of(99), None, "no such entity");
            assert_eq!(EntityReach::id_of_uuid(0x1234), None, "the memory world holds no players");
        });
        assert_eq!(EntityReach::kind_of(2), None, "outside a scope nothing is named");
        assert_eq!(EntityReach::id_of_uuid(0x1234), None);
    }

    #[test]
    fn a_knockback_a_push_and_a_lift_are_recorded_for_the_living() {
        let mut ops = world();
        with_entities(&mut ops, || {
            assert!(EntityReach::knockback(2, 1.5, -0.5, 0.25));
            assert!(EntityReach::push(3, [0.1, 0.4, -0.2]));
            assert!(EntityReach::leave_ground(2));
            assert!(!EntityReach::knockback(1, 1.0, 1.0, 0.0), "an item entity cannot be knocked back");
            assert!(!EntityReach::push(1, [0.0, 1.0, 0.0]), "an item entity is not pushed");
            assert!(!EntityReach::leave_ground(1), "an item entity is not lifted");
            assert!(!EntityReach::knockback(99, 1.0, 1.0, 0.0), "no such entity");
        });
        assert_eq!(ops.knocked, vec![(2, 1.5, -0.5, 0.25)]);
        assert_eq!(ops.pushed, vec![(3, [0.1, 0.4, -0.2])]);
        assert_eq!(ops.lifted, vec![2]);
    }

    #[test]
    fn riding_mounts_dismounts_and_refuses_cycles() {
        let mut ops = world();
        with_entities(&mut ops, || {
            assert!(EntityReach::start_riding(2, 3), "the zombie mounts the cow");
            assert_eq!(EntityReach::vehicle_of(2), Some(3));
            assert_eq!(EntityReach::passengers_of(3), vec![2]);
            assert!(!EntityReach::start_riding(2, 1), "already riding");
            assert!(!EntityReach::start_riding(3, 3), "an entity cannot ride itself");
            assert!(!EntityReach::start_riding(3, 2), "the cow mounting its own rider would close a cycle");
            assert!(!EntityReach::start_riding(2, 99), "no such vehicle");
            assert!(!EntityReach::start_riding(99, 3), "no such passenger");
            assert!(EntityReach::stop_riding(2));
            assert_eq!(EntityReach::vehicle_of(2), None);
            assert!(EntityReach::passengers_of(3).is_empty());
            assert!(!EntityReach::stop_riding(2), "no longer riding");
            assert!(EntityReach::start_riding(3, 2), "the freed pair can mount the other way");
            assert_eq!(EntityReach::passengers_of(2), vec![3]);
        });
        assert_eq!(ops.mounts, vec![(3, 2)]);
    }

    #[test]
    fn effects_velocity_and_bounds_are_read_and_empty_for_the_unknown() {
        let mut ops = world();
        ops.effect_lists = vec![(2, vec![("tconstruct:magnetic".to_owned(), 0, 120), ("tconstruct:returning".to_owned(), 1, 1)])];
        ops.velocities = vec![(2, [0.0, 0.3, 0.0])];
        ops.bounding_boxes = vec![(2, [[2.0, 64.0, 2.0], [2.6, 65.8, 2.6]])];
        with_entities(&mut ops, || {
            assert_eq!(EntityReach::effects_of(2), vec![("tconstruct:magnetic".to_owned(), 0, 120), ("tconstruct:returning".to_owned(), 1, 1)]);
            assert_eq!(EntityReach::velocity_of(2), Some([0.0, 0.3, 0.0]));
            assert_eq!(EntityReach::bounds_of(2), Some([[2.0, 64.0, 2.0], [2.6, 65.8, 2.6]]));
            assert!(EntityReach::effects_of(99).is_empty(), "no such entity");
            assert_eq!(EntityReach::velocity_of(99), None, "no such entity");
            assert_eq!(EntityReach::bounds_of(99), None, "no such entity");
            assert!(EntityReach::effects_of(1).is_empty(), "the item entity carries no effect");
            assert_eq!(EntityReach::velocity_of(1), None, "no velocity was given for it");
            assert_eq!(EntityReach::bounds_of(1), None, "no box was given for it");
        });
        assert!(EntityReach::effects_of(2).is_empty(), "outside a scope nothing is read");
        assert_eq!(EntityReach::velocity_of(2), None);
        assert_eq!(EntityReach::bounds_of(2), None);
    }

    #[test]
    fn a_velocity_is_set_for_an_item_and_a_mob_but_not_the_unknown() {
        let mut ops = world();
        ops.velocities = vec![(1, [0.0, -0.1, 0.0])];
        with_entities(&mut ops, || {
            assert!(EntityReach::set_velocity(1, [0.1, 0.2, 0.3]), "an item entity's motion is set");
            assert_eq!(EntityReach::velocity_of(1), Some([0.1, 0.2, 0.3]));
            assert!(EntityReach::set_velocity(2, [0.0, 0.5, 0.0]), "a mob's motion is set");
            assert_eq!(EntityReach::velocity_of(2), Some([0.0, 0.5, 0.0]));
            assert!(!EntityReach::set_velocity(99, [0.0, 0.0, 0.0]), "no such entity");
            assert!(!EntityReach::set_velocity(2, [f64::NAN, 0.0, 0.0]), "a motion that is not finite is refused");
            assert_eq!(EntityReach::velocity_of(2), Some([0.0, 0.5, 0.0]), "the refused set changed nothing");
        });
        assert!(!EntityReach::set_velocity(2, [0.0, 0.0, 0.0]), "outside a scope nothing is set");
    }
}
