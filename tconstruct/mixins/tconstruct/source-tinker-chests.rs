// Source-specific storage rules; callers supply component-aware equality and
// the resolved parts tag, so ordinary inventories keep their own limits.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ChestKind { Part, Tinkers }

impl ChestKind {
    pub(super) fn slots(self) -> usize {
        match self { Self::Part => 256, Self::Tinkers => 64 }
    }
    pub(super) fn slot_limit(self, item_limit: u32) -> u32 {
        item_limit.min(match self { Self::Part => 8, Self::Tinkers => 16 })
    }
    /// `first_equal` must use the full item and component identity, not count.
    pub(super) fn accepts(self, slot: usize, first_equal: Option<usize>, tagged_part: bool) -> bool {
        if slot >= self.slots() { return false; }
        match self {
            Self::Tinkers => true,
            Self::Part => first_equal.map_or(tagged_part, |first| first == slot),
        }
    }
}

/// Original scaling inventory shows one spare slot after the last occupied
/// position, capped by its own capacity. It does not limit other inventories.
pub(super) fn part_visual_size(occupied: impl DoubleEndedIterator<Item = bool> + ExactSizeIterator, capacity: usize) -> usize {
    occupied.enumerate().rev().find_map(|(index, present)| present.then_some(index + 2))
        .unwrap_or(1).min(capacity)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn per_chest_limits_and_component_identity() {
        assert_eq!((ChestKind::Part.slots(), ChestKind::Tinkers.slots()), (256, 64));
        assert_eq!(ChestKind::Part.slot_limit(64), 8);
        assert_eq!(ChestKind::Tinkers.slot_limit(64), 16);
        assert_eq!(ChestKind::Part.slot_limit(1), 1);
        assert!(ChestKind::Part.accepts(7, None, true));
        assert!(!ChestKind::Part.accepts(7, None, false));
        assert!(ChestKind::Part.accepts(7, Some(7), true));
        assert!(!ChestKind::Part.accepts(8, Some(7), true));
        assert!(ChestKind::Tinkers.accepts(8, Some(7), false));
        assert!(!ChestKind::Tinkers.accepts(64, None, true));
    }
    #[test]
    fn part_inventory_grows_and_shrinks_to_one_spare_slot() {
        assert_eq!(part_visual_size([false; 256].into_iter(), 256), 1);
        let mut slots = [false; 256]; slots[13] = true;
        assert_eq!(part_visual_size(slots.into_iter(), 256), 15);
        slots[255] = true;
        assert_eq!(part_visual_size(slots.into_iter(), 256), 256);
        slots[255] = false; slots[13] = false;
        assert_eq!(part_visual_size(slots.into_iter(), 256), 1);
    }
}
