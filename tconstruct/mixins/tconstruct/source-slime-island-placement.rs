//! Height and rotation rules from TConstruct IslandPlacement at f4e77d4.
//!
//! Callers supply the original four first-occupied surface heights and the
//! bounded random draw; this module does not install a worldgen hook.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IslandRotation { None, Clockwise90, Clockwise180, Counterclockwise90 }

pub(crate) fn corner_offsets(rotation: IslandRotation) -> (i32, i32) {
    match rotation {
        IslandRotation::None => (5, 5),
        IslandRotation::Clockwise90 => (-5, 5),
        IslandRotation::Clockwise180 => (-5, -5),
        IslandRotation::Counterclockwise90 => (5, -5),
    }
}

pub(crate) fn sky_height(occupied: [i32; 4], draw: u32, generation_depth: i32) -> Option<i32> {
    if draw >= 50 { return None; }
    let base = *occupied.iter().min()?;
    Some(base.checked_add(60)?.checked_add(draw as i32)?.min(generation_depth.checked_sub(20)?))
}

pub(crate) fn sea_height(sea_level: i32) -> Option<i32> {
    Some(sea_level.checked_sub(7)?.max(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_rotation_offsets_are_preserved() {
        assert_eq!(corner_offsets(IslandRotation::None), (5, 5));
        assert_eq!(corner_offsets(IslandRotation::Clockwise90), (-5, 5));
        assert_eq!(corner_offsets(IslandRotation::Clockwise180), (-5, -5));
        assert_eq!(corner_offsets(IslandRotation::Counterclockwise90), (5, -5));
    }

    #[test]
    fn sky_uses_lowest_first_occupied_height_and_original_cap() {
        assert_eq!(sky_height([74, 61, 65, 70], 0, 384), Some(121));
        assert_eq!(sky_height([74, 61, 65, 70], 49, 384), Some(170));
        assert_eq!(sky_height([300, 310, 330, 320], 49, 384), Some(364));
        assert_eq!(sky_height([1; 4], 50, 384), None);
    }

    #[test]
    fn sea_uses_sea_level_minus_seven_without_negative_height() {
        assert_eq!(sea_height(63), Some(56));
        assert_eq!(sea_height(6), Some(0));
        assert_eq!(sea_height(i32::MIN), None);
    }
}
