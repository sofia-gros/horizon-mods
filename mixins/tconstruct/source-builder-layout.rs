// PartBuilderScreen's four-column, four-row pattern list. Coordinates are
// relative to the 176x184 screen, and scroll progress is retained as a float.
const LEFT: i32 = 51;
const TOP: i32 = 15;
const SIZE: i32 = 18;
const SHOWN: usize = 16;

pub(super) fn hidden_rows(count: usize) -> usize { count.div_ceil(4).saturating_sub(4) }

pub(super) fn offset(count: usize, progress: f32) -> usize {
    if count <= SHOWN { return 0; }
    ((progress.clamp(0.0, 1.0) * hidden_rows(count) as f32 + 0.5).floor() as usize) * 4
}

pub(super) fn update_display(count: usize, state: &mut (f32, usize, bool, Vec<String>)) {
    let hidden = hidden_rows(count);
    if hidden == 0 { state.0 = 0.0; state.1 = 0; }
    else {
        state.1 = state.1.min(count - SHOWN);
        state.0 = state.1 as f32 / 4.0 / hidden as f32;
    }
}

pub(super) fn wheel(count: usize, progress: f32, delta: f64) -> f32 {
    let hidden = hidden_rows(count);
    if hidden == 0 { return progress; }
    ((f64::from(progress) - delta / hidden as f64) as f32).clamp(0.0, 1.0)
}

pub(super) fn drag(count: usize, progress: f32, mouse_y: f32) -> f32 {
    if hidden_rows(count) == 0 { return progress; }
    ((mouse_y - TOP as f32 - 7.5) / 57.0).clamp(0.0, 1.0)
}

pub(super) fn button_at(count: usize, start: usize, x: i32, y: i32) -> Option<usize> {
    let (dx, dy) = (x - LEFT, y - TOP);
    if !(0..SIZE * 4).contains(&dx) || !(0..SIZE * 4).contains(&dy) { return None; }
    let index = start + (dy / SIZE) as usize * 4 + (dx / SIZE) as usize;
    (index < count).then_some(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_grid_scroll_and_hit_boundaries() {
        assert_eq!(hidden_rows(16), 0);
        assert_eq!(hidden_rows(17), 1);
        assert_eq!(button_at(20, 0, 51, 15), Some(0));
        assert_eq!(button_at(20, 0, 122, 86), Some(15));
        assert_eq!(button_at(20, 0, 123, 86), None);
        assert_eq!(offset(20, wheel(20, 0.0, -1.0)), 4);
        assert_eq!(button_at(20, 4, 51, 15), Some(4));
        assert_eq!(offset(20, drag(20, 0.0, 94.0)), 4);
        let mut state = (1.0, 4, false, Vec::new());
        update_display(17, &mut state);
        assert_eq!((state.0, state.1), (0.25, 1));
    }
}
