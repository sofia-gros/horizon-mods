// Single-layer geometry from Mantle 1.11.115 MantleItemLayerModel.
// Atlas frame extraction and native renderer integration remain separate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Side { Up, Down, West, East, North, South }
#[derive(Debug, PartialEq)]
pub(super) struct Quad { pub side: Side, pub positions: [[f32; 3]; 4], pub uv: [[f32; 2]; 4] }

fn side_quad(side: Side, u: usize, v: usize, size: usize, width: usize, height: usize, offset: [f32; 2]) -> Quad {
    let (mut x0, mut y0) = (u as f32 / width as f32, v as f32 / height as f32);
    let (mut x1, mut y1) = (x0, y0);
    let (mut z0, mut z1) = (7.5 / 16.0, 8.5 / 16.0);
    match side {
        Side::West | Side::East => { if side == Side::West { std::mem::swap(&mut z0, &mut z1); } y1 = (v + size) as f32 / height as f32; },
        Side::Up | Side::Down => { if side == Side::Down { std::mem::swap(&mut z0, &mut z1); } x1 = (u + size) as f32 / width as f32; },
        _ => unreachable!(),
    }
    let dx = match side { Side::West => -0.01 / width as f32, Side::East => 0.01 / width as f32, _ => 0.0 };
    let dy = match side { Side::Down => -0.01 / height as f32, Side::Up => 0.01 / height as f32, _ => 0.0 };
    let (u0, u1, v0, v1) = (x0 - dx, x1 - dx, 1.0 - y0 - dy, 1.0 - y1 - dy);
    x0 += offset[0] / 16.0; x1 += offset[0] / 16.0;
    y0 -= offset[1] / 16.0; y1 -= offset[1] / 16.0;
    Quad { side: match side { Side::Up => Side::Down, Side::Down => Side::Up, other => other },
        positions: [[x0,y0,z0],[x1,y1,z0],[x1,y1,z1],[x0,y0,z1]],
        uv: [[u0,v0],[u1,v1],[u1,v1],[u0,v0]] }
}

/// `frames` contains source alpha bytes in top-to-bottom row order, one buffer
/// per unique animation frame. This models the `pixels == null` path used by a
/// standalone material part, including Mantle's translucent run splitting.
pub(super) fn source_part_quads(width: usize, height: usize, frames: &[Vec<u8>], offset: [f32; 2]) -> Result<Vec<Quad>, &'static str> {
    let size = width.checked_mul(height).ok_or("invalid sprite dimensions")?;
    if width == 0 || height == 0 || frames.iter().any(|frame| frame.len() != size) { return Err("invalid sprite frames"); }
    let mut faces = [vec![false;size], vec![false;size], vec![false;size], vec![false;size]];
    let mut translucent = false;
    for frame in frames {
        let mut previous_vertical = vec![true;width];
        for v in 0..height {
            let mut previous_horizontal = true;
            for u in 0..width {
                let alpha = frame[(height - v - 1) * width + u];
                let transparent = alpha <= 25;
                translucent |= !transparent && alpha < 255;
                if previous_horizontal && !transparent { faces[0][v * width + u] = true; }
                if !previous_horizontal && transparent { faces[1][v * width + u - 1] = true; }
                if previous_vertical[u] && !transparent { faces[2][v * width + u] = true; }
                if !previous_vertical[u] && transparent { faces[3][(v - 1) * width + u] = true; }
                previous_horizontal = transparent;
                previous_vertical[u] = transparent;
            }
            if !previous_horizontal { faces[1][v * width + width - 1] = true; }
        }
        for (u, transparent) in previous_vertical.iter().enumerate() {
            if !transparent { faces[3][(height - 1) * width + u] = true; }
        }
    }
    let mut output = Vec::new();
    for (face, side) in [(2,Side::Up),(3,Side::Down)] {
        for v in 0..height {
            let (mut start, mut end, mut building) = (0,0,false);
            for u in 0..width {
                if faces[face][v * width + u] { end = u + 1; if !building { building = true; start = u; } }
                else if building && translucent { output.push(side_quad(side,start,v + usize::from(side == Side::Down),end-start,width,height,offset)); building = false; }
            }
            if building { output.push(side_quad(side,start,v + usize::from(side == Side::Down),end-start,width,height,offset)); }
        }
    }
    for (face, side) in [(0,Side::West),(1,Side::East)] {
        for u in 0..width {
            let (mut start, mut end, mut building) = (0,0,false);
            for v in 0..height {
                if faces[face][v * width + u] { end = v + 1; if !building { building = true; start = v; } }
                else if building && translucent { output.push(side_quad(side,u + usize::from(side == Side::East),start,end-start,width,height,offset)); building = false; }
            }
            if building { output.push(side_quad(side,u + usize::from(side == Side::East),start,end-start,width,height,offset)); }
        }
    }
    let (dx,dy) = (offset[0]/16.0,-offset[1]/16.0);
    output.push(Quad { side: Side::North, positions: [[dx,dy,7.5/16.0],[dx,1.0+dy,7.5/16.0],[1.0+dx,1.0+dy,7.5/16.0],[1.0+dx,dy,7.5/16.0]], uv: [[0.0,1.0],[0.0,0.0],[1.0,0.0],[1.0,1.0]] });
    output.push(Quad { side: Side::South, positions: [[dx,dy,8.5/16.0],[1.0+dx,dy,8.5/16.0],[1.0+dx,1.0+dy,8.5/16.0],[dx,1.0+dy,8.5/16.0]], uv: [[0.0,1.0],[1.0,1.0],[1.0,0.0],[0.0,0.0]] });
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transparent_threshold_and_offsets_follow_mantle() {
        let empty = source_part_quads(1,1,&[vec![25]],[0.0,0.0]).unwrap();
        assert_eq!(empty.len(), 2);
        let present = source_part_quads(1,1,&[vec![26]],[-2.0,1.0]).unwrap();
        assert_eq!(present.len(), 6);
        assert_eq!(present.last().unwrap().positions[0], [-0.125,-0.0625,8.5/16.0]);
        assert_eq!(present.iter().map(|q| q.side).collect::<Vec<_>>(), [Side::Down,Side::Up,Side::West,Side::East,Side::North,Side::South]);
    }
    #[test]
    fn unique_frames_union_and_invalid_frame_sizes() {
        let single = source_part_quads(2,1,&[vec![255,0]],[0.0,0.0]).unwrap();
        let multiple = source_part_quads(2,1,&[vec![255,0],vec![0,255]],[0.0,0.0]).unwrap();
        assert!(multiple.len() > single.len());
        assert!(source_part_quads(2,1,&[vec![0]],[0.0,0.0]).is_err());
    }
    #[test]
    fn translucent_gaps_split_source_side_runs_and_uvs() {
        let opaque = source_part_quads(3,1,&[vec![255,0,255]],[0.0,0.0]).unwrap();
        let translucent = source_part_quads(3,1,&[vec![128,0,255]],[0.0,0.0]).unwrap();
        assert_eq!(opaque.iter().filter(|q| q.side == Side::Down).count(), 1);
        assert_eq!(translucent.iter().filter(|q| q.side == Side::Down).count(), 2);
        let one = source_part_quads(1,1,&[vec![255]],[0.0,0.0]).unwrap();
        let west = one.iter().find(|q| q.side == Side::West).unwrap();
        assert_eq!(west.positions[0], [0.0,0.0,8.5/16.0]);
        assert!((west.uv[0][0] - 0.01).abs() < f32::EPSILON);
        assert_eq!(west.uv[0][1], 1.0);
    }
}
