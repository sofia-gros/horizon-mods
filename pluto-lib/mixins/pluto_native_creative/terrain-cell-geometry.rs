// Coordinate and winding conventions derived from official Horizon 624c4f62.
// MIT License
//
// Copyright (c) 2026 Ludovic Marie and the Horizon contributors
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
// Allocation-free explicit cell-face positions/UVs for staged terrain geometry.
// No world lookup, fluid identity, visibility, shading, material or draw policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    Up,
    Down,
    North,
    South,
    West,
    East,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopDiagonal {
    NorthWestSouthEast,
    NorthEastSouthWest,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Heights {
    pub north_west: f32,
    pub north_east: f32,
    pub south_west: f32,
    pub south_east: f32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Surface {
    /// UVs follow the front face's documented vertex order. Back faces reverse
    /// those same position/UV pairs, never independently mirror their UVs.
    pub uv: [[f32; 2]; 4],
    pub back: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell {
    /// Section-relative cell coordinates, each 0..=15.
    pub origin: [u8; 3],
    /// Final, already-adjusted heights. No epsilon or smoothing is inferred.
    pub heights: Heights,
    pub bottom: f32,
    /// Applied only to the horizontal plane of side faces, in [0, 0.5).
    pub side_inset: f32,
    pub top_diagonal: TopDiagonal,
    pub top: Option<Surface>,
    pub down: Option<Surface>,
    /// North, south, west, east. None omits the face; the caller owns why.
    pub sides: [Option<Surface>; 4],
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quad {
    pub face: Face,
    pub back: bool,
    pub vertices: [Vertex; 4],
}
const EMPTY_VERTEX: Vertex = Vertex {
    position: [0.0; 3],
    uv: [0.0; 2],
};
const EMPTY_QUAD: Quad = Quad {
    face: Face::Up,
    back: false,
    vertices: [EMPTY_VERTEX; 4],
};
/// At most six front faces and six reversed back faces. This is an unpublished
/// value, not a native mesh mutation or a retained resource/sprite capability.
#[derive(Clone, Debug, PartialEq)]
pub struct Batch {
    quads: [Quad; 12],
    len: u8,
}
impl Batch {
    pub fn as_slice(&self) -> &[Quad] {
        &self.quads[..usize::from(self.len)]
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    fn push(&mut self, face: Face, positions: [[f32; 3]; 4], surface: Surface, rotate: bool) {
        let mut vertices = std::array::from_fn(|i| Vertex {
            position: positions[i],
            uv: surface.uv[i],
        });
        if rotate {
            vertices.rotate_left(1);
        }
        self.quads[usize::from(self.len)] = Quad {
            face,
            back: false,
            vertices,
        };
        self.len += 1;
        if surface.back {
            self.quads[usize::from(self.len)] = Quad {
                face,
                back: true,
                vertices: [vertices[0], vertices[3], vertices[2], vertices[1]],
            };
            self.len += 1;
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Origin,
    Height,
    Bottom,
    Inset,
    Uv,
}
fn unit(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}
/// Build a fixed-size, fully validated batch. Front order: Up, Down, North,
/// South, West, East, with each optional back face immediately after its front.
/// Top vertices are NW, SW, SE, NE before an optional one-corner rotation.
/// Down is NW, NE, SE, SW. Sides keep the official clockwise front winding.
///
/// Every supplied input is checked before any geometry is staged. Inputs must
/// be finite; UVs and heights lie in [0,1], bottom <= every corner, and origin
/// lies in the native section. These are explicit geometry bounds, not a fluid
/// simulation law. No normals, flags, sprite IDs, light or color are fabricated.
pub fn build(cell: &Cell) -> Result<Batch, Error> {
    if cell.origin.iter().any(|&v| v > 15) {
        return Err(Error::Origin);
    }
    let Heights {
        north_west: nw,
        north_east: ne,
        south_west: sw,
        south_east: se,
    } = cell.heights;
    if [nw, ne, sw, se].iter().any(|&h| !unit(h)) {
        return Err(Error::Height);
    }
    if !unit(cell.bottom) || [nw, ne, sw, se].iter().any(|&h| cell.bottom > h) {
        return Err(Error::Bottom);
    }
    if !cell.side_inset.is_finite() || !(0.0..0.5).contains(&cell.side_inset) {
        return Err(Error::Inset);
    }
    for surface in [cell.top, cell.down]
        .into_iter()
        .chain(cell.sides)
        .flatten()
    {
        if surface.uv.into_iter().flatten().any(|value| !unit(value)) {
            return Err(Error::Uv);
        }
    }
    let [fx, fy, fz] = cell.origin.map(f32::from);
    let mut batch = Batch {
        quads: [EMPTY_QUAD; 12],
        len: 0,
    };
    if let Some(top) = cell.top {
        batch.push(
            Face::Up,
            [
                [fx, fy + nw, fz],
                [fx, fy + sw, fz + 1.0],
                [fx + 1.0, fy + se, fz + 1.0],
                [fx + 1.0, fy + ne, fz],
            ],
            top,
            cell.top_diagonal == TopDiagonal::NorthEastSouthWest,
        );
    }
    if let Some(down) = cell.down {
        let y0 = fy + cell.bottom;
        batch.push(
            Face::Down,
            [
                [fx, y0, fz],
                [fx + 1.0, y0, fz],
                [fx + 1.0, y0, fz + 1.0],
                [fx, y0, fz + 1.0],
            ],
            down,
            false,
        );
    }
    for (i, face) in [Face::North, Face::South, Face::West, Face::East]
        .into_iter()
        .enumerate()
    {
        let Some(surface) = cell.sides[i] else {
            continue;
        };
        let (h0, h1, [x0, z0], [x1, z1]) = match face {
            Face::North => (
                nw,
                ne,
                [fx, fz + cell.side_inset],
                [fx + 1.0, fz + cell.side_inset],
            ),
            Face::South => (
                se,
                sw,
                [fx + 1.0, fz + 1.0 - cell.side_inset],
                [fx, fz + 1.0 - cell.side_inset],
            ),
            Face::West => (
                sw,
                nw,
                [fx + cell.side_inset, fz + 1.0],
                [fx + cell.side_inset, fz],
            ),
            Face::East => (
                ne,
                se,
                [fx + 1.0 - cell.side_inset, fz],
                [fx + 1.0 - cell.side_inset, fz + 1.0],
            ),
            Face::Up | Face::Down => unreachable!(),
        };
        batch.push(
            face,
            [
                [x0, fy + h0, z0],
                [x1, fy + h1, z1],
                [x1, fy + cell.bottom, z1],
                [x0, fy + cell.bottom, z0],
            ],
            surface,
            false,
        );
    }
    Ok(batch)
}
/// Where the explicit extents are anchored inside their cell. This is a
/// coordinate transform, not a rule about the direction a fluid should flow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerticalAnchor {
    Bottom,
    Top,
}
/// Additive orientation API. Bottom is exactly `build`; Top reflects the
/// validated result about the cell's horizontal middle plane, swaps Up/Down,
/// and reverses vertex/UV pairs to preserve outward winding. Back-face flags
/// and all other face identities remain unchanged. No allocation or world read.
pub fn build_oriented(cell: &Cell, anchor: VerticalAnchor) -> Result<Batch, Error> {
    let mut batch = build(cell)?;
    if anchor == VerticalAnchor::Top {
        let plane_twice = 2.0 * f32::from(cell.origin[1]) + 1.0;
        for quad in &mut batch.quads[..usize::from(batch.len)] {
            for vertex in &mut quad.vertices {
                vertex.position[1] = plane_twice - vertex.position[1];
            }
            quad.face = match quad.face {
                Face::Up => Face::Down,
                Face::Down => Face::Up,
                face => face,
            };
            let v = quad.vertices;
            quad.vertices = [v[0], v[3], v[2], v[1]];
        }
    }
    Ok(batch)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn cell() -> Cell {
        let face = Some(Surface {
            uv: [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
            back: true,
        });
        Cell {
            origin: [0, 0, 0],
            heights: Heights {
                north_west: 1.0,
                north_east: 0.8,
                south_west: 0.6,
                south_east: 0.4,
            },
            bottom: 0.001,
            side_inset: 0.001,
            top_diagonal: TopDiagonal::NorthWestSouthEast,
            top: face,
            down: face,
            sides: [face; 4],
        }
    }
    #[test]
    fn fixed_capacity_order_and_paired_back_winding() {
        let batch = build(&cell()).unwrap();
        assert_eq!(batch.as_slice().len(), 12);
        for (pair, face) in batch.as_slice().chunks_exact(2).zip([
            Face::Up,
            Face::Down,
            Face::North,
            Face::South,
            Face::West,
            Face::East,
        ]) {
            assert_eq!(pair[0].face, face);
            assert!(!pair[0].back);
            assert!(pair[1].back);
            let v = pair[0].vertices;
            assert_eq!(pair[1].vertices, [v[0], v[3], v[2], v[1]]);
        }
    }
    #[test]
    fn diagonal_rotates_position_uv_pairs_only_on_top() {
        let mut c = cell();
        let before = build(&c).unwrap();
        c.top_diagonal = TopDiagonal::NorthEastSouthWest;
        let after = build(&c).unwrap();
        let mut top = before.as_slice()[0].vertices;
        top.rotate_left(1);
        assert_eq!(after.as_slice()[0].vertices, top);
        assert_eq!(&after.as_slice()[2..], &before.as_slice()[2..]);
    }
    #[test]
    fn omissions_and_section_boundary_remain_bounded() {
        let mut c = cell();
        c.origin = [15; 3];
        let batch = build(&c).unwrap();
        assert!(
            batch
                .as_slice()
                .iter()
                .flat_map(|q| q.vertices)
                .flat_map(|v| v.position)
                .all(|v| (0.0..=16.0).contains(&v))
        );
        c.top = None;
        c.down = None;
        c.sides = [None; 4];
        assert!(build(&c).unwrap().is_empty());
    }
    #[test]
    fn invalid_inputs_fail_before_returning_any_batch() {
        let original = cell();
        let mut c = original;
        c.origin[0] = 16;
        assert_eq!(build(&c), Err(Error::Origin));
        for bad in [f32::NAN, f32::INFINITY, -0.001, 1.001] {
            c = original;
            c.heights.north_west = bad;
            assert_eq!(build(&c), Err(Error::Height));
        }
        c = original;
        c.bottom = 0.5;
        assert_eq!(build(&c), Err(Error::Bottom));
        for bad in [f32::NAN, f32::INFINITY, -0.001, 0.5] {
            c = original;
            c.side_inset = bad;
            assert_eq!(build(&c), Err(Error::Inset));
        }
        for bad in [f32::NAN, f32::INFINITY, -0.001, 1.001] {
            c = original;
            c.sides[3].as_mut().unwrap().uv[3][1] = bad;
            assert_eq!(build(&c), Err(Error::Uv));
        }
    }
    #[test]
    fn bottom_orientation_is_exact_legacy_output() {
        let c = cell();
        assert_eq!(build(&c), build_oriented(&c, VerticalAnchor::Bottom));
    }
    #[test]
    fn top_reflects_positions_uv_pairs_faces_and_back_winding() {
        let c = cell();
        let low = build(&c).unwrap();
        let high = build_oriented(&c, VerticalAnchor::Top).unwrap();
        for (a, b) in low.as_slice().iter().zip(high.as_slice()) {
            assert_eq!(a.back, b.back);
            assert_eq!(
                b.face,
                match a.face {
                    Face::Up => Face::Down,
                    Face::Down => Face::Up,
                    f => f,
                }
            );
            for (out, input) in [0, 3, 2, 1].into_iter().enumerate() {
                assert_eq!(b.vertices[out].uv, a.vertices[input].uv);
                assert_eq!(b.vertices[out].position[0], a.vertices[input].position[0]);
                assert_eq!(b.vertices[out].position[2], a.vertices[input].position[2]);
                assert_eq!(
                    b.vertices[out].position[1],
                    1.0 - a.vertices[input].position[1]
                );
            }
        }
        for pair in high.as_slice().chunks_exact(2) {
            let v = pair[0].vertices;
            assert_eq!(pair[1].vertices, [v[0], v[3], v[2], v[1]]);
        }
    }
    #[test]
    fn top_orientation_keeps_section_origin_and_shared_validation() {
        let mut c = cell();
        c.origin = [15, 15, 15];
        let top = build_oriented(&c, VerticalAnchor::Top).unwrap();
        assert!(
            top.as_slice()
                .iter()
                .flat_map(|q| q.vertices)
                .all(|v| v.position.iter().all(|n| (15.0..=16.0).contains(n)))
        );
        c.heights.north_west = f32::NAN;
        assert_eq!(build_oriented(&c, VerticalAnchor::Top), Err(Error::Height));
        c = cell();
        c.bottom = 1.0;
        assert_eq!(build_oriented(&c, VerticalAnchor::Top), Err(Error::Bottom));
    }
}
