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
