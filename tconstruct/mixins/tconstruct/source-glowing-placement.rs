// Source GlowBlock.addGlow: facing points toward the supporting face.
const DIRECTIONS: [[i32; 3]; 6] = [
    [0, -1, 0], [0, 1, 0], [0, 0, -1], [0, 0, 1], [-1, 0, 0], [1, 0, 0],
];

/// Choose a source-valid support direction without modifying the world.
/// `full_face` checks the support's face looking back toward the glow.
pub(super) fn support_direction(
    pos: [i32; 3], preferred: [i32; 3], replaceable: bool, liquid: bool, existing_glow: bool,
    mut full_face: impl FnMut([i32; 3], [i32; 3]) -> bool,
) -> Option<[i32; 3]> {
    if !replaceable || liquid || existing_glow || !DIRECTIONS.contains(&preferred) { return None; }
    std::iter::once(preferred).chain(DIRECTIONS.into_iter().filter(|direction| *direction != preferred))
        .find(|direction| {
            let Some(x) = pos[0].checked_add(direction[0]) else { return false; };
            let Some(y) = pos[1].checked_add(direction[1]) else { return false; };
            let Some(z) = pos[2].checked_add(direction[2]) else { return false; };
            full_face([x, y, z], [-direction[0], -direction[1], -direction[2]])
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prefers_clicked_face_then_uses_source_direction_order() {
        let pos = [4, 8, 12];
        let mut visits = Vec::new();
        let result = support_direction(pos, [1,0,0], true, false, false, |at, face| {
            visits.push((at, face)); at == [4, 7, 12]
        });
        assert_eq!(result, Some([0,-1,0]));
        assert_eq!(visits, vec![([5,8,12],[-1,0,0]), ([4,7,12],[0,1,0])]);
        assert_eq!(support_direction(pos, [1,0,0], true, false, false, |at,_| at == [5,8,12]), Some([1,0,0]));
    }
    #[test]
    fn rejects_liquid_existing_glow_nonreplaceable_and_missing_support() {
        let pos = [0,0,0]; let down = [0,-1,0];
        for (replaceable, liquid, existing) in [(false,false,false),(true,true,false),(true,false,true)] {
            assert_eq!(support_direction(pos, down, replaceable, liquid, existing, |_,_| panic!("invalid destination")), None);
        }
        assert_eq!(support_direction(pos, down, true, false, false, |_,_| false), None);
    }
    #[test]
    fn overflow_does_not_wrap_into_a_support() {
        let result = support_direction([i32::MAX,0,0], [1,0,0], true, false, false, |at,_| at[0] == i32::MIN);
        assert_eq!(result, None);
    }
}
