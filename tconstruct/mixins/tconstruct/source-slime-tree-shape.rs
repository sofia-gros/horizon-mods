// Source slime-tree canopy operations from SlimeTreeFeature at f4e77d4.
// The caller supplies the trunk-top position and performs the original
// replaceability checks and randomized vine-state selection.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Operation {
    Leaf([i32; 3]),
    Air([i32; 3]),
    /// Positions sharing a group use the same randomized vine state.
    Vine { offset: [i32; 3], group: u8, upward: bool },
}

/// Source draws one height value and, only when allowed, a one-in-ten
/// double-height value. `double_draw` is absent when that draw is not made.
pub(crate) fn height(base: i32, random_height: u32, draw: u32, double_draw: Option<u32>) -> Option<i32> {
    if random_height == 0 || draw >= random_height || double_draw.is_some_and(|value| value >= 10) { return None; }
    let height = base.checked_add(i32::try_from(draw).ok()?)?;
    if double_draw == Some(0) { height.checked_mul(2) } else { Some(height) }
}

/// The source places only above slimy soil and inside world build bounds.
pub(crate) fn valid_site(y: i32, height: i32, min_y: i32, max_y: i32, slimy_soil: bool) -> bool {
    if !slimy_soil || height <= 0 { return false; }
    min_y.checked_add(1).zip(y.checked_add(height).and_then(|v| v.checked_add(1)))
        .is_some_and(|(minimum, top)| y >= minimum && top <= max_y)
}

/// Check every chunk intersecting the square read/write area, including
/// intermediate chunks. Coordinates are block coordinates, also for negatives.
pub(crate) fn chunks_loaded(x: i32, z: i32, radius: i32, mut loaded: impl FnMut(i32, i32) -> bool) -> bool {
    if radius < 0 { return false; }
    let Some((min_x, max_x, min_z, max_z)) = x.checked_sub(radius).zip(x.checked_add(radius))
        .zip(z.checked_sub(radius).zip(z.checked_add(radius)))
        .map(|((a, b), (c, d))| (a, b, c, d)) else { return false; };
    for cx in min_x.div_euclid(16)..=max_x.div_euclid(16) {
        for cz in min_z.div_euclid(16)..=max_z.div_euclid(16) {
            let Some((bx, bz)) = cx.checked_mul(16).zip(cz.checked_mul(16)) else { return false; };
            if !loaded(bx, bz) { return false; }
        }
    }
    true
}

/// Stage zero advances without placing a tree; stage one needs the same
/// loaded-area guard as growth, before bone meal is consumed.
pub(crate) fn bonemeal_ready(stage: Option<&str>, loaded: impl FnOnce() -> bool) -> bool {
    match stage { Some("0") => true, Some("1") => loaded(), _ => false }
}

/// Hanging propagules only age; planted stage-one propagules may place a tree.
pub(crate) fn propagule_bonemeal_ready(hanging: bool, age: Option<u8>, stage: Option<&str>, loaded: impl FnOnce() -> bool) -> bool {
    if hanging { age.is_some_and(|n| n < 4) } else { bonemeal_ready(stage, loaded) }
}

pub(crate) fn canopy(has_vines: bool) -> Vec<Operation> {
    let mut result = Vec::new();
    for layer in 0i32..4 {
        let radius = layer + 1;
        for x in -radius..=radius {
            for z in -radius..=radius {
                if x.abs() + z.abs() <= radius {
                    result.push(Operation::Leaf([x, -layer, z]));
                }
            }
        }
    }
    for [x, z] in [[4, 0], [-4, 0], [0, 4], [0, -4]] {
        result.push(Operation::Air([x, -3, z]));
    }
    if has_vines {
        for [x, z] in [[1, 1], [1, -1], [-1, 1], [-1, -1]] {
            result.push(Operation::Air([x, -3, z]));
        }
    }
    for [x, z] in [[3, 0], [-3, 0], [0, -3], [0, 3]] {
        result.push(Operation::Leaf([x, -4, z]));
    }
    if !has_vines {
        // Preserve the source's repeated [-3, 0] operation.
        for [x, z] in [[1, 1], [-3, 0], [-1, 1], [-1, -1]] {
            result.push(Operation::Leaf([x, -4, z]));
        }
    } else {
        for (group, [x, z]) in [[3, 0], [-3, 0], [0, -3], [0, 3]].into_iter().enumerate() {
            result.push(Operation::Vine { offset: [x, -5, z], group: group as u8, upward: true });
        }
        for (index, [x, z]) in [[2, 2], [2, -2], [-2, 2], [-2, -2]].into_iter().enumerate() {
            let group = 4 + index as u8;
            result.push(Operation::Vine { offset: [x, -4, z], group, upward: true });
            result.push(Operation::Vine { offset: [x, -5, z], group, upward: false });
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bonemeal_checks_chunks_only_for_tree_placement() {
        assert!(bonemeal_ready(Some("0"), || panic!("stage zero does not place a tree")));
        assert!(!bonemeal_ready(Some("1"), || false));
        assert!(bonemeal_ready(Some("1"), || true));
        assert!(!bonemeal_ready(None, || true));
    }

    #[test]
    fn propagule_bonemeal_distinguishes_hanging_age_and_planted_stage() {
        assert!(propagule_bonemeal_ready(true, Some(3), None, || panic!("hanging does not place a tree")));
        assert!(!propagule_bonemeal_ready(true, Some(4), None, || true));
        assert!(!propagule_bonemeal_ready(true, None, None, || true));
        assert!(propagule_bonemeal_ready(false, None, Some("0"), || false));
        assert!(!propagule_bonemeal_ready(false, None, Some("1"), || false));
        assert!(propagule_bonemeal_ready(false, None, Some("1"), || true));
    }

    #[test]
    fn propagule_radius_checks_same_chunks_as_original_grid() {
        use std::collections::BTreeSet;
        for x in -16i32..16 { for z in -16i32..16 {
            let mut actual = BTreeSet::new();
            assert!(chunks_loaded(x, z, 24, |a, b| { actual.insert((a.div_euclid(16), b.div_euclid(16))); true }));
            let mut before = BTreeSet::new();
            for dx in [-24, -8, 8, 24] { for dz in [-24, -8, 8, 24] { before.insert(((x+dx).div_euclid(16), (z+dz).div_euclid(16))); } }
            assert_eq!(actual, before);
        }}
    }

    #[test]
    fn all_intersecting_chunks_are_checked_including_negative_middle() {
        use std::collections::BTreeSet;
        for (x, z) in [(8, 8), (-8, -8), (0, 0)] {
            let mut seen = BTreeSet::new();
            assert!(chunks_loaded(x, z, 10, |a, b| { seen.insert((a.div_euclid(16), b.div_euclid(16))); true }));
            let expected: BTreeSet<_> = ((x - 10)..=(x + 10)).flat_map(|a| ((z - 10)..=(z + 10)).map(move |b| (a.div_euclid(16), b.div_euclid(16)))).collect();
            assert_eq!(seen, expected);
        }
        // Four corners of this 3x3 chunk area are loaded, but its center is not.
        assert!(!chunks_loaded(8, 8, 10, |x, z| !(x == 0 && z == 0)));
        assert!(!chunks_loaded(i32::MAX, 0, 10, |_, _| true));
    }

    #[test]
    fn source_diamond_layers_and_air_tips_are_ordered() {
        let ops = canopy(false);
        assert_eq!(ops.iter().filter(|op| matches!(op, Operation::Air(_))).count(), 4);
        assert_eq!(ops.iter().filter(|op| matches!(op, Operation::Leaf(_))).count(), 92);
        assert_eq!(ops.first(), Some(&Operation::Leaf([-1, 0, 0])));
        assert_eq!(ops.iter().filter(|op| **op == Operation::Leaf([-3, -4, 0])).count(), 2);
    }

    #[test]
    fn source_vine_pairs_share_state_and_preserve_up_property() {
        let ops = canopy(true);
        assert_eq!(ops.iter().filter(|op| matches!(op, Operation::Air(_))).count(), 8);
        let vines: Vec<_> = ops.iter().filter_map(|op| match op {
            Operation::Vine { group, upward, .. } => Some((*group, *upward)), _ => None,
        }).collect();
        assert_eq!(vines.len(), 12);
        for group in 4..8 {
            assert_eq!(vines.iter().filter(|(g, _)| *g == group).copied().collect::<Vec<_>>(), [(group, true), (group, false)]);
        }
    }

    #[test]
    fn source_height_and_soil_bounds_use_only_valid_draws() {
        assert_eq!(height(5, 4, 3, Some(0)), Some(16));
        assert_eq!(height(5, 4, 3, Some(1)), Some(8));
        assert_eq!(height(4, 3, 2, None), Some(6));
        assert_eq!(height(4, 3, 3, None), None);
        assert_eq!(height(5, 4, 0, Some(10)), None);
        assert!(valid_site(65, 6, -64, 320, true));
        assert!(!valid_site(65, 6, -64, 320, false));
        assert!(!valid_site(-64, 6, -64, 320, true));
        assert!(!valid_site(315, 6, -64, 320, true));
    }
}
