// Validate a detected candidate Smeltery cuboid. Position lookup and discovery
// are caller responsibilities; a missing/unloaded position must fail closed.
// Source SmelteryMultiblock uses a floor, no frame and no ceiling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Part { Floor, Wall, Interior }

/// Discover bounds from the block immediately behind a controller. The caller
/// supplies the source tag checks and treats unavailable chunks as invalid.
pub(super) fn discover(controller: [i32; 3], facing: [i32; 2], accepts: impl Fn([i32; 3], Part) -> bool) -> Option<([i32; 3], [i32; 3], u32)> {
    if !matches!(facing, [1, 0] | [-1, 0] | [0, 1] | [0, -1]) { return None; }
    let center = [controller[0].checked_sub(facing[0])?, controller[1], controller[2].checked_sub(facing[1])?];
    if !accepts(center, Part::Interior) { return None; }
    let edge = |axis: usize, step: i32| -> Option<i32> {
        let mut p = center;
        for _ in 0..15 {
            if !accepts(p, Part::Interior) { break; }
            p[axis] = p[axis].checked_add(step)?;
        }
        Some(p[axis])
    };
    let left = edge(0, -1)?; let right = edge(0, 1)?;
    let north = edge(2, -1)?; let south = edge(2, 1)?;
    let dx = right.checked_sub(left)?.checked_sub(1)?;
    let dz = south.checked_sub(north)?.checked_sub(1)?;
    if !(1..=14).contains(&dx) || !(1..=14).contains(&dz) { return None; }
    let layer = |y: i32| -> bool {
        for x in left + 1..right { for z in north + 1..south {
            if !accepts([x,y,z], Part::Interior) { return false; }
        }}
        for x in left + 1..right {
            if !accepts([x,y,north], Part::Wall) || !accepts([x,y,south], Part::Wall) { return false; }
        }
        for z in north + 1..south {
            if !accepts([left,y,z], Part::Wall) || !accepts([right,y,z], Part::Wall) { return false; }
        }
        true
    };
    if !layer(center[1]) { return None; }
    let mut low = center[1]; let mut high = center[1];
    while high.checked_sub(low)? < 62 {
        let next = low.checked_sub(1)?;
        if !layer(next) { break; }
        low = next;
    }
    while high.checked_sub(low)? < 62 {
        let next = high.checked_add(1)?;
        if !layer(next) { break; }
        high = next;
    }
    let min = [left, low.checked_sub(1)?, north]; let max = [right, high, south];
    Some((min, max, validate(min, max, accepts)?))
}

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

/// Foundries also require the vertical frame and the floor perimeter. Unlike
/// Smelteries, source capacity counts the walls and floor as well as the
/// interior: eight ingots per block in the enclosed outer volume.
pub(super) fn validate_foundry(min: [i32; 3], max: [i32; 3], mut accepts: impl FnMut([i32; 3], Part) -> bool) -> Option<u32> {
    validate(min, max, &mut accepts)?;
    for x in min[0]..=max[0] {
        for z in min[2]..=max[2] {
            if (x == min[0] || x == max[0] || z == min[2] || z == max[2])
                && !accepts([x, min[1], z], Part::Wall) { return None; }
        }
    }
    for y in min[1]+1..=max[1] {
        for x in [min[0], max[0]] {
            for z in [min[2], max[2]] {
                if !accepts([x, y, z], Part::Wall) { return None; }
            }
        }
    }
    let x = u32::try_from(max[0].checked_sub(min[0])?.checked_add(1)?).ok()?;
    let y = u32::try_from(max[1].checked_sub(min[1])?.checked_add(1)?).ok()?;
    let z = u32::try_from(max[2].checked_sub(min[2])?.checked_add(1)?).ok()?;
    x.checked_mul(y)?.checked_mul(z)?.checked_mul(720)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn foundry_requires_frame_and_counts_outer_volume() {
        assert_eq!(validate_foundry([0,0,0], [2,1,2], |_, _| true), Some(3 * 2 * 3 * 720));
        for missing in [[0,0,0], [1,0,0], [0,1,0]] {
            assert_eq!(validate_foundry([0,0,0], [2,1,2], |pos, _| pos != missing), None);
        }
    }
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
    #[test]
    fn discovers_source_one_block_structure_from_controller_direction() {
        let accepts = |p: [i32; 3], kind| match kind {
            Part::Floor => p == [1,0,1],
            Part::Interior => p == [1,1,1],
            Part::Wall => p[1] == 1 && ((p[0] == 1 && (p[2] == 0 || p[2] == 2)) || (p[2] == 1 && (p[0] == 0 || p[0] == 2))),
        };
        assert_eq!(discover([1,1,0], [0,-1], accepts), Some(([0,0,0], [2,1,2], 1080)));
        assert!(discover([1,1,0], [0,1], accepts).is_none());
    }
}
