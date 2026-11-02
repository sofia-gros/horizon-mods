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

/// The original box, circle and vein iterators all have a single-block form.
/// Evaluate the actual definition and expanded modifier rather than guessing
/// based on an item name; non-empty iterator shapes remain unsupported here.
pub(super) fn single_block(game: &super::Game<'_>, stack: &ItemStack) -> bool {
    let Some(traits) = super::tconstruct_source_material_traits::levels(&game.world, stack) else { return false; };
    let expanded = super::tconstruct_source_overslime::upgrade(stack, "tconstruct:expanded")
        + traits.levels.get("tconstruct:expanded").copied().unwrap_or(0);
    if expanded != 0 { return false; }
    let Some((namespace, name)) = stack.item.split_once(':') else { return false; };
    let Some(layers) = game.world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref() else { return false; };
    let Some(bytes) = layers.get(&format!("{namespace}:tinkering/tool_definitions/{name}")).and_then(|layers| layers.last()) else { return false; };
    let Ok(definition) = serde_json::from_slice::<serde_json::Value>(bytes) else { return false; };
    let Some(modules) = definition.get("modules").and_then(serde_json::Value::as_array) else { return false; };
    for module in modules {
        let Some(kind) = module.get("type").and_then(serde_json::Value::as_str) else { return false; };
        match kind {
            "tconstruct:box_aoe" => {
                if let Some(bonus) = module.get("bonus") {
                    let Some(bonus) = bonus.as_object() else { return false; };
                    if bonus.iter().any(|(key, value)| !matches!(key.as_str(), "width" | "height" | "depth") || value.as_i64() != Some(0)) { return false; }
                }
            }
            "tconstruct:circle_aoe" => if module.get("diameter").and_then(serde_json::Value::as_i64) != Some(1) { return false; },
            "tconstruct:vein_aoe" => if module.get("max_distance").map_or(Some(0), serde_json::Value::as_i64) != Some(0) { return false; },
            other if other.ends_with("_aoe") => return false,
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
    // A guest turn does not expose LanHost to this hook. Restrict the current
    // experimental path to packet scopes known to contain no other players.
    if let Some(conn) = game.lan_acting {
        if game.world.chunks.__horizon_mixins_tconstruct_splashing_guest_scope().get(&conn) != Some(&true) { return; }
    } else if game.lan.as_ref().is_some_and(|host| host.hub.guests.iter().any(|guest| guest.placed)) { return; }
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
    // Living armor stands also receive source effects, but their effect adapter
    // is not yet implemented. Do not perform a partial area operation.
    let radius = 1.0 + f64::from(expanded);
    if game.armor_stands.iter().any(|stand| !stand.marker && {
        let delta = [stand.position.x - origin[0], stand.position.y - origin[1], stand.position.z - origin[2]];
        delta.iter().map(|component| component * component).sum::<f64>() < radius * radius
    }) { return; }
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

/// Source zero-offset AOE paths for the bundled blazing-blood glow effect.
/// Non-empty tool AOE iterators and other block-effect forms stay unresolved.
pub(super) fn block(game: &mut super::Game<'_>, stack: &ItemStack, clicked: [i32; 3], face: [i32; 3]) -> Option<horizon_client::Result<bool>> {
    if game.remote.is_some() || !single_block(game, stack) || !right_click(game, stack) || !game.may_build() || super::tconstruct_source_melee_shape::intact(stack).is_none() { return None; }
    if !matches!(face, [0, -1, 0] | [0, 1, 0] | [0, 0, -1] | [0, 0, 1] | [-1, 0, 0] | [1, 0, 0]) { return None; }
    if game.cooldowns.is_on_cooldown(&stack.item) { return None; }
    let custom = stack.data()?.other("minecraft:custom_data")?;
    let Tag::Int(level) = custom.get("tic_upgrades")?.get("tconstruct:splashing")? else { return None; };
    if *level <= 0 || stack.count != 1 || stack.wire_patch().is_some() { return None; }
    let Tag::Int(capacity) = custom.get("tic_tank_capacity")? else { return None; };
    if *capacity <= 0 { return None; }
    let Tag::Float(speed) = custom.get("tic_draw_speed")? else { return None; };
    let speed = f32::from_bits(*speed);
    if !speed.is_finite() || speed <= 0.0 { return None; }
    let fluid = super::pluto_stack_fluid::read(stack, "tic_persistent", "tconstruct:tank_fluid")??;
    if fluid.amount > *capacity as u32 || ((game.lan.is_some() || game.lan_acting_uuid.is_some()) && fluid.tag.is_some()) { return None; }
    let recipe = super::tconstruct_source_slurping::raw_recipe(game, &fluid.id)?;
    let effects = recipe.get("block_effects")?.as_array()?;
    if effects.len() != 1 || effects[0].get("type")?.as_str()? != "tconstruct:place_block" || effects[0].get("block")?.as_str()? != "tconstruct:glow" { return None; }
    if effects[0].as_object()?.keys().any(|key| key != "type" && key != "block") { return None; }
    let amount = u32::try_from(recipe.pointer("/fluid/amount")?.as_u64()?).ok().filter(|amount| *amount > 0)?;
    let held = game.held()?;
    if held.damage != stack.damage || !held.same_item_same_components(stack) { return None; }
    let staged = if fluid.amount >= amount {
        let mut remaining = fluid.clone(); remaining.amount -= amount;
        Some(super::pluto_stack_fluid::write(stack, "tic_persistent", "tconstruct:tank_fluid", (remaining.amount > 0).then_some(&remaining), *capacity as u32)?)
    } else { None };
    game.add_cooldown(stack.item.clone(), (20.0 / speed) as i32);
    let result = (|| -> horizon_client::Result<bool> {
        let Some(drained) = staged else { return Ok(true); };
        let target = if game.res.facts().get(game.world.block(clicked[0], clicked[1], clicked[2])).replaceable { clicked }
            else {
                let Some(x) = clicked[0].checked_add(face[0]) else { return Ok(true); };
                let Some(y) = clicked[1].checked_add(face[1]) else { return Ok(true); };
                let Some(z) = clicked[2].checked_add(face[2]) else { return Ok(true); };
                [x, y, z]
            };
        if super::tconstruct_glowing_world::place(game, target, [-face[0], -face[1], -face[2]])? {
            let sound = game.res.facts().get(game.world.block(target[0], target[1], target[2])).sounds.place.clone();
            game.audio.play(&sound, Some([target[0] as f64 + 0.5, target[1] as f64 + 0.5, target[2] as f64 + 0.5]), 1.0, 1.0);
            game.post_block_game_event("minecraft:block_place", target, game.sculk_actor(), None);
            if !game.creative {
                *game.held_mut() = Some(drained);
                let Some(current) = game.held().cloned() else { return Ok(true); };
                let (loss, shield) = super::tconstruct_source_overslime_host::damage_with_cause(game, &current, *level as u32, Some("tconstruct:splashing"));
                if let Some(held) = game.held_mut().as_mut() {
                    super::tconstruct_source_overslime::spend(held, shield);
                    super::tconstruct_tx::fixed_damage(held, loss);
                }
            }
        }
        Ok(true)
    })();
    Some(result)
}
