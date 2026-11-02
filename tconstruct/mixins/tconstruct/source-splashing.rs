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

/// Source circle iterator, including its optional depth component. The source
/// walks depth, height and width in that order and uses integer radius squared.
pub(super) fn circle_offsets(game: &super::Game<'_>, stack: &ItemStack, origin: [i32; 3], face: [i32; 3]) -> Option<Vec<[i32; 3]>> {
    let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
    let expanded = super::tconstruct_source_overslime::upgrade(stack, "tconstruct:expanded") + traits.levels.get("tconstruct:expanded").copied().unwrap_or(0);
    let (namespace, name) = stack.item.split_once(':')?;
    let layers = game.world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref()?;
    let bytes = layers.get(&format!("{namespace}:tinkering/tool_definitions/{name}"))?.last()?;
    let definition: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let modules = definition.get("modules")?.as_array()?;
    let aoe: Vec<_> = modules.iter().filter(|module| module.get("type").and_then(serde_json::Value::as_str).is_some_and(|kind| kind.ends_with("_aoe"))).collect();
    if aoe.len() != 1 || aoe[0].get("type")?.as_str()? != "tconstruct:circle_aoe" { return None; }
    let diameter = i32::try_from(aoe[0].get("diameter").map_or(Some(1), serde_json::Value::as_i64)?).ok()?.checked_add(expanded)?;
    if !(1..=33).contains(&diameter) { return None; }
    if diameter == 1 { return Some(Vec::new()); }
    let is_3d = match aoe[0].get("3D") { None => false, Some(value) => value.as_bool()? };
    let radius = diameter / 2;
    let radius_squared = diameter * diameter / 4;
    let look = match ((game.yaw / 90.0 + 0.5).floor() as i32).rem_euclid(4) { 0 => [0, 0, 1], 1 => [-1, 0, 0], 2 => [0, 0, -1], _ => [1, 0, 0] };
    let (width, height) = if face[1] != 0 { ([-look[2], 0, look[0]], look) } else { ([face[2], 0, -face[0]], [0, 1, 0]) };
    let depth = [-face[0], -face[1], -face[2]];
    let mut out = Vec::new();
    for dz in 0..=(if is_3d { radius } else { 0 }) { for dy in -radius..=radius { for dx in -radius..=radius {
        if (dx == 0 && dy == 0 && dz == 0) || dx * dx + dy * dy + dz * dz > radius_squared { continue; }
        let mut at = origin;
        for axis in 0..3 { at[axis] = at[axis].checked_add(width[axis] * dx + height[axis] * dy + depth[axis] * dz)?; }
        out.push(at);
    }}}
    Some(out)
}

/// Source BoxAOEIterator: depth, then height, then width. Unknown or very
/// large definitions are refused until their iteration can be bounded safely.
pub(super) fn box_offsets(game: &super::Game<'_>, stack: &ItemStack, origin: [i32; 3], face: [i32; 3]) -> Option<Vec<[i32; 3]>> {
    let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
    let expanded = super::tconstruct_source_overslime::upgrade(stack, "tconstruct:expanded") + traits.levels.get("tconstruct:expanded").copied().unwrap_or(0);
    if expanded < 0 { return None; }
    let (namespace, name) = stack.item.split_once(':')?;
    let layers = game.world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref()?;
    let bytes = layers.get(&format!("{namespace}:tinkering/tool_definitions/{name}"))?.last()?;
    let definition: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let modules = definition.get("modules")?.as_array()?;
    let aoe: Vec<_> = modules.iter().filter(|module| module.get("type").and_then(serde_json::Value::as_str).is_some_and(|kind| kind.ends_with("_aoe"))).collect();
    if aoe.len() != 1 || aoe[0].get("type")?.as_str()? != "tconstruct:box_aoe" { return None; }
    let size = |raw: Option<&serde_json::Value>| -> Option<[i32; 3]> {
        let Some(raw) = raw else { return Some([0, 0, 0]); };
        let obj = raw.as_object()?;
        if obj.keys().any(|key| !matches!(key.as_str(), "width" | "height" | "depth")) { return None; }
        let mut out = [0; 3];
        for (i, key) in ["width", "height", "depth"].iter().enumerate() {
            out[i] = i32::try_from(obj.get(*key).map_or(Some(0), serde_json::Value::as_i64)?).ok()?;
            if out[i] < 0 { return None; }
        }
        Some(out)
    };
    let mut dimensions = size(aoe[0].get("bonus"))?;
    let expansions = match aoe[0].get("expansions") { None => Vec::new(), Some(value) => value.as_array()?.iter().map(|entry| size(Some(entry))).collect::<Option<Vec<_>>>()? };
    if !expansions.is_empty() {
        let cycles = expanded / i32::try_from(expansions.len()).ok()?;
        let remainder = usize::try_from(expanded % i32::try_from(expansions.len()).ok()?).ok()?;
        for (i, delta) in expansions.iter().enumerate() {
            let count = cycles.checked_add(i32::from(i < remainder))?;
            for axis in 0..3 { dimensions[axis] = dimensions[axis].checked_add(delta[axis].checked_mul(count)?)?; }
        }
    }
    let [w, h, d] = dimensions;
    if w > 16 || h > 16 || d > 16 { return None; }
    let look = match ((game.yaw / 90.0 + 0.5).floor() as i32).rem_euclid(4) { 0 => [0, 0, 1], 1 => [-1, 0, 0], 2 => [0, 0, -1], _ => [1, 0, 0] };
    let clockwise = |direction: [i32; 3]| [-direction[2], 0, direction[0]];
    let reverse = |direction: [i32; 3]| [-direction[0], -direction[1], -direction[2]];
    let (width, height, depth, down) = match aoe[0].get("expansion_direction")?.as_str()? {
        "tconstruct:side_hit" if face[1] != 0 => (clockwise(look), look, reverse(face), true),
        "tconstruct:side_hit" => ([face[2], 0, -face[0]], [0, 1, 0], reverse(face), true),
        "tconstruct:pitch" if game.pitch < -60.0 => (clockwise(look), look, [0, 1, 0], true),
        "tconstruct:pitch" if game.pitch > 60.0 => (clockwise(look), look, [0, -1, 0], true),
        "tconstruct:pitch" => (clockwise(look), [0, 1, 0], look, true),
        "tconstruct:height" => { let depth = if face[1] != 0 { look } else { reverse(face) }; (clockwise(depth), [0, 1, 0], depth, false) },
        _ => return None,
    };
    let mut out = Vec::new();
    for dz in 0..=d { for dy in (if down { -h } else { 0 })..=h { for dx in -w..=w {
        if dx == 0 && dy == 0 && dz == 0 { continue; }
        let mut at = origin;
        for axis in 0..3 { at[axis] = at[axis].checked_add(width[axis] * dx + height[axis] * dy + depth[axis] * dz)?; }
        out.push(at);
    }}}
    Some(out)
}

/// Resolve source vein distance. Traversal itself is lazy because a previous
/// placement may change a later candidate before it is visited.
pub(super) fn vein_distance(game: &super::Game<'_>, stack: &ItemStack) -> Option<i32> {
    let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
    let expanded = super::tconstruct_source_overslime::upgrade(stack, "tconstruct:expanded") + traits.levels.get("tconstruct:expanded").copied().unwrap_or(0);
    let (namespace, name) = stack.item.split_once(':')?;
    let layers = game.world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref()?;
    let bytes = layers.get(&format!("{namespace}:tinkering/tool_definitions/{name}"))?.last()?;
    let definition: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let aoe: Vec<_> = definition.get("modules")?.as_array()?.iter().filter(|module| module.get("type").and_then(serde_json::Value::as_str).is_some_and(|kind| kind.ends_with("_aoe"))).collect();
    if aoe.len() != 1 || aoe[0].get("type")?.as_str()? != "tconstruct:vein_aoe" { return None; }
    let base = i32::try_from(aoe[0].get("max_distance").map_or(Some(0), serde_json::Value::as_i64)?).ok()?;
    let radius = base.checked_add(expanded)?;
    (0..=4).contains(&radius).then_some(radius)
}

struct Vein {
    name: String,
    radius: i32,
    visited: std::collections::HashSet<[i32; 3]>,
    pending: std::collections::VecDeque<([i32; 3], i32)>,
}
impl Vein {
    fn new(origin: [i32; 3], name: String, radius: i32) -> Self {
        let mut result = Self { name, radius, visited: std::collections::HashSet::from([origin]), pending: std::collections::VecDeque::new() };
        if radius > 0 { result.enqueue(origin, 1); }
        result
    }
    fn enqueue(&mut self, origin: [i32; 3], distance: i32) {
        for direction in [[0,-1,0], [0,1,0], [0,0,-1], [0,0,1], [-1,0,0], [1,0,0]] {
            let [Some(x), Some(y), Some(z)] = [origin[0].checked_add(direction[0]), origin[1].checked_add(direction[1]), origin[2].checked_add(direction[2])] else { continue; };
            let position = [x, y, z];
            if self.visited.insert(position) { self.pending.push_back((position, distance)); }
        }
    }
    fn next(&mut self, game: &super::Game<'_>) -> Option<[i32; 3]> {
        while let Some((position, distance)) = self.pending.pop_front() {
            if !game.world.loaded(position[0], position[2]) { continue; }
            if game.res.facts().get(game.world.block(position[0], position[1], position[2])).name.as_ref() != self.name.as_str() { continue; }
            if distance < self.radius { self.enqueue(position, distance + 1); }
            return Some(position);
        }
        None
    }
}

/// The source tests the candidate box against the target's inflated box and
/// then compares full 3D entity positions using a strict squared radius.
pub(super) fn in_range(target: [f64; 3], candidate: [f64; 3], target_bounds: ([f64; 3], [f64; 3]), candidate_bounds: ([f64; 3], [f64; 3]), expanded: u32) -> bool {
    let radius = 1.0 + f64::from(expanded);
    let inflation = [radius, 0.25, radius];
    let intersects = (0..3).all(|i| target_bounds.0[i] - inflation[i] < candidate_bounds.1[i] && target_bounds.1[i] + inflation[i] > candidate_bounds.0[i]);
    intersects && (0..3).map(|i| { let delta = target[i] - candidate[i]; delta * delta }).sum::<f64>() < radius * radius
}

/// Full fluid makes supported independent target effects order-insensitive. With
/// just a primary and one secondary, their source order is known even when
/// the remaining fluid scales the second target's effect.
pub(super) fn deferred_recipe_supported(game: &super::Game<'_>, stack: &ItemStack, level: i32, count: usize, order_known: bool) -> bool {
    if stack.count != 1 || stack.wire_patch().is_some() || level <= 0 { return false; }
    let Some(Tag::Int(capacity)) = stack.data().and_then(|data| data.other("minecraft:custom_data")).and_then(|custom| custom.get("tic_tank_capacity")) else { return false; };
    if *capacity <= 0 { return false; }
    let Some(Some(fluid)) = super::pluto_stack_fluid::read(stack, "tic_persistent", "tconstruct:tank_fluid") else { return false; };
    if fluid.tag.is_some() || fluid.amount == 0 || fluid.amount > *capacity as u32 { return false; }
    let Some(recipe) = super::tconstruct_source_slurping::recipe(game, &fluid.id) else { return false; };
    let mut ids = std::collections::HashSet::new();
    let mut hunger_seen = false;
    if recipe.entity.is_empty() || recipe.entity.iter().any(|effect| match effect {
        super::tconstruct_source_fluid_effects::EntityEffect::Mob(effect) => horizon_client::brewing::effect(&effect.id).is_none_or(|known| horizon_client::brewing::is_mod_effect(known.id) || matches!(known.id, "instant_health" | "instant_damage") || !ids.insert(known.id)),
        super::tconstruct_source_fluid_effects::EntityEffect::Remove(id) => horizon_client::brewing::effect(id).is_none_or(|known| known.id != "poison" || !ids.insert(known.id)),
        super::tconstruct_source_fluid_effects::EntityEffect::Hunger { .. } => { let duplicate = hunger_seen; hunger_seen = true; duplicate },
        super::tconstruct_source_fluid_effects::EntityEffect::Push { .. } => true,
    }) { return false; }
    if order_known { return true; }
    u32::try_from(count).ok().and_then(|count| recipe.amount.checked_mul(level as u32).and_then(|amount| amount.checked_mul(count))).is_some_and(|required| fluid.amount >= required)
}

/// Experimental entity-use path, disabled by the Station recipe. Only mob
/// targets and mob area members are currently resolved; guest players and
/// non-mob entities need an authoritative host-backed enumeration.
pub(super) fn interact(game: &mut super::Game<'_>, entity: i32, stack: Option<&ItemStack>, claimed: &mut bool) {
    if *claimed || game.remote.is_some() { return; }
    // Guest player positions are captured from the active packet's LanHost;
    // do not guess if the packet-scoped snapshot is absent.
    let other_players = if let Some(conn) = game.lan_acting {
        let Some(positions) = game.world.chunks.__horizon_mixins_tconstruct_splashing_guest_scope().get(&conn) else { return; };
        positions.clone()
    } else {
        game.lan.as_ref().map(|host| host.hub.guests.iter().filter(|guest| guest.placed && guest.dimension == game.dim.id()).map(|guest| {
            let bounds = guest.bounds(); let min = bounds.min_position(); let max = bounds.max_position();
            (Some(guest.conn), guest.entity_id, guest.position, [min.x, min.y, min.z], [max.x, max.y, max.z], guest.motion.is_some())
        }).collect()).unwrap_or_default()
    };
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
    let target_mob = &game.mobs[target];
    let origin = [target_mob.position.x, target_mob.position.y, target_mob.position.z];
    let target_box = target_mob.bounds();
    let min = target_box.min_position(); let max = target_box.max_position();
    let target_bounds = ([min.x, min.y, min.z], [max.x, max.y, max.z]);
    // Living armor stands also receive source effects, but their effect adapter
    // is not yet implemented. Do not perform a partial area operation.
    let radius = 1.0 + f64::from(expanded);
    let mut player_targets = Vec::new();
    for (conn, _entity_id, position, min, max, ready) in other_players {
        if conn.is_none() {
            if (0..3).map(|axis| (position[axis] - origin[axis]).powi(2)).sum::<f64>() < radius * radius { return; }
        } else if in_range(origin, position, target_bounds, (min, max), expanded) {
            if !ready { return; }
            player_targets.push(conn.unwrap());
        }
    }
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
    if !player_targets.is_empty() {
        // This packet can only carry the mutable LanHost at its outer return.
        // Larger groups require full funding; a primary with one secondary
        // retains its known order even with partial fluid.
        let Some(conn) = game.lan_acting else { return; };
        if targets.iter().any(|index| !super::tconstruct_source_spilling::eligible_mob(&game.mobs[*index])) { return; }
        if !deferred_recipe_supported(game, &stack, *level, targets.len() + player_targets.len(), targets.len() == 1 && player_targets.len() == 1) { return; }
        let mob_ids = targets.iter().map(|index| game.mobs[*index].id).collect();
        let slot = game.held_slot();
        let creative = game.creative;
        let modifier_level = *level;
        game.world.chunks.__horizon_mixins_tconstruct_splashing_guest_pending_mut().insert(conn, (stack.clone(), slot, mob_ids, player_targets, creative, modifier_level, false));
        game.add_cooldown(stack.item.clone(), (20.0 / speed) as i32);
        *claimed = true;
        return;
    }
    // A primary target can reject an effect while a nearby target accepts it.
    if !targets.iter().any(|index| super::tconstruct_source_spilling::apply(game, *index, stack.clone(), "tconstruct:splashing", true).is_some()) { return; }
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

/// Source AOE paths for bundled glow/fire/snow placement effects.
/// Non-empty tool AOE iterators and other block-effect forms stay unresolved.
pub(super) fn block(game: &mut super::Game<'_>, stack: &ItemStack, clicked: [i32; 3], face: [i32; 3]) -> Option<horizon_client::Result<bool>> {
    if game.remote.is_some() || !right_click(game, stack) || super::tconstruct_source_melee_shape::intact(stack).is_none() { return None; }
    if !matches!(face, [0, -1, 0] | [0, 1, 0] | [0, 0, -1] | [0, 0, 1] | [-1, 0, 0] | [1, 0, 0]) { return None; }
    if game.cooldowns.is_on_cooldown(&stack.item) { return None; }
    let mut vein = None;
    let offsets = if single_block(game, stack) { Vec::new() }
        else if let Some(offsets) = circle_offsets(game, stack, clicked, face).or_else(|| box_offsets(game, stack, clicked, face)) { offsets }
        else {
            let radius = vein_distance(game, stack)?;
            let name = game.res.facts().get(game.world.block(clicked[0], clicked[1], clicked[2])).name.to_string();
            vein = Some(Vein::new(clicked, name, radius));
            Vec::new()
        };
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
    if effects.len() != 1 { return None; }
    let effect = effects[0].as_object()?;
    let kind = effect.get("type")?.as_str()?;
    let is_cloud = kind == "tconstruct:mob_effect_cloud";
    let mut cloud = Vec::new();
    let (block, sound) = if is_cloud {
        if effect.keys().any(|key| !matches!(key.as_str(), "type" | "effects")) { return None; }
        for entry in effect.get("effects")?.as_array()? {
            let entry = entry.as_object()?;
            if entry.keys().any(|key| !matches!(key.as_str(), "effect" | "time" | "level")) { return None; }
            let def = horizon_client::brewing::effect(entry.get("effect")?.as_str()?)?;
            if def.instant || horizon_client::brewing::is_mod_effect(def.id) { return None; }
            let duration = i32::try_from(entry.get("time").map_or(Some(-1), serde_json::Value::as_i64)?).ok()?;
            let level = i32::try_from(entry.get("level").map_or(Some(1), serde_json::Value::as_i64)?).ok()?;
            if !(1..=256).contains(&level) { return None; }
            if duration > 0 { cloud.push((def.id, duration, level - 1)); }
        }
        if cloud.is_empty() { return None; }
        ("", None)
    } else {
        if kind != "tconstruct:place_block" || !game.may_build() { return None; }
        let block = effect.get("block")?.as_str()?;
        if !matches!(block, "tconstruct:glow" | "minecraft:fire" | "minecraft:snow") { return None; }
        if effect.keys().any(|key| !matches!(key.as_str(), "type" | "block" | "sound")) { return None; }
        let sound = match effect.get("sound") { None => None, Some(value) => Some(value.as_str()?.to_owned()) };
        (block, sound)
    };
    let amount = u32::try_from(recipe.pointer("/fluid/amount")?.as_u64()?).ok().filter(|amount| *amount > 0)?;
    // Snow layers use BlockItem placement and require the actual hit point.
    // The game exposes the packet's forced hit here for guest interactions.
    let hit_point = if block == "minecraft:snow" || is_cloud {
        let hit = game.pick()?;
        if hit.pos != clicked || hit.face != face { return None; }
        Some(hit.point)
    } else { None };
    let held = game.held()?;
    if held.damage != stack.damage || !held.same_item_same_components(stack) { return None; }
    // Stage every possible remainder before changing the world. Failed
    // positions do not advance the stage; the source consumes once per success.
    let max_positions = vein.as_ref().map_or(offsets.len() + 1, |area| ((area.radius * 2 + 1) as usize).pow(3));
    let mut staged = Vec::new();
    let mut remaining = fluid.clone();
    while staged.len() < max_positions && remaining.amount > 0 {
        let (used, scaled_cloud) = if is_cloud {
            let scale = (*level as f32).min(remaining.amount as f32 / amount as f32);
            let mut scaled = Vec::new();
            for &(id, duration, amplifier) in &cloud {
                let value = duration as f32 * scale;
                if !value.is_finite() || value > (i32::MAX / 4) as f32 { return None; }
                let duration = value as i32;
                if duration > 10 { duration.checked_mul(4)?; scaled.push((id, duration, amplifier)); }
            }
            if scaled.is_empty() { break; }
            let used = ((amount as f32 * scale).ceil() as u32).min(remaining.amount);
            (used, scaled)
        } else {
            if remaining.amount < amount { break; }
            (amount, Vec::new())
        };
        if used == 0 { break; }
        remaining.amount -= used;
        let updated = super::pluto_stack_fluid::write(stack, "tic_persistent", "tconstruct:tank_fluid", (remaining.amount > 0).then_some(&remaining), *capacity as u32)?;
        staged.push((updated, scaled_cloud));
    }
    game.add_cooldown(stack.item.clone(), (20.0 / speed) as i32);
    let result = (|| -> horizon_client::Result<bool> {
        let mut successes = 0usize;
        let mut failure = None;
        let mut primary = Some(clicked);
        let mut offsets = offsets.into_iter();
        loop {
            if successes >= staged.len() { break; }
            let (position, is_primary) = if let Some(position) = primary.take() { (position, true) }
                else if let Some(area) = vein.as_mut() { let Some(position) = area.next(game) else { break; }; (position, false) }
                else { let Some(position) = offsets.next() else { break; }; (position, false) };
            if !is_primary && vein.is_none() && game.res.facts().is_air(game.world.block(position[0], position[1], position[2])) { continue; }
            let snow_context = if block == "minecraft:snow" {
                let direction = match face { [0,-1,0] => horizon_core::geometry::Direction::Down, [0,1,0] => horizon_core::geometry::Direction::Up, [0,0,-1] => horizon_core::geometry::Direction::North, [0,0,1] => horizon_core::geometry::Direction::South, [-1,0,0] => horizon_core::geometry::Direction::West, [1,0,0] => horizon_core::geometry::Direction::East, _ => continue };
                let hit = hit_point.unwrap();
                let offset = [position[0] - clicked[0], position[1] - clicked[1], position[2] - clicked[2]];
                Some(horizon_world::level::PlaceContext {
                    clicked_pos: horizon_core::position::BlockPos::new(position[0], position[1], position[2]),
                    face: direction, hit: horizon_core::geometry::Vec3::new(hit[0] + f64::from(offset[0]), hit[1] + f64::from(offset[1]), hit[2] + f64::from(offset[2])),
                    inside: false, yaw: game.yaw, pitch: game.pitch,
                    sneaking: game.keys.contains(&super::KeyCode::ShiftLeft),
                    item: "minecraft:snow".to_owned(), can_use_game_master_blocks: game.can_use_game_master_blocks(),
                })
            } else { None };
            let target = if is_cloud {
                let Some(x) = position[0].checked_add(face[0]) else { continue; };
                let Some(y) = position[1].checked_add(face[1]) else { continue; };
                let Some(z) = position[2].checked_add(face[2]) else { continue; };
                [x, y, z]
            } else if let Some(context) = snow_context.as_ref() {
                let Some((pos, state)) = game.with_level(|blocks, world, ticks| blocks.placement_state(world, ticks, context)) else { continue; };
                if game.res.facts().get(state).name.as_ref() != "minecraft:snow" { continue; }
                [pos.x, pos.y, pos.z]
            } else if game.res.facts().get(game.world.block(position[0], position[1], position[2])).replaceable { position }
            else {
                let Some(x) = position[0].checked_add(face[0]) else { continue; };
                let Some(y) = position[1].checked_add(face[1]) else { continue; };
                let Some(z) = position[2].checked_add(face[2]) else { continue; };
                [x, y, z]
            };
            let before = game.world.block(target[0], target[1], target[2]);
            let attempt = (|| -> horizon_client::Result<bool> {
                if is_cloud {
                    if !game.world.loaded(target[0], target[2]) || !game.res.facts().get(game.world.block(target[0], target[1], target[2])).replaceable { return Ok(false); }
                    let hit = hit_point.unwrap();
                    let at = [hit[0] + f64::from(position[0] - clicked[0]), hit[1] + f64::from(position[1] - clicked[1]), hit[2] + f64::from(position[2] - clicked[2])];
                    super::potions::tconstruct_source_fluid_cloud::spawn(game, at, staged[successes].1.clone());
                    return Ok(true);
                }
                if block == "tconstruct:glow" {
                    return super::tconstruct_glowing_world::place(game, target, [-face[0], -face[1], -face[2]]);
                }
                let permitted = match game.lan_acting {
                    Some(conn) => game.world.chunks.__horizon_mixins_tconstruct_glow_guest_protection().get(&conn)
                        .is_some_and(|policy| super::tconstruct_glowing_placement::outside_protection(*policy, target)),
                    None => game.options.dedicated.is_none(),
                };
                if !permitted || !game.world.loaded(target[0], target[2]) || (block != "minecraft:snow" && !game.res.facts().get(game.world.block(target[0], target[1], target[2])).replaceable) { return Ok(false); }
                if let Some(context) = snow_context.as_ref() {
                    let outcome = game.with_level(|blocks, world, ticks| blocks.place_block(world, ticks, context));
                    game.process_level_events()?;
                    return Ok(matches!(outcome, horizon_world::level::PlaceOutcome::Placed { pos, .. } if [pos.x, pos.y, pos.z] == target));
                }
                let pos = horizon_core::position::BlockPos::new(target[0], target[1], target[2]);
                let (state, survives) = game.with_level(|blocks, world, ticks| {
                    let state = blocks.fire_state_for(world, ticks, pos);
                    (state, blocks.can_survive(world, ticks, pos, state))
                });
                if !survives || game.res.facts().get(state).name.as_ref() != "minecraft:fire" { return Ok(false); }
                game.level_set(target, state)?;
                Ok(game.world.block(target[0], target[1], target[2]) == state)
            })();
            match attempt {
                Ok(true) => {
                    successes += 1;
                    if !is_cloud {
                        let default_sound = game.res.facts().get(game.world.block(target[0], target[1], target[2])).sounds.place.clone();
                        game.audio.play(sound.as_deref().unwrap_or(&default_sound), Some([target[0] as f64 + 0.5, target[1] as f64 + 0.5, target[2] as f64 + 0.5]), 1.0, 1.0);
                        game.post_block_game_event("minecraft:block_place", target, game.sculk_actor(), None);
                    }
                }
                Ok(false) => {}
                Err(error) => {
                    // level_set may report an event-processing error after the
                    // block write; account for a completed write before exiting.
                    let after = game.world.block(target[0], target[1], target[2]);
                    if before != after && game.res.facts().get(after).name.as_ref() == block { successes += 1; }
                    failure = Some(error);
                    break;
                }
            }
        }
        if successes > 0 && !game.creative {
            *game.held_mut() = Some(staged[successes - 1].0.clone());
            if let Some(current) = game.held().cloned() {
                let wear = (*level as u32).saturating_mul(successes as u32);
                let (loss, shield) = super::tconstruct_source_overslime_host::damage_with_cause(game, &current, wear, Some("tconstruct:splashing"));
                if let Some(held) = game.held_mut().as_mut() {
                    super::tconstruct_source_overslime::spend(held, shield);
                    super::tconstruct_tx::fixed_damage(held, loss);
                }
            }
        }
        if let Some(error) = failure { return Err(error); }
        Ok(true)
    })();
    Some(result)
}
