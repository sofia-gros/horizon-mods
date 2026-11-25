use crate::{geometry::*, oracle};
fn bits(q: oracle::Quad) -> [[u32; 5]; 4] {
    q.map(|(p, uv)| {
        [
            p[0].to_bits(),
            p[1].to_bits(),
            p[2].to_bits(),
            uv[0].to_bits(),
            uv[1].to_bits(),
        ]
    })
}
#[test]
fn every_selected_face_matches_exact_official_expressions_bit_for_bit() {
    let directions = [Face::North, Face::South, Face::West, Face::East];
    let mut cases = 0;
    for origin in [[0, 0, 0], [15, 15, 15], [1, 7, 12]] {
        for heights in [
            [1.0; 4],
            [8.0 / 9.0 - 0.001; 4],
            [0.25, 0.75, 0.5, 0.9],
            [0.0; 4],
        ] {
            let bottom = if heights[0] == 0.0 { 0.0 } else { 0.001 };
            for inset in [0.0, 0.001, 0.125, 0.49] {
                for rotate in [false, true] {
                    for back in [false, true] {
                        for mask in 0..64 {
                            let uv = [[0.125, 0.375], [0.25, 0.875], [0.875, 0.75], [0.625, 0.125]];
                            let top = oracle::top(origin, heights, uv, rotate);
                            let down = oracle::down(origin, bottom);
                            let sides = directions
                                .map(|face| oracle::side(origin, heights, bottom, inset, face));
                            let expected = [top, down, sides[0], sides[1], sides[2], sides[3]];
                            let surfaces = std::array::from_fn::<_, 6, _>(|i| {
                                if mask & (1 << i) != 0 {
                                    Some(Surface {
                                        uv: expected[i].map(|(_, uv)| uv),
                                        back,
                                    })
                                } else {
                                    None
                                }
                            });
                            // Input top UVs precede native top-diagonal rotation.
                            let mut top_surface = surfaces[0];
                            if let Some(ref mut s) = top_surface {
                                s.uv = uv;
                            }
                            let input = Cell {
                                origin,
                                heights: Heights {
                                    north_west: heights[0],
                                    north_east: heights[1],
                                    south_west: heights[2],
                                    south_east: heights[3],
                                },
                                bottom,
                                side_inset: inset,
                                top_diagonal: if rotate {
                                    TopDiagonal::NorthEastSouthWest
                                } else {
                                    TopDiagonal::NorthWestSouthEast
                                },
                                top: top_surface,
                                down: surfaces[1],
                                sides: [surfaces[2], surfaces[3], surfaces[4], surfaces[5]],
                            };
                            let batch = build(&input).unwrap();
                            let mut actual = batch.as_slice().iter();
                            for (i, face) in [
                                Face::Up,
                                Face::Down,
                                Face::North,
                                Face::South,
                                Face::West,
                                Face::East,
                            ]
                            .into_iter()
                            .enumerate()
                            {
                                if mask & (1 << i) == 0 {
                                    continue;
                                }
                                let quad = actual.next().unwrap();
                                assert_eq!(quad.face, face);
                                assert!(!quad.back);
                                assert_eq!(
                                    bits(quad.vertices.map(|v| (v.position, v.uv))),
                                    bits(expected[i])
                                );
                                if back {
                                    let quad = actual.next().unwrap();
                                    assert_eq!(quad.face, face);
                                    assert!(quad.back);
                                    assert_eq!(
                                        bits(quad.vertices.map(|v| (v.position, v.uv))),
                                        bits(oracle::back(expected[i]))
                                    );
                                }
                            }
                            assert!(actual.next().is_none());
                            cases += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases, 12288);
}
