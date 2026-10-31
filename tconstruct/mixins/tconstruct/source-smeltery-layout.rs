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
    }
}
