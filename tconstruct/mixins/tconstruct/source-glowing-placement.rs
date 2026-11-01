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

/// Original armor walk radius: X advances inside Z, at feet Y + 0.5.
pub(super) fn walk_candidates(at: [f64;3], level: i32, expanded: i32) -> Vec<[i32;3]> {
    if level <= 0 || at.iter().any(|v| !v.is_finite()) { return Vec::new(); }
    let radius = (2i32.saturating_add(level).saturating_add(expanded)).clamp(0,16);
    let center = [at[0].floor() as i32, (at[1]+0.5).floor() as i32, at[2].floor() as i32];
    let mut positions=Vec::new();
    for dz in -radius..=radius { for dx in -radius..=radius {
        let Some(x)=center[0].checked_add(dx) else { continue; };
        let Some(z)=center[2].checked_add(dz) else { continue; };
        let distance=(f64::from(x)+0.5-at[0]).powi(2)+(f64::from(center[1])+0.5-at[1]).powi(2)+(f64::from(z)+0.5-at[2]).powi(2);
        if distance < f64::from(radius*radius) { positions.push([x,center[1],z]); }
    }}
    positions
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
    fn walk_uses_source_radius_height_and_order() {
        let positions = walk_candidates([0.5,64.0,0.5],1,0);
        assert_eq!(positions.first(),Some(&[-2,64,-2]));
        assert!(positions.contains(&[0,64,0]));
        assert!(!positions.contains(&[3,64,0]));
        assert!(walk_candidates([0.0,0.0,0.0],0,0).is_empty());
        assert!(walk_candidates([f64::NAN,0.0,0.0],1,0).is_empty());
    }
    #[test]
    fn overflow_does_not_wrap_into_a_support() {
        let result = support_direction([i32::MAX,0,0], [1,0,0], true, false, false, |at,_| at[0] == i32::MIN);
        assert_eq!(result, None);
    }
}

/// The host resolves the actor using its native spawn-protection predicate;
/// this applies that actor's resolved square to an individual candidate.
pub(super) fn outside_protection(protection: Option<([i32;3],u32)>, at: [i32;3]) -> bool {
    protection.is_none_or(|(spawn,radius)| at[0].abs_diff(spawn[0]).max(at[2].abs_diff(spawn[2])) > radius)
}

#[cfg(test)]
mod protection_tests {
    use super::outside_protection;
    #[test]
    fn source_square_respects_boundary_and_authorized_actor() {
        let protected=Some(([100,64,200],16));
        assert!(!outside_protection(protected,[116,300,200]));
        assert!(outside_protection(protected,[117,-50,200]));
        assert!(!outside_protection(protected,[84,64,184]));
        assert!(outside_protection(None,[100,64,200]));
    }
}
