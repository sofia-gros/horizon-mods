//! Throwing tools and the shuriken tool (SlimeKnights, MIT; see LICENSE): ThrowingModule and
//! ThrownTool, ModifiableShurikenItem and ThrownShuriken.
//!
//! The throw gate is the game's own use machinery, through PlutoLib's per-stack use and its release
//! telling: a throwing tool starts a use with the SPEAR pose and a duration of 72000, and letting go
//! fires the release telling with the ticks the use was held — the charge. Past ten ticks the tool
//! leaves the hand as a summoned arrow (the only summonable body with a thrown trident's physics and
//! a native stick; `flight_hook` flies it, lands its melee hit and brings it back for loyalty). The
//! stack registration here is made after the edible and blocking ones and replaces them for every
//! usable tool item, composing the uses per stack as the original's hook order does: a stack that
//! can be eaten is eaten, a blocking stack blocks, a brush stack brushes (for the brush module's
//! 200 ticks, in `brush_hook`), a throwing stack is thrown.
//!
//! The shuriken is thrown instantly in the original (no charge): its use is registered with no pose
//! and the shortest hold, so a click's release throws it at once — the release fires the declared
//! `tconstruct:thrown_shuriken` type with the tool's own velocity and power, spends one from the
//! stack, and the release's `used` mark starts the item's cooldown (the content carries the original's
//! 10 ticks as a `use_cooldown` of half a second). The host's own instant throw is not used: it fixes
//! the speed at 1.5 and the damage at the type's registration, where the original's numbers come from
//! the tool's stats.
//!
//! The throws play their canon sounds: the shuriken's is `tconstruct:throw.shuriken`
//! (ModifiableShurikenItem.use, NEUTRAL, 0.5, pitch `0.4F / (random * 0.4F + 0.8F)`), played here
//! because the spawn itself is silent; a throwing tool's is the vanilla trident throw
//! (ThrowingModule.onStoppedUsing, PLAYERS, 1, 1). Both go through `World::play_sound`, which plays
//! in the block category where Java plays NEUTRAL/PLAYERS.
use crate::{break_speed_hook::with_held_tool, edible, edible_hook, flight_hook, tool_stack, tool_stats::Stat};
use pluto_lib::{
    CarriedEdit, CarriedSlot, Identifier, ItemEvents, ItemSlot, ItemUseRegistry, Nbt, ProjectileSpawn,
    ReleaseUsing, ServerPlayer, StackData, StackSlot, StackUse, ThrownType, ThrownTypeRegistry, UseAction, World, spawn_projectile,
};

/// ThrowingModule.getUseDuration.
const THROW_TICKS: i32 = 72000;
/// The thrown shuriken entity type (TinkerTools.thrownShuriken), declared in the content.
const SHURIKEN_ENTITY: &str = "thrown_shuriken";
/// The shuriken item.
const SHURIKEN_ITEM: &str = "shuriken";
/// The shuriken throw sound (ModifiableShurikenItem.use: Sounds.SHURIKEN_THROW, NEUTRAL, 0.5).
const SHURIKEN_THROW_SOUND: &str = "tconstruct:throw.shuriken";

/// Whether the stack is a whole Tinkers tool whose modifiers carry the blocking module.
fn blocking(stack: &StackData) -> bool {
    with_held_tool(stack, |env, tool| tool_stack::blocking(&tool.custom_data, env)).unwrap_or(false)
}

/// Whether the stack is a whole Tinkers tool whose modifiers carry the brush module
/// (BrushModule: the use brushes, for its 200 ticks, in `brush_hook`).
fn brush(stack: &StackData) -> bool {
    with_held_tool(stack, |env, tool| tool_stack::brush(&tool.custom_data, env)).unwrap_or(false)
}

/// Whether the stack can start being eaten, as the edible registration asks.
fn eatable(stack: &StackData) -> bool {
    edible_hook::eating(stack).is_some_and(|(hunger, _)| edible::can_start(hunger, false))
}

/// Whether the stack is a throwing tool (ThrowingModule), whole and unbroken.
fn throwing(stack: &StackData) -> bool {
    with_held_tool(stack, |env, tool| tool_stack::throwing(&tool.custom_data, env)).unwrap_or(false)
}

/// Whether the stack is a whole Tinkers tool carrying a sling module (SlingModule: the
/// use charges like a bow and the release spends the charge, in `sling_hook`).
fn sling(stack: &StackData) -> bool {
    with_held_tool(stack, |env, tool| !tool_stack::sling_modules(&tool.custom_data, env).is_empty()).unwrap_or(false)
}

/// Whether the stack is a whole Tinkers tool carrying a fireball module (FireballModule:
/// the use fires a fireball, in `fireball_hook`; the long use time here means a click
/// and a hold both end in the release that fires it).
fn fireball(stack: &StackData) -> bool {
    with_held_tool(stack, |env, tool| tool_stack::fireball_module(&tool.custom_data, env).is_some()).unwrap_or(false)
}

/// Whether the stack is the shuriken tool, whole enough to throw (its data initialized, not broken).
fn shuriken(stack: &StackData) -> bool {
    stack.item == format!("tconstruct:{SHURIKEN_ITEM}")
        && with_held_tool(stack, |_env, tool| !tool_stack::is_broken(&tool.custom_data)).unwrap_or(false)
}

/// A tool stat of the stack, 0 when the stack is not a whole tool.
fn stat(stack: &StackData, stat: Stat) -> f32 {
    with_held_tool(stack, |_env, tool| tool_stack::stat(&tool.custom_data, stat)).unwrap_or(0.0)
}

/// A tool's conditional velocity for a holder at height `y` in `dimension`, for the dwarven velocity half
/// (a thrown tool's holder is not digging, so no dig reports the level's build range — the host's own
/// range for the dimension answers, the vanilla table when it has not); None for the height
/// leaves the dwarven arm at the stored value. 0 when the stack is not a whole tool.
fn conditional_velocity(stack: &StackData, y: Option<f64>, dimension: &str) -> f32 {
    let holder = pluto_lib::EntityState { y, ..Default::default() };
    let entities = crate::entity_facts::Entities { attacker: Some(&holder), target: None };
    let build_height = Some(crate::mining_speed::level_build_height(dimension));
    with_held_tool(stack, |env, tool| tool_stack::conditional_stat_at(&tool.custom_data, env, Stat::Velocity, &|_| false, &entities, build_height)).unwrap_or(0.0)
}

/// The hand a released stack was in, and the edit that empties it (a creative player keeps the tool:
/// the original leaves it in the hand and marks the thrown copy creative-only pickup, which the
/// flight's drop does not distinguish — a creative throw duplicates the tool where it lands, a
/// documented reduction).
fn consume_edit(release: &ReleaseUsing) -> Option<CarriedEdit> {
    let same = |slot: &ItemSlot| slot.item.to_string() == release.stack.item && slot.custom_data == release.stack.custom_data;
    if let Some(hand) = &release.main_hand {
        if same(hand) {
            let mut emptied = hand.clone();
            emptied.count = 0;
            return Some(CarriedEdit { slot: CarriedSlot::Equipment(StackSlot::MainHand), stack: emptied });
        }
    }
    if let Some(hand) = &release.off_hand {
        if same(hand) {
            let mut emptied = hand.clone();
            emptied.count = 0;
            return Some(CarriedEdit { slot: CarriedSlot::Equipment(StackSlot::OffHand), stack: emptied });
        }
    }
    None
}

/// ThrowingModule.getToolCharge: the vanilla bow curve over the draw time.
pub(crate) fn charge(ticks_used: u32, drawtime: f32) -> f32 {
    let charge = ticks_used as f32 / drawtime.max(1.0);
    ((charge * charge + charge * 2.0) / 3.0).min(1.0)
}

/// The pitch of Java's throw sounds (ShurikenItem.use, GlowBallItem.use, EFLNItem.use,
/// ModifiableShurikenItem.use): `0.4F / (random * 0.4F + 0.8F)` for a draw in [0, 1).
fn throw_pitch(random: f32) -> f32 {
    0.4 / (random * 0.4 + 0.8)
}

/// Plays the shuriken throw sound for a thrown shuriken: the spawn itself is silent, so the throw
/// plays it at the eye, pitched from the level's own random stream (the mean pitch when the stream
/// is not handed over). The host's own throw sound is the block category through `World::play_sound`,
/// where Java plays NEUTRAL; the event, volume and pitch match.
fn play_shuriken_throw(world: &World, at: [f64; 3]) {
    world.play_sound(SHURIKEN_THROW_SOUND, at, 0.5, throw_pitch(world.random_float().unwrap_or(0.5)));
}

/// Plays the trident throw for a thrown tool: the arrow summon is silent, so the throw plays it at
/// the eye. Same block-category note as `play_shuriken_throw`, where Java plays PLAYERS.
fn play_thrown_tool_throw(world: &World, at: [f64; 3]) {
    world.play_sound("minecraft:item.trident.throw", at, 1.0, 1.0);
}

/// Throws the tool (ThrowingModule.onStoppedUsing past ten ticks).
fn throw_tool(release: &ReleaseUsing) {
    if release.ticks_used <= 10 { return; }
    let stack = &release.stack;
    // Draw time from the draw speed, with the attack speed for a melee weapon (a tool that deals
    // melee damage; the item tag the original reads is not in the release's reach).
    let mut speed = stat(stack, Stat::DrawSpeed);
    if stat(stack, Stat::AttackDamage) > 0.0 { speed *= stat(stack, Stat::AttackSpeed); }
    let drawtime = (20.0 / speed.max(f32::MIN_POSITIVE)).ceil();
    let charge = charge(release.ticks_used, drawtime);
    let velocity = conditional_velocity(stack, Some(release.position[1]), &release.dimension);
    let power = f64::from(charge * velocity * 2.0);
    let motion = [release.look[0] * power, release.look[1] * power, release.look[2] * power];
    let world = release.world();
    let spawn = ProjectileSpawn::new(Identifier::parse("minecraft:arrow").expect("valid kind"), release.eye, motion)
        .owner(release.player.uuid);
    let Some(id) = spawn_projectile(&world, &spawn) else { return };
    play_thrown_tool_throw(&world, release.eye);
    let mut carried = stack.clone();
    carried.count = 1;
    let profile = flight_hook::profile_of(stack);
    flight_hook::bind(&release.dimension, id, flight_hook::thrown_flight(profile, carried, velocity, charge, !release.creative));
    if !release.creative {
        if let Some(edit) = consume_edit(release) { release.edits.borrow_mut().push(edit); }
    }
    release.set_used();
}

/// Fires one shuriken of `stack` from `eye` along `look` and binds its flight (the spawn half of
/// ModifiableShurikenItem.use); false when the thrown type could not be summoned. `holder_y` is the
/// holder's height for the dwarven velocity half; None when the caller does not know it (a stored
/// stack thrown from a worn piece's inventory), which leaves the dwarven arm at the stored value.
fn spawn_shuriken(world: &World, dimension: &str, eye: [f64; 3], look: [f64; 3], owner: u128, stack: &StackData, holder_y: Option<f64>) -> bool {
    let velocity = f64::from(conditional_velocity(stack, holder_y, dimension));
    let motion = [look[0] * velocity, look[1] * velocity, look[2] * velocity];
    let thrown = Identifier::parse(&format!("tconstruct:{SHURIKEN_ENTITY}")).expect("valid thrown type");
    let Some(id) = pluto_lib::spawn_thrown(world, &thrown, eye, motion, Some(owner), pluto_lib::NbtCompound::new()) else { return false };
    play_shuriken_throw(world, eye);
    let mut carried = stack.clone();
    carried.count = 1;
    // The knockback a shot's modules added rides the fired stack's persistent data, when present.
    let knockback = carried.custom_data.get(tool_stack::TAG_PERSISTENT)
        .and_then(|data| if let Nbt::Compound(data) = data { data.get("tconstruct:knockback").and_then(Nbt::as_f64) } else { None })
        .unwrap_or(0.0) as f32;
    let profile = flight_hook::profile_of(stack);
    flight_hook::bind(dimension, id, flight_hook::shuriken_flight(profile, carried, stat(stack, Stat::ProjectileDamage), knockback));
    true
}

/// Throws one shuriken from a stored stack (SleevesModule's use of the selected stack in a worn
/// piece's inventory): the same spawn the release fires, aimed by the caller's eye and look. The
/// stack is in no hand, so there is no release to spend from or to mark used: the caller stores
/// the spent remainder itself, and the cooldown the release's `used` mark starts has no surface
/// here (see sleeves_hook's notes). False when the stack is not this port's shuriken tool, whole
/// and unbroken, or the thrown type could not be summoned.
pub(crate) fn throw_stored_shuriken(player: &ServerPlayer, dimension: &str, eye: [f64; 3], look: [f64; 3], thrown: &ItemSlot) -> bool {
    let stack = StackData {
        item: thrown.item.to_string(), count: thrown.count, damage: thrown.damage, max_damage: thrown.max_damage,
        custom_data: thrown.custom_data.clone(), custom_model_data: thrown.custom_model_data.clone(),
        enchantments: thrown.enchantments.clone().map(|map| map.into_iter().collect()).unwrap_or_default(),
    };
    if !shuriken(&stack) { return false; }
    spawn_shuriken(&World::new(dimension), dimension, eye, look, player.uuid, &stack, None)
}

/// Throws one shuriken (ModifiableShurikenItem.use: instant, the stack shrinks by one).
fn throw_shuriken(release: &ReleaseUsing) {
    let stack = &release.stack;
    if !spawn_shuriken(&release.world(), &release.dimension, release.eye, release.look, release.player.uuid, stack, Some(release.position[1])) { return; }
    if !release.creative {
        // Spend one from the hand the shuriken was thrown from.
        let same = |slot: &ItemSlot| slot.item.to_string() == release.stack.item && slot.custom_data == release.stack.custom_data;
        for (hand, slot_kind) in [(&release.main_hand, StackSlot::MainHand), (&release.off_hand, StackSlot::OffHand)] {
            if let Some(hand) = hand {
                if same(hand) {
                    let mut spent = hand.clone();
                    spent.count = spent.count.saturating_sub(1);
                    release.edits.borrow_mut().push(CarriedEdit { slot: CarriedSlot::Equipment(slot_kind), stack: spent });
                    break;
                }
            }
        }
    }
    release.set_used();
}

/// The release pass: a throwing tool past its charge gate, or a shuriken's instant throw.
fn on_release(release: &ReleaseUsing) {
    if release.stack.item == format!("tconstruct:{SHURIKEN_ITEM}") {
        if shuriken(&release.stack) { throw_shuriken(release); }
        return;
    }
    if throwing(&release.stack) { throw_tool(release); }
}

/// Registers the uses, the thrown shuriken type and the release callback. Called once from the
/// product's hooks, after the edible and blocking hooks (this registration replaces theirs for the
/// usable tool items, composing all of them: eat, block, brush, sling, throw, fireball, and the tank drinks, spits and bucket pickup).
pub(crate) fn register() {
    for path in edible_hook::USE_ITEMS {
        let Ok(item) = Identifier::new("tconstruct", path) else { continue };
        ItemUseRegistry::register_stack(item, StackUse::new(UseAction::None)
            .can_use(|stack| eatable(stack) || blocking(stack) || brush(stack) || throwing(stack) || sling(stack) || fireball(stack) || crate::tank_hook::slurpable(stack).is_some() || crate::tank_hook::spittable(stack) || crate::tank_hook::bucket_pickupable(stack))
            .action_for(|stack| {
                if eatable(stack) { return Some(UseAction::Eat); }
                if blocking(stack) { return Some(UseAction::Block); }
                if brush(stack) { return Some(UseAction::Brush); }
                if sling(stack) { return Some(UseAction::Bow); }
                if throwing(stack) { return Some(UseAction::Spear); }
                if fireball(stack) { return Some(UseAction::None); }
                if crate::tank_hook::slurpable(stack).is_some() { return Some(UseAction::Drink); }
                if crate::tank_hook::spittable(stack) { return Some(UseAction::Bow); }
                if crate::tank_hook::bucket_pickupable(stack) { return Some(UseAction::None); }
                None
            })
            .use_time(|stack| {
                if eatable(stack) { return edible_hook::eating(stack).map_or(0, |(_, duration)| edible::use_ticks(duration)); }
                if blocking(stack) { return THROW_TICKS; }
                if brush(stack) { return crate::brush::USE_DURATION; }
                if let Some(duration) = crate::tank_hook::slurpable(stack) { return duration; }
                if throwing(stack) || sling(stack) || fireball(stack) || crate::tank_hook::spittable(stack) || crate::tank_hook::bucket_pickupable(stack) { THROW_TICKS } else { 0 }
            }));
    }
    if let Ok(item) = Identifier::new("tconstruct", SHURIKEN_ITEM) {
        ItemUseRegistry::register_stack(item, StackUse::new(UseAction::None)
            .can_use(|stack| shuriken(stack))
            .use_time(|_| THROW_TICKS));
    }
    if let (Ok(entity), Ok(item), Ok(sound)) = (
        Identifier::new("tconstruct", SHURIKEN_ENTITY),
        Identifier::new("tconstruct", SHURIKEN_ITEM),
        Identifier::parse(SHURIKEN_THROW_SOUND),
    ) {
        let _ = ThrownTypeRegistry::register(
            ThrownType::new(entity, item, 0.0).expect("a valid thrown type").throw_sound(sound),
        );
    }
    ItemEvents::RELEASE_USING.register(on_release);
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::{MemoryWorldOps, with_world};

    #[test]
    fn the_charge_curve_is_the_bows() {
        assert_eq!(charge(0, 20.0), 0.0);
        assert!((charge(10, 20.0) - 0.4167).abs() < 0.001, "half draw: (0.25 + 1) / 3");
        assert_eq!(charge(20, 20.0), 1.0);
        assert_eq!(charge(60, 20.0), 1.0, "capped at a full charge");
    }

    #[test]
    fn the_throw_pitch_is_javas() {
        assert_eq!(throw_pitch(0.0), 0.5);
        assert_eq!(throw_pitch(0.5), 0.4);
        assert!((throw_pitch(1.0) - (0.4 / 1.2)).abs() < 1e-6);
    }

    #[test]
    fn a_thrown_shuriken_and_a_thrown_tool_play_their_canon_sounds() {
        let mut ops = MemoryWorldOps::default();
        with_world(&mut ops, || {
            let world = World::new("minecraft:overworld");
            play_shuriken_throw(&world, [1.0, 64.0, 1.0]);
            play_thrown_tool_throw(&world, [2.0, 64.0, 2.0]);
        });
        assert_eq!(
            ops.sounds,
            [
                ("tconstruct:throw.shuriken".to_string(), [1.0, 64.0, 1.0]),
                ("minecraft:item.trident.throw".to_string(), [2.0, 64.0, 2.0]),
            ]
        );
    }
}
