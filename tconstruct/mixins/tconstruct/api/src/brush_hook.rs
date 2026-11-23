//! Connects the brush module to PlutoLib's per-stack item use ticks. The use itself — the BRUSH pose and
//! the 200 tick duration for a stack carrying the module — is composed into the throwing hook's per-stack
//! registration (a stack that can be eaten eats and a blocking stack blocks first, as the original's hook
//! order runs them; see `throwing_hook`), so this hook only runs the strokes: on a stroke tick it raytraces
//! the player's view, brushes the hit block and the tool's AOE transform neighbours through the world brush
//! call, and writes the stroke's wear onto the hand holding the tool. What the strokes cannot do here is
//! in `brush`'s notes (the release on a missed ray, the highlight, the client-side fallback sound).
use crate::{break_speed_hook::with_held_tool, brush, edible, edible_hook, tool_damage_hook, tool_slot, tool_stack};
use pluto_lib::{Direction, EntityReach, Identifier, ItemEvents, ItemSlot, Nbt, StackData, StackSlot, UseTick, World};

/// Whether the stack can start being eaten, as the edible registration asks (EdibleModule.onToolUse).
fn eatable(stack: &StackData) -> bool {
    edible_hook::eating(stack).is_some_and(|(hunger, _)| edible::can_start(hunger, false))
}

/// Whether this stack's use is a brush use: it carries the brush module (whole — a broken tool's
/// modules do not run), and no earlier use of the composed registration owns it (an eatable stack
/// eats, a blocking stack blocks).
fn brush_use(stack: &StackData) -> bool {
    if eatable(stack) {
        return false;
    }
    with_held_tool(stack, |env, tool| tool_stack::brush(&tool.custom_data, env) && !tool_stack::blocking(&tool.custom_data, env)).unwrap_or(false)
}

/// The face as the AOE iterator names it.
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

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] { [a[0] + b[0], a[1] + b[1], a[2] + b[2]] }
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
fn scale(a: [f64; 3], by: f64) -> [f64; 3] { [a[0] * by, a[1] * by, a[2] * by] }
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
fn length(a: [f64; 3]) -> f64 { dot(a, a).sqrt() }

/// Whether a living entity stands between the eye and the block the ray hit, nearer than it:
/// BrushModule.calculateHitResult returns the entity's hit then, not the block's, and the use
/// releases without a stroke. Read as the sling's bonk search reads it — an entity whose
/// middle the ray passes within 0.8, inside the box the ray sweeps to the block.
fn blocked_by_entity(tick: &UseTick, distance: f64) -> bool {
    let self_id = EntityReach::id_of_uuid(tick.player.uuid);
    let end = add(tick.eye, scale(tick.look, distance));
    let min = [tick.eye[0].min(end[0]) - 1.0, tick.eye[1].min(end[1]) - 1.0, tick.eye[2].min(end[2]) - 1.0];
    let max = [tick.eye[0].max(end[0]) + 1.0, tick.eye[1].max(end[1]) + 1.0, tick.eye[2].max(end[2]) + 1.0];
    for entity in EntityReach::in_box(min, max) {
        if entity.living.is_none() || Some(entity.id) == self_id {
            continue;
        }
        let Some(position) = EntityReach::position_of(entity.id) else { continue };
        let middle = [position[0], position[1] + 0.9, position[2]];
        let along = dot(sub(middle, tick.eye), tick.look);
        if along < 0.0 || along >= distance {
            continue;
        }
        if length(sub(middle, add(tick.eye, scale(tick.look, along)))) <= 0.8 {
            return true;
        }
    }
    false
}

/// The tool's AOE transform neighbours of the hit block (the definition's AOE_ITERATOR over
/// the hit, as the break hook computes its area): box and circle candidates still owe the
/// default predicate's other half to the caller — here only air is skipped, as the original
/// skips it, and the stroke itself tests each candidate for brushability.
fn neighbours(stack: &StackData, world: &World, origin: [i32; 3], face: [i32; 3], yaw: f32, pitch: f32) -> Vec<[i32; 3]> {
    let Ok(positions) = with_held_tool(stack, |env, tool| {
        let Some(iterator) = env.definition.and_then(|definition| definition.aoe()) else { return Vec::new(); };
        let expanded = match tool.custom_data.get(tool_stack::TAG_VOLATILE) {
            Some(Nbt::Compound(volatile)) => crate::modifier_entries::nbt_int(volatile.get("tconstruct:expanded")),
            _ => 0,
        };
        let block_at = |pos: [i32; 3]| world.get_block_state(pos).map(|state| state.block.clone());
        let in_tag_at = |pos: [i32; 3], tag: &Identifier| world.get_block_state(pos).is_some_and(|state| state.is_in_tag(&tag.to_string()));
        let positions = iterator.positions(origin, direction(face), yaw, pitch, expanded, &block_at, &in_tag_at);
        if matches!(iterator, crate::aoe::Iterator::Box { .. } | crate::aoe::Iterator::Circle { .. }) {
            positions.into_iter().filter(|pos| world.get_block_state(*pos).is_some_and(|state| state.block.to_string() != "minecraft:air")).collect()
        } else {
            positions
        }
    }) else { return Vec::new() };
    positions
}

/// The item slot view of a stack, for the tool damage pipeline (which works on slots).
/// Shared with the fireball's helmet wear, which damages a worn piece the same way.
pub(crate) fn slot_of(stack: &StackData) -> Option<ItemSlot> {
    let mut slot = ItemSlot::new(Identifier::parse(&stack.item).ok()?, stack.count);
    slot.damage = stack.damage;
    slot.max_damage = stack.max_damage;
    slot.custom_data = stack.custom_data.clone();
    slot.custom_model_data = stack.custom_model_data.clone();
    if !stack.enchantments.is_empty() {
        slot.enchantments = Some(stack.enchantments.clone());
    }
    Some(slot)
}

/// The stack a worn slot view writes back as. The hand writes carry count, damage and custom
/// data (the main hand's `update_stack` writes exactly those), so nothing else is invented.
/// Shared with the fireball's helmet wear, which writes a worn piece back the same way.
pub(crate) fn stack_of(slot: &ItemSlot) -> StackData {
    StackData {
        item: slot.item.to_string(),
        count: slot.count,
        damage: slot.damage,
        max_damage: slot.max_damage,
        custom_data: slot.custom_data.clone(),
        custom_model_data: slot.custom_model_data.clone(),
        enchantments: slot.enchantments.clone().unwrap_or_default(),
    }
}

/// The stroke's wear, written onto the hand that holds the tool (BrushModule's
/// damageAnimated, with the brushing modifier as its cause): the main hand through its
/// selected slot, the off hand through its equipment slot. A tool the wear breaks is
/// written back broken; the use itself ends at its release or its 200 ticks, as a use
/// tick cannot end it (see `brush`'s notes).
fn wear(tick: &UseTick, amount: i32) {
    if amount <= 0 {
        return;
    }
    let cause = Identifier::parse(brush::MODIFIER).expect("valid id");
    let Ok(Some(worn)) = with_held_tool(&tick.stack, |env, _tool| {
        let mut slot = slot_of(&tick.stack)?;
        tool_slot::damage_slot(&mut slot, env, amount, !tick.creative, true, Some(&cause), &mut tool_damage_hook::random).ok()?;
        Some(stack_of(&slot))
    }) else { return };
    let main = tick.player.main_hand_stack().is_some_and(|held| held.item == tick.stack.item && held.custom_data == tick.stack.custom_data);
    if main {
        if let Some(selected) = tick.player.selected_slot() {
            tick.player.update_stack(selected, &worn);
        }
    } else {
        tick.player.set_equipment(StackSlot::OffHand, Some(&worn));
    }
}

/// One tick of a brush use: the stroke, when this tick is one (see `brush`).
fn on_tick(tick: &UseTick) {
    if tick.duration != brush::USE_DURATION as u32 || !brush_use(&tick.stack) {
        return;
    }
    if !brush::is_phase(tick.ticks_used) {
        return;
    }
    let world = tick.world();
    let Some((hit, face, distance)) = brush::ray_hit(tick.eye, tick.look, brush::reach(tick.creative), &|pos| world.get_block_state(pos).map(|state| state.block.to_string())) else { return };
    if blocked_by_entity(tick, distance) {
        return;
    }
    // The look as the AOE iterator asks for it (yaw and pitch in degrees), from the tick's
    // look vector: the inverse of the release telling's own construction of it.
    let yaw = (-tick.look[0]).atan2(tick.look[2]).to_degrees() as f32;
    let pitch = (-tick.look[1]).clamp(-1.0, 1.0).asin().to_degrees() as f32;
    let neighbours = neighbours(&tick.stack, &world, hit, face, yaw, pitch);
    let damage = brush::stroke(&world, hit, face, &neighbours);
    wear(tick, damage);
}

/// Registers the use tick callback. Called once from the product's hooks, after the
/// throwing hook (whose registration composes the brush use itself).
pub(crate) fn register() {
    ItemEvents::USE_TICK.register(on_tick);
}
