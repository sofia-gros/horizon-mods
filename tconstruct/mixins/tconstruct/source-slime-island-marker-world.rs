// Connect one original tree marker to the source tree implementation.
// The owning piece supplies the already-selected tree and its shared budget.
use crate::worldgen::structure::legacy::{PieceContext, Result};
use super::{tconstruct_slime_island_markers::{TreeBudget, MAX_TREES}, tconstruct_slime_tree_world::{self, TreeConfig}};

pub(crate) fn tree_marker(
    ctx: &mut PieceContext<'_>,
    budget: &mut TreeBudget,
    selected: Option<TreeConfig<'_>>,
    pos: [i32; 3],
) -> Result<bool> {
    let Some(config) = selected else { return Ok(false); };
    // IslandPiece checks tree presence and the limit before consuming its
    // boolean draw, and spends the limit only after successful placement.
    if budget.placed() >= MAX_TREES || !ctx.random.boolean() { return Ok(false); }
    let placed = tconstruct_slime_tree_world::place(ctx, pos, config)?;
    if placed { budget.tree_marker(true, || true, || true); }
    Ok(placed)
}
