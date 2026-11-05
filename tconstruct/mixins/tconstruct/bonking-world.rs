use super::{Game, tconstruct_bonking as rule};
use horizon_client::items::ItemStack;
use horizon_core::{geometry::Vec3, random::RandomSource};
use horizon_world::nbt::Tag;
use std::cell::Cell;

#[derive(Clone, Copy)]
struct Attack {
    actor: Option<u32>,
    target: u32,
    charge: f32,
    depth: usize,
}
thread_local! { static ATTACK: Cell<Option<Attack>> = const { Cell::new(None) }; }
struct Scope(Option<Attack>);
impl Drop for Scope {
    fn drop(&mut self) {
        ATTACK.with(|a| a.set(self.0));
    }
}
pub(super) fn attack_enter(game: &Game<'_>) {
    ATTACK.with(|cell| {
        if let Some(mut a) = cell.get() {
            if a.actor == game.lan_acting {
                a.depth += 1;
                cell.set(Some(a));
            }
        }
    });
}
pub(super) fn attack_leave(game: &Game<'_>) {
    ATTACK.with(|cell| {
        if let Some(mut a) = cell.get() {
            if a.actor == game.lan_acting {
                a.depth = a.depth.saturating_sub(1);
                cell.set(Some(a));
            }
        }
    });
}
pub(super) fn attack_charge(game: &Game<'_>) -> Option<f32> {
    ATTACK.with(|cell| {
        cell.get()
            .filter(|a| a.actor == game.lan_acting && a.depth == 1)
            .map(|a| a.charge)
    })
}
pub(super) fn suppress_knockback(entity: i32) -> bool {
    ATTACK.with(|cell| {
        cell.get()
            .is_some_and(|a| a.depth > 0 && a.target as i32 == entity)
    })
}
fn hinted(stack: &ItemStack) -> bool {
    stack.item.as_ref() == "tconstruct:battlesign"
        && stack.count == 1
        && super::tconstruct_shield_world::valid_stack(stack)
        && stack.enchantments().is_empty()
}
fn same_tool(a: &ItemStack, b: &ItemStack) -> bool {
    let custom = |s: &ItemStack, key: &str| {
        s.data()
            .and_then(|d| d.other("minecraft:custom_data"))
            .and_then(|d| d.get(key))
            .cloned()
    };
    a.item == b.item
        && a.count == b.count
        && ["tic_materials", "tic_upgrades"]
            .into_iter()
            .all(|key| custom(a, key) == custom(b, key))
}
#[derive(Clone, Copy)]
struct Config {
    draw: i32,
    knockback: i32,
}
fn config(game: &Game<'_>, stack: &ItemStack) -> Option<Config> {
    if game.remote.is_some() || !hinted(stack) {
        return None;
    }
    let tags = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_resolved_item_tags()
        .as_ref()?
        .as_ref()
        .ok()?;
    let has = |tag: &str| {
        tags.get(tag)
            .is_some_and(|set| set.contains(stack.item.as_ref()))
    };
    if ![
        "tconstruct:modifiable/durability",
        "tconstruct:modifiable/interactable/charge",
        "tconstruct:modifiable/melee",
    ]
    .into_iter()
    .all(has)
    {
        return None;
    }
    let raw = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_modifier_resource_snapshot()
        .as_ref()?
        .as_ref()
        .ok()?;
    let definition = |id| super::tconstruct_modifier_snapshot::highest_definition(raw, id);
    if !rule::known(
        "bonking",
        definition("tconstruct:tinkering/modifiers/bonking"),
    ) || !rule::known(
        "knockback",
        definition("tconstruct:tinkering/modifiers/knockback"),
    ) || !super::tconstruct_shield::known_blocking(definition(
        "tconstruct:tinkering/modifiers/blocking",
    )) {
        return None;
    }
    let target_tag: serde_json::Value = serde_json::from_slice(
        raw.1
            .get("tconstruct:tinkering/tags/modifiers/knockback_slings/target")?
            .last()?,
    )
    .ok()?;
    if target_tag != serde_json::json!({"values":["tconstruct:bonking"]}) {
        return None;
    }
    let materials = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_material_snapshot()
        .as_ref()?;
    if !rule::known(
        "tool",
        materials
            .get("tconstruct:tinkering/tool_definitions/battlesign")?
            .last()
            .map(Vec::as_slice),
    ) {
        return None;
    }
    let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
    let level =
        |id| super::tconstruct_source_material_traits::level_with_upgrade(stack, &traits, id);
    if level("tconstruct:bonking")? != 1 || level("tconstruct:blocking")? <= 0 {
        return None;
    }
    let knockback = level("tconstruct:knockback")?;
    if !(0..=16).contains(&knockback) {
        return None;
    }
    for id in super::tconstruct_source_material_effects::ordered_modifier_ids(stack, &traits.levels)
    {
        if matches!(
            id.as_str(),
            "tconstruct:bonking" | "tconstruct:blocking" | "tconstruct:knockback"
        ) {
            continue;
        }
        if level(&id)? == 0 {
            continue;
        }
        let (ns, path) = id.split_once(':')?;
        let bytes = definition(&format!("{ns}:tinkering/modifiers/{path}"))?;
        if id == "tconstruct:blockade" {
            if !super::tconstruct_shield::known_blockade(Some(bytes)) {
                return None;
            }
        } else if !rule::safe_module(&serde_json::from_slice(bytes).ok()?) {
            return None;
        }
    }
    let item = game.items().get(&stack.item)?;
    if item.consumable.is_some()
        || item.use_cooldown.is_some()
        || ["minecraft:consumable", "minecraft:use_cooldown"]
            .into_iter()
            .any(|id| stack.data().is_some_and(|d| d.other(id).is_some()))
    {
        return None;
    }
    let speed = if has("tconstruct:modifiable/melee/weapon") {
        match stack
            .data()?
            .other("minecraft:custom_data")?
            .get("tic_attack_speed")?
        {
            Tag::Float(bits) => f32::from_bits(*bits),
            _ => return None,
        }
    } else {
        1.0
    };
    Some(Config {
        draw: rule::draw_ticks(speed)?,
        knockback,
    })
}
pub(super) fn start(game: &mut Game<'_>, stack: &ItemStack) {
    let actor = game.lan_acting;
    game.world
        .chunks
        .__horizon_mixins_tconstruct_bonking_draw_mut()
        .remove(&actor);
    if game.remote.is_some() || game.held_slot() != game.selected {
        return;
    }
    let Some(config) = config(game, stack) else {
        return;
    };
    let Some(p) = game.player.as_ref() else {
        return;
    };
    if p.is_passenger || p.sleeping || p.health.dead_or_dying() || game.spectator() {
        return;
    }
    game.world
        .chunks
        .__horizon_mixins_tconstruct_bonking_draw_mut()
        .insert(actor, (game.selected, stack.clone(), config.draw));
}
fn sound(game: &mut Game<'_>, success: bool) {
    let Some(p) = game.player.as_ref() else {
        return;
    };
    let at = [p.position.x, p.position.y, p.position.z];
    game.with_level(|_, level, _| {
        level.native_play_sound(
            "tconstruct:bonk",
            at,
            7,
            1.0,
            if success { 0.5 } else { 1.0 },
        )
    });
}
fn target(game: &Game<'_>, eye: [f64; 3], look: [f64; 3]) -> Option<u32> {
    let p = game.player.as_ref()?;
    // Reject unloaded ray columns instead of allowing native reads to see air.
    for i in 0..=40 {
        let d = f64::from(i) * 0.125;
        if !game.world.loaded(
            (eye[0] + look[0] * d).floor() as i32,
            (eye[2] + look[2] * d).floor() as i32,
        ) {
            return None;
        }
    }
    let broad = p
        .bounds()
        .expand_towards(Vec3::new(look[0] * 5.0, look[1] * 5.0, look[2] * 5.0))
        .expand_towards(Vec3::new(1.0, 1.0, 1.0));
    let mut best = None;
    for mob in &game.mobs {
        if mob.removed || !mob.alive() || !mob.bounds().intersects(broad) {
            continue;
        }
        if let Some(distance) = super::ray_box(
            eye,
            look,
            5.0,
            mob.bounds().inflate(Vec3::new(0.3, 0.3, 0.3)),
        ) && best.is_none_or(|(_, d)| distance < d)
        {
            best = Some((mob.id, distance));
        }
    }
    let (id, _) = best?;
    let mob = game.mobs.iter().find(|m| m.id == id)?;
    let target_eye = [mob.position.x, mob.eye_y(), mob.position.z];
    let distance = (0..3)
        .map(|i| (eye[i] - target_eye[i]).powi(2))
        .sum::<f64>();
    // Source compares the target eyes to the obstructing block center, not
    // the actual ray-entry distance. Player POV item clipping uses five blocks.
    if let Some(hit) = game.clip_ray(eye, look, 5.0) {
        let block_distance = (0..3)
            .map(|i| (eye[i] - (f64::from(hit.pos[i]) + 0.5)).powi(2))
            .sum::<f64>();
        if distance >= block_distance {
            return None;
        }
    }
    Some(id)
}
pub(super) fn release(game: &mut Game<'_>, stack: &ItemStack, remaining: i32) -> bool {
    let actor = game.lan_acting;
    let Some((slot, original, draw)) = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_bonking_draw_mut()
        .remove(&actor)
    else {
        return false;
    };
    if !game.native_authoritative()
        || game.remote.is_some()
        || game.held_slot() != slot
        || slot != game.selected
        || !same_tool(&original, stack)
        || game.stack_in(slot) != Some(stack)
    {
        return false;
    }
    let Some(config) = config(game, stack) else {
        return false;
    };
    let Some(p) = game.player.as_ref() else {
        return false;
    };
    if p.is_passenger
        || p.sleeping
        || p.health.dead_or_dying()
        || p.auto_spin_attack_dmg > 0.0
        || game.spectator()
    {
        return false;
    }
    let charge = rule::charge(72000i32.saturating_sub(remaining), draw);
    if charge <= 0.0 {
        return false;
    }
    let Some(eye) = game.eye_position() else {
        return false;
    };
    let direction = horizon_world::view_direction::view_vector(game.pitch, game.yaw);
    let look = [direction.x, direction.y, direction.z];
    let Some(id) = target(game, eye, look) else {
        sound(game, false);
        return false;
    };
    let Some(index) = game.mobs.iter().position(|m| m.id == id) else {
        return false;
    };
    {
        let _scope = Scope(ATTACK.with(|cell| {
            cell.replace(Some(Attack {
                actor,
                target: id,
                charge,
                depth: 0,
            }))
        }));
        game.attack_mob(index);
    }
    // Re-resolve the stable ID after callbacks; native knockback owns resistance,
    // velocity changes and mob network synchronization.
    if let Some(index) = game.mobs.iter().position(|m| m.id == id) {
        let inaccuracy = f64::from(3.0f32 * (1.0 / 0.75 - 1.0) * 0.0075);
        let angle_x = -look[0] + game.world.random.next_gaussian() * inaccuracy;
        let angle_z = -look[2] + game.world.random.next_gaussian() * inaccuracy;
        let from = game
            .player
            .as_ref()
            .map(|p| [p.position.x, p.position.y, p.position.z]);
        if let Some(from) = from {
            let source = game.attack_source(from);
            game.mobs[index].knockback(
                f64::from(rule::force(charge, config.knockback)),
                angle_x,
                angle_z,
                &source,
            );
        }
    }
    // The ordinary melee body already charged its own successful-hit wear.
    // Bonking adds one more wear, even when the hurt was rejected.
    if let Some(current) = game.stack_in(slot).cloned().filter(|s| same_tool(stack, s)) {
        if !game.creative && super::tconstruct_source_melee_shape::intact(&current).is_some() {
            if let Some(mut updated) =
                super::tconstruct_source_tool_normalize::clone_for_tool_mutation(&current)
            {
                let (loss, spent) = super::tconstruct_source_overslime_host::damage_with_cause(
                    game,
                    &current,
                    1,
                    Some("tconstruct:bonking"),
                );
                super::tconstruct_source_overslime::spend(&mut updated, spent);
                super::tconstruct_tx::fixed_damage(&mut updated, loss);
                let broken = super::tconstruct_source_ranged_wear::is_broken(&updated);
                *game.stack_in_mut(slot) = Some(updated);
                if broken {
                    game.apply_equipment();
                }
            }
        }
    }
    if let Some(p) = game.player.as_mut() {
        p.cause_food_exhaustion(0.2);
    }
    game.add_cooldown(stack.item.clone(), 3);
    sound(game, true);
    true
}
