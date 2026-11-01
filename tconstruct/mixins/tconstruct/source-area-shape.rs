// Original side-hit box iterator for the unexpanded 3x3 hammer/shovel plane.
pub(super) fn plane(origin: [i32; 3], face: [i32; 3], yaw: f32) -> Option<Vec<[i32; 3]>> {
    let horizontal = match ((yaw / 90.0 + 0.5).floor() as i32).rem_euclid(4) {
        0 => [0, 0, 1], 1 => [-1, 0, 0], 2 => [0, 0, -1], _ => [1, 0, 0],
    };
    let (width, height) = match face {
        [0, 1, 0] | [0, -1, 0] => ([-horizontal[2], 0, horizontal[0]], horizontal),
        [0, 0, 1] | [0, 0, -1] | [1, 0, 0] | [-1, 0, 0] => ([face[2], 0, -face[0]], [0, 1, 0]),
        _ => return None,
    };
    let mut positions = Vec::with_capacity(8);
    for h in -1i32..=1 { for w in -1i32..=1 {
        if w == 0 && h == 0 { continue; }
        let mut pos = origin;
        for axis in 0..3 { pos[axis] = pos[axis].checked_add(width[axis]*w + height[axis]*h)?; }
        positions.push(pos);
    }}
    Some(positions)
}

pub(super) fn compatible_hardness(center: f32, candidate: f32) -> bool {
    candidate >= 0.0 && if center == 0.0 { candidate == 0.0 } else { candidate / center <= 3.0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn all_faces_have_eight_positions_on_hit_plane() {
        for face in [[0,1,0],[0,-1,0],[0,0,1],[0,0,-1],[1,0,0],[-1,0,0]] {
            for yaw in [0.0,90.0,180.0,270.0] {
                let p = plane([10,20,30],face,yaw).unwrap();
                assert_eq!(p.len(),8); assert!(!p.contains(&[10,20,30]));
                assert!(p.iter().all(|p| (p[0]-10)*face[0]+(p[1]-20)*face[1]+(p[2]-30)*face[2]==0));
                assert_eq!(p.iter().copied().collect::<std::collections::HashSet<_>>().len(),8);
            }
        }
    }
    #[test] fn source_traversal_and_hardness() {
        assert_eq!(plane([0,0,0],[0,0,1],0.0).unwrap()[0],[-1,-1,0]);
        assert!(compatible_hardness(2.0,6.0)); assert!(!compatible_hardness(2.0,6.01));
        assert!(compatible_hardness(0.0,0.0)); assert!(!compatible_hardness(0.0,0.1));
        assert!(!compatible_hardness(2.0,-1.0)); assert!(plane([0,0,0],[0,0,0],0.0).is_none());
    }
}
