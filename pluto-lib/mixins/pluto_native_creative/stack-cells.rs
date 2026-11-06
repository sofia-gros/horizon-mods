// Count-only operations on explicitly borrowed plain inventory cells.
//
// These helpers do not dispatch native menu hooks, resolve slot permissions,
// save, synchronize, or establish actor/world/slot authority. Callers choose
// the cells, effective limit and merge equivalence. Stacks are moved or cloned
// intact; a merge retains the destination's identity under that supplied
// equivalence, which need not preserve distinct opaque wire identities.
use horizon_items::items::ItemStack;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellError {
    ZeroCount,
    ZeroLimit,
}

/// An index in one of the two caller-supplied cell groups. This is not a native
/// inventory/window index or a permission to access live storage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    First(usize),
    Second(usize),
}

fn valid(cell: &Option<ItemStack>) -> Result<(), CellError> {
    if cell.as_ref().is_some_and(|stack| stack.count == 0) {
        Err(CellError::ZeroCount)
    } else {
        Ok(())
    }
}

/// Apply a plain left/right pickup to caller-owned values. The caller has
/// already admitted both taking and placing in this cell. `same` must be a
/// read-only merge policy; it is not a snapshot comparator. Positive counts and
/// limit are required, and malformed input is rejected before any mutation.
/// Over-limit existing stacks may be taken, but cannot be topped up.
pub fn pickup(
    cell: &mut Option<ItemStack>,
    carried: &mut Option<ItemStack>,
    right: bool,
    limit: u32,
    same: impl FnOnce(&ItemStack, &ItemStack) -> bool,
) -> Result<bool, CellError> {
    if limit == 0 { return Err(CellError::ZeroLimit); }
    valid(cell)?;
    valid(carried)?;
    let Some(mut cursor) = carried.take() else {
        let Some(mut existing) = cell.take() else { return Ok(false); };
        if right && existing.count > 1 {
            let count = existing.count.div_ceil(2);
            let mut taken = existing.clone();
            taken.count = count;
            existing.count -= count;
            *cell = Some(existing);
            *carried = Some(taken);
        } else {
            *carried = Some(existing);
        }
        return Ok(true);
    };
    if let Some(existing) = cell {
        if same(existing, &cursor) {
            let wanted = if right { 1 } else { cursor.count };
            let moved = wanted.min(limit.saturating_sub(existing.count));
            existing.count += moved;
            cursor.count -= moved;
            if cursor.count > 0 { *carried = Some(cursor); }
            return Ok(moved > 0);
        }
        if cursor.count > limit {
            *carried = Some(cursor);
            return Ok(false);
        }
        *carried = cell.replace(cursor);
        return Ok(true);
    }
    let moved = if right { 1 } else { cursor.count.min(limit) };
    let mut inserted = cursor.clone();
    inserted.count = moved;
    *cell = Some(inserted);
    cursor.count -= moved;
    if cursor.count > 0 { *carried = Some(cursor); }
    Ok(true)
}

/// Remove one item or the entire stack from a plain cell. This does not spawn
/// a dropped entity or authorize a discard. Unknown data and runtime fields
/// remain on the whole moved/cloned stack. A zero-count stack is left untouched.
pub fn take(cell: &mut Option<ItemStack>, all: bool) -> Result<Option<ItemStack>, CellError> {
    valid(cell)?;
    let Some(mut stack) = cell.take() else { return Ok(None); };
    if all || stack.count == 1 { return Ok(Some(stack)); }
    stack.count -= 1;
    let mut taken = stack.clone();
    taken.count = 1;
    *cell = Some(stack);
    Ok(Some(taken))
}

/// Collect matching items into the carried stack in the supplied order,
/// visiting non-full stacks first and full/over-limit stacks second. Missing
/// indexes are ignored; duplicate entries are visited as supplied in each pass.
/// The caller chooses visibility, take permission and the read-only `same`
/// merge policy. The carried identity wins when stacks are merged.
///
/// Zero limit, zero carried count, or a zero-count selected cell is rejected
/// before any mutation or equality call. Unselected cells are not inspected.
/// This is not a native double-click implementation or a rollback boundary for
/// a panicking/side-effecting predicate.
pub fn collect_ordered(
    first: &mut [Option<ItemStack>],
    second: &mut [Option<ItemStack>],
    carried: &mut Option<ItemStack>,
    order: &[Cell],
    limit: u32,
    mut same: impl FnMut(&ItemStack, &ItemStack) -> bool,
) -> Result<bool, CellError> {
    if limit == 0 { return Err(CellError::ZeroLimit); }
    valid(carried)?;
    for entry in order {
        let cell = match *entry { Cell::First(i) => first.get(i), Cell::Second(i) => second.get(i) };
        if let Some(cell) = cell { valid(cell)?; }
    }
    let Some(cursor) = carried.as_mut() else { return Ok(false); };
    if cursor.count >= limit { return Ok(false); }
    let before = cursor.count;
    for full_pass in [false, true] {
        for entry in order {
            let cell = match *entry { Cell::First(i) => first.get_mut(i), Cell::Second(i) => second.get_mut(i) };
            let Some(cell) = cell else { continue; };
            let Some(stack) = cell.as_mut() else { continue; };
            if !same(stack, cursor) || (!full_pass && stack.count >= limit) { continue; }
            let moved = (limit - cursor.count).min(stack.count);
            cursor.count += moved;
            stack.count -= moved;
            if stack.count == 0 { *cell = None; }
            if cursor.count == limit { return Ok(cursor.count != before); }
        }
    }
    Ok(cursor.count != before)
}
