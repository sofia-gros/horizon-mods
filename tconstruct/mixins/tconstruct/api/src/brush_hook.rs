//! Connects the brush module to PlutoLib's per-stack item use ticks. The use itself — the BRUSH pose and
//! the 200 tick duration for a stack carrying the module — is composed into the throwing hook's per-stack
//! registration (a stack that can be eaten eats and a blocking stack blocks first, as the original's hook
//! order runs them; see `throwing_hook`), so this hook runs the strokes and the use's ends: on every tick
//! it raytraces the player's view and releases the use when the ray stops hitting a block; on a stroke
//! tick it brushes the hit block and the tool's AOE transform neighbours through the world brush call,
//! writes the stroke's wear onto the hand holding the tool, and stops the use when the wear broke it —
//! the ends BrushModule.onUsingTick runs, asked of PlutoLib's player surface and applied by the game at
//! its seam (see `brush`'s notes). It also answers the brush's highlight through PlutoLib's highlight
//! surface (see `highlight`). The range the strokes aim over is the player's block-interaction-range
//! attribute (see `block_reach`), and a stroke that brushes nothing plays the generic brush sound for
//! the acting player alone (see `brush`'s notes for the dust that stays open).
use crate::{break_speed_hook::with_held_tool, brush, edible, edible_hook, tool_damage_hook, tool_slot, tool_stack};
use pluto_lib::highlight_api_v1::{HighlightFacts, HighlightRegistry};
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
    let block_at = |pos: [i32; 3]| world.get_block_state(pos).map(|state| state.block.clone());
    let in_tag_at = |pos: [i32; 3], tag: &Identifier| world.get_block_state(pos).is_some_and(|state| state.is_in_tag(&tag.to_string()));
    neighbours_with(stack, origin, face, yaw, pitch, &block_at, &in_tag_at)
}

/// The neighbours over caller-read blocks: the stroke reads them from the tick's world
/// scope and the highlight from the frame facts' own reads, and both walk the same
/// iterator over the same origin and face, so the outlined set is the stroke's own set.
fn neighbours_with(stack: &StackData, origin: [i32; 3], face: [i32; 3], yaw: f32, pitch: f32, block_at: &dyn Fn([i32; 3]) -> Option<Identifier>, in_tag_at: &dyn Fn([i32; 3], &Identifier) -> bool) -> Vec<[i32; 3]> {
    let Ok(positions) = with_held_tool(stack, |env, tool| {
        let Some(iterator) = env.definition.and_then(|definition| definition.aoe()) else { return Vec::new() };
        let expanded = match tool.custom_data.get(tool_stack::TAG_VOLATILE) {
            Some(Nbt::Compound(volatile)) => crate::modifier_entries::nbt_int(volatile.get("tconstruct:expanded")),
            _ => 0,
        };
        let positions = iterator.positions(origin, direction(face), yaw, pitch, expanded, block_at, in_tag_at);
        if matches!(iterator, crate::aoe::Iterator::Box { .. } | crate::aoe::Iterator::Circle { .. }) {
            positions.into_iter().filter(|pos| block_at(*pos).is_some_and(|block| block.to_string() != "minecraft:air")).collect()
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
/// selected slot, the off hand through its equipment slot. True when the wear broke
/// the tool, as damageAnimated answers — the answer onUsingTick stops the use by.
fn wear(tick: &UseTick, amount: i32) -> bool {
    if amount <= 0 {
        return false;
    }
    let cause = Identifier::parse(brush::MODIFIER).expect("valid id");
    let Ok(Some((worn, broke))) = with_held_tool(&tick.stack, |env, _tool| {
        let mut slot = slot_of(&tick.stack)?;
        let outcome = tool_slot::damage_slot(&mut slot, env, amount, !tick.creative, true, Some(&cause), &mut tool_damage_hook::random).ok()?;
        Some((stack_of(&slot), outcome.broke))
    }) else { return false };
    let main = tick.player.main_hand_stack().is_some_and(|held| held.item == tick.stack.item && held.custom_data == tick.stack.custom_data);
    let written = if main {
        tick.player.selected_slot().is_some_and(|selected| tick.player.update_stack(selected, &worn))
    } else {
        tick.player.set_equipment(StackSlot::OffHand, Some(&worn))
    };
    // damageAnimated's other half: a stroke that broke the tool announces it, at the hand that held it.
    if written && broke {
        tick.player.broadcast_item_break(if main { StackSlot::MainHand } else { StackSlot::OffHand }, &worn);
    }
    broke
}

/// The range BrushModule.calculateHitResult aims over: the player's block-interaction-range
/// attribute as the game computes it, falling back to the attribute's base values when the
/// game cannot answer (see `brush::reach_with`).
fn block_reach(tick: &UseTick) -> f64 {
    brush::reach_with(tick.player.attribute_value(brush::BLOCK_REACH), tick.creative)
}

/// One tick of a brush use (BrushModule.onUsingTick): the ray decides the use's end on
/// every tick, the stroke lands on the stroke ticks, and a stroke whose wear breaks the
/// tool stops the use. The ends are asked of the player surface and applied by the game
/// at its seam (see `brush`'s notes).
fn on_tick(tick: &UseTick) {
    if tick.duration != brush::USE_DURATION as u32 || !brush_use(&tick.stack) {
        return;
    }
    let world = tick.world();
    let hit = brush::ray_hit(tick.eye, tick.look, block_reach(tick), &|pos| world.get_block_state(pos).map(|state| state.block.to_string()));
    let blocked = hit.as_ref().is_some_and(|(_, _, distance)| blocked_by_entity(tick, *distance));
    if brush::releases(hit.is_some(), blocked) {
        tick.player.release_use();
        return;
    }
    if !brush::is_phase(tick.ticks_used) {
        return;
    }
    let Some((hit, face, _)) = hit else { return };
    // The look as the AOE iterator asks for it (yaw and pitch in degrees), from the tick's
    // look vector: the inverse of the release telling's own construction of it.
    let yaw = (-tick.look[0]).atan2(tick.look[2]).to_degrees() as f32;
    let pitch = (-tick.look[1]).clamp(-1.0, 1.0).asin().to_degrees() as f32;
    let neighbours = neighbours(&tick.stack, &world, hit, face, yaw, pitch);
    let damage = brush::stroke(&world, hit, face, &neighbours);
    if brush::fallback(damage) {
        world.brush_fallback_sound(hit);
    }
    if wear(tick, damage) {
        tick.player.stop_use();
    }
}

/// The brush's highlight (BrushModule.shouldHighlight, asked by the original's render
/// event over the AOE iterator's DISPLAY predicate): while the player holds a brush
/// tool and aims at a block, the neighbours the stroke would brush — the brushable
/// ones — are outlined. The aimed block itself is not returned: the game draws its
/// own outline around the target, as vanilla does, and the original's event adds
/// only the extra blocks. The frame's view hit is the game's own ray, which runs
/// far past the brush's reach; a target past the frame's block reach highlights
/// nothing (the reach is the player's game mode's own, recorded by the
/// pluto-highlight-mode reach).
fn highlight(facts: &HighlightFacts<'_>) -> Vec<[i32; 3]> {
    let Some(stack) = facts.main_hand else { return Vec::new() };
    let stack = stack_of(stack);
    if !brush_use(&stack) {
        return Vec::new();
    }
    let Some(target) = &facts.target else { return Vec::new() };
    if target.distance > facts.block_reach {
        return Vec::new();
    }
    neighbours_with(&stack, target.pos, target.face, facts.yaw, facts.pitch, facts.block_at, facts.in_tag_at)
        .into_iter()
        .filter(|pos| (facts.block_at)(*pos).is_some_and(|block| brush::brushable(&block.to_string())))
        .collect()
}

/// Registers the use tick callback and the highlight provider. Called once from the
/// product's hooks, after the throwing hook (whose registration composes the brush
/// use itself).
pub(crate) fn register() {
    ItemEvents::USE_TICK.register(on_tick);
    HighlightRegistry::register(highlight);
}
