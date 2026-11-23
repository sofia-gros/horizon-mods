//! Fishing with a mod's own rod (Minecraft: `FishingHook` thrown by an item other than the vanilla rod).
//!
//! The game's fishing is built around its own hook: its rod use is gated to the vanilla rod item, and
//! its hook is a private structure no event names. PlutoLib therefore carries a mod rod's cast the way
//! it carries its other woven reaches: a callback here computes the rod's [`FishingProfile`] from the
//! stack used, [`Fishing::use_rod`] queues the use, and the game's own fishing tick — reached by a
//! small woven transport — casts, hooks and retrieves with that profile, reporting the rod's wear
//! back through [`Fishing::take_wear`]. The numbers are the original combat hook's (`CombatFishingHook`): a cast flies the
//! profile's velocity with its inaccuracy spread, a hooked entity is hurt by the impact-speed damage
//! and pulled at retrieve, and a grapple hook pulls its owner instead.
//!
//! The reach is the local player's: the game's fishing simulation is single-hook and local (a guest's
//! hook is a remote replica the host does not simulate), so the transport acts on a queued use for the
//! local player's hook, whichever player queued it — in a LAN world a guest's rod use can therefore be
//! the one acted on, the same local-only shape as the game's own fishing. What the transport cannot do
//! is listed where it is decided: the vanilla catch at retrieve is the game's own, and the drill's spin
//! attack has no host surface.

use super::base::Identifier;
use super::player_api::{ServerPlayer, StackData};
use std::collections::BTreeMap;
use std::sync::{Mutex, RwLock};

/// How a grapple hook behaves (the original's `GrappleType`): not a grapple, a dash (the owner is
/// pulled when the hook lands or catches), or a drill (also when it hooks an entity).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Grapple {
    /// No grapple: the hook only fishes and fights.
    #[default]
    None,
    /// The owner is pulled toward the hook when it lands.
    Dash,
    /// The owner is pulled toward the hook when it lands and when it hooks an entity.
    Drill,
}

/// The numbers a rod casts with, computed from the stack used (the original's `FishingModule` reads
/// them from the tool's conditional stats and actions).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FishingProfile {
    /// The luck of the cast (the SEA_LUCK stat, as an integer, plus any luck the rod's own effects add).
    pub luck: i32,
    /// The lure of the cast (the LURE stat, as an integer).
    pub lure: i32,
    /// How fast the hook flies (the VELOCITY stat).
    pub velocity: f32,
    /// The cast's spread (from the ACCURACY stat: `3 * (1 / accuracy - 1)`).
    pub inaccuracy: f32,
    /// The hook's power: a hooked entity is hurt by the impact-speed damage scaled by this (the
    /// PROJECTILE_DAMAGE stat). 0 fishes without fighting.
    pub power: f32,
    /// The pull bonus on a hooked entity (the rod's knockback): the pull's scale grows by
    /// `0.25 * knockback / distance`.
    pub knockback: f32,
    /// Whether the hook grapples, and how.
    pub grapple: Grapple,
    /// Whether the hook collects what it touches (items fly to the owner at retrieve).
    pub collecting: bool,
}

impl Default for FishingProfile {
    /// A plain rod: no luck or lure, velocity 1, no spread, no power, no grapple.
    fn default() -> Self {
        FishingProfile { luck: 0, lure: 0, velocity: 1.0, inaccuracy: 0.0, power: 0.0, knockback: 0.0, grapple: Grapple::None, collecting: false }
    }
}

/// The damage a hooked entity takes at retrieve (the original's `CombatFishingHook.getDamage`): the
/// impact speed times the power times 15, rounded up to a whole tenth. The clamp's top is the integer
/// maximum, as the original clamps there.
pub fn combat_damage(impact_speed: f64, power: f32) -> f32 {
    let scaled = (impact_speed * f64::from(power) * 15.0).clamp(0.0, f64::from(i32::MAX));
    (scaled.ceil() / 10.0) as f32
}

/// The velocity a hooked entity gains at retrieve (the original's pull in `pullEntity`): toward the
/// owner, a tenth of the way, plus the knockback bonus `0.25 * knockback / distance` when the rod has
/// knockback. A hook at the owner's feet pulls nothing.
pub fn pull_delta(owner: [f64; 3], hook: [f64; 3], knockback: f32) -> [f64; 3] {
    let delta = [owner[0] - hook[0], owner[1] - hook[1], owner[2] - hook[2]];
    let distance = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
    if distance == 0.0 {
        return [0.0; 3];
    }
    let mut scale = 0.1;
    if knockback > 0.0 {
        scale += f64::from(knockback) * 0.25 / distance;
    }
    [delta[0] * scale, delta[1] * scale, delta[2] * scale]
}

/// The velocity the owner gains from a grapple (the original's `pullGrapple`): toward the hook, scaled
/// by `0.58 / sqrt(distance)`. A hook at the owner's feet pulls nothing.
pub fn grapple_delta(owner: [f64; 3], hook: [f64; 3]) -> [f64; 3] {
    let delta = [hook[0] - owner[0], hook[1] - owner[1], hook[2] - owner[2]];
    let squared = delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2];
    if squared == 0.0 {
        return [0.0; 3];
    }
    let scale = 0.58 * squared.powf(-0.25);
    [delta[0] * scale, delta[1] * scale, delta[2] * scale]
}

/// The rod wear a retrieve costs (the original's `CombatFishingHook.retrieve` over the vanilla
/// retrieve's own number): a grounded hook costs at least 2 and at most 3; anything else costs at
/// most 3 (a mob hook's 5 becomes 3, a catch's 1 stays 1, a plain reel's 0 stays 0).
pub fn retrieve_damage(underlying: u32, grounded: bool) -> u32 {
    if grounded {
        underlying.clamp(2, 3)
    } else {
        underlying.min(3)
    }
}

type CastCallback = dyn Fn(&StackData) -> Option<FishingProfile> + Send + Sync;

static CASTS: RwLock<BTreeMap<String, Box<CastCallback>>> = RwLock::new(BTreeMap::new());
static RODS: RwLock<Vec<String>> = RwLock::new(Vec::new());

/// The registry of mod fishing rods: which items cast with a computed profile, and which items count
/// as rods at all (a hook stays alive only while its owner holds one — the game's own check names the
/// vanilla rod, and the transport asks this registry for the rest).
pub struct FishingRegistry;

impl FishingRegistry {
    /// Registers the cast profile computation for `item`: when the item is used, `cast` reads the stack
    /// and answers the profile to cast with, or None to leave the use to the game.
    pub fn register(item: Identifier, cast: impl Fn(&StackData) -> Option<FishingProfile> + Send + Sync + 'static) {
        if let Ok(mut casts) = CASTS.write() {
            casts.insert(item.to_string(), Box::new(cast));
        }
        Self::register_rod(item);
    }

    /// Names `item` a fishing rod for the hook's lifetime check, without a cast computation of its own.
    pub fn register_rod(item: Identifier) {
        if let Ok(mut rods) = RODS.write() {
            let item = item.to_string();
            if !rods.contains(&item) {
                rods.push(item);
            }
        }
    }

    /// Whether `item` is a registered rod.
    pub fn is_rod(item: &str) -> bool {
        RODS.read().map(|rods| rods.iter().any(|rod| rod == item)).unwrap_or(false)
    }

    /// The profile `stack` casts with, when its item registered a computation and the computation
    /// answers one.
    pub fn profile(stack: &StackData) -> Option<FishingProfile> {
        CASTS.read().ok()?.get(&stack.item)?(stack)
    }
}

static USE_QUEUE: Mutex<Option<(u128, FishingProfile)>> = Mutex::new(None);
static WEAR: Mutex<BTreeMap<u128, u32>> = Mutex::new(BTreeMap::new());

/// Fishing with a mod's rod: the calls a mod makes. Register the rod with [`FishingRegistry`], answer
/// a use with [`Fishing::use_rod`], and pay the rod's wear with [`Fishing::take_wear`].
pub struct Fishing;

impl Fishing {
    /// Queues a use of a fishing rod by `player` with `profile`: the game's fishing tick casts with
    /// the profile when no hook of this rod is out, and retrieves when one is (the original's
    /// `FishingModule` decides the same way, by whether the player is fishing). One queue slot: a
    /// second use before the tick replaces the first, as two uses in one tick would land on the same
    /// hook state anyway.
    pub fn use_rod(player: &ServerPlayer, profile: FishingProfile) {
        if let Ok(mut queue) = USE_QUEUE.lock() {
            *queue = Some((player.uuid, profile));
        }
    }

    /// Takes the queued use, if any. The transport calls this; a mod calls [`Fishing::use_rod`].
    pub fn take_use() -> Option<(u128, FishingProfile)> {
        USE_QUEUE.lock().ok()?.take()
    }

    /// Records rod wear the game side reports (a retrieve's cost, and the combat hook's wear of 1 on
    /// a cast and on a hit). The transport calls this; a mod calls [`Fishing::take_wear`].
    pub fn note_wear(uuid: u128, amount: u32) {
        if amount == 0 {
            return;
        }
        if let Ok(mut wear) = WEAR.lock() {
            *wear.entry(uuid).or_insert(0) += amount;
        }
    }

    /// The rod wear waiting for `player`, drained: the total the game side has reported since the
    /// last call. A mod applies it to the rod (its own damage system) at its next use of the rod, so
    /// the wear lands at most one use late.
    pub fn take_wear(player: &ServerPlayer) -> u32 {
        WEAR.lock().map(|mut wear| wear.remove(&player.uuid).unwrap_or(0)).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combat_damage_is_the_impact_speed_scaled_and_rounded_up_to_a_tenth() {
        // A hook at speed 2 with power 2: 2 * 2 * 15 = 60 -> 6.0.
        assert_eq!(combat_damage(2.0, 2.0), 6.0);
        // A slow hook: 0.5 * 1 * 15 = 7.5 -> ceil 8 -> 0.8.
        assert_eq!(combat_damage(0.5, 1.0), 0.8);
        // No power, no damage; a still hook hurts nothing.
        assert_eq!(combat_damage(3.0, 0.0), 0.0);
        assert_eq!(combat_damage(0.0, 2.0), 0.0);
    }

    #[test]
    fn a_pull_is_a_tenth_of_the_way_plus_the_knockback_bonus() {
        let owner = [0.0, 64.0, 0.0];
        let hook = [10.0, 64.0, 0.0];
        assert_eq!(pull_delta(owner, hook, 0.0), [-1.0, 0.0, 0.0], "a tenth of the 10 blocks");
        // Knockback 4 at distance 10: scale 0.1 + 0.25 * 4 / 10 = 0.2.
        assert_eq!(pull_delta(owner, hook, 4.0), [-2.0, 0.0, 0.0]);
        assert_eq!(pull_delta(owner, owner, 4.0), [0.0; 3], "no distance, no pull");
    }

    #[test]
    fn a_grapple_pull_strengthens_as_the_hook_lands_closer() {
        let owner = [0.0, 64.0, 0.0];
        // 16 blocks away: scale 0.58 / sqrt(16) = 0.145, delta 16 * 0.145 = 2.32.
        let far = grapple_delta(owner, [16.0, 64.0, 0.0]);
        assert!((far[0] - 2.32).abs() < 1e-9, "{far:?}");
        // 4 blocks away: scale 0.58 / 2 = 0.29, delta 1.16 — a gentler total, a stronger rate.
        let near = grapple_delta(owner, [4.0, 64.0, 0.0]);
        assert!((near[0] - 1.16).abs() < 1e-9, "{near:?}");
        assert_eq!(grapple_delta(owner, owner), [0.0; 3]);
    }

    #[test]
    fn retrieve_damage_clamps_as_the_original_does() {
        assert_eq!(retrieve_damage(2, true), 2, "a grounded plain retrieve");
        assert_eq!(retrieve_damage(5, true), 3, "a grounded mob hook is capped");
        assert_eq!(retrieve_damage(0, true), 2, "a grounded hook costs at least 2");
        assert_eq!(retrieve_damage(5, false), 3, "a mob hook's 5 becomes 3");
        assert_eq!(retrieve_damage(1, false), 1, "a catch stays 1");
        assert_eq!(retrieve_damage(0, false), 0, "a plain reel stays free");
    }

    #[test]
    fn a_registered_rod_answers_its_profile_and_counts_as_a_rod() {
        let item = Identifier::parse("example:war_rod").unwrap();
        FishingRegistry::register(item, |stack| (stack.count > 0).then_some(FishingProfile { power: 2.0, grapple: Grapple::Drill, ..FishingProfile::default() }));
        assert!(FishingRegistry::is_rod("example:war_rod"));
        assert!(!FishingRegistry::is_rod("example:other"));
        let stack = StackData { item: "example:war_rod".into(), count: 1, ..StackData::default() };
        let profile = FishingRegistry::profile(&stack).expect("the profile answers");
        assert_eq!((profile.power, profile.grapple), (2.0, Grapple::Drill));
        assert_eq!(FishingRegistry::profile(&StackData::default()), None);
    }

    #[test]
    fn a_use_queues_and_wear_accumulates_until_taken() {
        let player = ServerPlayer { uuid: 99, name: "Alex".into() };
        Fishing::use_rod(&player, FishingProfile::default());
        assert_eq!(Fishing::take_use(), Some((99, FishingProfile::default())));
        assert_eq!(Fishing::take_use(), None, "the queue is drained");
        Fishing::note_wear(99, 1);
        Fishing::note_wear(99, 3);
        Fishing::note_wear(7, 5);
        assert_eq!(Fishing::take_wear(&player), 4, "the cast's 1 and the retrieve's 3");
        assert_eq!(Fishing::take_wear(&player), 0, "drained");
    }
}
