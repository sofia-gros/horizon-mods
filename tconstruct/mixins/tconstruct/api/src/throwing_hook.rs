//! Throwing tools and the shuriken tool (SlimeKnights, MIT; see LICENSE): ThrowingModule and
//! ThrownTool, ModifiableShurikenItem and ThrownShuriken.
//!
//! The throw gate is the game's own use machinery, through PlutoLib's per-stack use and its release
//! telling: a throwing tool starts a use with the SPEAR pose and a duration of 72000, and letting go
//! fires the release telling with the ticks the use was held — the charge. Past ten ticks the tool
//! leaves the hand as a summoned arrow (the only summonable body with a thrown trident's physics and
//! a native stick; `flight_hook` flies it, lands its melee hit and brings it back for loyalty). The
//! stack registration here is made after the edible and blocking ones and replaces them for every
//! usable tool item, composing the three per stack as the original's hook order does: a stack that
//! can be eaten is eaten, a blocking stack blocks, a throwing stack is thrown.
//!
//! The shuriken is thrown instantly in the original (no charge): its use is registered with no pose
//! and the shortest hold, so a click's release throws it at once — the release fires the declared
//! `tconstruct:thrown_shuriken` type with the tool's own velocity and power, spends one from the
//! stack, and the release's `used` mark starts the item's cooldown (the content carries the original's
//! 10 ticks as a `use_cooldown` of half a second). The host's own instant throw is not used: it fixes
//! the speed at 1.5 and the damage at the type's registration, where the original's numbers come from
//! the tool's stats.
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

/// Whether the stack is a whole Tinkers tool whose modifiers carry the blocking module.
fn blocking(stack: &StackData) -> bool {
    with_held_tool(stack, |env, tool| tool_stack::blocking(&tool.custom_data, env)).unwrap_or(false)
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
fn charge(ticks_used: u32, drawtime: f32) -> f32 {
    let charge = ticks_used as f32 / drawtime.max(1.0);
    ((charge * charge + charge * 2.0) / 3.0).min(1.0)
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
    let velocity = stat(stack, Stat::Velocity);
    let power = f64::from(charge * velocity * 2.0);
    let motion = [release.look[0] * power, release.look[1] * power, release.look[2] * power];
    let world = release.world();
    let spawn = ProjectileSpawn::new(Identifier::parse("minecraft:arrow").expect("valid kind"), release.eye, motion)
        .owner(release.player.uuid);
    let Some(id) = spawn_projectile(&world, &spawn) else { return };
    let mut carried = stack.clone();
    carried.count = 1;
    let profile = flight_hook::profile_of(stack);
    flight_hook::bind(&release.dimension, id, flight_hook::thrown_flight(profile, carried, velocity));
    if !release.creative {
        if let Some(edit) = consume_edit(release) { release.edits.borrow_mut().push(edit); }
    }
    release.set_used();
}

/// Fires one shuriken of `stack` from `eye` along `look` and binds its flight (the spawn half of
/// ModifiableShurikenItem.use); false when the thrown type could not be summoned.
fn spawn_shuriken(world: &World, dimension: &str, eye: [f64; 3], look: [f64; 3], owner: u128, stack: &StackData) -> bool {
    let velocity = f64::from(stat(stack, Stat::Velocity));
    let motion = [look[0] * velocity, look[1] * velocity, look[2] * velocity];
    let thrown = Identifier::parse(&format!("tconstruct:{SHURIKEN_ENTITY}")).expect("valid thrown type");
    let Some(id) = pluto_lib::spawn_thrown(world, &thrown, eye, motion, Some(owner), pluto_lib::NbtCompound::new()) else { return false };
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
    spawn_shuriken(&World::new(dimension), dimension, eye, look, player.uuid, &stack)
}

/// Throws one shuriken (ModifiableShurikenItem.use: instant, the stack shrinks by one).
fn throw_shuriken(release: &ReleaseUsing) {
    let stack = &release.stack;
    if !spawn_shuriken(&release.world(), &release.dimension, release.eye, release.look, release.player.uuid, stack) { return; }
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
/// usable tool items, composing all three).
pub(crate) fn register() {
    for path in edible_hook::USE_ITEMS {
        let Ok(item) = Identifier::new("tconstruct", path) else { continue };
        ItemUseRegistry::register_stack(item, StackUse::new(UseAction::None)
            .can_use(|stack| eatable(stack) || blocking(stack) || throwing(stack) || sling(stack) || fireball(stack))
            .action_for(|stack| {
                if eatable(stack) { return Some(UseAction::Eat); }
                if blocking(stack) { return Some(UseAction::Block); }
                if sling(stack) { return Some(UseAction::Bow); }
                if throwing(stack) { return Some(UseAction::Spear); }
                fireball(stack).then_some(UseAction::None)
            })
            .use_time(|stack| {
                if eatable(stack) { return edible_hook::eating(stack).map_or(0, |(_, duration)| edible::use_ticks(duration)); }
                if blocking(stack) || throwing(stack) || sling(stack) || fireball(stack) { THROW_TICKS } else { 0 }
            }));
    }
    if let Ok(item) = Identifier::new("tconstruct", SHURIKEN_ITEM) {
        ItemUseRegistry::register_stack(item, StackUse::new(UseAction::None)
            .can_use(|stack| shuriken(stack))
            .use_time(|_| THROW_TICKS));
    }
    if let (Ok(entity), Ok(item)) = (Identifier::new("tconstruct", SHURIKEN_ENTITY), Identifier::new("tconstruct", SHURIKEN_ITEM)) {
        let _ = ThrownTypeRegistry::register(ThrownType::new(entity, item, 0.0).expect("a valid thrown type"));
    }
    ItemEvents::RELEASE_USING.register(on_release);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_charge_curve_is_the_bows() {
        assert_eq!(charge(0, 20.0), 0.0);
        assert!((charge(10, 20.0) - 0.4167).abs() < 0.001, "half draw: (0.25 + 1) / 3");
        assert_eq!(charge(20, 20.0), 1.0);
        assert_eq!(charge(60, 20.0), 1.0, "capped at a full charge");
    }
}
