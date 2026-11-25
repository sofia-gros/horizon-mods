//! The tank modules' in-game consumers: the behaviours of TankModule and the fluid modules of
//! `slimeknights.tconstruct.tools.modules` (SlimeKnights, MIT; see LICENSE) that work through a
//! tool's own tank, wired to the game's tellings. The modules' logic over the tool's data is
//! `tool_tank`; what lives here is the wiring the §4 audit found missing:
//!
//! - **SpillingModule** (MELEE_HIT, MONSTER_MELEE_HIT, LAUNCHER_HIT): a hit applies the tank
//!   fluid's entity effects to the target. The melee half runs on the dealt telling
//!   (`on_weapon_hit_dealt`), the weapon hit heard again after its damage landed with what
//!   came off the target's health, the launcher half on the impact telling (in `flight_hook`,
//!   fed by the flight profile's spilling snapshot). A monster's melee hit with a tank tool
//!   is the same telling with a mob attacker; the port's telling names the weapon either way.
//! - **WettingModifier / BurstingModifier** (UseFluidOnHitModifier): being hit applies the worn
//!   tank's fluid — wetting onto the wearer, bursting onto the attacker — each at a 25% chance
//!   per level, the application always at level 1. Both run on the damage-modification telling
//!   (its MODIFY_DAMAGE hook): it names the damage type and its directness, which the
//!   wearer-hit telling does not (it fires before the source is worked out), and it fires when
//!   the attack will land, as the original's on-attacked hook does.
//! - **SlurpingModule** (GENERAL_INTERACT, TOOL_USING, ARMOR_INTERACT, INVENTORY_TICK): drinking
//!   the tank's fluid applies its effects to the drinker. The held form rides the stack use the
//!   throwing registration composes (Drink pose, the module's duration), sounds on the use tick,
//!   and the application on the finish telling; the armor form starts from the armor-interact
//!   key (`armor_start`/`armor_stop`, called by `armor_interact_hook`'s receiver) and finishes
//!   on the inventory tick.
//! - **SpittingModule** (GENERAL_INTERACT, ARMOR_INTERACT, INVENTORY_TICK): charging and
//!   releasing fires the tank's fluid as `tconstruct:fluid_spit` projectiles. The held form
//!   rides the composed use (Bow pose) and fires on the release telling; the helmet form starts
//!   from the armor key and fires on its stop.
//! - **SplashingModule** (ENTITY_INTERACT, BLOCK_INTERACT): using the tool on an entity or a
//!   block applies the fluid's effects over the tool's area of effect.
//! - **TankInteractionModule** (BLOCK_INTERACT) and **BucketModule**: a use on a block with a
//!   fluid handler trades fluid with the tool's tank; sneaking with a bucket tool places its
//!   fluid's block, a plain use on a fluid source block picks it up.
//! - **TankModule's slot interaction** (the `tconstruct:slot_stack` hook): clicking a tank tool
//!   onto a fluid container in a menu, or a container onto a tank tool, trades fluid between
//!   them (the stacked-on-other telling, `on_stacked_on_other`).
//! - **OverburnModule** (INVENTORY_TICK): the inventory tick drives the burn now that this
//!   consumer keeps the holder's tick count itself (the remembered player facts below).
//!
//! The standing reductions, each the closest the tellings allow: a spilling application's
//! effects and tank spend land with the damage events' action queues (the attacker's and the
//! target's next ticks), not in the hit's own call, as wetting's and bursting's do; a creative holder's tank is spent
//! like a survival one's where a telling does not name the game mode (the capacity write-backs
//! accept the same); the splashing item cooldown (20 / draw speed) is started at both
//! splash sites through the item-cooldown surface the sleeves lane built in PlutoLib
//! (`ServerPlayer::start_item_cooldown`); the bucket module has no cauldron
//! (`LiquidBlockContainer`) half — no such block exists in
//! the port — and no `mayInteract` re-check, as the host already gates the block use; a mob
//! wearer's piece is never spent (the stack-edit actions reach players only); the helmet spit's
//! aim is the wearer's last look a use telling reported (the aim stash below), as no telling
//! names a player's look outside a use.
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, RwLock};

use pluto_lib::{
    ActionResult, ActionTarget, CarriedEdit, CarriedSlot, Direction, EntityAction, EntityReach, FluidItems, FluidStack,
    FluidWorld, Identifier, ItemEvents, ItemSlot, Nbt, NbtCompound, PlayerInventoryEvents, ReleaseUsing, ServerLivingEntityEvents,
    ServerPlayer, StackData, StackSlot, StackedOnOtherDecision, StackedOnOtherRegistry, StackedOnOtherRequest, StackedSound,
    UseItemFinishCallback, UseItemOnBlockCallback, UseItemOnEntityContext, UseItemOnBlockContext, UseEntityCallback, World,
};
use pluto_lib::entity_events_v1::{DamageModification, PlayerTick, WeaponHitDealt};

use crate::{
    break_speed_hook::with_held_tool,
    common,
    fluid_effects::{self, EntityContext, Env, Holder},
    fluid_world::{self, LiveTarget, LiveWorld},
    modifier_entries::Entry,
    tool_damage_hook, tool_slot, tool_stack, tool_stats::Stat, tool_tank,
};

/* Shared facts and helpers */

/// What this consumer remembers of a player from their ticks: where the other tellings (a hit on
/// the wearer, an inventory tick, an armor key) can find them, and the player's own tick count,
/// which the armor timers and the overburn burn measure on (the inventory tick reports neither).
#[derive(Clone, Debug)]
pub(crate) struct PlayerFacts {
    pub entity: i32,
    pub dimension: String,
    pub position: [f64; 3],
    pub armor: Vec<(String, StackData)>,
    /// The player's ticks as this consumer counted them (one per player tick heard).
    pub tick: i32,
}

static PLAYERS: Mutex<BTreeMap<u128, PlayerFacts>> = Mutex::new(BTreeMap::new());
/// A world clock for the overburn burn's expiration: every player tick heard, from any player.
static TICKS_HEARD: Mutex<i64> = Mutex::new(0);
/// The last look a use telling reported for a player, and the eye it was reported from: the
/// helmet spit's aim (see the module note).
static AIMS: Mutex<BTreeMap<u128, ([f64; 3], [f64; 3])>> = Mutex::new(BTreeMap::new());

/// The remembered facts of a player, when a tick of theirs was heard.
pub(crate) fn player_facts(player: u128) -> Option<PlayerFacts> {
    PLAYERS.lock().ok()?.get(&player).cloned()
}

fn remember_aim(player: u128, eye: [f64; 3], look: [f64; 3]) {
    if let Ok(mut aims) = AIMS.lock() { aims.insert(player, (eye, look)); }
}

/// The fluid effects data and the fluid tags an application runs against, as the other consumers
/// read them.
fn effects_data() -> Option<(Arc<RwLock<fluid_effects::Data>>, crate::fluid_ingredient::FluidTags)> {
    let effects = common::fluid_effects()?.clone();
    let tags = common::smeltery_data()?.read().ok()?.tags.clone();
    Some((effects, tags))
}

/// What the matching entry says of a fluid: (has block effects, has entity effects, millibuckets
/// per level); None when no entry matches.
fn describe(fluid: &FluidStack) -> Option<(bool, bool, i32)> {
    let (effects, tags) = effects_data()?;
    effects.read().ok()?.describe(fluid, &tags)
}

/// Runs the matching entry's entity effects against `target`, returning the millibuckets
/// consumed (FluidEffectManager.find + FluidEffects.applyToEntity at `level`).
fn apply_entity(dimension: &str, fluid: &FluidStack, level: f32, holder: Holder, target: &mut LiveTarget, location: [f64; 3]) -> i32 {
    let Some((effects, tags)) = effects_data() else { return 0 };
    let Ok(effects) = effects.read() else { return 0 };
    let mut world = LiveWorld::new(World::new(dimension));
    let mut context = EntityContext { env: Env { world: &mut world, holder }, target, location };
    effects.apply_to_entity(fluid, level, &mut context, &tags)
}

/// What apply_entity would consume, simulate phase only: the gate the slurp forms ask before a
/// drink may start (FluidAction.SIMULATE in the original's start checks).
fn simulate_entity(dimension: &str, fluid: &FluidStack, level: f32, holder: Holder, target: &mut LiveTarget, location: [f64; 3]) -> i32 {
    let Some((effects, tags)) = effects_data() else { return 0 };
    let Ok(effects) = effects.read() else { return 0 };
    let mut world = LiveWorld::new(World::new(dimension));
    let mut context = EntityContext { env: Env { world: &mut world, holder }, target, location };
    effects.simulate_to_entity(fluid, level, &mut context, &tags)
}

/// The holder a tank's user makes for an application: themselves, with the tank tool as the
/// context's stack.
fn self_holder(entity: i32, player: bool, stack: &StackData, position: [f64; 3]) -> Holder {
    let mut holder = fluid_world::holder(fluid_world::stack_of_data(stack), None, position);
    holder.entity = Some(entity);
    holder.player = player;
    holder
}

/// The tank write-back edits for a stack action: the persistent-data edits between the stack's
/// data as reported and the clone the consumer changed.
fn tank_edits(before: &NbtCompound, after: &NbtCompound) -> Vec<pluto_lib::StackDataEdit> {
    crate::capacity::persistent_edits(before, after)
}

/// The armor slot an EditStackData names for a slot name of the tick's armor list.
fn armor_stack_slot(slot: &str) -> StackSlot {
    match slot {
        "feet" => StackSlot::Feet,
        "legs" => StackSlot::Legs,
        "chest" => StackSlot::Chest,
        _ => StackSlot::Head,
    }
}

/// The carried slot number of an armor slot name (the game's numbering the inventory tick and
/// ServerPlayer::update_stack share: feet 100, legs 101, chest 102, head 103).
fn armor_slot_number(slot: &str) -> i32 {
    match slot {
        "feet" => 100,
        "legs" => 101,
        "chest" => 102,
        _ => 103,
    }
}

/// Whether `slot` (an inventory tick's number) is one of the four armor slots.
fn is_armor_slot(slot: i32) -> bool { (100..=103).contains(&slot) }

/// Writes a changed tool stack back to the hand that holds it, the brush consumer's shape: the
/// main hand through the selected slot when it still holds the stack, the off hand otherwise.
fn write_back_hand(player: &ServerPlayer, before: &StackData, after: &ItemSlot) {
    let after_data = StackData {
        item: after.item.to_string(), count: after.count, damage: after.damage, max_damage: after.max_damage,
        custom_data: after.custom_data.clone(), custom_model_data: after.custom_model_data.clone(),
        enchantments: after.enchantments.clone().unwrap_or_default(),
    };
    if let Some(main) = player.main_hand_stack() {
        if main.item == before.item && main.custom_data == before.custom_data {
            if let Some(selected) = player.selected_slot() { player.update_stack(selected, &after_data); }
            return;
        }
    }
    player.set_equipment(StackSlot::OffHand, Some(&after_data));
}

/// A StackData as the ItemSlot the slot-level tools (wear, the tank transfers) work on.
fn slot_of(stack: &StackData) -> ItemSlot {
    let item = Identifier::parse(&stack.item).unwrap_or_else(|_| Identifier::new("minecraft", "air").expect("valid id"));
    let mut slot = ItemSlot::new(item, stack.count);
    slot.damage = stack.damage;
    slot.max_damage = stack.max_damage;
    slot.custom_data = stack.custom_data.clone();
    slot.custom_model_data = stack.custom_model_data.clone();
    slot.enchantments = if stack.enchantments.is_empty() { None } else { Some(stack.enchantments.clone()) };
    slot
}

/// The reverse of slot_of.
fn data_of(slot: &ItemSlot) -> StackData {
    StackData {
        item: slot.item.to_string(), count: slot.count, damage: slot.damage, max_damage: slot.max_damage,
        custom_data: slot.custom_data.clone(), custom_model_data: slot.custom_model_data.clone(),
        enchantments: slot.enchantments.clone().unwrap_or_default(),
    }
}

/// The drink sound of the slurp forms (SoundEvents.GENERIC_DRINK, volume 0.5, the original's
/// pitch draw).
fn drink_sound(world: &World, at: [f64; 3]) {
    let pitch = 0.9 + tool_damage_hook::random() * 0.1;
    world.play_sound("minecraft:entity.generic.drink", at, 0.5, pitch);
}

/// The breath sound of the spit helmet form (SoundEvents.PLAYER_BREATH, as above).
fn breath_sound(world: &World, at: [f64; 3]) {
    let pitch = 0.9 + tool_damage_hook::random() * 0.1;
    world.play_sound("minecraft:entity.player.breath", at, 0.5, pitch);
}

/* Spilling, wetting, bursting: the on-hit applications */

/// SpillingModule.afterMeleeHit: the weapon's tank fluid applied to the mob, heard after the
/// hit's damage landed with what came off the mob's health — the original's damageDealt, so a
/// hit the armor absorbed whole (dealt 0) applies nothing, as the original's gate reads. The
/// fully-charged gate reads the hit as the weapon-hit telling reported it.
fn on_weapon_hit_dealt(dealt: &WeaponHitDealt) {
    let hit = &dealt.hit;
    let Some(weapon) = &hit.weapon else { return };
    if hit.dimension.is_empty() { return; }
    let _ = with_held_tool(weapon, |env, tool| {
        let modules = tool_stack::tank_modules(&tool.custom_data, env);
        if modules.spilling.is_empty() || tool_stack::is_broken(&tool.custom_data) { return; }
        // SpillingModule.afterMeleeHit: the hit must have dealt damage — what came off the target's health, not the
        // incoming amount the weapon-hit telling reported — and the swing must have been fully charged.
        if dealt.dealt <= 0.0 { return; }
        if !hit.fully_charged(tool_stack::stat(&tool.custom_data, Stat::AttackSpeed)) { return; }
        let Some(fluid) = tool_tank::fluid(&tool.custom_data) else { return };
        if !describe(&fluid).map_or(false, |(_, has_entity, _)| has_entity) { return; }
        let attacker_is_player = hit.attacker_state.as_ref().map_or(false, |state| state.kind == "minecraft:player");
        let position = hit.target_position.unwrap_or([0.0, 0.0, 0.0]);
        for level in &modules.spilling {
            let holder = self_holder(hit.attacker, attacker_is_player, weapon, hit.attacker_position.unwrap_or(position));
            let mut target = LiveTarget::party(hit.entity, &hit.kind, position, &hit.actions, ActionTarget::Victim);
            let consumed = apply_entity(&hit.dimension, &fluid, *level, holder, &mut target, position);
            if consumed > 0 {
                fluid_world::spawn_fluid_particles(&World::new(&hit.dimension), &fluid, position, 10);
                // ModifierUtil.consumesResources: only a player's tank is spent (a mob
                // attacker's is not), through the hit's own stack edits.
                if attacker_is_player {
                    let mut work = tool.custom_data.clone();
                    if tool_tank::consume(&mut work, consumed) > 0 {
                        let edits = tank_edits(&tool.custom_data, &work);
                        if !edits.is_empty() {
                            hit.actions.borrow_mut().push(EntityAction::EditStackData { target: ActionTarget::Attacker, slot: StackSlot::MainHand, item: weapon.item.clone(), edits });
                        }
                    }
                }
            }
        }
    });
}

/// UseFluidOnHitModifier.useFluid for one worn piece: the chance roll (a quarter per level,
/// doubled for a blocking shield), the tank's fluid applied at level 1 to `target`, and the
/// piece's tank spent by what the application consumed, written back as `party`'s edit of the
/// piece's slot. Returns whether the application consumed anything.
#[allow(clippy::too_many_arguments)]
fn use_fluid_on_hit(
    dimension: &str,
    piece: &StackData,
    slot: &str,
    modifier: &str,
    wearer: (i32, [f64; 3]),
    target: &mut LiveTarget,
    target_position: [f64; 3],
    actions: &std::cell::RefCell<Vec<EntityAction>>,
    party: ActionTarget,
) -> bool {
    let Ok(done) = with_held_tool(piece, |env, tool| {
        let level = tool_stack::modifier_level(&tool.custom_data, env, modifier);
        if level <= 0 || tool_stack::is_broken(&tool.custom_data) { return false; }
        let mut chance = 0.25 * level as f32;
        if tool_stack::blocking(&tool.custom_data, env) { chance *= 2.0; }
        if tool_damage_hook::random() >= chance { return false; }
        let Some(fluid) = tool_tank::fluid(&tool.custom_data) else { return false };
        if !describe(&fluid).map_or(false, |(has_block, has_entity, _)| has_block || has_entity) { return false; }
        let holder = self_holder(wearer.0, true, piece, wearer.1);
        let consumed = apply_entity(dimension, &fluid, 1.0, holder, target, target_position);
        if consumed <= 0 { return false; }
        fluid_world::spawn_fluid_particles(&World::new(dimension), &fluid, target_position, 10);
        let mut work = tool.custom_data.clone();
        if tool_tank::consume(&mut work, consumed) > 0 {
            let edits = tank_edits(&tool.custom_data, &work);
            if !edits.is_empty() {
                actions.borrow_mut().push(EntityAction::EditStackData { target: party, slot: armor_stack_slot(slot), item: piece.item.clone(), edits });
            }
        }
        true
    }) else { return false };
    done
}

/// WettingModifier.modifyDamageTaken / BurstingModifier.onAttacked: a worn tank answers a hit on its wearer —
/// wetting douses the wearer, bursting answers the attacker. Both run on the damage-modification telling (see the
/// module note): it names the damage type and its directness, and it fires when the attack will land. A mob
/// wearing a piece is answered too; its piece is never spent (the stack-edit actions reach players only), but its
/// fluid still answers. The attacker's position comes from the reach, falling back to the victim's.
fn on_damage_modification(modification: &DamageModification) {
    let damage = &modification.damage;
    if damage.attacker == damage.entity { return; }
    if modification.armor.is_empty() { return; }
    // Wetting skips damage whose type bypasses effects or invulnerability (the original's two tag gates).
    let wetting = !(damage.in_damage_type_tag("minecraft:bypasses_effects")
        || damage.in_damage_type_tag("minecraft:bypasses_invulnerability"));
    // Bursting answers a direct hit only (the original's isDirectDamage: a causer, the direct entity none or the
    // causer, and a type outside `minecraft:avoids_guardian_thorns`).
    let bursting = damage.is_direct_damage();
    if !wetting && !bursting { return; }
    let (dimension, position) = match PLAYERS.lock().ok().and_then(|players| players.values().find(|facts| facts.entity == damage.entity).map(|facts| (facts.dimension.clone(), facts.position))) {
        Some(found) => found,
        None => match EntityReach::position_of(damage.entity as u64) {
            Some(position) => (String::new(), position),
            None => return,
        },
    };
    // A player's dimension comes from their tick; a mob's is not named by the telling, and the
    // application needs one — the acting player's dimension is the standing answer for a mob
    // the game placed near one, and without any player the application cannot run.
    let dimension = if !dimension.is_empty() {
        dimension
    } else if let Some(facts) = PLAYERS.lock().ok().and_then(|players| players.values().next().map(|facts| facts.dimension.clone())) {
        facts
    } else {
        return;
    };
    if wetting {
        for (slot, piece) in &modification.armor {
            let mut target = LiveTarget::party(damage.entity, &damage.kind, position, &modification.actions, ActionTarget::Victim);
            use_fluid_on_hit(&dimension, piece, slot, "tconstruct:wetting", (damage.entity, position), &mut target, position, &modification.actions, ActionTarget::Victim);
        }
    }
    if bursting {
        let attacker_position = EntityReach::position_of(damage.attacker as u64).unwrap_or(position);
        let attacker_kind = if !modification.attacker_kind.is_empty() {
            modification.attacker_kind.clone()
        } else if modification.attacker_is_player == Some(true) {
            "minecraft:player".to_owned()
        } else {
            String::new()
        };
        for (slot, piece) in &modification.armor {
            let mut target = LiveTarget::party(damage.attacker, &attacker_kind, attacker_position, &modification.actions, ActionTarget::Attacker);
            use_fluid_on_hit(&dimension, piece, slot, "tconstruct:bursting", (damage.entity, position), &mut target, attacker_position, &modification.actions, ActionTarget::Victim);
        }
    }
}

/* The tick-driven forms: armor timers, overburn, the drink and the spit */

/// The player tick's half here: remember the player for the tellings that name no player, and
/// count their ticks for the armor timers and the overburn burn.
fn on_player_tick(tick: &PlayerTick) {
    if let Ok(mut heard) = TICKS_HEARD.lock() { *heard += 1; }
    if let Ok(mut players) = PLAYERS.lock() {
        let facts = players.entry(tick.player).or_insert_with(|| PlayerFacts {
            entity: tick.entity, dimension: tick.dimension.clone(), position: tick.position, armor: tick.armor.clone(), tick: 0,
        });
        facts.entity = tick.entity;
        facts.dimension = tick.dimension.clone();
        facts.position = tick.position;
        facts.armor = tick.armor.clone();
        facts.tick += 1;
    }
}

/// SlurpingModule.finishDrinking against a stack being written back: the drink sound and
/// particles, the fluid's effects on the holder, and the tank spent by what they consumed.
/// `stack` is the tool as reported; the spend lands in `work`, its changed custom data.
fn finish_drink(work: &mut NbtCompound, stack: &StackData, entry: &Entry, slurping: &tool_tank::Slurping, facts: &PlayerFacts) {
    let Some(fluid) = tool_tank::fluid(work) else { return };
    let world = World::new(&facts.dimension);
    drink_sound(&world, facts.position);
    fluid_world::spawn_fluid_particles(&world, &fluid, facts.position, 16);
    let holder = self_holder(facts.entity, true, stack, facts.position);
    let mut target = LiveTarget::direct(facts.entity, "minecraft:player", facts.position);
    let consumed = apply_entity(&facts.dimension, &fluid, slurping.strength(entry), holder, &mut target, facts.position);
    if consumed > 0 { tool_tank::consume(work, consumed); }
}

/// The inventory tick: the armor forms' timers (SlurpingModule/SpittingModule.onInventoryTick),
/// the unequip clear, and OverburnModule's burn. The tick names neither the holder's tick count
/// nor the game time, so both come from the remembered facts (the player's own count, and the
/// count of every player tick heard as the world clock).
fn on_inventory_tick(tick: &pluto_lib::InventoryTick) {
    let (Some(player), Some(stack)) = (&tick.player, &tick.stack) else { return };
    let facts = player_facts(player.uuid);
    let now = facts.as_ref().map_or(0, |facts| facts.tick);
    let _ = with_held_tool(stack, |env, tool| {
        let modules = tool_stack::tank_modules(&tool.custom_data, env);
        let has_overburn = tool.custom_data.get(tool_stack::TAG_MODIFIERS).is_some()
            && tool_stack::saved_modifiers(&tool.custom_data, env.modifiers).iter().any(|(_, modifier)| modifier.modules().iter().any(|module| module.is_overburn()));
        if modules.is_empty() && !has_overburn { return; }
        let mut work = tool.custom_data.clone();
        let mut changed = false;
        if is_armor_slot(tick.slot) {
            if let Some(facts) = &facts {
                let world = World::new(&facts.dimension);
                for (entry, slurping) in &modules.slurping {
                    match tool_stack::with_persistent(&mut work, |persistent| slurping.armor_tick(persistent, entry, now)) {
                        tool_tank::ArmorTick::Finished => {
                            changed = true;
                            finish_drink(&mut work, stack, entry, slurping, facts);
                        }
                        tool_tank::ArmorTick::Sound => {
                            drink_sound(&world, facts.position);
                            if let Some(fluid) = tool_tank::fluid(&work) { fluid_world::spawn_fluid_particles(&world, &fluid, facts.position, 5); }
                        }
                        tool_tank::ArmorTick::Idle => {}
                    }
                }
                for (entry, spitting) in &modules.spitting {
                    match tool_stack::with_persistent(&mut work, |persistent| spitting.armor_tick(persistent, entry, now)) {
                        tool_tank::ArmorTick::Finished => {
                            // The spit itself fired at the key's stop; the timer's end is
                            // only the breath and the particles (and the key, already removed).
                            changed = true;
                            breath_sound(&world, facts.position);
                            if let Some(fluid) = tool_tank::fluid(&work) { fluid_world::spawn_fluid_particles(&world, &fluid, facts.position, 16); }
                        }
                        tool_tank::ArmorTick::Sound => {
                            breath_sound(&world, facts.position);
                            if let Some(fluid) = tool_tank::fluid(&work) { fluid_world::spawn_fluid_particles(&world, &fluid, facts.position, 5); }
                        }
                        tool_tank::ArmorTick::Idle => {}
                    }
                }
            }
        } else if tick.slot >= 0 {
            // onUnequip: a piece carrying a live armor timer anywhere but an armor slot has
            // been taken off; the timers clear. (The original also asks whether the tool
            // changed; a piece in the inventory is off the body either way.)
            for (entry, slurping) in &modules.slurping {
                let live = tool_stack::with_persistent(&mut work, |persistent| crate::modifier_entries::nbt_int(persistent.get(&entry.id.to_string())));
                if live > 0 {
                    tool_stack::with_persistent(&mut work, |persistent| slurping.stop(persistent, entry));
                    changed = true;
                }
            }
            for (entry, spitting) in &modules.spitting {
                let live = tool_stack::with_persistent(&mut work, |persistent| crate::modifier_entries::nbt_int(persistent.get(&entry.id.to_string())));
                if live > 0 {
                    tool_stack::with_persistent(&mut work, |persistent| spitting.stop(persistent, entry));
                    changed = true;
                }
            }
        }
        if has_overburn {
            let game_time = TICKS_HEARD.lock().map_or(0, |heard| *heard);
            let fuel = |fluid: &Identifier| -> Option<tool_tank::Fuel> {
                let data = common::smeltery_data()?.read().ok()?;
                let tags = data.tags.clone();
                let in_tag = |fluid: &Identifier, tag: &Identifier| tags.contains(fluid, tag);
                let found = data.recipes.find_fuel(fluid, &in_tag)?;
                Some(tool_tank::Fuel { amount: found.amount(fluid, &in_tag), duration: found.duration, rate: found.rate })
            };
            // using_this: the tick cannot name the stack in use; the gate only pauses a burn
            // while that very stack is used, and a use of it is short beside a burn.
            if tool_stack::overburn_tick(&mut work, env, now, game_time, false, &fuel, &mut tool_damage_hook::random) {
                changed = true;
            }
        }
        if changed {
            let mut after = stack.clone();
            after.custom_data = work;
            player.update_stack(tick.slot, &after);
        }
    });
}

/// The held slurp's use tick (SlurpingModule.onUsingTick): every four ticks, while the drink
/// can still apply, the drink sound and five particles. The original's active-modifier mark
/// collapses here: the composed use is the drink, so a drink that still applies is the case.
fn on_use_tick(tick: &pluto_lib::UseTick) {
    remember_aim(tick.player.uuid, tick.eye, tick.look);
    if tick.ticks_used == 0 || tick.ticks_used % 4 != 0 { return; }
    let _ = with_held_tool(&tick.stack, |env, tool| {
        let modules = tool_stack::tank_modules(&tool.custom_data, env);
        let Some((entry, slurping)) = modules.slurping.first() else { return };
        let Some(fluid) = tool_tank::fluid(&tool.custom_data) else { return };
        let Some(entity) = EntityReach::id_of_uuid(tick.player.uuid) else { return };
        let holder = self_holder(entity as i32, true, &tick.stack, tick.position);
        let mut target = LiveTarget::direct(entity as i32, "minecraft:player", tick.position);
        if simulate_entity(&tick.dimension, &fluid, slurping.strength(entry), holder, &mut target, tick.position) > 0 {
            drink_sound(&tick.world(), tick.position);
            fluid_world::spawn_fluid_particles(&tick.world(), &fluid, tick.position, 5);
        }
    });
}

/// The held slurp's finish (SlurpingModule's drink completing): finishDrinking with its sound.
fn on_use_finish(done: &pluto_lib::UseItemFinishContext) {
    let _ = with_held_tool(&done.stack, |env, tool| {
        let modules = tool_stack::tank_modules(&tool.custom_data, env);
        let Some((entry, slurping)) = modules.slurping.first() else { return };
        let Some(fluid) = tool_tank::fluid(&tool.custom_data) else { return };
        let world = World::new(&done.dimension);
        drink_sound(&world, done.position);
        fluid_world::spawn_fluid_particles(&world, &fluid, done.position, 16);
        let entity = EntityReach::id_of_uuid(done.player.uuid).map_or(0, |id| id as i32);
        let holder = self_holder(entity, true, &done.stack, done.position);
        let mut target = LiveTarget::direct(entity, "minecraft:player", done.position);
        let consumed = apply_entity(&done.dimension, &fluid, slurping.strength(entry), holder, &mut target, done.position);
        if consumed > 0 && !done.creative {
            if let Some(slot) = &done.slot {
                let mut work = slot.clone();
                if tool_tank::consume(&mut work.custom_data, consumed) > 0 {
                    *done.replacement.borrow_mut() = Some(work);
                }
            }
        }
    });
}

/// Rotates a look vector around the up axis by `degrees` (the spit fan's spread).
fn rotate_up(look: [f64; 3], degrees: f32) -> [f64; 3] {
    let radians = (degrees as f64).to_radians();
    let (sin, cos) = (radians.sin(), radians.cos());
    [cos * look[0] + sin * look[2], look[1], -sin * look[0] + cos * look[2]]
}

/// GeneralInteractionModifierHook.getDrawtime for the spit's speed factor 1.5.
fn spit_drawtime(stack: &StackData) -> f32 {
    let speed = with_held_tool(stack, |_env, tool| tool_stack::stat(&tool.custom_data, Stat::DrawSpeed)).unwrap_or(0.0);
    (20.0 * 1.5 / speed.max(f32::MIN_POSITIVE)).ceil()
}

/// SpittingModule.spit: the plan, the fanned shots, the tank spent and the tool worn by the
/// shots. Returns the stack after the spend and wear for the caller to write back; None when
/// nothing was fired. `consumes` is ModifierUtil.consumesResources for the shooter.
#[allow(clippy::too_many_arguments)]
fn spit_fire(
    player: &ServerPlayer,
    dimension: &str,
    eye: [f64; 3],
    look: [f64; 3],
    stack: &StackData,
    env: &tool_stack::Environment,
    entry: &Entry,
    spitting: &tool_tank::Spitting,
    charge: f32,
    consumes: bool,
) -> Option<ItemSlot> {
    let fluid = tool_tank::fluid(&stack.custom_data)?;
    let (has_block, has_entity, amount_per_level) = describe(&fluid)?;
    if !has_block && !has_entity { return None; }
    let plan = spitting.plan(
        entry,
        fluid.amount as i32,
        amount_per_level,
        charge,
        tool_stack::stat(&stack.custom_data, Stat::ProjectileDamage),
        tool_stack::stat(&stack.custom_data, Stat::Velocity),
    )?;
    let world = World::new(dimension);
    let engine_stack = fluid_world::stack_of_data(stack);
    let inaccuracy = 3.0 * (1.0 / tool_stack::stat(&stack.custom_data, Stat::Accuracy).max(f32::MIN_POSITIVE) - 1.0);
    let start_angle = -5.0 * (plan.shots - 1) as f32;
    for shot in 0..plan.shots {
        let angle = start_angle + 10.0 * shot as f32;
        let direction = rotate_up(look, angle);
        let gaussians = crate::fluid_projectile::gaussians(&world);
        let jitter = 0.0075 * inaccuracy as f64;
        let motion = [
            (direction[0] + gaussians[0] * jitter) * plan.velocity as f64,
            (direction[1] + gaussians[1] * jitter) * plan.velocity as f64,
            (direction[2] + gaussians[2] * jitter) * plan.velocity as f64,
        ];
        let shot_fluid = FluidStack { amount: plan.amount_per_shot as u32, ..fluid.clone() };
        crate::fluid_projectile::launch_shot(&world, eye, motion, shot_fluid, plan.power, tool_stack::stat(&stack.custom_data, Stat::WaterInertia), Some(player.uuid), engine_stack.clone());
        // TinkerSounds.SPIT, at the original's pitch for the shot's angle.
        let pitch = 1.0 / (tool_damage_hook::random() * 0.4 + 1.2) + charge * 0.5 + angle / 10.0;
        world.play_sound("tconstruct:spit", eye, 1.0, pitch);
    }
    let mut slot = slot_of(stack);
    if consumes { tool_tank::consume(&mut slot.custom_data, plan.amount_per_shot * plan.shots); }
    let _ = tool_slot::damage_slot(&mut slot, env, plan.shots, consumes, true, Some(&entry.id), &mut tool_damage_hook::random);
    Some(slot)
}

/// The held spit's release (SpittingModule.onStoppedUsing): the charge from the ticks held,
/// over the draw time the use started with.
/// The hand holding the released stack, for a release's write-back (the spit and the bucket pickup share it).
fn release_hand(release: &ReleaseUsing) -> Option<CarriedSlot> {
    let same = |slot: &ItemSlot| slot.item.to_string() == release.stack.item && slot.custom_data == release.stack.custom_data;
    if release.main_hand.as_ref().is_some_and(|hand| same(hand)) {
        Some(CarriedSlot::Equipment(StackSlot::MainHand))
    } else if release.off_hand.as_ref().is_some_and(|hand| same(hand)) {
        Some(CarriedSlot::Equipment(StackSlot::OffHand))
    } else {
        None
    }
}

/// The source-fluid cell a look reaches (the original's Block.OUTLINE + SOURCE_ONLY trace): stepped from the
/// eye along the look, the first source fluid is the target, the first solid block stops the trace, flowing
/// fluid and air are passed through. None when nothing is reached within `reach` or the trace leaves loaded
/// chunks (a cell the world does not name).
fn ray_trace_source(world: &World, eye: [f64; 3], look: [f64; 3], reach: f64) -> Option<[i32; 3]> {
    let mut last = [i32::MIN, i32::MIN, i32::MIN];
    let mut travelled = 0.0;
    while travelled <= reach {
        let at = [eye[0] + look[0] * travelled, eye[1] + look[1] * travelled, eye[2] + look[2] * travelled];
        let cell = [at[0].floor() as i32, at[1].floor() as i32, at[2].floor() as i32];
        if cell != last {
            last = cell;
            let Some(state) = world.get_block_state(cell) else { return None };
            if let Some((_, 0)) = FluidWorld::fluid_of(&state) { return Some(cell); }
            if FluidWorld::fluid_of(&state).is_some() { travelled += 0.05; continue; }
            if !matches!(state.block.to_string().as_str(), "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air") {
                return None;
            }
        }
        travelled += 0.05;
    }
    None
}

/// BucketModule.onToolUse (the pickup half): the release of a bucket tool's use picks up the source fluid the
/// player looks at, into the tank (set when empty, grown when the same fluid). A spittable tool never reaches
/// this: its use is a spit (see the composed use's order). A click's release only (ten ticks or less); a hold
/// is a spit charge or a throw, not a pickup. The source is removed and its neighbours woken so they settle;
/// the fill sound plays and the tank is written back to the hand holding the tool. The original checks no game
/// mode here either. Returns the stack after the pickup, when one happened.
fn bucket_pickup(release: &ReleaseUsing) -> Option<ItemSlot> {
    if spittable(&release.stack) { return None; }
    let Ok(after) = with_held_tool(&release.stack, |env, tool| {
        if tool_stack::is_broken(&tool.custom_data) { return None; }
        let modules = tool_stack::tank_modules(&tool.custom_data, env);
        let Some(bucket) = modules.bucket.first() else { return None };
        let in_tag = bucket_in_tag();
        if !bucket.can_pickup(&tool.custom_data, &in_tag) { return None; }
        let world = World::new(&release.dimension);
        let reach = if release.creative { 5.0 } else { 4.5 };
        let Some(target) = ray_trace_source(&world, release.eye, release.look, reach) else { return None };
        let Some(existing) = world.get_block_state(target) else { return None };
        let Some((fluid, 0)) = FluidWorld::fluid_of(&existing) else { return None };
        if !bucket.fluids.matches(&fluid, &in_tag) { return None; }
        if let Some(stored) = tool_tank::fluid(&tool.custom_data) {
            if stored.fluid != fluid { return None; }
        }
        // Remove the source and wake its neighbours so they settle (the host flows water and lava itself; a mod
        // fluid's neighbours are woken by their own scheduled tick).
        let air = pluto_lib::BlockState::new(Identifier::parse("minecraft:air").expect("a valid block"));
        if !world.set_block_state(target, &air) { return None; }
        if fluid.namespace() != "minecraft" {
            if let Some(def) = pluto_lib::FluidRegistry::get(&fluid.to_string()) {
                for offset in [[1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, -1, 0], [0, 0, 1], [0, 0, -1]] {
                    let neighbour = [target[0] + offset[0], target[1] + offset[1], target[2] + offset[2]];
                    world.schedule_tick(neighbour, &FluidWorld::block_id(&fluid).to_string(), i32::from(def.flow.tick_delay.max(1)));
                }
            }
        }
        let mut slot = slot_of(&release.stack);
        if !bucket.pickup(&mut slot.custom_data, &fluid) { return None; }
        bucket_sound(&world, &FluidStack::new(fluid, crate::fluid_values::BUCKET as u32), [target[0] as f64 + 0.5, target[1] as f64 + 0.5, target[2] as f64 + 0.5], true);
        Some(slot)
    }) else { return None };
    after
}

fn on_release(release: &ReleaseUsing) {
    remember_aim(release.player.uuid, release.eye, release.look);
    // The bucket pickup is a click's release (ten ticks or less, a hold is a spit charge or a throw).
    if release.ticks_used <= 10 {
        if let Some(after) = bucket_pickup(release) {
            if let Some(slot) = release_hand(release) {
                release.edits.borrow_mut().push(CarriedEdit { slot, stack: after });
            }
            return;
        }
    }
    if release.ticks_used == 0 { return; }
    let _ = with_held_tool(&release.stack, |env, tool| {
        let modules = tool_stack::tank_modules(&tool.custom_data, env);
        let Some((entry, spitting)) = modules.spitting.first() else { return };
        if tool_stack::is_broken(&tool.custom_data) { return; }
        let charge = crate::throwing_hook::charge(release.ticks_used, spit_drawtime(&release.stack));
        if let Some(after) = spit_fire(&release.player, &release.dimension, release.eye, release.look, &release.stack, env, entry, spitting, charge, !release.creative) {
            if let Some(slot) = release_hand(release) {
                release.edits.borrow_mut().push(CarriedEdit { slot, stack: after });
            }
        }
    });
}

/* The armor key forms (called by armor_interact_hook's receiver) */

/// The worn piece an armor key works: its slot name and stack, from the player tick's memory
/// (the receiver reads the same memory the self-destructive form reads).
fn worn_piece(player: u128, packet_slot: u8) -> Option<(&'static str, StackData)> {
    let slot = match packet_slot { 0 => "head", 1 => "legs", _ => return None };
    let (_, piece) = pluto_lib::entity_events_v1::equipment_of(player).into_iter().find(|(name, _)| name == slot)?;
    Some((slot, piece))
}

/// KeybindInteractModifierHook.startInteract for the tank modules on the worn piece, at the
/// normal tooltip key: a slurp starts when its simulate passes; a spit starts its helmet charge
/// (HelmetChargingEffect.startUsingHelmet: the draw time stored, the timer under the modifier's
/// ID, and the charging effect for the draw plus twenty ticks). Whether anything started.
pub(crate) fn armor_start(player: u128, packet_slot: u8) -> bool {
    let Some((slot, piece)) = worn_piece(player, packet_slot) else { return false };
    let Some(facts) = player_facts(player) else { return false };
    let wearer = ServerPlayer { uuid: player, name: String::new() };
    let Ok(started) = with_held_tool(&piece, |env, tool| {
        if tool_stack::is_broken(&tool.custom_data) { return false; }
        let modules = tool_stack::tank_modules(&tool.custom_data, env);
        let Some(fluid) = tool_tank::fluid(&tool.custom_data) else { return false };
        let mut work = tool.custom_data.clone();
        for (entry, slurping) in &modules.slurping {
            let holder = self_holder(facts.entity, true, &piece, facts.position);
            let mut target = LiveTarget::direct(facts.entity, "minecraft:player", facts.position);
            if simulate_entity(&facts.dimension, &fluid, slurping.strength(entry), holder, &mut target, facts.position) > 0 {
                tool_stack::with_persistent(&mut work, |persistent| slurping.armor_start(persistent, entry, facts.tick));
                let mut after = piece.clone();
                after.custom_data = work;
                return wearer.update_stack(armor_slot_number(slot), &after);
            }
        }
        for (entry, spitting) in &modules.spitting {
            let has_effects = describe(&fluid).map_or(false, |(has_block, has_entity, _)| has_block || has_entity);
            if tool_tank::Spitting::can_start(entry, Some(&fluid), has_effects) {
                let charge_time = spit_drawtime(&piece) as i32;
                tool_stack::with_persistent(&mut work, |persistent| {
                    spitting.armor_start(persistent, entry, facts.tick, charge_time);
                    // startDrawing's store, which the stop's charge reads back.
                    persistent.insert("tconstruct:drawtime".into(), Nbt::Int(charge_time));
                });
                wearer.add_effect(&pluto_lib::player_api::EffectInstance::new("tconstruct:helmet_charging", charge_time + 20));
                let mut after = piece.clone();
                after.custom_data = work;
                return wearer.update_stack(armor_slot_number(slot), &after);
            }
        }
        false
    }) else { return false };
    started
}

/// KeybindInteractModifierHook.stopInteract for the tank modules: a slurp's timer clears; a
/// spit's charge fires (its charge time from the stored draw time and the ticks since the
/// start), aimed by the wearer's last reported look (see the module note). Whether anything ran.
pub(crate) fn armor_stop(player: u128, packet_slot: u8) -> bool {
    let Some((slot, piece)) = worn_piece(player, packet_slot) else { return false };
    let Some(facts) = player_facts(player) else { return false };
    let wearer = ServerPlayer { uuid: player, name: String::new() };
    let Ok(ran) = with_held_tool(&piece, |env, tool| {
        let modules = tool_stack::tank_modules(&tool.custom_data, env);
        let mut work = tool.custom_data.clone();
        let mut changed = false;
        let mut fired = false;
        for (entry, slurping) in &modules.slurping {
            let live = tool_stack::with_persistent(&mut work, |persistent| crate::modifier_entries::nbt_int(persistent.get(&entry.id.to_string())));
            if live > 0 {
                tool_stack::with_persistent(&mut work, |persistent| slurping.stop(persistent, entry));
                changed = true;
            }
        }
        for (entry, spitting) in &modules.spitting {
            let (finish, drawtime) = tool_stack::with_persistent(&mut work, |persistent| {
                (crate::modifier_entries::nbt_int(persistent.get(&entry.id.to_string())), crate::modifier_entries::nbt_int(persistent.get("tconstruct:drawtime")))
            });
            if finish <= 0 { continue; }
            tool_stack::with_persistent(&mut work, |persistent| spitting.stop(persistent, entry));
            changed = true;
            let charge_time = facts.tick - (finish - drawtime);
            if charge_time > 0 {
                if let Some((_, look)) = AIMS.lock().ok().and_then(|aims| aims.get(&player).copied()) {
                    let charge = crate::throwing_hook::charge(charge_time as u32, drawtime.max(1) as f32);
                    let eye = [facts.position[0], facts.position[1] + 1.62, facts.position[2]];
                    // The spend and wear land in `work` through the returned stack's data.
                    let current = { let mut current = piece.clone(); current.custom_data = work.clone(); current };
                    if let Some(after) = spit_fire(&wearer, &facts.dimension, eye, look, &current, env, entry, spitting, charge, true) {
                        work = after.custom_data;
                        fired = true;
                    }
                }
            }
        }
        if changed {
            let mut after = piece.clone();
            after.custom_data = work;
            wearer.update_stack(armor_slot_number(slot), &after);
        }
        changed || fired
    }) else { return false };
    ran
}

/* Splashing, the tank interaction and the bucket: the use tellings */

/// The face as the neighbours and the AOE iterator name it (brush_hook's conversion).
fn direction(face: [i32; 3]) -> Direction {
    match face {
        [0, -1, 0] => Direction::Down,
        [0, 0, -1] => Direction::North,
        [0, 0, 1] => Direction::South,
        [-1, 0, 0] => Direction::West,
        [1, 0, 0] => Direction::East,
        _ => Direction::Up,
    }
}

/// The tool's EXPANDED volatile level (the splash's area range, the AOE iterator's growth).
fn expanded_of(nbt: &NbtCompound) -> i32 {
    match nbt.get(tool_stack::TAG_VOLATILE) {
        Some(Nbt::Compound(volatile)) => crate::modifier_entries::nbt_int(volatile.get("tconstruct:expanded")),
        _ => 0,
    }
}

/// Runs the matching entry's block effects at a position, returning the millibuckets consumed
/// (FluidEffects.applyToBlock at `level`).
fn apply_block(dimension: &str, fluid: &FluidStack, level: f32, holder: Holder, pos: [i32; 3], face: Direction, location: [f64; 3]) -> i32 {
    let Some((effects, tags)) = effects_data() else { return 0 };
    let Ok(effects) = effects.read() else { return 0 };
    let mut world = LiveWorld::new(World::new(dimension));
    let mut context = fluid_world::block_context(&mut world, holder, pos, face, location);
    effects.apply_to_block(fluid, level, &mut context, &tags)
}

/// SplashingModule.beforeEntityUse: the tank's fluid over the entity the tool is used on and
/// the living entities around it. The module's item cooldown — (int)(20 / draw speed) on the
/// tool's item, applied first in the original as a recursion guard — is started through
/// the item-cooldown surface the sleeves lane built (`ServerPlayer::start_item_cooldown`)
/// once the module's gates pass. Marker armor
/// stands are splashed with the rest: no telling names a stand's marker flag.
fn on_entity_use(click: &UseItemOnEntityContext) -> ActionResult {
    let (Some(player), Some(held)) = (&click.player, &click.held) else { return ActionResult::Pass };
    if click.dimension.is_empty() { return ActionResult::Pass; }
    let Ok(result) = with_held_tool(held, |env, tool| {
        let modules = tool_stack::tank_modules(&tool.custom_data, env);
        if modules.splashing.is_empty() || tool_stack::is_broken(&tool.custom_data) { return ActionResult::Pass; }
        let Some(fluid) = tool_tank::fluid(&tool.custom_data) else { return ActionResult::Pass };
        if !describe(&fluid).map_or(false, |(_, has_entity, _)| has_entity) { return ActionResult::Pass; }
        player.start_item_cooldown(&held.item, tool_tank::Splashing::cooldown(tool_stack::draw_speed(&tool.custom_data, env, &crate::entity_facts::Entities::default())));
        let Some(target_position) = EntityReach::position_of(click.entity as u64) else { return ActionResult::Pass };
        let player_entity = EntityReach::id_of_uuid(player.uuid).map_or(0, |id| id as i32);
        let player_position = if player_entity > 0 { EntityReach::position_of(player_entity as u64).unwrap_or(target_position) } else { target_position };
        let world = World::new(&click.dimension);
        let mut working = fluid.clone();
        let mut total_damage = 0;
        for (entry, splashing) in &modules.splashing {
            if working.amount == 0 { break; }
            let level = splashing.strength(entry);
            let mut targets = 0;
            let holder = self_holder(player_entity, true, held, player_position);
            let mut target = LiveTarget::direct(click.entity, &click.kind, target_position);
            let consumed = apply_entity(&click.dimension, &working, level, holder, &mut target, target_position);
            if consumed > 0 {
                targets += 1;
                fluid_world::spawn_fluid_particles(&world, &working, target_position, 10);
                working.amount = working.amount.saturating_sub(consumed as u32);
                let range = 1.0 + expanded_of(&tool.custom_data) as f64;
                let around = EntityReach::in_box(
                    [target_position[0] - range, target_position[1] - 0.25, target_position[2] - range],
                    [target_position[0] + range, target_position[1] + 0.25, target_position[2] + range],
                );
                for entity in around {
                    if working.amount == 0 { break; }
                    if entity.id as i32 == click.entity || entity.id as i32 == player_entity || entity.living.is_none() { continue; }
                    let Some(position) = EntityReach::position_of(entity.id) else { continue };
                    let delta = [position[0] - target_position[0], position[1] - target_position[1], position[2] - target_position[2]];
                    if delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2] >= range * range { continue; }
                    let holder = self_holder(player_entity, true, held, player_position);
                    let mut target = LiveTarget::direct(entity.id as i32, &entity.kind.to_string(), position);
                    let consumed = apply_entity(&click.dimension, &working, level, holder, &mut target, position);
                    if consumed > 0 {
                        targets += 1;
                        fluid_world::spawn_fluid_particles(&world, &working, position, 10);
                        working.amount = working.amount.saturating_sub(consumed as u32);
                    }
                }
            }
            total_damage += tool_tank::Splashing::damage(targets, level);
        }
        if total_damage == 0 { return ActionResult::Pass; }
        let mut slot = slot_of(held);
        if working.amount == 0 { tool_tank::set_fluid(&mut slot.custom_data, None); } else { tool_tank::set_fluid(&mut slot.custom_data, Some(&working)); }
        let _ = tool_slot::damage_slot(&mut slot, env, total_damage, true, true, None, &mut tool_damage_hook::random);
        write_back_hand(player, held, &slot);
        ActionResult::Success
    }) else { return ActionResult::Pass };
    result
}

/// TankInteractionModule.beforeBlockUse against the clicked block's handler, for the entries
/// whose source matches: the trades of tool_tank::TankInteraction, run through the neighbours
/// reach. `write_back` stores the tool's changed data wherever the tool lives (a hand, a chest
/// slot). Some(Success) when a handler exists and an entry ran — the original answers success
/// then even when nothing moved (the sneaking empty-tool case plans nothing) — None when the
/// click is not this module's.
fn tank_trade(
    click: &UseItemOnBlockContext,
    stack: &StackData,
    modules: &tool_stack::TankModules,
    source: tool_tank::InteractionSource,
    write_back: &dyn Fn(&NbtCompound),
) -> Option<ActionResult> {
    let face = direction(click.face);
    pluto_lib::Neighbours::fluid_tanks(click.pos, face)?;
    let world = World::new(&click.dimension);
    let centre = [click.pos[0] as f64 + 0.5, click.pos[1] as f64 + 0.5, click.pos[2] as f64 + 0.5];
    let mut work = stack.custom_data.clone();
    let mut ran = false;
    for interaction in &modules.tank_interaction {
        let Some(plan) = interaction.plan(&work, source, click.sneaking) else { continue };
        ran = true;
        match plan {
            tool_tank::TankTransfer::FillTarget => {
                let Some(fluid) = tool_tank::fluid(&work) else { continue };
                let added = pluto_lib::Neighbours::fill_fluid(click.pos, face, &fluid, true);
                if added > 0 && tool_tank::TankInteraction::filled_target(&mut work, added as i32) {
                    world.play_sound("minecraft:item.bucket.empty", centre, 1.0, 1.0);
                }
            }
            tool_tank::TankTransfer::DrainToTool { max } => {
                if let Some(drained) = pluto_lib::Neighbours::drain_fluid(click.pos, face, max.max(0) as u32) {
                    if tool_tank::TankInteraction::drained_to_tool(&mut work, &drained) {
                        world.play_sound("minecraft:item.bucket.fill", centre, 1.0, 1.0);
                    }
                }
            }
            tool_tank::TankTransfer::TopUp { wanted } => {
                // The neighbours drain takes what the handler holds, so the wanted fluid is
                // checked against the handler's tank first (the original drains the named
                // fluid or nothing).
                let held = pluto_lib::Neighbours::fluid_tanks(click.pos, face).and_then(|tanks| tanks.into_iter().find_map(|tank| tank.fluid));
                if held.as_ref().is_some_and(|fluid| fluid.same_fluid(&wanted)) {
                    if let Some(drained) = pluto_lib::Neighbours::drain_fluid(click.pos, face, wanted.amount) {
                        if drained.same_fluid(&wanted) && tool_tank::TankInteraction::drained_to_tool(&mut work, &drained) {
                            world.play_sound("minecraft:item.bucket.fill", centre, 1.0, 1.0);
                        }
                    }
                }
            }
        }
    }
    if !ran {
        // Sneaking with an empty tool plans nothing, but a source-matching entry still
        // answers success when a handler exists.
        let source_matches = modules.tank_interaction.iter().any(|interaction| interaction.source.is_none() || interaction.source == Some(source));
        if !(click.sneaking && source_matches && tool_tank::fluid(&work).is_none()) { return None; }
    }
    if work != stack.custom_data { write_back(&work); }
    Some(ActionResult::Success)
}

/// SplashingModule.afterBlockUse: the tank's fluid on the clicked block and the tool's area
/// around it. The item cooldown starts the same way as the entity form's, once the gates pass.
fn splash_block(click: &UseItemOnBlockContext, stack: &StackData, env: &tool_stack::Environment, modules: &tool_stack::TankModules) -> bool {
    let Some(fluid) = tool_tank::fluid(&stack.custom_data) else { return false };
    if !describe(&fluid).map_or(false, |(has_block, _, _)| has_block) { return false; }
    click.player.start_item_cooldown(&stack.item, tool_tank::Splashing::cooldown(tool_stack::draw_speed(&stack.custom_data, env, &crate::entity_facts::Entities::default())));
    let player_entity = EntityReach::id_of_uuid(click.player.uuid).map_or(0, |id| id as i32);
    let player_position = if player_entity > 0 { EntityReach::position_of(player_entity as u64).unwrap_or([0.0, 0.0, 0.0]) } else { [0.0, 0.0, 0.0] };
    let face = direction(click.face);
    let location = [click.pos[0] as f64 + 0.5, click.pos[1] as f64 + 0.5, click.pos[2] as f64 + 0.5];
    let world = World::new(&click.dimension);
    let mut working = fluid.clone();
    let mut total_damage = 0;
    for (entry, splashing) in &modules.splashing {
        if working.amount == 0 { break; }
        let level = splashing.strength(entry);
        let holder = self_holder(player_entity, true, stack, player_position);
        let consumed = apply_block(&click.dimension, &working, level, holder, click.pos, face, location);
        if consumed <= 0 { continue; }
        let mut targets = 1;
        fluid_world::spawn_fluid_particles(&world, &working, location, 10);
        working.amount = working.amount.saturating_sub(consumed as u32);
        // The area around the click, through the tool's own AOE iterator (the yaw and pitch
        // the telling does not carry read as level, the harvest consumer's shape).
        if let Some(iterator) = env.definition.and_then(|definition| definition.aoe()) {
            if let Some(ticking) = World::ticking() {
                let expanded = expanded_of(&stack.custom_data);
                let block_at = |pos: [i32; 3]| ticking.get_block_state(pos).map(|state| state.block.clone());
                let in_tag_at = |pos: [i32; 3], tag: &Identifier| ticking.get_block_state(pos).is_some_and(|state| state.is_in_tag(&tag.to_string()));
                for pos in iterator.positions(click.pos, face, 0.0, 0.0, expanded, &block_at, &in_tag_at) {
                    if working.amount == 0 { break; }
                    if pos == click.pos { continue; }
                    let holder = self_holder(player_entity, true, stack, player_position);
                    let at = [pos[0] as f64 + 0.5, pos[1] as f64 + 0.5, pos[2] as f64 + 0.5];
                    let consumed = apply_block(&click.dimension, &working, level, holder, pos, face, at);
                    if consumed > 0 {
                        targets += 1;
                        fluid_world::spawn_fluid_particles(&world, &working, at, 10);
                        working.amount = working.amount.saturating_sub(consumed as u32);
                    }
                }
            }
        }
        total_damage += tool_tank::Splashing::damage(targets, level);
    }
    if total_damage == 0 { return false; }
    let mut slot = slot_of(stack);
    if working.amount == 0 { tool_tank::set_fluid(&mut slot.custom_data, None); } else { tool_tank::set_fluid(&mut slot.custom_data, Some(&working)); }
    let _ = tool_slot::damage_slot(&mut slot, env, total_damage, true, true, None, &mut tool_damage_hook::random);
    write_back_hand(&click.player, stack, &slot);
    true
}

/// The smeltery tags' read for a bucket module's fluid predicate (a tag the data does not name matches nothing).
fn bucket_in_tag() -> impl Fn(&Identifier, &Identifier) -> bool {
    let tags = common::smeltery_data().and_then(|data| data.read().ok()).map(|data| data.tags.clone());
    move |fluid: &Identifier, tag: &Identifier| tags.as_ref().map_or(false, |tags| tags.contains(fluid, tag))
}

/// A bucket's sound for the fluid: its def's own, else the plain bucket's.
fn bucket_sound(world: &World, fluid: &FluidStack, at: [f64; 3], filling: bool) {
    let sound = pluto_lib::FluidRegistry::get(&fluid.fluid.to_string())
        .and_then(|def| if filling { def.bucket_fill_sound.clone() } else { def.bucket_empty_sound.clone() })
        .unwrap_or_else(|| if filling { "minecraft:item.bucket.fill".to_owned() } else { "minecraft:item.bucket.empty".to_owned() });
    world.play_sound(&sound, at, 1.0, 1.0);
}

/// A bucket volume leaves the tank, written back to the hand the tool is in. The original drains the tank even
/// in creative — it checks no game mode — and damages no tool.
fn drain_bucket(player: &ServerPlayer, stack: &StackData) -> bool {
    let mut slot = slot_of(stack);
    if !tool_tank::Bucket::place_consume(&mut slot.custom_data) { return false; }
    write_back_hand(player, stack, &slot);
    true
}

/// BucketModule.afterBlockUse (the place half, sneaking only): a bucket tool pours its tank's fluid as a source
/// block at the clicked face's offset. The port has no `LiquidBlockContainer` (cauldron) blocks, so the clicked
/// block never contains the fluid and the target is always the offset; the `mayInteract` re-check is not
/// repeated, as the host already gates the block use. Water placed in the Nether vaporizes instead of placing
/// (the ultra-warm check the port can see: only the Nether is a known ultra-warm dimension). A source of the
/// same fluid already there counts as placed with no block set. A replaceable solid is broken with its drops
/// first (the set is queued to follow the queued break); air and a fluid cell are set at once. A mod fluid
/// asks for its first flow tick; the host flows water and lava itself.
fn bucket_place(click: &UseItemOnBlockContext, stack: &StackData, modules: &tool_stack::TankModules) -> bool {
    let Some(bucket) = modules.bucket.first() else { return false };
    let Some(fluid) = tool_tank::fluid(&stack.custom_data) else { return false };
    let in_tag = bucket_in_tag();
    if !bucket.can_place(Some(&fluid), &in_tag) { return false; }
    let Some(source) = FluidWorld::source_block(&fluid.fluid) else { return false };
    let target = [click.pos[0] + click.face[0], click.pos[1] + click.face[1], click.pos[2] + click.face[2]];
    let world = World::new(&click.dimension);
    let centre = [target[0] as f64 + 0.5, target[1] as f64 + 0.5, target[2] as f64 + 0.5];
    // Water placed in the Nether vaporizes: no block, the extinguish sound and smoke, the tank still drained.
    if fluid.fluid.to_string() == "minecraft:water" && click.dimension == "minecraft:the_nether" {
        let pitch = 2.6 + (world.random_float().unwrap_or(0.5) - world.random_float().unwrap_or(0.5)) * 0.8;
        world.play_sound("minecraft:block.fire.extinguish", centre, 0.5, pitch);
        world.send_particles("minecraft:large_smoke", &pluto_lib::ParticleData::Plain, centre, [0.25, 0.25, 0.25], 0.1, 8);
        return drain_bucket(&click.player, stack);
    }
    let Some(existing) = world.get_block_state(target) else { return false };
    if FluidWorld::fluid_of(&existing) == Some((fluid.fluid.clone(), 0)) {
        bucket_sound(&world, &fluid, centre, false);
        return drain_bucket(&click.player, stack);
    }
    if !crate::fluid_world::replaceable(&existing) { return false; }
    let is_air = matches!(existing.block.to_string().as_str(), "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air");
    let is_fluid = FluidWorld::fluid_of(&existing).is_some();
    if !is_air && !is_fluid {
        world.break_block(target, true);
        if !world.set_block_state_queued(target, &source) { return false; }
    } else if !world.set_block_state(target, &source) {
        return false;
    }
    if fluid.fluid.namespace() != "minecraft" {
        if let Some(def) = pluto_lib::FluidRegistry::get(&fluid.fluid.to_string()) {
            world.schedule_tick(target, &FluidWorld::block_id(&fluid.fluid).to_string(), i32::from(def.flow.tick_delay.max(1)));
        }
    }
    bucket_sound(&world, &fluid, centre, false);
    drain_bucket(&click.player, stack)
}

/// A use on a block: TankInteractionModule's before-use first (the right-click source for a
/// held tool), then BucketModule's place half (sneaking), then SplashingModule's after-use.
/// The pickup half runs on the release telling (see `on_release`): a plain use on a fluid
/// source block starts the use whose release picks it up.
fn on_block_use(click: &UseItemOnBlockContext) -> ActionResult {
    let Some(held) = &click.held else {
        // The armor-sourced form: an empty hand, the chest piece's module answering for the
        // wearer (bucketing's shipped interaction_source is "armor").
        let Some(facts) = player_facts(click.player.uuid) else { return ActionResult::Pass };
        let Some((_, chest)) = facts.armor.iter().find(|(slot, _)| slot == "chest") else { return ActionResult::Pass };
        let chest = chest.clone();
        let Ok(result) = with_held_tool(&chest, |_env, tool| {
            let modules = tool_stack::tank_modules(&tool.custom_data, _env);
            if modules.tank_interaction.is_empty() { return ActionResult::Pass; }
            tank_trade(click, &chest, &modules, tool_tank::InteractionSource::Armor, &|work: &NbtCompound| {
                let mut after = chest.clone();
                after.custom_data = work.clone();
                click.player.update_stack(armor_slot_number("chest"), &after);
            }).unwrap_or(ActionResult::Pass)
        }) else { return ActionResult::Pass };
        return result;
    };
    let Ok(result) = with_held_tool(held, |env, tool| {
        let modules = tool_stack::tank_modules(&tool.custom_data, env);
        if !modules.tank_interaction.is_empty() {
            let traded = tank_trade(click, held, &modules, tool_tank::InteractionSource::RightClick, &|work: &NbtCompound| {
                let mut slot = slot_of(held);
                slot.custom_data = work.clone();
                write_back_hand(&click.player, held, &slot);
            });
            if let Some(result) = traded { return result; }
        }
        if click.sneaking && !tool_stack::is_broken(&tool.custom_data) && bucket_place(click, held, &modules) {
            return ActionResult::Success;
        }
        if !modules.splashing.is_empty() && !tool_stack::is_broken(&tool.custom_data) && splash_block(click, held, env, &modules) {
            return ActionResult::Success;
        }
        ActionResult::Pass
    }) else { return ActionResult::Pass };
    result
}

/* The composed-use gates (throwing_hook's registration asks these) */

/// Whether the stack can start a slurp, and the drink's duration when it can
/// (SlurpingModule.onToolUse's gates): a slurping module, unbroken, a tank fluid with entity
/// effects. The original's SIMULATE slurp needs the drinker as a live target, which the
/// composition cannot build; a fluid that applies nothing finishes dry instead.
pub(crate) fn slurpable(stack: &StackData) -> Option<i32> {
    with_held_tool(stack, |env, tool| {
        if tool_stack::is_broken(&tool.custom_data) { return None; }
        let modules = tool_stack::tank_modules(&tool.custom_data, env);
        let (entry, slurping) = modules.slurping.first()?;
        let fluid = tool_tank::fluid(&tool.custom_data)?;
        if !describe(&fluid).map_or(false, |(_, has_entity, _)| has_entity) { return None; }
        Some(slurping.duration(entry))
    }).ok().flatten()
}

/// Whether the stack can start a spit (SpittingModule.onToolUse's gates).
pub(crate) fn spittable(stack: &StackData) -> bool {
    with_held_tool(stack, |env, tool| {
        if tool_stack::is_broken(&tool.custom_data) { return false; }
        let modules = tool_stack::tank_modules(&tool.custom_data, env);
        let Some((entry, _)) = modules.spitting.first() else { return false };
        let fluid = tool_tank::fluid(&tool.custom_data);
        let has_effects = fluid.as_ref().map_or(false, |fluid| describe(fluid).map_or(false, |(has_block, has_entity, _)| has_block || has_entity));
        tool_tank::Spitting::can_start(entry, fluid.as_ref(), has_effects)
    }).unwrap_or(false)
}

/// Whether the stack can start a bucket pickup (BucketModule.onToolUse's tank gates): a bucket
/// module, unbroken, a tank the bucket can still fill (a bucket's worth of free space, a stored
/// fluid the predicate allows and carrying no tag). The world half — the ray trace to a source —
/// runs on the release.
pub(crate) fn bucket_pickupable(stack: &StackData) -> bool {
    with_held_tool(stack, |env, tool| {
        if tool_stack::is_broken(&tool.custom_data) { return false; }
        let modules = tool_stack::tank_modules(&tool.custom_data, env);
        let Some(bucket) = modules.bucket.first() else { return false };
        bucket.can_pickup(&tool.custom_data, &bucket_in_tag())
    }).unwrap_or(false)
}

/* TankModule's slot interaction (the tconstruct:slot_stack hook) */

/// A tool's tank as a fluid target over a clone of its data, for the container transfers.
struct ToolTankTarget<'a> {
    nbt: &'a mut NbtCompound,
}

impl pluto_lib::FluidTarget for ToolTankTarget<'_> {
    fn fill(&mut self, fluid: &FluidStack, execute: bool) -> u32 {
        tool_tank::fill(self.nbt, fluid, execute).max(0) as u32
    }
    fn drain(&mut self, wanted: Option<&Identifier>, amount: u32, execute: bool) -> Option<FluidStack> {
        match wanted {
            Some(wanted) => {
                let current = tool_tank::fluid(self.nbt)?;
                if current.fluid != *wanted { return None; }
                tool_tank::drain_fluid(self.nbt, &FluidStack { amount, ..current }, execute)
            }
            None => tool_tank::drain(self.nbt, amount.min(i32::MAX as u32) as i32, execute),
        }
    }
}

/// Whether a menu stack is a tool carrying a tank.
fn is_tank_tool(slot: &ItemSlot) -> bool {
    with_held_tool(&data_of(slot), |env, tool| tool_tank::has_tank(&tool.custom_data, env)).unwrap_or(false)
}

/// TankItem.mayHaveFluid for a menu stack: a registered fluid container, or a tank tool.
fn may_have_fluid(slot: &ItemSlot) -> bool {
    FluidItems::is_container(slot) || is_tank_tool(slot)
}

/// Trades between a tool's tank (its data, changed in place) and a container stack, one
/// direction first: `reverse` fills the container from the tank before filling the tank (the
/// stacked-on-other direction), otherwise the tank first (the other-stack direction, the
/// port's Auto reading). Returns the container afterwards and whether the fluid went into
/// the tank; None when nothing moved either way.
fn transfer_with_container(tool_nbt: &mut NbtCompound, container: &ItemSlot, reverse: bool) -> Option<(ItemSlot, bool)> {
    let mut target = ToolTankTarget { nbt: tool_nbt };
    let (first, second) = if reverse {
        (pluto_lib::TransferDirection::FromTank, pluto_lib::TransferDirection::ToTank)
    } else {
        (pluto_lib::TransferDirection::ToTank, pluto_lib::TransferDirection::FromTank)
    };
    let interaction = FluidItems::interact(Some(container), &mut target, first)
        .or_else(|| FluidItems::interact(Some(container), &mut target, second))?;
    Some((interaction.held?, interaction.into_tank))
}

/// Moves fluid between two tools' tanks, as much of the source's fluid as the target takes.
/// Whether anything moved.
fn move_fluid(from: &mut NbtCompound, to: &mut NbtCompound) -> bool {
    let Some(fluid) = tool_tank::fluid(from) else { return false };
    let taken = tool_tank::fill(to, &fluid, false);
    if taken <= 0 { return false; }
    let Some(drained) = tool_tank::drain_fluid(from, &FluidStack { amount: taken as u32, ..fluid }, true) else { return false };
    tool_tank::fill(to, &drained, true) == drained.amount as i32
}

/// Trades between two tools' tanks, one direction first as transfer_with_container orders
/// them. Returns whether the fluid went into the carried tool's tank; None when nothing moved.
fn transfer_tools(carried: &mut NbtCompound, clicked: &mut NbtCompound, reverse: bool) -> Option<bool> {
    if reverse {
        if move_fluid(carried, clicked) { return Some(false); }
        if move_fluid(clicked, carried) { return Some(true); }
    } else {
        if move_fluid(clicked, carried) { return Some(true); }
        if move_fluid(carried, clicked) { return Some(false); }
    }
    None
}

/// The transfer's sound: the container's own in the original; the bucket pair stands in, as
/// the block trades in fluid_items leave the same sound unplayed — here the decision carries
/// one, so it is named. Into the tank is the container emptying; out of it, filling.
fn transfer_sound(into_tank: bool) -> StackedSound {
    if into_tank {
        StackedSound { id: "minecraft:item.bucket.empty", volume: 1.0, pitch: 1.0 }
    } else {
        StackedSound { id: "minecraft:item.bucket.fill", volume: 1.0, pitch: 1.0 }
    }
}

/// Trades a carried container with a clicked tank tool, one unit at a time (TankItem.updateHeldItem's
/// split): the transfer runs on a single unit of the carried stack; when the carried stack holds more
/// than one, the filled or emptied unit is handed to the player (`give`) and the rest stays carried.
/// Returns the carried stack afterwards, the stack to give (when the carried stack had more than one),
/// and whether the fluid went into the tank; None when nothing moved.
fn transfer_carried_container(tool_nbt: &mut NbtCompound, carried: &ItemSlot) -> Option<(ItemSlot, Option<ItemSlot>, bool)> {
    let mut single = carried.clone();
    single.count = 1;
    let (result, into_tank) = transfer_with_container(tool_nbt, &single, false)?;
    if carried.count == 1 {
        Some((result, None, into_tank))
    } else {
        let mut rest = carried.clone();
        rest.count -= 1;
        Some((rest, Some(result), into_tank))
    }
}

/// TankModule's two slot hooks as one callback (the telling asks the carried item only, so
/// both directions are decided here): overrideStackedOnOther when the carried stack is the
/// tool, overrideOtherStackedOnMe when the clicked stack is. Both answer the click (Consume,
/// or Apply when fluid moved) whenever the other stack may have fluid, as the original
/// returns true on that gate alone. A carried container of more than one in the second
/// direction splits in the original (the filled one is handed to the player); the decision's
/// `give` carries that split unit, and the transport hands it to the clicking player.
fn on_stacked_on_other(request: &StackedOnOtherRequest) -> StackedOnOtherDecision {
    if is_tank_tool(request.carried) {
        let Some(clicked) = request.clicked else { return StackedOnOtherDecision::Pass };
        if !may_have_fluid(clicked) { return StackedOnOtherDecision::Pass; }
        if clicked.count != 1 { return StackedOnOtherDecision::Consume; }
        let mut tool_nbt = request.carried.custom_data.clone();
        if is_tank_tool(clicked) {
            let mut clicked_nbt = clicked.custom_data.clone();
            return match transfer_tools(&mut tool_nbt, &mut clicked_nbt, true) {
                Some(into_carried) => {
                    let mut carried_after = request.carried.clone();
                    carried_after.custom_data = tool_nbt;
                    let mut clicked_after = clicked.clone();
                    clicked_after.custom_data = clicked_nbt;
                    StackedOnOtherDecision::Apply { clicked: Some(clicked_after), carried: Some(carried_after), give: None, sound: Some(transfer_sound(into_carried)) }
                }
                None => StackedOnOtherDecision::Consume,
            };
        }
        return match transfer_with_container(&mut tool_nbt, clicked, true) {
            Some((container_after, into_tank)) => {
                let mut carried_after = request.carried.clone();
                carried_after.custom_data = tool_nbt;
                StackedOnOtherDecision::Apply { clicked: Some(container_after), carried: Some(carried_after), give: None, sound: Some(transfer_sound(into_tank)) }
            }
            None => StackedOnOtherDecision::Consume,
        };
    }
    if let Some(clicked) = request.clicked {
        if is_tank_tool(clicked) {
            if !may_have_fluid(request.carried) { return StackedOnOtherDecision::Pass; }
            let mut tool_nbt = clicked.custom_data.clone();
            if is_tank_tool(request.carried) {
                let mut carried_nbt = request.carried.custom_data.clone();
                return match transfer_tools(&mut carried_nbt, &mut tool_nbt, false) {
                    Some(into_carried) => {
                        let mut clicked_after = clicked.clone();
                        clicked_after.custom_data = tool_nbt;
                        let mut carried_after = request.carried.clone();
                        carried_after.custom_data = carried_nbt;
                        StackedOnOtherDecision::Apply { clicked: Some(clicked_after), carried: Some(carried_after), give: None, sound: Some(transfer_sound(into_carried)) }
                    }
                    None => StackedOnOtherDecision::Consume,
                };
            }
            return match transfer_carried_container(&mut tool_nbt, request.carried) {
                Some((carried_after, give, into_tank)) => {
                    let mut clicked_after = clicked.clone();
                    clicked_after.custom_data = tool_nbt;
                    StackedOnOtherDecision::Apply { clicked: Some(clicked_after), carried: Some(carried_after), give, sound: Some(transfer_sound(into_tank)) }
                }
                None => StackedOnOtherDecision::Consume,
            };
        }
    }
    StackedOnOtherDecision::Pass
}

/// Every modifiable tool and armor item (the tool_definitions folder's ids), for the slot
/// interaction's registration: the callback must be registered for a stack to be asked as the
/// carried one, in either direction.
const TOOL_ITEMS: &[&str] = &[
    "arrow", "battlesign", "broad_axe", "cleaver", "crossbow", "dagger", "earth_staff", "ender_staff",
    "excavator", "fishing_rod", "flint_and_brick", "hand_axe", "ichor_staff", "javelin", "kama", "longbow",
    "mattock", "melting_pan", "minotaur_axe", "pickadze", "pickaxe", "plate_boots", "plate_chestplate",
    "plate_helmet", "plate_leggings", "plate_shield", "scythe", "shuriken", "sky_staff", "sledge_hammer",
    "slime_boots", "slime_chestplate", "slime_helmet", "slime_leggings", "slime_wings", "swasher", "sword",
    "throwing_axe", "travelers_boots", "travelers_chestplate", "travelers_helmet", "travelers_leggings",
    "travelers_shield", "vein_hammer", "war_pick",
];

/// Registers the tank consumers. Called once from the product's hooks, after the fluid items
/// are registered (the slot interaction asks their registry).
pub(crate) fn register() {
    ServerLivingEntityEvents::WEAPON_HIT_DEALT.register(on_weapon_hit_dealt);
    ServerLivingEntityEvents::MODIFY_DAMAGE.register(on_damage_modification);
    ServerLivingEntityEvents::PLAYER_TICK.register(on_player_tick);
    PlayerInventoryEvents::TICK.register(on_inventory_tick);
    ItemEvents::USE_TICK.register(on_use_tick);
    ItemEvents::RELEASE_USING.register(on_release);
    UseItemFinishCallback::EVENT.register(on_use_finish);
    UseEntityCallback::EVENT.register(on_entity_use);
    UseItemOnBlockCallback::EVENT.register(on_block_use);
    for path in TOOL_ITEMS {
        if let Ok(item) = Identifier::new("tconstruct", path) {
            StackedOnOtherRegistry::register(item, on_stacked_on_other);
        }
    }
    for def in FluidItems::registered() {
        StackedOnOtherRegistry::register(def.item.clone(), on_stacked_on_other);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{materials::Definitions as Materials, modifiers::Registry, tool_definitions::Data};
    use serde_json::json;

    struct Always;
    impl crate::materials::Conditions for Always {
        type Parsed = ();
        fn decode(&self, _: &serde_json::Value) -> Result<(), String> { Ok(()) }
        fn test(&self, _: &()) -> Result<bool, String> { Ok(true) }
    }

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }

    /// The tank handler (code) and the shipped tank modifiers, from their own JSON shapes.
    fn registry() -> Registry {
        let mut registry = Registry::default();
        for modifier in tool_tank::code_modifiers().expect("code modifiers") {
            registry.register_static(modifier).expect("the handler registers");
        }
        registry.reload(&[
            (id("tconstruct:spilling"), json!({"priority": 1000, "modules": [{"type": "tconstruct:spilling", "each_level": 1.0}]})),
            (id("tconstruct:slurping"), json!({"priority": 40, "modules": [{"type": "tconstruct:slurping", "strength": {"each_level": 1.0}, "duration": {"flat": 21}}]})),
            (id("tconstruct:spitting"), json!({"priority": 120, "modules": [{"type": "tconstruct:spitting", "shots": {"each_level": 1}}]})),
            (id("tconstruct:splashing"), json!({"modules": [{"type": "tconstruct:splashing", "strength": {"each_level": 1.0}}]})),
        ], &Always);
        registry
    }

    fn environment<'a>(data: &'a Data, materials: &'a Materials, registry: &'a Registry) -> tool_stack::Environment<'a> {
        tool_stack::Environment { definition: Some(data), has_materials: false, materials, modifiers: registry, item_tags_loaded: true, item: None, item_tags: None }
    }

    fn loaded_materials() -> Materials {
        let mut materials = Materials::default();
        materials.server_loaded = true;
        materials
    }

    /// A tool's custom data: the modifiers saved through the modifier list, a tank capacity
    /// stat of 2000, and the tank's fluid when named.
    fn tool_data(modifiers: &[(&str, i32)], fluid: Option<(&str, u32)>) -> NbtCompound {
        let mut nbt = NbtCompound::new();
        let mut stats = NbtCompound::new();
        stats.insert("tconstruct:tank_capacity".into(), Nbt::Float(2000.0));
        nbt.insert(tool_stack::TAG_STATS.into(), Nbt::Compound(stats));
        let mut list = crate::modifier_entries::List::default();
        for (name, level) in modifiers { list = list.with_modifier(id(name), *level).expect("modifier list"); }
        nbt.insert(tool_stack::TAG_MODIFIERS.into(), list.to_nbt());
        if let Some((fluid, amount)) = fluid { tool_tank::set_fluid(&mut nbt, Some(&FluidStack::new(id(fluid), amount))); }
        nbt
    }

    #[test]
    fn tank_modules_enumerates_the_tank_family_and_the_profile_carries_the_spill() {
        let registry = registry();
        let materials = loaded_materials();
        let data = Data::default();
        let env = environment(&data, &materials, &registry);
        let nbt = tool_data(
            &[("tconstruct:tank_handler", 1), ("tconstruct:spilling", 2), ("tconstruct:slurping", 1)],
            Some(("tconstruct:molten_iron", 500)),
        );
        let modules = tool_stack::tank_modules(&nbt, &env);
        assert_eq!(modules.spilling, vec![2.0], "each_level 1.0 at entry level 2");
        assert_eq!(modules.slurping.len(), 1);
        assert_eq!(modules.slurping[0].1.duration(&modules.slurping[0].0), 21);
        assert!(modules.spitting.is_empty() && modules.splashing.is_empty());
        assert!(!modules.is_empty());
        // The flight profile carries the same spill with the tank's fluid snapshot.
        let profile = tool_stack::flight_profile(&nbt, &env);
        assert_eq!(profile.spilling.len(), 1);
        assert_eq!(profile.spilling[0].0, 2.0);
        assert_eq!(profile.spilling[0].1.as_ref().map(|fluid| (fluid.fluid.to_string(), fluid.amount)), Some(("tconstruct:molten_iron".into(), 500)));
        // A tool with none of the family enumerates nothing.
        let bare = tool_data(&[("tconstruct:tank_handler", 1)], None);
        assert!(tool_stack::tank_modules(&bare, &env).is_empty());
    }

    #[test]
    fn the_armor_timer_sounds_on_the_fourth_tick_and_finishes_past_its_end() {
        let slurping = tool_tank::Slurping::decode(json!({"strength": {"each_level": 1.0}, "duration": {"flat": 21}}).as_object().unwrap()).expect("slurping decodes");
        let entry = Entry::new(id("tconstruct:slurping"), 1);
        let mut nbt = tool_data(&[("tconstruct:slurping", 1)], Some(("tconstruct:venom", 250)));
        // startInteract at tick 100: the finish lands at 121.
        tool_stack::with_persistent(&mut nbt, |persistent| slurping.armor_start(persistent, &entry, 100));
        let tick = |nbt: &mut NbtCompound, now: i32| tool_stack::with_persistent(nbt, |persistent| slurping.armor_tick(persistent, &entry, now));
        assert_eq!(tick(&mut nbt, 105), tool_tank::ArmorTick::Sound, "16 left, a multiple of 4");
        assert_eq!(tick(&mut nbt, 120), tool_tank::ArmorTick::Idle, "1 left");
        assert_eq!(tick(&mut nbt, 122), tool_tank::ArmorTick::Finished, "past the finish");
        // The finish cleared the key: the timer is idle from here.
        assert_eq!(tick(&mut nbt, 123), tool_tank::ArmorTick::Idle);
        // stopInteract clears a live timer without finishing it.
        tool_stack::with_persistent(&mut nbt, |persistent| slurping.armor_start(persistent, &entry, 200));
        tool_stack::with_persistent(&mut nbt, |persistent| slurping.stop(persistent, &entry));
        assert_eq!(tick(&mut nbt, 205), tool_tank::ArmorTick::Idle);
    }

    #[test]
    fn tool_to_tool_transfers_follow_the_direction_priority() {
        let mut carried = tool_data(&[("tconstruct:tank_handler", 1)], Some(("tconstruct:molten_iron", 1500)));
        let mut clicked = tool_data(&[("tconstruct:tank_handler", 1)], None);
        // Reverse (the carried tool clicked onto the other): the carried tank fills the
        // clicked one first.
        assert_eq!(transfer_tools(&mut carried, &mut clicked, true), Some(false));
        // A full drain removes the tank key: an emptied tank reads as no fluid, not zero.
        assert_eq!(tool_tank::fluid(&carried).map(|fluid| fluid.amount), None);
        assert_eq!(tool_tank::fluid(&clicked).map(|fluid| fluid.amount), Some(1500));
        // Nothing left to move out of an empty tank.
        let mut empty = tool_data(&[("tconstruct:tank_handler", 1)], None);
        assert_eq!(transfer_tools(&mut empty, &mut clicked, true), Some(true), "the reverse falls to filling the carried tank");
        assert_eq!(tool_tank::fluid(&empty).map(|fluid| fluid.amount), Some(1500));
        assert_eq!(tool_tank::fluid(&clicked).map(|fluid| fluid.amount), None);
        // And with both empty, nothing moves.
        let mut other_empty = tool_data(&[("tconstruct:tank_handler", 1)], None);
        assert_eq!(transfer_tools(&mut clicked, &mut other_empty, false), None);
    }

    #[test]
    fn a_partial_transfer_moves_only_what_the_target_takes() {
        let mut carried = tool_data(&[("tconstruct:tank_handler", 1)], Some(("tconstruct:molten_iron", 1500)));
        let mut clicked = tool_data(&[("tconstruct:tank_handler", 1)], Some(("tconstruct:molten_iron", 1200)));
        // The clicked tank holds 1200 of its 2000: only 800 fits.
        assert!(move_fluid(&mut carried, &mut clicked));
        assert_eq!(tool_tank::fluid(&carried).map(|fluid| fluid.amount), Some(700));
        assert_eq!(tool_tank::fluid(&clicked).map(|fluid| fluid.amount), Some(2000));
        // A different fluid cannot move into an occupied tank.
        let mut other = tool_data(&[("tconstruct:tank_handler", 1)], Some(("tconstruct:molten_gold", 100)));
        assert!(!move_fluid(&mut other, &mut clicked));
    }

    #[test]
    fn container_transfers_run_through_the_fluid_items() {
        crate::fluids::register();
        crate::fluid_items::register();
        // The carried tool clicked onto a glass bottle (reverse): the bottle fills from the
        // tank first — a bottle of venom is 250 of the tank's 500.
        let mut tool = tool_data(&[("tconstruct:tank_handler", 1)], Some(("tconstruct:venom", 500)));
        let bottle = ItemSlot::new(id("minecraft:glass_bottle"), 1);
        let (container, into_tank) = transfer_with_container(&mut tool, &bottle, true).expect("the bottle fills");
        assert!(!into_tank, "the fluid left the tank");
        assert_ne!(container.item.to_string(), "minecraft:glass_bottle", "the bottle is filled: {}", container.item);
        assert_eq!(tool_tank::fluid(&tool).map(|fluid| fluid.amount), Some(250));
        // The other direction: a filled bottle clicked onto an empty tool (auto) empties
        // into the tank.
        let mut empty_tool = tool_data(&[("tconstruct:tank_handler", 1)], None);
        let (container, into_tank) = transfer_with_container(&mut empty_tool, &container, false).expect("the bottle empties");
        assert!(into_tank);
        assert_eq!(container.item.to_string(), "minecraft:glass_bottle");
        assert_eq!(tool_tank::fluid(&empty_tool).map(|fluid| (fluid.fluid.to_string(), fluid.amount)), Some(("tconstruct:venom".into(), 250)));
    }

    #[test]
    fn a_carried_stack_of_containers_splits_one_off_to_the_player() {
        crate::fluids::register();
        crate::fluid_items::register();
        // Three empty bottles onto a tool holding 500 venom: one fills (250) and is handed to the
        // player, two stay carried, the tank keeps 250.
        let mut tool = tool_data(&[("tconstruct:tank_handler", 1)], Some(("tconstruct:venom", 500)));
        let bottles = ItemSlot::new(id("minecraft:glass_bottle"), 3);
        let (carried, give, into_tank) = transfer_carried_container(&mut tool, &bottles).expect("one bottle fills");
        assert!(!into_tank, "the fluid left the tank");
        assert_eq!(carried.count, 2, "the rest stays carried");
        assert_eq!(carried.item, bottles.item);
        let given = give.expect("the filled bottle is handed to the player");
        assert_eq!(given.count, 1);
        assert_ne!(given.item.to_string(), "minecraft:glass_bottle", "the given bottle is filled");
        assert_eq!(tool_tank::fluid(&tool).map(|fluid| fluid.amount), Some(250));
        // A single bottle: no split, the filled bottle is the carried one.
        let mut tool = tool_data(&[("tconstruct:tank_handler", 1)], Some(("tconstruct:venom", 500)));
        let bottle = ItemSlot::new(id("minecraft:glass_bottle"), 1);
        let (carried, give, _) = transfer_carried_container(&mut tool, &bottle).expect("the bottle fills");
        assert!(give.is_none(), "a single container stays carried");
        assert_ne!(carried.item.to_string(), "minecraft:glass_bottle");
        // Nothing to move: no split, no give.
        let mut empty_tool = tool_data(&[("tconstruct:tank_handler", 1)], None);
        assert!(transfer_carried_container(&mut empty_tool, &bottles).is_none());
    }

    #[test]
    fn the_pickup_ray_finds_a_source_and_stops_at_a_solid_block() {
        let world = World::new("minecraft:overworld");
        let mut ops = pluto_lib::MemoryWorldOps::default();
        ops.blocks.insert(("minecraft:overworld".to_owned(), [0, 64, 0]), "minecraft:air".to_owned());
        ops.blocks.insert(("minecraft:overworld".to_owned(), [0, 64, 1]), "minecraft:water[level=3]".to_owned());
        ops.blocks.insert(("minecraft:overworld".to_owned(), [0, 64, 2]), "minecraft:water[level=0]".to_owned());
        // A source two blocks ahead, flowing water in between: the trace passes the flowing cell.
        let found = pluto_lib::with_world(&mut ops, || ray_trace_source(&world, [0.5, 64.5, 0.5], [0.0, 0.0, 1.0], 4.5));
        assert_eq!(found, Some([0, 64, 2]));
        // A stone wall in between stops it.
        ops.blocks.insert(("minecraft:overworld".to_owned(), [0, 64, 1]), "minecraft:stone".to_owned());
        let found = pluto_lib::with_world(&mut ops, || ray_trace_source(&world, [0.5, 64.5, 0.5], [0.0, 0.0, 1.0], 4.5));
        assert_eq!(found, None, "a solid block stops the trace");
        // No source that way: nothing.
        let found = pluto_lib::with_world(&mut ops, || ray_trace_source(&world, [0.5, 64.5, 0.5], [0.0, 0.0, -1.0], 4.5));
        assert_eq!(found, None);
    }

    #[test]
    fn the_tank_write_back_is_one_persistent_edit() {
        let before = tool_data(&[("tconstruct:tank_handler", 1)], Some(("tconstruct:molten_iron", 500)));
        let mut after = before.clone();
        assert_eq!(tool_tank::consume(&mut after, 200), 200);
        let edits = tank_edits(&before, &after);
        assert_eq!(edits.len(), 1, "only the tank's key changed: {edits:?}");
        assert_eq!(edits[0].path, vec![tool_stack::TAG_PERSISTENT.to_string(), tool_tank::FLUID_KEY.to_string()]);
        assert!(edits[0].value.is_some(), "a partial spend keeps the key");
        let mut emptied = before.clone();
        assert_eq!(tool_tank::consume(&mut emptied, 500), 500);
        let edits = tank_edits(&before, &emptied);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].value.is_none(), "an emptied tank removes the key");
    }

    #[test]
    fn the_fan_rotates_around_the_up_axis() {
        assert_eq!(rotate_up([0.0, 0.0, 1.0], 0.0), [0.0, 0.0, 1.0]);
        let turned = rotate_up([0.0, 0.0, 1.0], 90.0);
        assert!(turned[0].abs() > 0.99 && turned[2].abs() < 0.01, "a quarter turn takes +z toward +x: {turned:?}");
        assert_eq!(rotate_up([0.0, 1.0, 0.0], 35.0), [0.0, 1.0, 0.0], "the up axis itself does not move");
    }
}
