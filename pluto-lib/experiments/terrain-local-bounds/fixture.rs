#[cfg(test)]
mod bounds_tests {
    use super::{candidate, geometry as g, original};
    use std::hint::black_box;
    use std::time::Instant;

    fn shape(mask: u8, back: bool) -> g::Cell {
        let surface = g::Surface {
            uv: [[0., 0.], [0., 1.], [1., 1.], [1., 0.]], back,
        };
        g::Cell {
            origin: [0; 3],
            heights: g::Heights {
                north_west: 1., north_east: 0.75,
                south_west: 0.5, south_east: 0.25,
            },
            bottom: 0., side_inset: 0.,
            top_diagonal: g::TopDiagonal::NorthWestSouthEast,
            top: (mask & 1 != 0).then_some(surface),
            down: (mask & 2 != 0).then_some(surface),
            sides: std::array::from_fn(|i| (mask & (4 << i) != 0).then_some(surface)),
        }
    }
    fn check(batch: &g::Batch) -> bool {
        // The precondition is independently checked on the ACTUAL built value,
        // before comparing predicates. No stand-in geometry or modeled floats.
        assert!(batch.as_slice().iter().all(|q| q.vertices.iter().all(|v| {
            v.position.iter().all(|x| x.is_finite() && *x >= 0.)
                && v.uv.iter().all(|x| x.is_finite() && (0.0..=1.0).contains(x))
        })));
        let expected = original::cell_local(batch);
        assert_eq!(candidate::cell_local(batch), expected);
        expected
    }
    fn both(c: &g::Cell) {
        for anchor in [g::VerticalAnchor::Bottom, g::VerticalAnchor::Top] {
            let batch = g::build_oriented(c, anchor).unwrap();
            let answer = check(&batch);
            let cloned = batch.clone();
            assert_eq!(batch, cloned);
            assert_eq!(check(&cloned), answer);
        }
    }
    #[test]
    fn every_face_mask_origin_corner_diagonal_and_back() {
        for mask in 0..64 {
            for corner in 0..8 {
                for back in [false, true] {
                    for diagonal in [g::TopDiagonal::NorthWestSouthEast,
                                     g::TopDiagonal::NorthEastSouthWest] {
                        let mut c = shape(mask, back);
                        c.origin = std::array::from_fn(|axis| (corner >> axis) & 1);
                        c.top_diagonal = diagonal;
                        both(&c);
                    }
                }
            }
        }
    }
    #[test]
    fn empty_geometry_is_local_at_every_section_origin() {
        let mut c = shape(0, true);
        for x in 0..16 { for y in 0..16 { for z in 0..16 {
            c.origin = [x, y, z];
            for anchor in [g::VerticalAnchor::Bottom, g::VerticalAnchor::Top] {
                let b = g::build_oriented(&c, anchor).unwrap();
                assert!(b.is_empty()); assert!(check(&b));
            }
        }}}
    }
    #[test]
    fn nonzero_origin_can_be_local_and_top_changes_the_answer() {
        // An origin != 0 shortcut would be WRONG: an isolated boundary plane
        // may still be local. Preserve these admitted degenerate cases.
        let mut c = shape(16, false); // West only.
        c.origin = [1, 0, 0];
        assert!(check(&g::build(&c).unwrap()));
        c.sides.swap(2, 3); // East is now x == 2.
        assert!(!check(&g::build(&c).unwrap()));
        c = shape(2, false); c.origin = [0, 1, 0]; // Down at y == 1.
        assert!(check(&g::build(&c).unwrap()));
        assert!(!check(&g::build_oriented(&c, g::VerticalAnchor::Top).unwrap()));
        c = shape(1, false); c.origin = [0, 1, 0];
        c.heights = g::Heights { north_west: 1., north_east: 1., south_west: 1., south_east: 1. };
        assert!(!check(&g::build(&c).unwrap()));
        assert!(check(&g::build_oriented(&c, g::VerticalAnchor::Top).unwrap()));
    }
    #[test]
    fn exact_zero_one_subnormal_and_inset_boundaries() {
        let values = [0., -0., f32::from_bits(1), f32::from_bits(0x007fffff),
                      f32::MIN_POSITIVE, 0.5, f32::from_bits(0x3f7fffff), 1.];
        for height in values {
            for inset in [0., -0., f32::from_bits(1), f32::from_bits(0x3effffff)] {
                let mut c = shape(63, true);
                c.heights = g::Heights { north_west: height, north_east: height,
                    south_west: height, south_east: height };
                c.bottom = height; c.side_inset = inset;
                for x in [0, 1, 15] { c.origin = [x; 3]; both(&c); }
            }
        }
    }
    #[test]
    fn deterministic_float_bits_and_clone_keep_constructor_invariants() {
        let mut seed = 0x1c63d14bu32;
        for i in 0..4096u32 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let unit = f32::from_bits(seed & 0x007fffff | 0x3f800000) - 1.;
            let mut c = shape((seed & 63) as u8, seed & 64 != 0);
            c.origin = [(i & 15) as u8, ((i >> 4) & 15) as u8, ((i >> 8) & 15) as u8];
            c.heights = g::Heights { north_west: unit, north_east: 1.,
                south_west: unit / 2., south_east: 0. };
            c.side_inset = unit / 2.;
            if seed & 128 != 0 { c.top_diagonal = g::TopDiagonal::NorthEastSouthWest; }
            for s in c.top.iter_mut().chain(c.down.iter_mut()).chain(c.sides.iter_mut().flatten()) {
                s.uv = [[unit, 0.], [0., unit], [1., 1.], [unit, unit]];
            }
            both(&c);
        }
    }
    #[test]
    fn invalid_values_are_rejected_before_a_batch_exists() {
        let invalid = [f32::NAN, f32::from_bits(0xffc00001), f32::INFINITY,
            f32::NEG_INFINITY, -f32::from_bits(1), f32::from_bits(0x3f800001)];
        for bad in invalid {
            for anchor in [g::VerticalAnchor::Bottom, g::VerticalAnchor::Top] {
                let mut c = shape(63, false); c.heights.north_west = bad;
                assert_eq!(g::build_oriented(&c, anchor), Err(g::Error::Height));
                c = shape(63, false); c.bottom = bad;
                assert_eq!(g::build_oriented(&c, anchor), Err(g::Error::Bottom));
                c = shape(63, false); c.side_inset = bad;
                assert_eq!(g::build_oriented(&c, anchor), Err(g::Error::Inset));
                c = shape(63, false); c.top.as_mut().unwrap().uv[0][0] = bad;
                assert_eq!(g::build_oriented(&c, anchor), Err(g::Error::Uv));
            }
        }
        let mut c = shape(63, false); c.origin = [16, 0, 0];
        assert_eq!(g::build(&c), Err(g::Error::Origin));
        c = shape(63, false); c.side_inset = 0.5;
        assert_eq!(g::build(&c), Err(g::Error::Inset));
    }
    fn timed(action: fn(&g::Batch) -> bool, batch: &g::Batch) -> u64 {
        for _ in 0..128 { black_box(action(black_box(batch))); }
        let start = Instant::now();
        for _ in 0..4096 { black_box(action(black_box(batch))); }
        u64::try_from(start.elapsed().as_nanos()).unwrap()
    }
    #[test]
    fn paired_validation_microtiming_is_not_mesher_timing() {
        let mut reject = shape(63, true); reject.origin = [0, 0, 1];
        let cases = [("front6", g::build(&shape(63, false)).unwrap()),
                     ("paired12", g::build(&shape(63, true)).unwrap()),
                     ("nonlocal", g::build(&reject).unwrap()),
                     ("empty", g::build(&shape(0, false)).unwrap())];
        let mut clock = u64::MAX;
        for _ in 0..128 {
            let now = Instant::now();
            let ns = u64::try_from(now.elapsed().as_nanos()).unwrap();
            if ns != 0 { clock = clock.min(ns); }
        }
        for (name, batch) in cases {
            check(&batch);
            let mut old = Vec::new(); let mut new = Vec::new();
            for round in 0..7 {
                if round % 2 == 0 {
                    old.push(timed(original::cell_local, &batch));
                    new.push(timed(candidate::cell_local, &batch));
                } else {
                    new.push(timed(candidate::cell_local, &batch));
                    old.push(timed(original::cell_local, &batch));
                }
            }
            let inconclusive = old.iter().chain(&new)
                .any(|&ns| ns < 1_000_000 || ns < clock.saturating_mul(100));
            println!("LOCAL_BOUNDS_MICRO case={name} iterations=4096 warmup=128 samples=7 old_ns={old:?} candidate_ns={new:?} observed_clock_ns={clock} inconclusive={inconclusive}");
        }
        // No speed assertion. Profile/compiler evidence must come from a
        // future controlled runner; no label or test pass establishes it.
    }
}
