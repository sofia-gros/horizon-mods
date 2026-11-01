// Unexpanded source SIDE_HIT box iterator: width/height before depth.
pub(super) fn box_area(origin:[i32;3], face:[i32;3], yaw:f32, depth:i32) -> Option<Vec<[i32;3]>> {
    let plane=super::tconstruct_source_area_shape::plane(origin,face,yaw)?;
    let mut out=Vec::new();
    for d in 0..=depth {
        let shift = |p:[i32;3]| -> Option<[i32;3]> {
            Some([p[0].checked_sub(face[0]*d)?,p[1].checked_sub(face[1]*d)?,p[2].checked_sub(face[2]*d)?])
        };
        for (i,pos) in plane.iter().enumerate() {
            if d>0 && i==4 { out.push(shift(origin)?); }
            out.push(shift(*pos)?);
        }
    }
    Some(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn scythe_has_source_side_hit_depth_for_all_faces() {
        for face in [[0,1,0],[0,-1,0],[0,0,1],[0,0,-1],[1,0,0],[-1,0,0]] {
            for yaw in [0.,90.,180.,270.] {
                let cells=box_area([4,5,6],face,yaw,2).unwrap();
                assert_eq!(cells.len(),26);
                assert_eq!(cells.iter().copied().collect::<std::collections::HashSet<_>>().len(),26);
                assert!(cells.iter().all(|p| { let dot=(p[0]-4)*face[0]+(p[1]-5)*face[1]+(p[2]-6)*face[2]; (-2..=0).contains(&dot) }));
            }
        }
    }
}
