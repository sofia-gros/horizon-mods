// The source side inventory caps its height to the parent screen.
pub(super) fn columns(count: usize) -> usize { if count > 27 { 6 } else { 3 } }
pub(super) fn rows(count: usize) -> usize { count.div_ceil(columns(count)) }
pub(super) fn visible_rows(count: usize) -> usize { rows(count).min(8) }
pub(super) fn max_offset(count: usize) -> usize { rows(count).saturating_sub(visible_rows(count)) }
pub(super) fn left(count: usize) -> f32 {
    (if columns(count) == 6 { -176.0 } else { -122.0 }) - if max_offset(count) > 0 { 10.0 } else { 0.0 }
}
pub(super) fn slot_position(count: usize, offset: usize, index: usize) -> Option<(f32, f32)> {
    let columns = columns(count);
    let first = offset.min(max_offset(count)) * columns;
    if index >= count || index < first || index >= first + visible_rows(count) * columns { return None; }
    let visible = index - first;
    Some((left(count) + 8.0 + (visible % columns) as f32 * 18.0, 8.0 + (visible / columns) as f32 * 18.0))
}
pub(super) fn drag_offset(count: usize, y: f32) -> usize {
    let max = max_offset(count);
    if max == 0 { return 0; }
    let travel = (visible_rows(count) as f32 * 18.0 - 15.0).max(1.0);
    (((y - 7.0 - 7.5) / travel).clamp(0.0, 1.0) * max as f32).round() as usize
}

/// Keep both modules as visible as possible when their combined width exceeds
/// the centered table's available margins.
pub(super) fn adjusted_origin(width: f32, base: f32, left: f32, right: f32) -> f32 {
    if base + left >= 0.0 && base + right <= width { base }
    else { ((width - (right - left)) / 2.0 - left).round() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn double_storage_keeps_slot_ids_and_scrolls_by_rows() {
        assert_eq!(columns(54), 6);
        assert_eq!(visible_rows(54), 8);
        assert_eq!(max_offset(54), 1);
        assert!(slot_position(54, 0, 48).is_none());
        assert!(slot_position(54, 1, 0).is_none());
        assert_eq!(slot_position(54, 1, 6), slot_position(54, 0, 0));
        assert!(slot_position(54, 1, 53).is_some());
        assert_eq!(drag_offset(54, 7.0), 0);
        assert_eq!(drag_offset(54, 151.0), 1);
        assert_eq!(max_offset(27), 1);
        assert_eq!(max_offset(9), 0);
        assert_eq!(max_offset(5), 0);
        let x = adjusted_origin(480.0, 152.0, left(54), 306.0);
        assert_eq!(x, 180.0);
        assert!(x + slot_position(54, 0, 0).unwrap().0 >= 0.0);
    }
}
