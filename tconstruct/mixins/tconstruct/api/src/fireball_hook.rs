//! FireballModule firing and the CustomFireball entity (TinkerModifiers' `fireball`).
//!
//! The entity is a derived type over the game's own fireball body, registered like the
//! fluid spit is: no content declaration, just the registration and the spawn. A use of
//! a tool carrying the module consumes one carried ammo an option matches (the first
//! such stack in the inventory, whose matching option decides the shot's type) and
//! summons the fireball from the player's eyes with the tool's projectile damage as its
//! power. On a hit it hurts by the original's own formula — the power times the damage
//! multiplier times the motion's length, rounded up, through the tool's projectile hit
//! hooks — and discards; the game's own fireball hit never runs. Its saved data carries
//! the original's keys (`damage_power`, `damage_multiplier`, `damage_type`,
//! `enderference_type`) plus the carried item, whose name names the fireball.
//!
//! The firing rides the release telling, not the press: a fireball tool's use is given
//! the long use time in the composed registration, so a click and a hold both end in
//! one release and one shot, where the original shoots at the press and would shoot
//! again per press.
//!
//! The helmet path rides the armor-interact packet instead (the Z key with the normal
//! tooltip key): the worn helmet shoots from the wearer's aim as of their last tick,
//! and the `tconstruct:fireball_cooldown` effect gates the next shot for the tool's
//! drawtime. The shot's added modifiers ride the merged modifier list (the tool's,
//! plus the module's and the type's ammo modifiers when the module names any), bound
//! as a flight profile so their projectile hooks run; a bounce sends the shot on
//! instead of ending it. The flight ends 64 past either build limit, as the original's
//! checkBelowWorld ends it.
//!
//! The general path's cooldown is the original's `addCooldown`
//! (GeneralInteractionModifierHook, speed factor 1): the tool's item cools down for
//! the tool's drawtime, started through the item-cooldown surface — a release's
//! dispatch holds no game, so the start queues for the release drain, which applies
//! it after the host's own content cooldown, and the host's own use-start gate
//! refuses the next use while it runs. And the helmet path's ammo spend reads the
//! wearer's carried stacks through the acting player, which the packet receiver only
//! reaches if the host names the sender there (a smoke item, not a compile one).
use crate::{
    break_speed_hook::with_held_tool, brush_hook, common,
    modifier_entries::{Entry, List},
    projectile_modules::{clamp_to, Fireball, FireballType},
    sling_hook, tool_damage_hook, tool_slot, tool_stack, tool_stats::Stat,
};
use pluto_lib::{
    player_api::{EffectInstance, ServerPlayer},
    ActionTarget, EntityAction, EntityTypeRegistry, Identifier, ItemEvents, Nbt, NbtCompound,
    ProjectileEvents, ProjectileFlight, ProjectileImpact, ProjectileSpawn, ReleaseUsing,
    StackData, StackSlot, World, spawn_projectile,
};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

/// The fireball entity type (TinkerModifiers.fireball).
const FIREBALL: &str = "tconstruct:fireball";

/// The cooldown effect the helmet path gates on (FireballModule's `fireballCooldownEffect`).
const COOLDOWN_EFFECT: &str = "tconstruct:fireball_cooldown";

/// A shot in flight: what its hit needs that the entity's saved data does not say.
struct Shot {
    power: f32,
    damage_multiplier: f32,
    /// The pair's ranged type, for a hit the enderference effect does not protect.
    damage_type: String,
    /// The pair's melee type, for an enderman the enderference effect protects.
    enderference_type: String,
    /// The tool as fired, for its projectile hit hooks at the hit.
    tool: StackData,
}

static SHOTS: LazyLock<Mutex<HashMap<(String, u32), Shot>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Registers the entity type and the firing, flight and impact callbacks. Called once
/// from the product's hooks, after the throwing hook (whose use registration this
/// composes into).
pub(crate) fn register() {
    if let (Ok(kind), Ok(factory)) = (Identifier::parse(FIREBALL), Identifier::parse("minecraft:fireball")) {
        let _ = EntityTypeRegistry::register_derived(kind, factory, 1.0, 1.0, 4, true);
    }
    ItemEvents::RELEASE_USING.register(on_release);
    ProjectileEvents::FLIGHT.register(on_flight);
    ProjectileEvents::IMPACT.register(on_impact);
}

fn on_release(release: &ReleaseUsing) {
    let _ = with_held_tool(&release.stack, |env, tool| {
        let Some((entry, id, module)) = tool_stack::fireball_module(&tool.custom_data, env) else { return };
        let Some(usage) = launch(&release.player, &release.dimension, release.position, release.look, release.creative, &release.stack, env, &tool.custom_data, &entry, &module) else { return };
        // The original's addCooldown (GeneralInteractionModifierHook.addCooldown, speed
        // factor 1): the tool's item cools down for the tool's drawtime. A release's
        // dispatch holds no game, so the start queues for the release drain; the host's
        // own use-start gate refuses the next use while it runs.
        release.player.start_item_cooldown(&release.stack.item, general_cooldown_ticks(&tool.custom_data, env));
        if release.creative {
            release.set_used();
        } else if usage > 0 {
            sling_hook::spend(release, usage, &id);
        } else {
            release.set_used();
        }
    });
}

/// The general path's cooldown in ticks (FireballModule.onToolUse's addCooldown, speed
/// factor 1): the tool's drawtime.
fn general_cooldown_ticks(nbt: &NbtCompound, env: &tool_stack::Environment) -> i32 {
    tool_stack::drawtime(nbt, env, &crate::entity_facts::Entities::default(), 1.0)
}

/// The shot's modifier list (FireballModule.shoot): the tool's saved entries, plus the module's
/// and the type's ammo modifiers — only when the module names any of its own (the original's
/// `if (!ammoModifiers.isEmpty())` quirk: an empty module list leaves the tool's list alone,
/// and the type's list with it). Levels add for duplicate ids, as ModifierNBT.Builder.add does.
fn merged_modifiers(nbt: &NbtCompound, module: &Fireball, option: &FireballType) -> List {
    let mut merged = List::from_nbt(nbt.get(tool_stack::TAG_MODIFIERS));
    if !module.ammo_modifiers.is_empty() {
        for (id, level) in module.ammo_modifiers.iter().chain(option.ammo_modifiers.iter()) {
            if *level > 0 {
                if let Ok(next) = merged.with_modifier(id.clone(), *level) { merged = next; }
            }
        }
    }
    merged
}

/// FireballModule.shoot, shared by the release path and the helmet path: find the ammo, spend it,
/// and launch the fireball. Returns the durability one shot costs when a shot was fired.
#[allow(clippy::too_many_arguments)]
fn launch(player: &ServerPlayer, dimension: &str, position: [f64; 3], look: [f64; 3], creative: bool, stack: &StackData, env: &tool_stack::Environment, nbt: &NbtCompound, entry: &Entry, module: &Fireball) -> Option<i32> {
    // BowAmmoModifierHook.consumeAmmo: the first carried stack an option matches; the
    // option matching it decides the type (getFireballType). A creative shooter still
    // needs the ammo carried — the consume finds it the same way — it just is not spent.
    let snapshot = common::item_snapshot();
    let in_tag = |item: &Identifier, tag: &Identifier| {
        snapshot.as_ref().is_some_and(|lookup| lookup.get_tag(tag).is_some_and(|members| members.contains(item)))
    };
    let mut found = None;
    for slot in 0..36 {
        let Some(carried) = player.stack(slot) else { continue };
        if carried.count == 0 {
            continue;
        }
        let Ok(item) = Identifier::parse(&carried.item) else { continue };
        if let Some(option) = module.option_for(&item, &in_tag) {
            found = Some((slot, carried, option.clone()));
            break;
        }
    }
    let Some((slot, ammo, option)) = found else { return None };
    if !creative {
        if ammo.count > 1 {
            let mut shrunk = ammo.clone();
            shrunk.count -= 1;
            player.update_stack(slot, &shrunk);
        } else {
            player.take_item(&ammo.item, 1);
        }
    }
    let world = World::new(dimension);
    let entities = crate::entity_facts::Entities::default();
    let power = tool_stack::conditional_stat(nbt, env, Stat::ProjectileDamage, &|_| false, &entities);
    // The holder's height and the level's build range, for the dwarven velocity half (a shot's
    // holder is not digging, so no dig reports them); the velocity alone reads the holder, so the
    // power above keeps the holder-less entities it always had.
    let holder = pluto_lib::EntityState { y: Some(position[1]), ..Default::default() };
    let velocity_entities = crate::entity_facts::Entities { attacker: Some(&holder), target: None };
    let build_height = Some(crate::mining_speed::level_build_height(dimension));
    let velocity = f64::from(tool_stack::conditional_stat_at(nbt, env, Stat::Velocity, &|_| false, &velocity_entities, build_height));
    let inaccuracy = f64::from(tool_stack::inaccuracy(nbt, env, &entities)) / 16.0;
    // The look scaled by two, jittered on x and z only, the whole scaled by the velocity.
    let mut motion = [
        (look[0] * 2.0 + sling_hook::gaussian(&world) * inaccuracy) * velocity,
        look[1] * 2.0 * velocity,
        (look[2] * 2.0 + sling_hook::gaussian(&world) * inaccuracy) * velocity,
    ];
    // entity.getY(0.5) + 0.5, with the player's height: feet + 0.9 + 0.5.
    let spawn_position = [position[0], position[1] + 1.4, position[2]];
    let (ranged, melee) = option.damage_type.clone().unwrap_or_else(|| module.damage_type.clone());
    let damage_multiplier = module.damage_multiplier * option.damage_multiplier;
    // The profile the shot's hooks run from: the merged modifier list, not the tool's alone
    // (the original runs the projectile hooks over the shot's list), read through a synthetic
    // stack whose modifiers are the merged ones and whose persistent data is the tool's.
    let mut synthetic = nbt.clone();
    synthetic.insert(tool_stack::TAG_MODIFIERS.into(), merged_modifiers(nbt, module, &option).to_nbt());
    let profile = tool_stack::flight_profile(&synthetic, env);
    // RestrictAngleModule clamps the shot's direction at the shoot, as the spawn pass clamps
    // a fired stack's (a summoned fireball has no stack, so the spawn pass cannot do it).
    if let Some(directions) = profile.angle_directions {
        motion = clamp_to(directions, motion);
    }
    let mut data = NbtCompound::new();
    data.insert("damage_power".into(), Nbt::Float(power));
    data.insert("damage_multiplier".into(), Nbt::Float(damage_multiplier));
    data.insert("damage_type".into(), Nbt::String(ranged.to_string()));
    data.insert("enderference_type".into(), Nbt::String(melee.to_string()));
    // The carried item rides as the entity's item, as setItem leaves it.
    let mut item = NbtCompound::new();
    item.insert("id".into(), Nbt::String(ammo.item.clone()));
    item.insert("count".into(), Nbt::Int(1));
    data.insert("Item".into(), Nbt::Compound(item));
    // The tool's projectile launch hooks record their multipliers in the shot's data,
    // as they do for a shot the game fires.
    tool_stack::power_shoot(nbt, env, false, &mut data);
    let Ok(kind) = Identifier::parse(FIREBALL) else { return None };
    let spawn = ProjectileSpawn::new(kind, spawn_position, motion).owner(player.uuid).data(data);
    let Some(projectile) = spawn_projectile(&world, &spawn) else { return None };
    if !profile.is_empty() {
        crate::flight_hook::bind(dimension, projectile, crate::flight_hook::profile_flight(profile));
    }
    if let Ok(mut shots) = SHOTS.lock() {
        shots.insert(
            (dimension.to_owned(), projectile),
            Shot { power, damage_multiplier, damage_type: ranged.to_string(), enderference_type: melee.to_string(), tool: stack.clone() },
        );
    }
    let pitch = 1.0 + (world.random_float().unwrap_or(0.5) - world.random_float().unwrap_or(0.5)) * 0.2;
    world.play_sound(&module.sound.to_string(), position, 2.0, pitch);
    Some(module.usage(entry))
}

/// FireballModule.startInteract: the helmet key (tooltip key normal) shoots a fireball from the
/// worn helmet, then the cooldown effect gates the next shot for the tool's drawtime. True when
/// a shot was fired. The aim is the wearer's as of their last tick (see `aim_of`); without one
/// there is nothing to aim with.
pub(crate) fn on_armor_interact(player_uuid: u128) -> bool {
    let player = ServerPlayer { uuid: player_uuid, name: String::new() };
    // The gate first, as the original's startInteract checks the effect before it shoots.
    if player.effects().iter().any(|effect| effect.id == COOLDOWN_EFFECT) { return false; }
    let Some((_, head)) = pluto_lib::entity_events_v1::equipment_of(player_uuid).into_iter().find(|(slot, _)| slot == "head") else { return false; };
    let Some(aim) = pluto_lib::entity_events_v1::aim_of(player_uuid) else { return false; };
    with_held_tool(&head, |env, tool| {
        let Some((entry, id, module)) = tool_stack::fireball_module(&tool.custom_data, env) else { return false; };
        let Some(usage) = launch(&player, &aim.dimension, aim.position, aim.look, aim.creative, &head, env, &tool.custom_data, &entry, &module) else { return false; };
        if !aim.creative && usage > 0 {
            wear_helmet(&player, &head, env, usage, &id);
        }
        // The original adds the effect after a shot, for the drawtime (speed factor 1).
        let ticks = tool_stack::drawtime(&tool.custom_data, env, &crate::entity_facts::Entities::default(), 1.0);
        player.add_effect(&EffectInstance::new(COOLDOWN_EFFECT, ticks));
        true
    }).unwrap_or(false)
}

/// The wear a helmet shot costs (damageAnimated, with the firing modifier as its cause),
/// written back onto the head slot: the release path's wear, for a piece no release carries.
fn wear_helmet(player: &ServerPlayer, head: &StackData, env: &tool_stack::Environment, amount: i32, cause: &Identifier) {
    if amount <= 0 { return; }
    let Some(mut slot) = brush_hook::slot_of(head) else { return };
    if tool_slot::damage_slot(&mut slot, env, amount, true, true, Some(cause), &mut tool_damage_hook::random).is_err() { return; }
    player.set_equipment(StackSlot::Head, Some(&brush_hook::stack_of(&slot)));
}

/// CustomFireball.checkBelowWorld: the flight ends 64 past either build limit, read from
/// the host for the flight's dimension (a level publishes its range on every tick, so the
/// flight telling — which runs in no world callback — reads it all the same). When the host
/// has not published the range, the flight is not ended on height; only the 2400-tick half
/// of the tick runs.
fn below_build_height(dimension: &str, y: f64) -> bool {
    match World::new(dimension).build_height() {
        Some([min, max]) => y < f64::from(min) - 64.0 || y > f64::from(max) + 64.0,
        None => false,
    }
}

/// CustomFireball.tick: the flight ends after 2400 ticks, or past the build height.
fn on_flight(flight: &ProjectileFlight) {
    if flight.kind != FIREBALL {
        return;
    }
    // The flight hook may already have ended it (a fuse): the shot's state ends with it.
    if flight.age > 2400 || below_build_height(&flight.dimension, flight.position[1]) || flight.discard.get() {
        flight.discard.set(true);
        if let Ok(mut shots) = SHOTS.lock() {
            shots.remove(&(flight.dimension.clone(), flight.projectile));
        }
        crate::flight_hook::unbind(&flight.dimension, flight.projectile);
    }
}

/// CustomFireball's damage type on a hit: the pair's melee type when the target is an enderman
/// the enderference effect protects, the ranged type otherwise
/// (TinkerEffects.needsEnderferenceOverride).
fn damage_type<'a>(shot: &'a Shot, impact: &ProjectileImpact) -> &'a str {
    if impact.entity_kind == "minecraft:enderman"
        && impact.entity_effects.iter().any(|effect| effect.as_str() == crate::teleport_dodge::ENDERFERENCE)
    {
        &shot.enderference_type
    } else {
        &shot.damage_type
    }
}

/// CustomFireball.onHit: an entity hit hurts and every hit discards, server side.
fn on_impact(impact: &ProjectileImpact) -> bool {
    if impact.kind != FIREBALL {
        return true;
    }
    // The flight hook may have sent the shot on with a new motion (a bounce): the shot flies
    // on and its hit hooks already ran there, so nothing here ends.
    if impact.motion_after.borrow().is_some() {
        return false;
    }
    let shot = SHOTS.lock().ok().and_then(|mut shots| shots.remove(&(impact.dimension.clone(), impact.projectile)));
    if impact.entity.is_some() {
        if let Some(shot) = shot {
            // getDamage: the power times the damage multiplier times the motion's
            // length, rounded up, through the tool's projectile hit hooks.
            let speed = (impact.motion[0].powi(2) + impact.motion[1].powi(2) + impact.motion[2].powi(2)).sqrt();
            let base = (f64::from(shot.power) * f64::from(shot.damage_multiplier) * speed).ceil() as f32;
            let amount = with_held_tool(&shot.tool, |env, tool| {
                let mut data = impact.data.clone();
                tool_stack::power_hit(
                    &tool.custom_data,
                    env,
                    &mut data,
                    &crate::conditional_stat::PowerContext { holder_in_tag: &|_| false, target_in_tag: &|_| false, entities: crate::entity_facts::Entities::default() },
                    base,
                )
            })
            .unwrap_or(base);
            if amount > 0.0 {
                impact.actions.borrow_mut().push(EntityAction::Hurt { target: ActionTarget::Victim, amount, damage_type: damage_type(&shot, impact).to_owned() });
            }
        }
    }
    impact.discard.set(true);
    // The flight hook put the flight back; the projectile ends here, so drop it too.
    crate::flight_hook::unbind(&impact.dimension, impact.projectile);
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::{with_world, MemoryWorldOps};
    use std::cell::{Cell, RefCell};

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }

    fn tool_nbt(modifiers: &[(&str, i32)]) -> NbtCompound {
        let mut list = List::default();
        for (name, level) in modifiers {
            list = list.with_modifier(id(name), *level).unwrap();
        }
        let mut nbt = NbtCompound::new();
        nbt.insert(tool_stack::TAG_MODIFIERS.into(), list.to_nbt());
        nbt
    }


    fn flight(dimension: &str, age: u32, y: f64) -> ProjectileFlight {
        ProjectileFlight {
            dimension: dimension.to_owned(),
            projectile: 1,
            kind: FIREBALL.to_owned(),
            owner: -1,
            owner_position: None,
            age,
            position: [0.0, y, 0.0],
            motion: [0.0; 3],
            in_ground: false,
            no_gravity: false,
            arrow: None,
            data: NbtCompound::new(),
            motion_after: RefCell::new(None),
            discard: Cell::new(false),
            no_gravity_after: Cell::new(None),
            pickup_after: Cell::new(None),
            release_from_ground: Cell::new(false),
            actions: RefCell::new(Vec::new()),
        }
    }

    fn fireball_module(json: serde_json::Value) -> Fireball {
        Fireball::decode(json.as_object().unwrap()).unwrap().unwrap()
    }

    #[test]
    fn the_shot_keeps_the_tool_list_alone_when_the_module_names_no_ammo_modifier() {
        // The original's isEmpty quirk: an empty module list leaves the tool's list alone,
        // and the matched type's list with it.
        let module = fireball_module(serde_json::json!({
            "ammo_modifiers": [],
            "damage_multiplier": 1.5,
            "damage_type": {"melee": "tconstruct:fluid_impact_melee", "ranged": "tconstruct:fluid_impact_ranged"},
            "durability": {"flat": 1},
            "options": [
                {"ammo_modifiers": [{"level": 2, "name": "tconstruct:drawback"}], "damage_multiplier": 0.67, "match": {"tag": "forge:slimeball/earth"}}
            ],
            "sound": "tconstruct:slimy_bounce"
        }));
        let option = &module.options[0];
        let merged = merged_modifiers(&tool_nbt(&[("tconstruct:spiny", 1)]), &module, option);
        assert_eq!(merged.entries(), &[Entry::new(id("tconstruct:spiny"), 1)]);
    }

    #[test]
    fn the_shot_adds_the_module_and_type_modifiers_when_the_module_names_any() {
        let module = fireball_module(serde_json::json!({
            "ammo_modifiers": [{"level": 1, "name": "tconstruct:bounce"}],
            "damage_multiplier": 1.5,
            "damage_type": {"melee": "tconstruct:fluid_impact_melee", "ranged": "tconstruct:fluid_impact_ranged"},
            "durability": {"flat": 1},
            "options": [
                {"ammo_modifiers": [{"level": 2, "name": "tconstruct:spiny"}], "damage_multiplier": 0.67, "match": {"tag": "forge:slimeball/earth"}}
            ],
            "sound": "tconstruct:slimy_bounce"
        }));
        let option = &module.options[0];
        let merged = merged_modifiers(&tool_nbt(&[("tconstruct:spiny", 1)]), &module, option);
        assert_eq!(merged.entries(), &[Entry::new(id("tconstruct:spiny"), 3), Entry::new(id("tconstruct:bounce"), 1)], "levels add for a duplicate id, as ModifierNBT.Builder.add does");
    }

    #[test]
    fn the_general_path_cools_down_for_the_tool_drawtime() {
        // A bare environment reads the stored stat: no definition, no modifiers.
        let registry = crate::modifiers::Registry::default();
        let materials = crate::materials::Definitions::default();
        let env = tool_stack::Environment { definition: None, has_materials: false, materials: &materials, modifiers: &registry, item_tags_loaded: true, item: None, item_tags: None };
        let stats = |draw_speed: f32| {
            let mut nbt = NbtCompound::new();
            nbt.insert(tool_stack::TAG_STATS.into(), Nbt::Compound([("tconstruct:draw_speed".to_owned(), Nbt::Float(draw_speed))].into_iter().collect()));
            nbt
        };
        assert_eq!(general_cooldown_ticks(&stats(2.0), &env), 10, "draw speed 2.0 cools down for ten ticks, as addCooldown with speed factor 1");
        assert_eq!(general_cooldown_ticks(&stats(1.0), &env), 20);
        assert_eq!(general_cooldown_ticks(&NbtCompound::new(), &env), 20, "a tool with no stored draw speed defaults to 1.0");
    }

    #[test]
    fn a_fireball_discards_past_the_build_height_and_not_inside_it() {
        // The overworld's range, and a custom dimension's own range, as the host publishes them.
        let mut ops = MemoryWorldOps::default();
        ops.build_heights.insert("minecraft:overworld".into(), [-64, 320]);
        ops.build_heights.insert("example:custom".into(), [-32, 128]);
        with_world(&mut ops, || {
            // Inside the overworld's range and its 64-block margin it flies on.
            let flying = flight("minecraft:overworld", 10, 383.0);
            on_flight(&flying);
            assert!(!flying.discard.get(), "384 is the top of the margin, 383 still flies");
            // Past the top margin it ends: max 320 + 64 = 384.
            let high = flight("minecraft:overworld", 10, 384.5);
            on_flight(&high);
            assert!(high.discard.get());
            // Below the bottom margin it ends too: min -64 - 64 = -128.
            let low = flight("minecraft:overworld", 10, -128.5);
            on_flight(&low);
            assert!(low.discard.get());
            // A custom dimension reads its own range: its top margin is 128 + 64 = 192.
            let custom_inside = flight("example:custom", 10, 191.0);
            on_flight(&custom_inside);
            assert!(!custom_inside.discard.get());
            let custom_past = flight("example:custom", 10, 192.5);
            on_flight(&custom_past);
            assert!(custom_past.discard.get());
            // The 2400-tick half still ends a flight inside the range.
            let old = flight("minecraft:overworld", 2401, 64.0);
            on_flight(&old);
            assert!(old.discard.get());
        });
        // With no range published for the dimension, only the 2400-tick half runs.
        let unknown = flight("example:unpublished", 10, 9999.0);
        on_flight(&unknown);
        assert!(!unknown.discard.get(), "no host range: a high flight is not discarded");
        let unknown_old = flight("example:unpublished", 2401, 64.0);
        on_flight(&unknown_old);
        assert!(unknown_old.discard.get());
        // Another kind is not a fireball: the flight is left alone.
        let mut other = flight("minecraft:overworld", 10, 9999.0);
        other.kind = "minecraft:arrow".into();
        on_flight(&other);
        assert!(!other.discard.get());
    }

    #[test]
    fn an_enderman_with_the_effect_is_hurt_by_the_melee_type() {
        let shot = Shot { power: 1.0, damage_multiplier: 1.0, damage_type: "tconstruct:fluid_impact_ranged".into(), enderference_type: "tconstruct:fluid_impact_melee".into(), tool: StackData::default() };
        let impact = |kind: &str, effects: Vec<String>| ProjectileImpact {
            dimension: "minecraft:overworld".into(),
            projectile: 1,
            kind: FIREBALL.into(),
            owner: 1,
            entity: Some(9),
            entity_kind: kind.into(),
            entity_effects: effects,
            block: [0, 64, 0],
            face: [0, 1, 0],
            at: [0.5, 64.5, 0.5],
            motion: [0.0; 3],
            data: NbtCompound::new(),
            piercing_ignored: Vec::new(),
            pierced_and_killed: Vec::new(),
            shot_punch: 0,
            discard: Cell::new(false),
            motion_after: RefCell::new(None),
            ignore_entities: RefCell::new(Vec::new()),
            actions: RefCell::new(Vec::new()),
        };
        assert_eq!(damage_type(&shot, &impact("minecraft:enderman", vec![crate::teleport_dodge::ENDERFERENCE.into()])), "tconstruct:fluid_impact_melee");
        assert_eq!(damage_type(&shot, &impact("minecraft:enderman", vec![])), "tconstruct:fluid_impact_ranged", "a plain enderman takes the ranged type");
        assert_eq!(damage_type(&shot, &impact("minecraft:zombie", vec![crate::teleport_dodge::ENDERFERENCE.into()])), "tconstruct:fluid_impact_ranged", "the effect alone does not override");
    }
}
