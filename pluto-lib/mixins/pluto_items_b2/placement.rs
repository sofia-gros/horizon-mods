// One authoritative placement using the host's existing block update path.
use horizon_core::geometry::{Direction, ShapeOp, VoxelShape};
use horizon_core::position::BlockPos;
use pluto_api::item::{BlockTarget, PlacementRequest};

fn direction(n: [i32; 3]) -> Option<Direction> {
    match n {
        [0, -1, 0] => Some(Direction::Down),
        [0, 1, 0] => Some(Direction::Up),
        [0, 0, -1] => Some(Direction::North),
        [0, 0, 1] => Some(Direction::South),
        [-1, 0, 0] => Some(Direction::West),
        [1, 0, 0] => Some(Direction::East),
        _ => None,
    }
}

/// `None` is a normal rejection. Guest placement requires a matching
/// host-captured spawn-protection decision for this exact packet.
pub(super) fn prepare(
    game: &super::Game<'_>,
    target: BlockTarget,
    request: &PlacementRequest,
) -> Option<([i32; 3], super::BlockStateId)> {
    if game.remote.is_some()
        || (game.lan_acting.is_none() && game.options.dedicated.is_some())
        || !game.may_build()
    {
        return None;
    }
    direction(target.face)?;
    let at = [
        target.position[0].checked_add(target.face[0])?,
        target.position[1].checked_add(target.face[1])?,
        target.position[2].checked_add(target.face[2])?,
    ];
    if let Some(conn) = game.lan_acting {
        let decision = game
            .world
            .chunks
            .__horizon_mixins_pluto_items_b2_placement_guest_check()
            .get(&conn)
            .copied();
        if !super::pluto_guest_policy::permits(decision, at) {
            return None;
        }
    }
    if at != request.position {
        return None;
    }
    if at[1] < game.world.shape.min_y
        || at[1] >= game.world.shape.max_y()
        || !game.world.loaded(at[0], at[2])
        || !game.border.contains(f64::from(at[0]), f64::from(at[2]))
    {
        return None;
    }
    let old = game.world.block(at[0], at[1], at[2]);
    let facts = game.res.facts().get(old);
    if !facts.replaceable
        || (!request.replace_liquids && facts.has_class("LiquidBlock"))
        || (request.reject_same_block && facts.name.as_ref() == request.block_id.as_str())
    {
        return None;
    }
    if request.supports.is_empty() || request.supports.len() > 6 {
        return None;
    }
    let registry = game.pack.assets.registry;
    let default = registry.default_state(&request.block_id)?;
    let mut candidates = Vec::with_capacity(request.supports.len());
    for candidate in &request.supports {
        let normal = candidate.direction;
        direction(normal)?;
        if candidates.iter().any(|(seen, _)| *seen == normal) {
            return None;
        }
        let mut state = registry.state(default)?.clone();
        for (key, value) in &candidate.properties {
            state.properties.insert(key.clone(), value.clone());
        }
        candidates.push((normal, registry.resolve(&state).ok()?));
    }
    let state = super::pluto_support_policy::select(candidates, |normal| {
        let Some(x) = at[0].checked_add(normal[0]) else {
            return false;
        };
        let Some(y) = at[1].checked_add(normal[1]) else {
            return false;
        };
        let Some(z) = at[2].checked_add(normal[2]) else {
            return false;
        };
        if !game.world.loaded(x, z) {
            return false;
        }
        let Some(face) = direction([-normal[0], -normal[1], -normal[2]]) else {
            return false;
        };
        let supporting = game.world.block(x, y, z);
        let Some((_, shape)) = game
            .pack
            .context
            .tables
            .visibility
            .occlusion_face(supporting, face)
        else {
            return false;
        };
        shape
            .join_is_not_empty(&VoxelShape::block(), ShapeOp::XOR)
            .is_ok_and(|difference| !difference)
    })?;
    Some((at, state))
}

/// `true` means the host accepted a raw block write. Events are processed by
/// the caller after committing the prepared item data.
pub(super) fn apply(game: &mut super::Game<'_>, at: [i32; 3], state: super::BlockStateId) -> bool {
    game.with_level(|b, level, ticks| {
        b.set_block(
            level,
            ticks,
            BlockPos::new(at[0], at[1], at[2]),
            state,
            super::level_flags::UPDATE_ALL,
        )
    })
}
