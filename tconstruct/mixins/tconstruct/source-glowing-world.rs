// Authoritative source glow placement, shared by direct use, walking and hits.
use horizon_core::geometry::{Direction, ShapeOp, VoxelShape};

fn direction(normal: [i32; 3]) -> Option<Direction> {
    match normal {
        [0,-1,0] => Some(Direction::Down), [0,1,0] => Some(Direction::Up),
        [0,0,-1] => Some(Direction::North), [0,0,1] => Some(Direction::South),
        [-1,0,0] => Some(Direction::West), [1,0,0] => Some(Direction::East), _ => None,
    }
}
fn name(normal: [i32; 3]) -> Option<&'static str> {
    match normal {
        [0,-1,0] => Some("down"), [0,1,0] => Some("up"), [0,0,-1] => Some("north"),
        [0,0,1] => Some("south"), [-1,0,0] => Some("west"), [1,0,0] => Some("east"), _ => None,
    }
}

/// Check the original FULL occlusion face, not general collision or sturdiness.
pub(super) fn full_face(game: &super::Game<'_>, at: [i32;3], toward: [i32;3]) -> bool {
    if !game.world.loaded(at[0], at[2]) { return false; }
    let Some(face) = direction(toward) else { return false; };
    let state = game.world.block(at[0], at[1], at[2]);
    let Some((_, shape)) = game.pack.context.tables.visibility.occlusion_face(state, face) else { return false; };
    shape.join_is_not_empty(&VoxelShape::block(), ShapeOp::XOR).is_ok_and(|difference| !difference)
}

pub(super) fn place(game: &mut super::Game<'_>, at: [i32;3], preferred: [i32;3]) -> anyhow::Result<bool> {
    let permitted = match game.lan_acting {
        Some(conn) => game.world.chunks.__horizon_mixins_tconstruct_glow_guest_protection().get(&conn)
            .is_some_and(|policy| super::tconstruct_glowing_placement::outside_protection(*policy, at)),
        // A dedicated server has no local player actor outside a guest turn.
        None => game.options.dedicated.is_none(),
    };
    if !permitted { return Ok(false); }
    if !game.world.loaded(at[0], at[2]) { return Ok(false); }
    let before = game.world.block(at[0], at[1], at[2]);
    let facts = game.res.facts().get(before);
    let chosen = super::tconstruct_glowing_placement::support_direction(
        at, preferred, facts.replaceable, facts.has_class("LiquidBlock"), &*facts.name == "tconstruct:glow",
        |support, face| full_face(game, support, face),
    );
    let Some(chosen) = chosen else { return Ok(false); };
    let registry = game.pack.assets.registry;
    let Some(default) = registry.default_state("tconstruct:glow") else { return Ok(false); };
    let Some(mut state) = registry.state(default).cloned() else { return Ok(false); };
    state.properties.insert("facing".into(), name(chosen).unwrap().into());
    let Ok(state) = registry.resolve(&state) else { return Ok(false); };
    game.level_set(at, state)?;
    Ok(game.world.block(at[0], at[1], at[2]) == state)
}

/// Only the glow's recorded support can keep an existing block alive.
pub(super) fn survives(game: &super::Game<'_>, at: [i32;3]) -> bool {
    let state = game.world.block(at[0], at[1], at[2]);
    let Some(block) = game.pack.assets.registry.state(state) else { return false; };
    if block.name != "tconstruct:glow" { return true; }
    let facing = match block.properties.get("facing").map(String::as_str) {
        Some("down") => [0,-1,0], Some("up") => [0,1,0], Some("north") => [0,0,-1],
        Some("south") => [0,0,1], Some("west") => [-1,0,0], Some("east") => [1,0,0], _ => return false,
    };
    let Some(x) = at[0].checked_add(facing[0]) else { return false; };
    let Some(y) = at[1].checked_add(facing[1]) else { return false; };
    let Some(z) = at[2].checked_add(facing[2]) else { return false; };
    // Do not destroy a saved glow just because its neighboring chunk is not loaded.
    if !game.world.loaded(x,z) { return true; }
    full_face(game, [x,y,z], [-facing[0],-facing[1],-facing[2]])
}

pub(super) fn process_support_changes(game: &mut super::Game<'_>) -> anyhow::Result<()> {
    let pending = std::mem::take(game.world.chunks.__horizon_mixins_tconstruct_glow_support_checks_mut());
    for at in pending {
        if !game.world.loaded(at[0],at[2]) { continue; }
        let state = game.world.block(at[0],at[1],at[2]);
        if &*game.res.facts().get(state).name == "tconstruct:glow" && !survives(game,at) {
            game.level_set(at, game.res.facts().air)?;
        }
    }
    Ok(())
}

pub(super) fn has_glowing(game: &super::Game<'_>, stack: &horizon_items::items::ItemStack) -> bool {
    let traits = super::tconstruct_source_material_traits::levels(&game.world, stack);
    super::tconstruct_source_overslime::upgrade(stack, "tconstruct:glowing")
        + traits.as_ref().and_then(|data| data.levels.get("tconstruct:glowing").copied()).unwrap_or(0) > 0
}

/// Check source item tags and interaction toggles before consuming a click.
pub(super) fn eligible(game: &super::Game<'_>, stack: &horizon_items::items::ItemStack, left: bool) -> bool {
    use horizon_world::nbt::Tag;
    if super::tconstruct_source_ranged_wear::is_broken(stack) || !has_glowing(game, stack) { return false; }
    let Some(tags) = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v| v.as_ref().ok()) else { return false; };
    let belongs = |tag: &str| tags.get(tag).is_some_and(|values| values.contains(stack.item.as_ref()));
    let persisted = stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|n| n.get("tic_persistent"));
    let toggled = |key: &str| matches!(persisted.and_then(|n| n.get(key)), Some(Tag::List(values)) if values.iter().any(|v| matches!(v, Tag::String(s) if s.to_string_lossy() == "tconstruct:glowing")));
    super::tconstruct_glowing_placement::interaction_allowed(
        belongs("tconstruct:modifiable/interactable/left"), belongs("tconstruct:modifiable/interactable/right"),
        belongs("tconstruct:modifiable/interactable/dual"), &*stack.item == "tconstruct:fishing_rod",
        toggled("tconstruct:attack_modifiers"), toggled("tconstruct:interact_modifiers"), left,
    )
}

pub(super) fn off_cooldown(game: &super::Game<'_>, stack: &horizon_items::items::ItemStack) -> bool {
    game.res.items().get(&stack.item).is_some_and(|info|
        !game.cooldowns.is_on_cooldown(&horizon_client::cooldowns::group(stack, info.use_cooldown.as_ref())))
}

/// Right-click after the tool's ordinary block interaction has passed.
pub(super) fn activate(game: &mut super::Game<'_>, stack: &horizon_items::items::ItemStack, clicked: [i32;3], face: [i32;3]) -> Option<anyhow::Result<bool>> {
    activate_for(game,stack,clicked,face,false,false)
}

pub(super) fn activate_for(game: &mut super::Game<'_>, stack: &horizon_items::items::ItemStack, clicked: [i32;3], face: [i32;3], left: bool, guest_checked: bool) -> Option<anyhow::Result<bool>> {
    if !game.may_build() || !eligible(game,stack,left) || !off_cooldown(game,stack) { return None; }
    if direction(face).is_none() { return Some(Ok(true)); }
    let Some(x) = clicked[0].checked_add(face[0]) else { return Some(Ok(true)); };
    let Some(y) = clicked[1].checked_add(face[1]) else { return Some(Ok(true)); };
    let Some(z) = clicked[2].checked_add(face[2]) else { return Some(Ok(true)); };
    let at = [x,y,z];
    if let Some(conn) = game.lan_acting.filter(|_| !guest_checked) {
        let allowed = game.world.chunks.__horizon_mixins_tconstruct_transform_guest_hits().get(&conn)
            .is_some_and(|(target, blocked)| *target == clicked && !blocked.contains(&(x,z)));
        if !allowed { return Some(Ok(true)); }
    }
    let result = (|| -> anyhow::Result<bool> {
        if place(game, at, [-face[0], -face[1], -face[2]])? {
            if !game.creative {
                let (loss, shield) = super::tconstruct_source_overslime_host::damage_with_cause(game, stack, 5, Some("tconstruct:glowing"));
                if let Some(held) = game.held_mut().as_mut().filter(|held| held.item == stack.item) {
                    super::tconstruct_source_overslime::spend(held, shield);
                    super::tconstruct_tx::fixed_damage(held, loss);
                }
            }
            let sound = game.res.facts().get(game.world.block(x,y,z)).sounds.place.clone();
            game.audio.play(&sound, Some([f64::from(x)+0.5,f64::from(y)+0.5,f64::from(z)+0.5]), 1.0, 1.0);
        }
        // Source consumes the interaction even when no support was found.
        Ok(true)
    })();
    Some(result)
}
