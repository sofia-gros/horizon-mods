// HeatingStructureScreen uses up to four 22-pixel columns, each chosen for
// seven source slots. The side panel is capped by the 220-pixel parent screen.
// The two bucket slots precede melting slots in the source container.
pub(super) fn columns(count: usize) -> usize { count.div_ceil(7).min(4) }
pub(super) fn rows(count: usize) -> usize {
    let cols = columns(count);
    if cols == 0 { 0 } else { count.div_ceil(cols) }
}
pub(super) fn visible_rows(count: usize) -> usize { rows(count).min(10) }
pub(super) fn max_offset(count: usize) -> usize { rows(count).saturating_sub(visible_rows(count)) }
pub(super) fn panel_width(count: usize) -> f32 {
    (columns(count) * 22 + 14 + if max_offset(count) > 0 { 10 } else { 0 }) as f32
}
/// Return the source tank layer under a relative screen y coordinate.
pub(super) fn fluid_heights(amounts: &[u32], capacity: u32) -> Vec<u32> {
    if amounts.is_empty() { return Vec::new(); }
    let total = amounts.iter().fold(0u32, |sum, amount| sum.saturating_add(*amount));
    let capacity = capacity.max(total);
    if capacity == 0 { return vec![0; amounts.len()]; }
    let mut heights: Vec<u32> = amounts.iter().map(|amount|
        ((106.0 * *amount as f64 / capacity as f64).ceil() as u32).max(3)).collect();
    let available = if total < capacity { 103 } else { 106 };
    while heights.iter().sum::<u32>() > available {
        let Some(index) = heights.iter().enumerate().max_by_key(|(index, height)| (**height, std::cmp::Reverse(*index))).map(|(index, _)| index) else { break; };
        if heights[index] == 0 { break; }
        heights[index] -= 1;
    }
    heights
}

pub(super) fn fluid_at(amounts: &[u32], capacity: u32, y: f32) -> Option<usize> {
    if !(16.0..122.0).contains(&y) { return None; }
    let from_bottom = 122.0 - y;
    let mut bottom = 0.0f32;
    for (index, height) in fluid_heights(amounts, capacity).into_iter().enumerate() {
        let height = height as f32;
        if from_bottom <= bottom + height && from_bottom > bottom { return Some(index); }
        bottom += height;
    }
    None
}
pub(super) fn drag_offset(count: usize, y: f32) -> usize {
    let max = max_offset(count);
    if max == 0 { return 0; }
    let travel = (visible_rows(count) as f32 * 18.0 - 15.0).max(1.0);
    (((y - 11.0 - 7.5) / travel).clamp(0.0, 1.0) * max as f32).round() as usize
}
/// Resolve a visible container slot to the real melting slot. No virtual slot
/// is ever a destination; callers must reject a None before mutating items.
pub(super) fn melting_index(count: usize, offset: usize, menu_slot: usize) -> Option<usize> {
    let visible = menu_slot.checked_sub(2)?;
    let cols = columns(count);
    if cols == 0 || visible >= visible_rows(count) * cols { return None; }
    let index = offset.min(max_offset(count)).checked_mul(cols)?.checked_add(visible)?;
    (index < count).then_some(index)
}
pub(super) fn position(count: usize, offset: usize, menu_slot: usize) -> Option<(f32, f32)> {
    match menu_slot {
        0 => Some((125.0, 46.0)), 1 => Some((125.0, 104.0)),
        _ => {
            let index = melting_index(count, offset, menu_slot)?;
            let cols = columns(count);
            let visible = index - offset.min(max_offset(count)) * cols;
            Some((-panel_width(count) + 18.0 + (visible % cols) as f32 * 22.0,
                  12.0 + (visible / cols) as f32 * 18.0))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_columns_and_bucket_positions() {
        assert_eq!((columns(1), columns(7), columns(8), columns(28), columns(29)), (1,1,2,4,4));
        assert_eq!(position(1, 0, 0), Some((125.0, 46.0)));
        assert_eq!(position(1, 0, 1), Some((125.0, 104.0)));
        assert_eq!(melting_index(1, 0, 2), Some(0));
        assert_eq!(melting_index(1, 0, 3), None);
    }
    #[test]
    fn final_row_maps_to_real_slots_and_rejects_padding() {
        let count = 71;
        assert_eq!((columns(count), visible_rows(count), max_offset(count)), (4,10,8));
        assert_eq!(melting_index(count, 999, 2), Some(32));
        assert_eq!(melting_index(count, 999, 40), Some(70));
        assert_eq!(melting_index(count, 999, 41), None);
        assert_eq!(position(count, 999, 2), position(count, 0, 2));
        assert_eq!(drag_offset(count, 11.0), 0);
        assert_eq!(drag_offset(count, 191.0), max_offset(count));
    }
    #[test]
    fn fluid_hit_uses_the_same_bottom_up_scale_as_rendering() {
        assert_eq!(fluid_at(&[90, 90], 180, 110.0), Some(0));
        assert_eq!(fluid_at(&[90, 90], 180, 20.0), Some(1));
        assert_eq!(fluid_at(&[90], 180, 20.0), None);
        assert_eq!(fluid_at(&[90], 0, 110.0), Some(0));
        assert_eq!(fluid_heights(&[180, 20], 12960), vec![3, 3]);
        assert_eq!(fluid_at(&[180, 20], 12960, 117.5), Some(1));
    }
}
