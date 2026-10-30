// Validate a detected candidate Smeltery cuboid. Position lookup and discovery
// are caller responsibilities; a missing/unloaded position must fail closed.
// Source SmelteryMultiblock uses a floor, no frame and no ceiling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Part { Floor, Wall, Interior }

pub(super) fn validate(min: [i32; 3], max: [i32; 3], mut accepts: impl FnMut([i32; 3], Part) -> bool) -> Option<u32> {
    let dx = max[0].checked_sub(min[0])?.checked_sub(1)?;
    let dz = max[2].checked_sub(min[2])?.checked_sub(1)?;
    let height = max[1].checked_sub(min[1])?;
    if !(1..=14).contains(&dx) || !(1..=14).contains(&dz) || !(1..=63).contains(&height) { return None; }
    for x in min[0]+1..max[0] {
        for z in min[2]+1..max[2] {
            if !accepts([x, min[1], z], Part::Floor) { return None; }
            for y in min[1]+1..=max[1] {
                if !accepts([x, y, z], Part::Interior) { return None; }
            }
        }
    }
    for y in min[1]+1..=max[1] {
        for x in min[0]+1..max[0] {
            if !accepts([x, y, min[2]], Part::Wall) || !accepts([x, y, max[2]], Part::Wall) { return None; }
        }
        for z in min[2]+1..max[2] {
            if !accepts([min[0], y, z], Part::Wall) || !accepts([max[0], y, z], Part::Wall) { return None; }
        }
    }
    // FluidValues.INGOT * 12 per interior block, using 90 mB per ingot.
    u32::try_from(dx).ok()?.checked_mul(u32::try_from(dz).ok()?)?.checked_mul(u32::try_from(height).ok()?)?.checked_mul(1080)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn one_block_interior_requires_floor_four_sides_and_space_not_corners() {
        let mut visited = Vec::new();
        assert_eq!(validate([0,0,0],[2,1,2], |pos, kind| { visited.push((pos, kind)); true }), Some(1080));
        assert_eq!(visited.len(), 6);
        assert!(!visited.iter().any(|(p, _)| p[0] != 1 && p[2] != 1));
    }
    #[test]
    fn missing_floor_wall_or_interior_rejects_without_side_effects() {
        for target in [[1,0,1],[0,1,1],[1,1,1]] {
            assert_eq!(validate([0,0,0],[2,1,2], |pos, _| pos != target), None);
        }
        assert_eq!(validate([0,0,0],[17,1,2], |_, _| true), None);
    }
}
