// PlutoLib's fishing reach, game side: casts, hooks and retrieves for a mod's rod, inside the game's
// own fishing tick. The rod's numbers arrive as plain data through `crate::entry::pluto_fishing_access`
// (the pure fishing API's profile, field for field); the formulas here are that API's, written out
// again because this code is part of the game, not of the mod's crate — the two are kept in step by
// the pure side's tests of the same numbers.
//
// What this does, after the original's `FishingModule` and `CombatFishingHook`:
// - a queued use with no hook out casts: the hook is the game's own `Hook`, built as its rod use
//   builds it, but with the profile's luck and lure and the combat cast's velocity (per axis, the
//   profile's velocity over the look vector's length plus a triangle spread of the inaccuracy).
// - a queued use with the combat hook out retrieves: a grounded grapple pulls the owner first; a
//   hooked entity is collected, hurt and pulled as the combat hook's `pullEntity` does; the catch
//   and the plain cases go through the game's own `retrieve_hook`, loot and all; the rod's wear is
//   reported to the mod, which damages its own tool (the game's `damage_held` would wear the item's
//   own damage, which is not where a mod tool keeps its wear).
// - a queued use with a vanilla hook out reels it in at the vanilla cost.
//
// Reductions, each for a named reason: the pull a hooked entity gets is the combat pull (the game's
// own retrieve is not run for a hooked combat hook, so nothing pulls twice); a collecting hook pulls
// its item strongly instead of the original's `playerTouch`, so the item arcs to the owner and is
// picked up on contact, and the discard-on-failed-pickup list is not applied; the hurt carries no
// attacker (the game's hurt-by-entity call resolves an attacker by entity id, and the hook's owner
// here is the local player, whose server id this code does not hold), so no kill credit or last-hurt
// mark is set, and the launcher-hit modifier hooks do not run; a boat, cart or TNT is pulled but not
// hurt (the hurt call reaches living beings and armour stands); the drill's spin attack has no game
// surface and is not started; the wall-stick of a grapple hook (`onHitBlock`) has no hook state here,
// so a grapple against a wall retrieves as a flying hook, not a grounded one.

use super::{cast_direction, pull_item, Hook, State};
use crate::entry::pluto_fishing_access as access;
use horizon_client::items::ItemStack;
use horizon_core::geometry::Vec3;
use horizon_core::random::RandomSource;

/// Runs at the head of the game's fishing tick: ends a cast whose hook is gone, then acts on one
/// queued rod use.
pub(crate) fn drain(game: &mut super::super::Game<'_>) {
    let Some(provider) = access::provider() else { return };
    if let Some(active) = access::active() {
        if game.hook.as_ref().map(|hook| hook.id) != Some(active.hook_id) {
            access::clear_active();
        }
    }
    let Some((uuid, profile)) = (provider.take_use)() else { return };
    if let Some(active) = access::active() {
        retrieve(game, active.uuid, active.profile);
    } else if game.hook.is_some() {
        let rod = rod_stack(game, &provider);
        let damage = game.retrieve_hook(&rod);
        (provider.note_wear)(uuid, damage);
    } else {
        cast(game, uuid, profile);
    }
}

/// The rod a retrieve is credited to: the held rod if there is one, else the offhand rod, else what
/// is held, else a vanilla rod (the game's retrieve reads the rod only for its hooked criteria).
fn rod_stack(game: &super::super::Game<'_>, provider: &access::Provider) -> ItemStack {
    let is_rod = |stack: &ItemStack| &*stack.item == "minecraft:fishing_rod" || (provider.is_rod)(&stack.item);
    if let Some(held) = game.held() {
        if is_rod(held) {
            return held.clone();
        }
    }
    if let Some(offhand) = &game.offhand {
        if is_rod(offhand) {
            return offhand.clone();
        }
    }
    if let Some(held) = game.held() {
        return held.clone();
    }
    ItemStack::new("minecraft:fishing_rod", 1)
}

/// Casts the combat hook with `profile`, as the game's own rod use casts its hook.
fn cast(game: &mut super::super::Game<'_>, uuid: u128, profile: access::UseProfile) {
    let Some(provider) = access::provider() else { return };
    let Some(player) = game.player.as_ref() else { return };
    let (y_sin, y_cos, dir_y) = cast_direction(game.yaw, game.pitch);
    let eye_y = player.position.y + f64::from(player.dimensions.eye_height);
    let start = Vec3::new(
        player.position.x - f64::from(y_sin) * 0.3,
        eye_y,
        player.position.z - f64::from(y_cos) * 0.3,
    );
    let at = [player.position.x, player.position.y, player.position.z];
    let base = Vec3::new(-f64::from(y_sin), dir_y, -f64::from(y_cos));
    let length = (base.x * base.x + base.y * base.y + base.z * base.z).sqrt();
    let mut random = horizon_client::random_source::entity();
    let spread = 0.03 * f64::from(profile.inaccuracy) * f64::from(profile.inaccuracy);
    let axis = |component: f64, random: &mut horizon_core::random::LegacyRandom| {
        component * (f64::from(profile.velocity) / length + random.triangle(0.5, spread))
    };
    let velocity = Vec3::new(axis(base.x, &mut random), axis(base.y, &mut random), axis(base.z, &mut random));
    game.next_entity += 1;
    let id = game.next_entity;
    game.hook = Some(Hook {
        id,
        position: start,
        previous: start,
        velocity,
        state: State::Flying,
        last_deflected_by: None,
        needs_sync: false,
        tick_count: 0,
        life: 0,
        on_ground: false,
        nibble: 0,
        time_until_lured: 0,
        time_until_hooked: 0,
        fish_angle: 0.0,
        out_of_water: 0,
        open_water: true,
        luck: profile.luck,
        lure: profile.lure,
        medium: None,
        lerp: super::HookLerp::default(),
        random,
    });
    access::set_active(access::Active { uuid, profile, hook_id: id });
    // The combat hook wears its rod on the cast as on a hit (`damageRod`).
    (provider.note_wear)(uuid, 1);
    let pitch = 0.4 / (game.world.random.next_float() * 0.4 + 0.8);
    game.audio.play("minecraft:entity.fishing_bobber.throw", Some(at), 0.5, pitch);
}

/// Retrieves the combat hook: the grapple, the hooked entity and the wear, as the original's
/// retrieve composes them.
fn retrieve(game: &mut super::super::Game<'_>, uuid: u128, profile: access::UseProfile) {
    let Some(provider) = access::provider() else { return };
    let Some(hook) = game.hook.as_ref() else {
        access::clear_active();
        return;
    };
    let grounded = hook.on_ground;
    let hook_pos = hook.position;
    let hooked = match hook.state {
        State::Hooked(id) => Some(id),
        _ => None,
    };
    let at = game.player.as_ref().map(|player| [player.position.x, player.position.y, player.position.z]);
    if grounded && profile.grapple != 0 {
        grapple_pull(game, hook_pos);
    }
    let underlying = if let Some(target) = hooked {
        if let Some(hit) = access::take_hit() {
            if hit.target == target {
                pull_entity(game, &profile, target, f64::from(hit.speed), hook_pos);
            }
        }
        game.hook = None;
        // The game's own retrieve prices a hooked item at 3 and anything else at 5.
        if game.dropped.iter().any(|item| item.id == target) {
            3
        } else {
            5
        }
    } else {
        let rod = rod_stack(game, &provider);
        game.retrieve_hook(&rod)
    };
    let damage = if grounded { underlying.clamp(2, 3) } else { underlying.min(3) };
    (provider.note_wear)(uuid, damage);
    if let Some(at) = at {
        let pitch = 0.4 / (game.world.random.next_float() * 0.4 + 0.8);
        game.audio.play("minecraft:entity.fishing_bobber.retrieve", Some(at), 1.0, pitch);
    }
    access::clear_active();
}

/// What the combat hook does to the entity it hooked, at retrieve (`pullEntity`): collect it if it
/// is an item and the hook collects, hurt it by the impact damage, pull it toward the owner, and —
/// for a drill — pull the owner toward it.
fn pull_entity(
    game: &mut super::super::Game<'_>,
    profile: &access::UseProfile,
    target: u32,
    impact_speed: f64,
    hook_pos: Vec3,
) {
    let Some(player) = game.player.as_ref() else { return };
    let owner = player.position;
    if profile.collecting {
        if let Some(item) = game.dropped.iter_mut().find(|item| item.id == target) {
            pull_item(item, Vec3::new((owner.x - hook_pos.x) * 0.3, (owner.y - hook_pos.y) * 0.3 + 0.1, (owner.z - hook_pos.z) * 0.3));
            return;
        }
    }
    if profile.power > 0.0 {
        // `getDamage`: the impact speed times the power times 15, rounded up to a whole tenth.
        let scaled = (impact_speed * f64::from(profile.power) * 15.0).clamp(0.0, f64::from(i32::MAX));
        let damage = (scaled.ceil() / 10.0) as f32;
        if damage > 0.0 {
            game.native_hurt_entity(game.dim.id(), target, damage, "tconstruct:fishing_hook", None);
        }
    }
    // The pull: a tenth of the way to the owner, plus the knockback bonus.
    let delta = Vec3::new(owner.x - hook_pos.x, owner.y - hook_pos.y, owner.z - hook_pos.z);
    let distance = (delta.x * delta.x + delta.y * delta.y + delta.z * delta.z).sqrt();
    if distance > 0.0 {
        let mut scale = 0.1;
        if profile.knockback > 0.0 {
            scale += f64::from(profile.knockback) * 0.25 / distance;
        }
        let pull = Vec3::new(delta.x * scale, delta.y * scale, delta.z * scale);
        if let Some(mob) = game.mobs.iter_mut().find(|mob| mob.id == target) {
            mob.velocity = mob.velocity.add_vector(pull);
        } else if let Some(item) = game.dropped.iter_mut().find(|item| item.id == target) {
            pull_item(item, pull);
        } else if let Some(tnt) = game.tnts.iter_mut().find(|tnt| tnt.id != 0 && tnt.id == target) {
            tnt.velocity = tnt.velocity.add_vector(pull);
        } else if let Some(stand) = game.armor_stands.iter_mut().find(|stand| stand.id == target) {
            stand.velocity = stand.velocity.add_vector(pull);
        } else if let Some(boat) = game.boats.iter_mut().find(|boat| boat.id == target) {
            boat.velocity = boat.velocity.add_vector(pull);
        } else if let Some(cart) = game.carts.iter_mut().find(|cart| cart.id == target) {
            cart.velocity = cart.velocity.add_vector(pull);
        } else if game.creatures.hook_targets.iter().any(|known| known.id == target) {
            game.creatures.hook_pulls.push((target, pull));
        }
    }
    if profile.grapple == 2 {
        grapple_pull(game, hook_pos);
    }
}

/// Pulls the owner toward the hook (`pullGrapple`): 0.58 / sqrt(distance) of the way, each component
/// clamped as the game's own player push clamps it.
fn grapple_pull(game: &mut super::super::Game<'_>, hook_pos: Vec3) {
    let Some(player) = game.player.as_mut() else { return };
    if player.spectator {
        return;
    }
    let delta = Vec3::new(hook_pos.x - player.position.x, hook_pos.y - player.position.y, hook_pos.z - player.position.z);
    let squared = delta.x * delta.x + delta.y * delta.y + delta.z * delta.z;
    if squared == 0.0 {
        return;
    }
    let scale = 0.58 * squared.powf(-0.25);
    let kicked = Vec3::new(
        (delta.x * scale).clamp(-4.0, 4.0),
        (delta.y * scale).clamp(-4.0, 4.0),
        (delta.z * scale).clamp(-4.0, 4.0),
    );
    player.velocity = player.velocity.add_vector(kicked);
    if kicked.y > 0.0 {
        player.ground.on_ground = false;
    }
}
