// Client target/tick adapter, source-declared at tick_block_entity_anims HEAD.
// This is the existing non-generic block-entity tick pass; the explicit client
// gate excludes its server calls. Client-level replace/unload reset attachments
// are source-declared too; native attachment plan/native lifecycle validation is pending.
// Pinned-source timing review: native_client_tick_pre/post also run on title
// screens and while paused, so neither is an unconditional entity-tick seam.
// remote::Game::tick_remote_level gates its block-entity pass on world_runs(),
// after remote entities and before hand/particles; this includes single player's
// client. Its Behaviours::client_tick_block_entities callee is generic, so do
// not assume a direct Mixins injection there is supported. The concrete
// animation-pass HEAD is declared below that caller, pending native attachment validation.
use super::*;
use sunrise_client_render::pluto_client_block_entity_state::{ClientCandidate, client_targets};

/// The entity's module-owned client state, keyed by its native identity.
fn client_state(entity: &sunrise_client_render::chunk::block_entity::ModBlockEntity)
    -> std::sync::Arc<sunrise_client_render::pluto_client_block_entity_state::ClientState<sunrise_world::nbt::Compound>> {
    sunrise_client_render::pluto_client_block_entity_state::client_state::<sunrise_world::nbt::Compound>(entity.revision().identity())
}

/// The current world's real Mod entities include untemplated custom blocks,
/// which Renderer::block_entities() may omit. No camera/frustum/fade test.
pub(super) fn targets(game: &Game<'_>, registered: fn(&str) -> bool) -> Vec<[i32; 3]> {
    let Some(renderer) = game.renderer.as_ref() else { return Vec::new() };
    let facts = game.res.facts();
    let candidates = (&game.world.block_entities).into_iter().filter_map(|(pos, entity)| {
        if !matches!(entity, sunrise_client::containers::BlockEntity::Mod(_)) { return None; }
        let state = game.world.block(pos.0, pos.1, pos.2);
        Some(ClientCandidate { pos: [pos.0, pos.1, pos.2],
            block: facts.get(state).name.as_ref(), is_mod_entity: true })
    });
    client_targets(candidates, true, [game.world.bottom_y(), game.world.top_y()], registered,
        |pos| game.world.loaded(pos[0], pos[2]),
        |section| renderer.has_section((section[0], section[1], section[2])))
}

/// Update every surviving Mod instance, including one that just left the
/// client's resident sections. Enter/leave resets once; repeated active ticks
/// retain animation. Re-fetch actual entities before executing any callback.
pub(super) fn observe(game: &Game<'_>, registered: fn(&str) -> bool) -> Vec<[i32; 3]> {
    let targets = targets(game, registered);
    let active: std::collections::BTreeSet<_> = targets.iter().copied().collect();
    // Mutable container access wakes server tickers; touch only the injected
    // interior state through shared references, never the saved-data revision.
    for (pos, entity) in &game.world.block_entities {
        if let sunrise_client::containers::BlockEntity::Mod(entity) = entity {
            let _ = client_state(entity)
                .set_client_active(active.contains(&[pos.0, pos.1, pos.2]));
        }
    }
    targets
}

/// Copy every loaded Mod block entity's `texture` record into the retexture
/// mesh cache, so the mesher can draw a retextured block with its texture.
/// Only block entities that name a texture write; the rest clear any stale
/// entry for their position (a block placed without a texture, or one whose
/// texture was removed). Runs on the client tick pass, so the cache is fresh
/// whenever the world runs; a paused world cannot change. The mesh seam
/// re-checks the current block against the pure table on every face, so this
/// feed needs no block check of its own: the `texture` key is only ever
/// written by the retextured placement hook and loot function.
fn refresh_retextures(game: &Game<'_>) {
    for (pos, entity) in &game.world.block_entities {
        let sunrise_client::containers::BlockEntity::Mod(entity) = entity else { continue };
        let texture = entity.nbt.iter().find_map(|(key, tag)| {
            if key.to_string_lossy() != "texture" {
                return None;
            }
            match tag {
                sunrise_world::nbt::Tag::String(value) => Some(value.to_string_lossy()),
                _ => None,
            }
        });
        sunrise_client::mesher::pluto_retexture::refresh(
            [pos.0, pos.1, pos.2],
            texture.as_deref(),
        );
    }
}

/// Called only at the existing block-entity tick pass, not a frame callback.
/// An owned lease releases all entity/world borrows before native signal queries.
/// The callback only receives read-only saved data and its own transient copy.
/// No get_mut/iter_mut/take/put wakes the server's block-entity scheduler.
pub(super) fn tick(game: &mut Game<'_>) {
    use sunrise_client_lib::pluto_block_entity_access::client_tick_sink;
    use sunrise_client_render::pluto_client_block_entity_state::client_tick_enabled;
    let client = game.remote.is_some();
    let has_level = game.remote.as_ref().is_some_and(|remote| remote.has_level());
    if !client_tick_enabled(client, has_level, game.pause, game.title.is_some(), game.clock.world_runs()) { return; }
    refresh_retextures(game);
    let Some(sink) = client_tick_sink() else { return };
    for pos in observe(game, sink.has) {
        let prepared = (|| {
            let entity = game.world.block_entities.get(&(pos[0], pos[1], pos[2]))?;
            let sunrise_client::containers::BlockEntity::Mod(entity) = entity else { return None };
            let state = game.world.block(pos[0], pos[1], pos[2]);
            let block = game.res.facts().get(state).name.to_string();
            if !(sink.has)(&block) { return None; }
            let update = client_state(entity).begin().ok()?;
            Some((block, entity.nbt.clone(), update))
        })();
        let Some((block, saved, mut update)) = prepared else { continue };
        let ran = game.with_level(|_, level, ticks| super::pluto_block_level::with_client_view(level, ticks,
            |view| (sink.tick)(&block, pos, &saved, update.data_mut(), view)));
        if ran { let _ = update.commit(); }
        // A false callback or unwind drops the lease without publishing its copy.
    }
}

fn reset_world(world: &sunrise_client::world::World) {
    for (_, entity) in &world.block_entities {
        if let sunrise_client::containers::BlockEntity::Mod(entity) = entity {
            let _ = client_state(entity).reset();
        }
    }
}

/// A REAL client-level clear/switch, not enter_level/as_guest temporary server
/// execution. Parked worlds may keep the same entities until the client returns.
/// Source helpers distinguish retained same-level respawns from actual level
/// replacement/disconnect/reconfiguration. Native validation remains pending.
pub(super) fn reset_client_level(game: &Game<'_>) {
    sunrise_client::mesher::pluto_retexture::clear();
    reset_world(&game.world);
    for world in game.levels.parked_worlds() { reset_world(world); }
}
