    // Excluded extension of the frozen successful native CPU fixture.
    // Never a global/world-dependent geometry cache or production optimization.
    mod profile {
        use super::*;
        use std::hint::black_box;
        use std::sync::OnceLock;

        const GEO_ORIGINAL: &str = "pluto_profile:geometry_original";
        const GEO_LEAN: &str = "pluto_profile:geometry_build";
        const GEO_CLONE: &str = "pluto_profile:geometry_clone";
        const GEO_CHECK: &str = "pluto_profile:geometry_check";
        const MAT_ORIGINAL: &str = "pluto_profile:material_original";
        const MAT_CLONE: &str = "pluto_profile:material_clone";
        const MAT_CHECK: &str = "pluto_profile:material_check";
        static FIXED: OnceLock<geometry::Batch> = OnceLock::new();

        fn shape() -> geometry::Cell {
            let surface = Some(geometry::Surface {
                uv: [[0., 0.], [0., 1.], [1., 1.], [1., 0.]], back: false,
            });
            geometry::Cell {
                origin: [0; 3],
                heights: geometry::Heights {
                    north_west: 0.5, north_east: 0.5,
                    south_west: 0.5, south_east: 0.5,
                },
                bottom: 0., side_inset: 0.,
                top_diagonal: geometry::TopDiagonal::NorthWestSouthEast,
                top: surface, down: surface, sides: [surface; 4],
            }
        }
        fn lean(context: &cells::CellContext<'_>) -> Option<geometry::Batch> {
            if context.binding.variant == 0 {
                return None;
            }
            let surface = Some(geometry::Surface {
                uv: [[0., 0.], [0., 1.], [1., 1.], [1., 0.]],
                back: false,
            });
            Some(
                geometry::build(&geometry::Cell {
                    origin: if context.binding.variant == 2 {
                        [1, 0, 0]
                    } else {
                        [0; 3]
                    },
                    heights: geometry::Heights {
                        north_west: 0.5,
                        north_east: 0.5,
                        south_west: 0.5,
                        south_east: 0.5,
                    },
                    bottom: 0.,
                    side_inset: 0.,
                    top_diagonal: geometry::TopDiagonal::NorthWestSouthEast,
                    top: surface,
                    down: surface,
                    sides: [surface; 4],
                })
                .unwrap(),
            )
        }
        fn cloned(context: &cells::CellContext<'_>) -> Option<geometry::Batch> {
            if context.binding.variant != 1 { return None; }
            Some(FIXED.get().expect("initialized before any mesh").clone())
        }
        fn painted(context: &cells::MaterialContext<'_>) -> Option<cells::PaintedBatch> {
            if context.cell.binding.variant != 2
                || context.appearance.anchor != geometry::VerticalAnchor::Bottom {
                return None;
            }
            Some(cells::PaintedBatch::new(
                FIXED.get().expect("initialized before any mesh").clone(),
                cells::MaterialSlot::Secondary,
            ))
        }
        // Verification-only callbacks receive the ACTUAL adapter context and
        // compare direct results. These groups are never used by timed loops.
        fn checked_geometry(context: &cells::CellContext<'_>) -> Option<geometry::Batch> {
            assert_eq!(context.binding.variant, 1);
            let original = provider(context);
            assert_eq!(original, lean(context));
            assert_eq!(original, cloned(context));
            original
        }
        fn checked_material(context: &cells::MaterialContext<'_>) -> Option<cells::PaintedBatch> {
            assert_eq!(context.cell.binding.variant, 2);
            assert_eq!(context.appearance.anchor, geometry::VerticalAnchor::Bottom);
            let original = material_provider(context).unwrap();
            let other = painted(context).unwrap();
            assert_eq!(original.geometry(), other.geometry());
            for i in 0..=original.geometry().as_slice().len() {
                assert_eq!(original.slot(i), other.slot(i));
            }
            Some(original)
        }
        fn prepared(root: &Path, group: &str, material: bool, emission: u8) -> Fixture {
            let f = fixture(root, emission);
            prepare_material(&f, group, if material { 2 } else { 1 }, 0xFFFFFF,
                geometry::VerticalAnchor::Bottom,
                material.then_some("pluto_fixture:block/flow"), false);
            f
        }
        fn equivalent(a: &Fixture, b: &Fixture, smooth: bool, pack: bool) {
            // Reuse each real Mesher, output and scratch through changed light,
            // neighbors, an empty early return, and a resumed claimed section.
            let mut ma = mesher(a, smooth, pack);
            let mut mb = mesher(b, smooth, pack);
            let mut sa = MeshScratch::default();
            let mut sb = MeshScratch::default();
            let mut oa = SectionMesh::default();
            let mut ob = SectionMesh::default();
            for (step, kind) in ["single", "mixed", "ordinary", "empty", "single"].into_iter().enumerate() {
                let next = chunk(kind);
                let neighbor = chunk("ordinary");
                let mut r = region(&next);
                if step > 0 { r[1][2] = Some(&neighbor); }
                oa = ma.mesh_reusing(&r, 0, oa, &mut sa);
                ob = mb.mesh_reusing(&r, 0, ob, &mut sb);
                equal(&oa, &ob);
                scratch_equal(&sa, &sb, LeafCullingMode::Off);
                if kind == "single" {
                    assert_eq!(y_bounds(&cell_vertices(a, &oa)), (8., 8.5),
                        "equal native fallback is not an admitted claim");
                }
                if kind == "mixed" {
                    assert_eq!(cell_vertices(a, &oa).len(), 256 * 6 * 4);
                }
                oa.sort_solid(); ob.sort_solid(); equal(&oa, &ob);
            }
        }
        fn duration(f: &Fixture, blocks: &Chunk, iterations: usize) -> u64 {
            let r = region(blocks);
            let mut m = mesher(f, true, false);
            let mut scratch = MeshScratch::default();
            let mut out = SectionMesh::default();
            for _ in 0..8 {
                out = m.mesh_reusing(&r, 0, out, &mut scratch);
                black_box(&out);
            }
            let start = Instant::now();
            for _ in 0..iterations {
                out = m.mesh_reusing(&r, 0, out, &mut scratch);
                black_box(&out);
            }
            u64::try_from(start.elapsed().as_nanos()).unwrap()
        }
        fn pair(a: &Fixture, b: &Fixture, label: &str, profile: &str, clock_floor: u64) {
            equivalent(a, b, true, false); // These exact timed owners, before clocks.
            let blocks = chunk("mixed");
            let mut left = Vec::new(); let mut right = Vec::new();
            for round in 0..7 {
                if round % 2 == 0 {
                    left.push(duration(a, &blocks, 64)); right.push(duration(b, &blocks, 64));
                } else {
                    right.push(duration(b, &blocks, 64)); left.push(duration(a, &blocks, 64));
                }
            }
            let mut x = left.clone(); let mut y = right.clone();
            x.sort_unstable(); y.sort_unstable();
            let ratio = (x[3] != 0).then(|| y[3] as f64 / x[3] as f64);
            let short = left.iter().chain(&right).any(|&ns| ns < 1_000_000 || ns < clock_floor.saturating_mul(100));
            println!("PLUTO_PROFILE_PAIR {}", json!({"case":label,"profile_label":profile,
                "warmup":8,"iterations_per_sample":64,"samples":7,"left_total_ns":left,
                "right_total_ns":right,"right_over_left_median":ratio,
                "sample_order":"left-right on even rounds; right-left on odd rounds",
                "timing_inconclusive_short_samples":short,"same_output_asserted":true,
                "scope":"constant fixture callbacks, adapter and native emission remain together"}));
        }
        fn micro<T>(mut action: impl FnMut() -> T) -> u64 {
            for _ in 0..128 { black_box(action()); }
            let start = Instant::now();
            for _ in 0..4096 { black_box(action()); }
            u64::try_from(start.elapsed().as_nanos()).unwrap()
        }
        fn microkernels(profile: &str, clock_floor: u64) {
            let shape = shape();
            let batch = FIXED.get().unwrap();
            let source = [[0.25, 0.25], [0.25, 0.75], [0.75, 0.75], [0.75, 0.25]];
            let inside = [[0.3, 0.3], [0.3, 0.7], [0.7, 0.7], [0.7, 0.3]];
            let outside = [[0., 0.], [0., 1.], [1., 1.], [1., 0.]];
            assert_eq!(&geometry::build(&shape).unwrap(), batch);
            assert!(cells::cell_local(batch));
            assert!(cells::uv_subset(&source, &inside));
            assert!(!cells::uv_subset(&source, &outside));
            let mut rows: [Vec<u64>; 5] = std::array::from_fn(|_| Vec::new());
            let names = ["geometry_build", "batch_clone", "cell_local", "uv_subset_accept", "uv_subset_reject"];
            for round in 0..7 {
                for offset in 0..5 {
                    let index = (round + offset) % 5;
                    let ns = match index {
                        0 => micro(|| geometry::build(black_box(&shape)).unwrap()),
                        1 => micro(|| geometry::Batch::clone(black_box(batch))),
                        2 => micro(|| cells::cell_local(black_box(batch))),
                        3 => micro(|| cells::uv_subset(black_box(&source), black_box(&inside))),
                        4 => micro(|| cells::uv_subset(black_box(&source), black_box(&outside))),
                        _ => unreachable!(),
                    };
                    rows[index].push(ns);
                }
            }
            for (name, totals) in names.into_iter().zip(rows) {
                let short = totals.iter().any(|&ns| ns < 1_000_000 || ns < clock_floor.saturating_mul(100));
                println!("PLUTO_PROFILE_MICRO {}", json!({"case":name,"profile_label":profile,
                    "warmup":128,"iterations_per_sample":4096,"samples":7,"total_ns":totals,
                    "sample_order":"rotating five-kernel order by round",
                    "timing_inconclusive_short_samples":short,
                    "scope":"actual pure function only; not subtracted from mesher time"}));
            }
        }
        #[test]
        fn same_output_callbacks_and_microkernels() {
            let root = PathBuf::from(std::env::var_os("PLUTO_TERRAIN_FIXTURE_RESOURCES").expect("exact resources required"));
            assert_eq!(std::env::var("PLUTO_TERRAIN_FIXTURE_MODE").unwrap(), "profile");
            let profile = std::env::var("PLUTO_TERRAIN_PROFILE_LABEL").expect("external actual compiler evidence required");
            assert!(matches!(profile.as_str(), "opt0-assertions" | "opt3-assertions"));
            assert!(cfg!(debug_assertions));
            FIXED.set(geometry::build(&shape()).unwrap()).unwrap();
            assert_eq!(FIXED.get().unwrap().as_slice().len(), 6);
            cells::register_all(&[(GEO_ORIGINAL, provider), (GEO_LEAN, lean),
                (GEO_CLONE, cloned), (GEO_CHECK, checked_geometry)]).unwrap();
            cells::register_materials(&[(MAT_ORIGINAL, material_provider),
                (MAT_CLONE, painted), (MAT_CHECK, checked_material)]).unwrap();
            let ordinary = root.join("ordinary");
            for smooth in [false, true] {
                for pack in [false, true] {
                    let original = prepared(&ordinary, GEO_ORIGINAL, false, if pack { 7 } else { 0 });
                    for group in [GEO_LEAN, GEO_CLONE, GEO_CHECK] {
                        let other = prepared(&ordinary, group, false, if pack { 7 } else { 0 });
                        equivalent(&original, &other, smooth, pack);
                        println!("PLUTO_PROFILE_EQUAL {}", json!({"family":"geometry","right":group,
                            "smooth":smooth,"pack":pack,"replay_sections":5}));
                    }
                    let original = prepared(&ordinary, MAT_ORIGINAL, true, if pack { 7 } else { 0 });
                    for group in [MAT_CLONE, MAT_CHECK] {
                        let other = prepared(&ordinary, group, true, if pack { 7 } else { 0 });
                        equivalent(&original, &other, smooth, pack);
                        println!("PLUTO_PROFILE_EQUAL {}", json!({"family":"material","right":group,
                            "smooth":smooth,"pack":pack,"replay_sections":5}));
                    }
                }
            }
            // The immutable shortcut explicitly declines unsupported contexts.
            for group in [GEO_LEAN, GEO_CLONE] {
                let f = fixture(&ordinary, 0);
                prepare_material(&f, group, 0, 0xFFFFFF, geometry::VerticalAnchor::Bottom, None, false);
                compare(&f, &chunk("single"), true, false, true);
            }
            let f = fixture(&ordinary, 0);
            prepare_material(&f, MAT_CLONE, 2, 0xFFFFFF, geometry::VerticalAnchor::Top,
                Some("pluto_fixture:block/flow"), false);
            compare(&f, &chunk("single"), true, false, true);
            let mut clock_floor = u64::MAX;
            for _ in 0..128 {
                let now = Instant::now(); let ns = u64::try_from(now.elapsed().as_nanos()).unwrap();
                if ns > 0 { clock_floor = clock_floor.min(ns); }
            }
            assert_ne!(clock_floor, u64::MAX, "monotonic clock has no observed positive interval");
            println!("PLUTO_PROFILE_CLOCK {}", json!({"min_observed_empty_interval_ns":clock_floor,
                "guaranteed_clock_resolution":false,"debug_assertions":cfg!(debug_assertions)}));
            let original = prepared(&ordinary, GEO_ORIGINAL, false, 0);
            let lean = prepared(&ordinary, GEO_LEAN, false, 0);
            let fixed = prepared(&ordinary, GEO_CLONE, false, 0);
            pair(&original, &lean, "geometry_original_vs_build", &profile, clock_floor);
            pair(&lean, &fixed, "geometry_build_vs_clone", &profile, clock_floor);
            let original = prepared(&ordinary, MAT_ORIGINAL, true, 0);
            let fixed = prepared(&ordinary, MAT_CLONE, true, 0);
            pair(&original, &fixed, "material_build_vs_clone", &profile, clock_floor);
            microkernels(&profile, clock_floor);
            println!("PLUTO_PROFILE_COMPLETE same-output callbacks and actual pure kernels; no GPU or shipping performance claim");
        }
    }

