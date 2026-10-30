// The original menu has three vertically arranged inputs and an optional fuel
// slot from the heater immediately below it. Chest protocol padding is hidden.
pub(super) fn position(slot: usize) -> Option<(f32, f32)> {
    Some(match slot {
        0..=8 => (8.0 + slot as f32 * 18.0, 142.0),
        9..=35 => (8.0 + ((slot - 9) % 9) as f32 * 18.0, 84.0 + ((slot - 9) / 9) as f32 * 18.0),
        300..=302 => (22.0, 16.0 + (slot - 300) as f32 * 18.0),
        327 => (151.0, 32.0),
        303..=326 | 328..=353 => (-10000.0, -10000.0),
        _ => return None,
    })
}

pub(super) fn hidden(slot: usize) -> bool {
    matches!(slot, 303..=326 | 328..=353)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn melter_and_heater_slots_match_source_without_exposing_padding() {
        assert_eq!((position(300), position(301), position(302)), (Some((22.0, 16.0)), Some((22.0, 34.0)), Some((22.0, 52.0))));
        assert_eq!(position(327), Some((151.0, 32.0)));
        assert!((303..=326).chain(328..=353).all(|slot| hidden(slot) && position(slot) == Some((-10000.0, -10000.0))));
        assert!(!hidden(327));
    }
}
