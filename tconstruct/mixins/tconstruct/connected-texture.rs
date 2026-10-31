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

// Resolve a baked axis-aligned quad's UV axes in world space. This handles
// blockstate rotations and UV flips without relying on the original JSON after
// baking. Returns None for a degenerate or non-axis-aligned mapping.
pub(crate) fn baked_uv_directions(positions: [[f32; 3]; 4], uv: [[f32; 2]; 4]) -> Option<[Direction; 4]> {
    fn axis(positions: [[f32; 3]; 4], uv: [[f32; 2]; 4], component: usize) -> Option<Direction> {
        const EPS: f32 = 0.0001;
        for a in 0..4 {
            for b in a + 1..4 {
                let du = uv[b][component] - uv[a][component];
                if du.abs() <= EPS || (uv[b][1 - component] - uv[a][1 - component]).abs() > EPS { continue; }
                let delta = [0, 1, 2].map(|i| (positions[b][i] - positions[a][i]) / du);
                let axes: Vec<_> = (0..3).filter(|&i| delta[i].abs() > EPS).collect();
                if axes.len() != 1 { continue; }
                return Some(match (axes[0], delta[axes[0]] > 0.0) {
                    (0, true) => Direction::East, (0, false) => Direction::West,
                    (1, true) => Direction::Up, (1, false) => Direction::Down,
                    (2, true) => Direction::South, (2, false) => Direction::North,
                    _ => unreachable!(),
                });
            }
        }
        None
    }
    let right = axis(positions, uv, 0)?;
    let down = axis(positions, uv, 1)?;
    Some([down.opposite(), down, right.opposite(), right])
}

#[cfg(test)]
mod baked_tests {
    use super::{baked_uv_directions, Direction::*};
    #[test]
    fn baked_axes_follow_uv_flip_and_rotation() {
        let p = [[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]];
        let uv = [[0., 1.], [1., 1.], [1., 0.], [0., 0.]];
        assert_eq!(baked_uv_directions(p, uv), Some([Up, Down, West, East]));
        let flipped = uv.map(|[u, v]| [1. - u, v]);
        assert_eq!(baked_uv_directions(p, flipped), Some([Up, Down, East, West]));
        let rotated = uv.map(|[u, v]| [v, 1. - u]);
        assert_eq!(baked_uv_directions(p, rotated), Some([East, West, Up, Down]));
        assert_eq!(baked_uv_directions(p, [[0., 0.]; 4]), None);
    }
}

impl Direction {
    pub(crate) fn normal(self) -> [i32; 3] {
        use Direction::*;
        match self { Down => [0, -1, 0], Up => [0, 1, 0], North => [0, 0, -1], South => [0, 0, 1], West => [-1, 0, 0], East => [1, 0, 0] }
    }
}

// Colors are the source model's constant RGB values; other IDs must never
// enter the connected-glass rendering path.
pub(crate) fn glass_kind(name: &str) -> Option<(bool, u32)> {
    let name = name.strip_prefix("tconstruct:")?;
    let (name, pane) = name.strip_suffix("_pane").map_or((name, false), |name| (name, true));
    let color = match name {
        "clear_glass" | "soul_glass" | "seared_glass" | "seared_soul_glass" | "seared_tinted_glass" | "scorched_glass" | "scorched_soul_glass" | "scorched_tinted_glass" | "white_clear_stained_glass" => 0xffffff,
        _ => match name.strip_suffix("_clear_stained_glass")? {
            "black" => 0x191919, "blue" => 0x334cb2, "brown" => 0x664c33,
            "cyan" => 0x4c7f99, "gray" => 0x4c4c4c, "green" => 0x667f33,
            "light_blue" => 0x6699d8, "light_gray" => 0x999999, "lime" => 0x7fcc19,
            "magenta" => 0xb24cd8, "orange" => 0xd87f33, "pink" => 0xf27fa5,
            "purple" => 0x7f3fb2, "red" => 0x993333, "yellow" => 0xe5e533,
            _ => return None,
        },
    };
    Some((pane, color))
}

#[cfg(test)]
mod glass_tests {
    use super::glass_kind;
    #[test]
    fn recognises_only_original_glass_families() {
        assert_eq!(glass_kind("tconstruct:red_clear_stained_glass_pane"), Some((true, 0x993333)));
        assert_eq!(glass_kind("tconstruct:scorched_soul_glass"), Some((false, 0xffffff)));
        assert_eq!(glass_kind("minecraft:red_stained_glass"), None);
        assert_eq!(glass_kind("tconstruct:seared_glass_fake"), None);
    }
}

// Mantle's pane predicate also requires both panes to be either center-only
// or horizontally connected; cubes only compare block identity.
pub(crate) fn connects(same_block: bool, pane: bool, current_has_arms: bool, other_has_arms: bool) -> bool {
    same_block && (!pane || current_has_arms == other_has_arms)
}

#[cfg(test)]
mod predicate_tests {
    use super::connects;
    #[test]
    fn pane_connections_match_source_center_and_arm_rule() {
        assert!(connects(true, true, false, false));
        assert!(connects(true, true, true, true));
        assert!(!connects(true, true, false, true));
        assert!(!connects(false, true, true, true));
        assert!(connects(true, false, false, true));
        assert!(!connects(false, false, false, false));
    }
}
