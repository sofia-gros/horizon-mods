//! Source right-click selection and nearby entity geometry for splashing.
//! Runtime effect handling and the Station recipe remain disabled until the
//! authoritative target enumeration includes guest players and other entities.
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;

/// Resolve the source definition's interaction module. Unknown interaction
/// types are refused rather than stealing a different item's click.
pub(super) fn right_click(game: &super::Game<'_>, stack: &ItemStack) -> bool {
    let Some(custom) = stack.data().and_then(|data| data.other("minecraft:custom_data")) else { return false; };
    if !matches!(custom.get("tic_upgrades").and_then(|upgrades| upgrades.get("tconstruct:splashing")), Some(Tag::Int(level)) if *level > 0) { return false; }
    let in_set = |key: &str| -> Option<bool> {
        match custom.get("tic_persistent").and_then(|data| data.get(key)) {
            None => Some(false),
            Some(Tag::List(entries)) => Some(entries.iter().any(|entry| matches!(entry, Tag::String(id) if id.to_string_lossy() == "tconstruct:splashing"))),
            _ => None,
        }
    };
    let Some((namespace, name)) = stack.item.split_once(':') else { return false; };
    let Some(layers) = game.world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref() else { return false; };
    let Some(bytes) = layers.get(&format!("{namespace}:tinkering/tool_definitions/{name}")).and_then(|layers| layers.last()) else { return false; };
    let Ok(definition) = serde_json::from_slice::<serde_json::Value>(bytes) else { return false; };
    let Some(modules) = definition.get("modules").and_then(serde_json::Value::as_array) else { return false; };
    for module in modules {
        let Some(kind) = module.get("type").and_then(serde_json::Value::as_str) else { return false; };
        match kind {
            "tconstruct:attack_interaction" => return false,
            "tconstruct:dual_option_interaction" => return in_set("tconstruct:attack_modifiers") == Some(false),
            "tconstruct:toggleable_set_interaction" => {
                if module.pointer("/interact_modifiers/type").and_then(serde_json::Value::as_str) != Some("tconstruct:single") || module.pointer("/interact_modifiers/modifier").and_then(serde_json::Value::as_str) != Some("tconstruct:fishing") { return false; }
                return in_set("tconstruct:interact_modifiers") == Some(true);
            }
            other if other.ends_with("_interaction") => return false,
            _ => {}
        }
    }
    true
}

/// The source tests the candidate box against the target's inflated box and
/// then compares full 3D entity positions using a strict squared radius.
pub(super) fn in_range(target: [f64; 3], candidate: [f64; 3], target_bounds: ([f64; 3], [f64; 3]), candidate_bounds: ([f64; 3], [f64; 3]), expanded: u32) -> bool {
    let radius = 1.0 + f64::from(expanded);
    let inflation = [radius, 0.25, radius];
    let intersects = (0..3).all(|i| target_bounds.0[i] - inflation[i] < candidate_bounds.1[i] && target_bounds.1[i] + inflation[i] > candidate_bounds.0[i]);
    intersects && (0..3).map(|i| { let delta = target[i] - candidate[i]; delta * delta }).sum::<f64>() < radius * radius
}

/// Experimental entity-use path, disabled by the Station recipe. Only mob
/// targets and mob area members are currently resolved; guest players and
/// non-mob entities need an authoritative host-backed enumeration.
pub(super) fn interact(game: &mut super::Game<'_>, entity: i32, stack: Option<&ItemStack>, claimed: &mut bool) {
    if *claimed || game.remote.is_some() { return; }
    let Some(stack) = stack.cloned() else { return; };
    if !right_click(game, &stack) || super::tconstruct_source_melee_shape::intact(&stack).is_none() || game.cooldowns.is_on_cooldown(&stack.item) { return; }
    let Some(target) = game.mobs.iter().position(|mob| i32::try_from(mob.id).ok() == Some(entity) && mob.alive()) else { return; };
    let Some(custom) = stack.data().and_then(|data| data.other("minecraft:custom_data")) else { return; };
    let Some(Tag::Int(level)) = custom.get("tic_upgrades").and_then(|data| data.get("tconstruct:splashing")) else { return; };
    if *level <= 0 { return; }
    let Some(Tag::Float(bits)) = custom.get("tic_draw_speed") else { return; };
    let speed = f32::from_bits(*bits);
    if !speed.is_finite() || speed <= 0.0 { return; }
    let Some(traits) = super::tconstruct_source_material_traits::levels(&game.world, &stack) else { return; };
    let expanded = super::tconstruct_source_overslime::upgrade(&stack, "tconstruct:expanded")
        + traits.levels.get("tconstruct:expanded").copied().unwrap_or(0);
    let Ok(expanded) = u32::try_from(expanded) else { return; };
    if super::tconstruct_source_spilling::apply(game, target, stack.clone(), "tconstruct:splashing", true).is_none() { return; }
    let target_mob = &game.mobs[target];
    let origin = [target_mob.position.x, target_mob.position.y, target_mob.position.z];
    let target_box = target_mob.bounds();
    let min = target_box.min_position(); let max = target_box.max_position();
    let target_bounds = ([min.x, min.y, min.z], [max.x, max.y, max.z]);
    let mut targets = vec![target];
    for (i, mob) in game.mobs.iter().enumerate() {
        if i == target || !mob.alive() { continue; }
        let pos = [mob.position.x, mob.position.y, mob.position.z];
        let bounds = mob.bounds(); let min = bounds.min_position(); let max = bounds.max_position();
        if in_range(origin, pos, target_bounds, ([min.x, min.y, min.z], [max.x, max.y, max.z]), expanded) { targets.push(i); }
    }
    game.add_cooldown(stack.item.clone(), (20.0 / speed) as i32);
    let mut applied = 0u32;
    let mut working = stack.clone();
    for index in targets {
        if let Some(drained) = super::tconstruct_source_spilling::apply(game, index, working.clone(), "tconstruct:splashing", false) {
            working = drained;
            applied += 1;
        }
    }
    if applied > 0 && !game.creative {
        if !game.held().is_some_and(|held| held.damage == stack.damage && held.same_item_same_components(&stack)) { return; }
        *game.held_mut() = Some(working);
        if let Some(current) = game.held().cloned() {
            let wear = applied.saturating_mul(*level as u32);
            let (loss, shield) = super::tconstruct_source_overslime_host::damage_with_cause(game, &current, wear, Some("tconstruct:splashing"));
            if let Some(held) = game.held_mut().as_mut().filter(|held| held.item == current.item) {
                super::tconstruct_source_overslime::spend(held, shield);
                super::tconstruct_tx::fixed_damage(held, loss);
            }
        }
    }
    *claimed = true;
}
