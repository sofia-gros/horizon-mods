//! Source slime-tree canopy operations from SlimeTreeFeature at f4e77d4.
//! The caller supplies the trunk-top position and performs the original
//! replaceability checks and randomized vine-state selection.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Operation {
    Leaf([i32; 3]),
    Air([i32; 3]),
    /// Positions sharing a group use the same randomized vine state.
    Vine { offset: [i32; 3], group: u8, upward: bool },
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
}
