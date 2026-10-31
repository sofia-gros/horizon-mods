use super::Game;

pub(super) fn is_smeltery(game: &Game<'_>) -> bool {
    if game.remote.is_some() {
        return game.remote_menu_title().map(|title| title.plain()).as_deref().is_some_and(|title| matches!(title, "TConstruct Smeltery" | "TConstruct Foundry"));
    }
    game.container.is_some_and(|pos| matches!(&*game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name, "tconstruct:smeltery_controller" | "tconstruct:foundry_controller"))
}

pub(super) fn is_foundry(game: &Game<'_>) -> bool {
    if game.remote.is_some() { return game.remote_menu_title().map(|title| title.plain()).as_deref() == Some("TConstruct Foundry"); }
    game.container.is_some_and(|pos| &*game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name == "tconstruct:foundry_controller")
}

pub(super) fn position(game: &Game<'_>) -> Option<(i32, i32, i32)> {
    if game.remote.is_some() { super::remote::tconstruct_source_machine_remote::position(game) } else { game.container }
}

pub(super) fn page(game: &Game<'_>) -> (usize, usize) {
    let Some(pos) = position(game) else { return (0, 0); };
    if game.remote.is_some() {
        game.world.chunks.__horizon_mixins_tconstruct_smeltery_ui_pages().get(&pos).copied().unwrap_or_default()
    } else {
        let count = super::tconstruct_source_melter_store::read(&game.world, pos, "tconstruct:smeltery")
            .and_then(|tag| super::tconstruct_source_smeltery_state::Smeltery::from_nbt(&tag)).map_or(0, |state| state.slots.len());
        let offset = game.world.chunks.__horizon_mixins_tconstruct_smeltery_ui_pages().get(&pos).map_or(0, |page| page.1);
        (count, offset)
    }
}

pub(super) fn set_offset(game: &mut Game<'_>, pos: (i32, i32, i32), count: usize, offset: usize) {
    let next = offset.min(super::tconstruct_source_smeltery_layout::max_offset(count));
    if game.remote.is_some() {
        if let Some((window, _)) = game.remote.as_ref().and_then(|remote| remote.menu.as_ref()) {
            game.send(horizon_protocol::packets::play::Serverbound::ContainerButtonClick(horizon_protocol::packets::play::ContainerButtonClick { container: *window, button: 33000 + next as i32 }));
        }
    } else {
        let Some(state) = super::tconstruct_source_melter_store::read(&game.world, pos, "tconstruct:smeltery")
            .and_then(|tag| super::tconstruct_source_smeltery_state::Smeltery::from_nbt(&tag)) else { return; };
        if let Some(horizon_client::containers::BlockEntity::Chest(slots)) = game.world.block_entities.get_mut(&pos) {
            let containers = game.world.chunks.__horizon_mixins_tconstruct_smeltery_local_containers().get(&pos)
                .map(|(slots, _)| slots.clone()).unwrap_or([None, None]);
            *slots = super::tconstruct_source_smeltery_page::project_with_containers(&state.slots, next, &containers);
            game.world.chunks.__horizon_mixins_tconstruct_smeltery_ui_pages_mut().insert(pos, (count, next));
        }
    }
}

/// Return only the currently visible heating slots. Remote metadata carries a
/// bounded progress page; item stacks use the existing container sync.
pub(super) fn visible_heat(game: &Game<'_>) -> Vec<(horizon_client::items::ItemStack, i32, f32, f32)> {
    use horizon_world::nbt::Tag;
    let Some(pos) = position(game) else { return Vec::new(); };
    let Some(tag) = super::tconstruct_source_melter_store::read(&game.world, pos, "tconstruct:smeltery") else { return Vec::new(); };
    let (count, offset) = page(game);
    let total = super::tconstruct_source_smeltery_layout::columns(count) * super::tconstruct_source_smeltery_layout::visible_rows(count);
    let full = super::tconstruct_source_smeltery_state::Smeltery::from_nbt(&tag);
    let remote_page = match (tag.get("progress_start"), tag.get("progress_page")) {
        (Some(Tag::Int(start)), Some(Tag::IntArray(values))) => usize::try_from(*start).ok().map(|start| (start, values)),
        _ => None,
    };
    let mut result = Vec::new();
    for visible in 0..total {
        let menu_slot = visible + 2;
        let Some(index) = super::tconstruct_source_smeltery_layout::melting_index(count, offset, menu_slot) else { continue; };
        let Some((x, y)) = super::tconstruct_source_smeltery_layout::position(count, offset, menu_slot) else { continue; };
        let pair = if let Some(state) = &full {
            state.slots.get(index).and_then(Option::as_ref).zip(state.progress.get(index)).map(|(stack, progress)| (stack.clone(), *progress))
        } else {
            let Some((start, values)) = remote_page else { continue; };
            let Some(progress) = index.checked_sub(start).and_then(|index| values.get(index)) else { continue; };
            game.container_kind().and_then(|entity| entity.slots().get(menu_slot)).and_then(Option::as_ref).map(|stack| (stack.clone(), *progress))
        };
        if let Some((stack, progress)) = pair { result.push((stack, progress, x, y)); }
    }
    result
}

pub(super) fn temperature(game: &Game<'_>) -> u32 {
    use horizon_world::nbt::Tag;
    position(game).and_then(|pos| super::tconstruct_source_melter_store::read(&game.world, pos, "tconstruct:smeltery"))
        .and_then(|tag| match tag.get("temperature") { Some(Tag::Long(n)) => u32::try_from(*n).ok(), _ => None }).unwrap_or(0)
}
