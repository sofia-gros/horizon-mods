// Mantle's cornerless_full names use face-local up, down, left, right order.
// An empty name selects the base sprite rather than a suffix sprite.
pub(crate) fn cornerless_suffix(up: bool, down: bool, left: bool, right: bool) -> String {
    let mut suffix = String::with_capacity(4);
    for (connected, letter) in [(up, 'u'), (down, 'd'), (left, 'l'), (right, 'r')] {
        if connected { suffix.push(letter); }
    }
    suffix
}

#[cfg(test)]
mod tests {
    use super::cornerless_suffix;

    #[test]
    fn names_all_face_local_connection_combinations() {
        let mut names = std::collections::HashSet::new();
        for mask in 0..16 {
            let suffix = cornerless_suffix(mask & 1 != 0, mask & 2 != 0, mask & 4 != 0, mask & 8 != 0);
            assert!(names.insert(suffix));
        }
        assert_eq!(cornerless_suffix(false, false, false, false), "");
        assert_eq!(cornerless_suffix(true, false, true, true), "ulr");
        assert_eq!(cornerless_suffix(true, true, true, true), "udlr");
        assert_eq!(names.len(), 16);
    }
}

// The source loader maps the texture's north/south/west/east directions
// through UV rotation and flips, then rotates them onto the rendered face.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Direction { Down, Up, North, South, West, East }

impl Direction {
    fn opposite(self) -> Self {
        use Direction::*;
        match self { Down => Up, Up => Down, North => South, South => North, West => East, East => West }
    }
    fn clockwise(self) -> Self {
        use Direction::*;
        match self { North => East, East => South, South => West, West => North, other => other }
    }
    fn counterclockwise(self) -> Self { self.clockwise().opposite() }
}

pub(crate) fn face_directions(face: Direction, rotation: u16, flip_u: bool, flip_v: bool) -> Option<[Direction; 4]> {
    use Direction::*;
    if !matches!(rotation, 0 | 90 | 180 | 270) { return None; }
    let transform = |mut d: Direction| {
        // Function composition in Mantle applies rotation before UV flipping.
        d = match rotation { 90 => d.clockwise(), 180 => d.opposite(), 270 => d.counterclockwise(), _ => d };
        if (flip_u && matches!(d, East | West)) || (flip_v && matches!(d, North | South)) { d = d.opposite(); }
        match face {
            Up => d,
            Down => if matches!(d, North | South) { d.opposite() } else { d },
            side => match d {
                North => Up,
                South => Down,
                East => side.counterclockwise(),
                West => side.clockwise(),
                _ => unreachable!(),
            },
        }
    };
    Some([North, South, West, East].map(transform))
}

#[cfg(test)]
mod direction_tests {
    use super::{face_directions, Direction::*};
    #[test]
    fn source_face_rotation_and_uv_transform() {
        assert_eq!(face_directions(Up, 0, false, false), Some([North, South, West, East]));
        assert_eq!(face_directions(Down, 0, false, false), Some([South, North, West, East]));
        assert_eq!(face_directions(North, 0, false, false), Some([Up, Down, East, West]));
        assert_eq!(face_directions(East, 0, false, false), Some([Up, Down, South, North]));
        assert_eq!(face_directions(Up, 90, true, false), Some([West, East, North, South]));
        assert_eq!(face_directions(Up, 45, false, false), None);
    }
}
