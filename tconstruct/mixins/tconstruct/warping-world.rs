use super::{Game, tconstruct_warping as rule};
use horizon_client::items::ItemStack;
use horizon_world::nbt::Tag;

pub(super) fn hinted(stack: &ItemStack) -> bool {
    if !rule::stock(&stack.item)
        || stack.count != 1
        || super::tconstruct_source_melee_shape::intact(stack).is_none()
        || !super::tconstruct_source_tool_normalize::patch_safe_for_tool_mutation(stack)
    {
        return false;
    }
    let Some(data) = stack.data() else {
        return false;
    };
    if !matches!(
        data.other("minecraft:custom_data")
            .and_then(|c| c.get("tic_broken")),
        None | Some(Tag::Byte(0))
    ) {
        return false;
    }
    matches!(
        stack
            .data()
            .and_then(|d| d.other("minecraft:custom_data"))
            .and_then(|d| d.get("tic_upgrades"))
            .and_then(|d| d.get("tconstruct:warping")),
        Some(Tag::Int(1))
    )
}
pub(super) fn config(game: &Game<'_>, stack: &ItemStack) -> Option<rule::Stats> {
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
    if !has("tconstruct:modifiable/durability")
        || !has("tconstruct:modifiable/interactable/charge")
        || [
            "tconstruct:modifiable/ranged",
            "tconstruct:modifiable/ammo",
            "tconstruct:modifiable/armor",
        ]
        .into_iter()
        .any(has)
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
    if !super::tconstruct_source_upgrade_rules::known_warping_definition(
        super::tconstruct_modifier_snapshot::highest_definition(
            raw,
            "tconstruct:tinkering/modifiers/warping",
        ),
    ) {
        return None;
    }
    let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
    if super::tconstruct_source_material_traits::level_with_upgrade(
        stack,
        &traits,
        "tconstruct:warping",
    )? != 1
    {
        return None;
    }
    for id in super::tconstruct_source_material_effects::ordered_modifier_ids(stack, &traits.levels)
    {
        if id == "tconstruct:warping" {
            continue;
        }
        if super::tconstruct_source_material_traits::level_with_upgrade(stack, &traits, &id)? == 0 {
            continue;
        }
        if matches!(
            id.as_str(),
            "tconstruct:flinging"
                | "tconstruct:springing"
                | "tconstruct:bonking"
                | "tconstruct:slurping"
                | "tconstruct:spitting"
                | "tconstruct:splashing"
                | "tconstruct:bucketing"
                | "tconstruct:tasty"
                | "tconstruct:savory"
                | "tconstruct:scrumptious"
                | "tconstruct:glowing"
        ) {
            return None;
        }
        let (namespace, path) = id.split_once(':')?;
        if let Some(bytes) = super::tconstruct_modifier_snapshot::highest_definition(
            raw,
            &format!("{namespace}:tinkering/modifiers/{path}"),
        ) {
            let definition: serde_json::Value = serde_json::from_slice(bytes).ok()?;
            if rule::competing(&definition) {
                return None;
            }
        } else if !matches!(
            id.as_str(),
            "tconstruct:cultivated"
                | "tconstruct:preserved"
                | "tconstruct:dense"
                | "tconstruct:reinforced"
                | "tconstruct:unbreakable"
                | "tconstruct:diamond"
                | "tconstruct:emerald"
                | "tconstruct:netherite"
                | "tconstruct:worldbound"
                | "tconstruct:writable"
                | "tconstruct:harmonious"
                | "tconstruct:recapitated"
                | "tconstruct:forecast"
                | "tconstruct:draconic"
                | "tconstruct:enhanced"
                | "tconstruct:haste"
                | "tconstruct:sharpness"
                | "tconstruct:swiftstrike"
        ) {
            return None;
        }
    }
    let item = game.items().get(&stack.item)?;
    if item.consumable.is_some()
        || item.blocks_attacks.is_some()
        || item.use_cooldown.is_some()
        || [
            "minecraft:use_cooldown",
            "minecraft:consumable",
            "minecraft:blocks_attacks",
        ]
        .into_iter()
        .any(|component| {
            stack
                .data()
                .is_some_and(|data| data.other(component).is_some())
        })
    {
        return None;
    }
    let materials = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_material_snapshot()
        .as_ref()?;
    let name = stack.item.strip_prefix("tconstruct:")?;
    rule::stats(
        materials
            .get(&format!("tconstruct:tinkering/tool_definitions/{name}"))?
            .last()?,
    )
}
pub(super) fn start(game: &mut Game<'_>, stack: &ItemStack, canceled: &mut bool) {
    let actor = game.lan_acting;
    game.world
        .chunks
        .__horizon_mixins_tconstruct_warping_draw_mut()
        .remove(&actor);
    if game.remote.is_some() || !hinted(stack) {
        return;
    }
    let Some(stats) = config(game, stack) else {
        *canceled = true;
        return;
    };
    let Some(player) = game.player.as_ref() else {
        *canceled = true;
        return;
    };
    if player.is_passenger || player.sleeping || player.health.dead_or_dying() || player.teleported
    {
        *canceled = true;
        return;
    }
    let slot = game.held_slot();
    game.world
        .chunks
        .__horizon_mixins_tconstruct_warping_draw_mut()
        .insert(actor, (slot, stack.clone(), stats.draw_ticks));
}
fn effects(game: &mut Game<'_>, at: [f64; 3], success: bool) {
    use horizon_client::particles::{ParticleOptions, ParticleType};
    use horizon_core::random::RandomSource;
    let mut particles = Vec::new();
    if success {
        for _ in 0..32 {
            let pos = [at[0], at[1] + game.world.random.next_double() * 2.0, at[2]];
            let velocity = [
                game.world.random.next_gaussian(),
                0.0,
                game.world.random.next_gaussian(),
            ];
            particles.push((pos, velocity));
        }
    }
    game.with_level(|_, level, _| {
        level.native_play_sound(
            "tconstruct:slime_sling.teleport",
            at,
            7,
            1.0,
            if success { 1.0 } else { 0.5 },
        );
        let portal = ParticleOptions::simple(ParticleType::Portal);
        for (pos, velocity) in particles {
            level.native_send_particles(&portal, None, pos, velocity, 1.0, 0);
        }
    });
}
pub(super) fn release(game: &mut Game<'_>, stack: &ItemStack, time_left: i32) -> bool {
    use horizon_core::{geometry::Vec3, random::RandomSource};
    let actor = game.lan_acting;
    let Some((slot, original, draw)) = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_warping_draw_mut()
        .remove(&actor)
    else {
        return false;
    };
    if !game.native_authoritative()
        || game.remote.is_some()
        || original != *stack
        || game.held_slot() != slot
        || game.stack_in(slot) != Some(stack)
    {
        return false;
    }
    let Some(stats) = config(game, stack) else {
        return false;
    };
    let Some(p) = game.player.as_ref() else {
        return false;
    };
    if p.is_passenger || p.sleeping || p.health.dead_or_dying() || p.teleported || game.spectator()
    {
        return false;
    }
    let position = [p.position.x, p.position.y, p.position.z];
    let ticks = 72000i32.saturating_sub(time_left);
    if ticks <= 0 {
        return false;
    }
    let force = stats.power * (rule::charge(ticks, draw) * 6.0);
    if !force.is_finite() || force <= 0.0 {
        effects(game, position, false);
        return false;
    }
    let look = horizon_world::view_direction::view_vector(game.pitch, game.yaw);
    let scale = f64::from(stats.inaccuracy);
    let angle = [
        look.x + game.world.random.next_gaussian() * scale,
        look.y + game.world.random.next_gaussian() * scale,
        look.z + game.world.random.next_gaussian() * scale,
    ];
    let offset = [
        angle[0] * f64::from(force),
        angle[1] * f64::from(force) + 1.0,
        angle[2] * f64::from(force),
    ];
    let target = rule::target(position, offset, |[x, y, z]| {
        if !game.world.loaded(x, z) {
            return None;
        }
        let state = game.world.block(x, y, z);
        Some((
            game.border.contains(f64::from(x), f64::from(z)),
            game.state_suffocating(state),
            game.res.facts().get(state).hardness == -1.0,
        ))
    });
    let Some(at) = target else {
        effects(game, position, false);
        return false;
    };
    // Context is synchronous; validate the exact input before spending anything.
    let Some(mut updated) = super::tconstruct_source_tool_normalize::clone_for_tool_mutation(stack)
    else {
        return false;
    };
    if !game.creative {
        let (loss, spent) = super::tconstruct_source_overslime_host::damage_with_cause(
            game,
            stack,
            1,
            Some("tconstruct:warping"),
        );
        super::tconstruct_source_overslime::spend(&mut updated, spent);
        super::tconstruct_tx::fixed_damage(&mut updated, loss);
    }
    if game.stack_in(slot) != Some(stack) {
        return false;
    }
    let broken = super::tconstruct_source_ranged_wear::is_broken(&updated);
    *game.stack_in_mut(slot) = Some(updated);
    if broken {
        game.apply_equipment();
    }
    if let Some(p) = game.player.as_mut() {
        p.position = Vec3::new(at[0], at[1], at[2]);
        p.fall_distance = 0.0;
        p.cause_food_exhaustion(0.2);
    }
    game.previous_position = Vec3::new(at[0], at[1], at[2]);
    if let Some(conn) = actor {
        game.world
            .chunks
            .__horizon_mixins_tconstruct_warping_teleport_mut()
            .insert(conn, (at, [game.yaw, game.pitch]));
    }
    game.add_cooldown(stack.item.clone(), 3);
    effects(game, at, true);
    true
}
