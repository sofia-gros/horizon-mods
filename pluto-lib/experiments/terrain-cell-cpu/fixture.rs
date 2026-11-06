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
// TEST-ONLY: real native CPU types with original synthetic inputs. Never shipped.
// No fallback/skipped test when resources are absent. No GPU is constructed.
#[cfg(test)]
mod tests {
    use super::super::pluto_terrain_bindings as binding;
    use super::super::pluto_terrain_cell_geometry as geometry;
    use super::super::pluto_terrain_cells as cells;
    use super::super::*;
    use crate::assets::SpritePixels;
    use crate::blocks::{Fluid, Sounds, StateFacts};
    use horizon_world::{
        block_render::{FaceVisibility, RenderDefinition, SkipRendering},
        collision::CachedCollisionRegistry,
        nbt::Tag,
    };
    use serde_json::{Value, json};
    use std::{
        collections::{BTreeMap, HashMap},
        path::{Path, PathBuf},
        sync::atomic::{AtomicUsize, Ordering},
        time::Instant,
    };

    const CELL: u32 = 7;
    const STONE: u32 = 3;
    const GROUP: &str = "pluto_fixture:geometry";
    const NAMES: [&str; 8] = [
        "minecraft:air",
        "minecraft:bedrock",
        "minecraft:dirt",
        "minecraft:stone",
        "minecraft:sand",
        "minecraft:oak_planks",
        "minecraft:bricks",
        "pluto_fixture:cell",
    ];

    fn provider(context: &cells::CellContext<'_>) -> Option<geometry::Batch> {
        assert_eq!(context.neighborhood.binding([0; 3]), Some(context.binding));
        assert!(context.neighborhood.binding([2, 0, 0]).is_none());
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
    struct Fixture {
        assets: Arc<Assets>,
        render: Arc<RenderAssets>,
        tables: Arc<MeshTables>,
        visibility: FaceVisibility,
        biomes: Biomes,
    }
    fn facts() -> BlockFacts {
        let silent: Arc<str> = "pluto_fixture:silent".into();
        let sounds = Sounds {
            volume: 1.,
            pitch: 1.,
            break_sound: silent.clone(),
            step: silent.clone(),
            place: silent.clone(),
            hit: silent.clone(),
            fall: silent,
        };
        let states = NAMES
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let full = i != 0 && i != 6 && i != CELL as usize;
                StateFacts {
                    name: (*name).into(),
                    emission: 0,
                    dampening: if full { 15 } else { 0 },
                    skylight_down: !full,
                    shape_occlusion: false,
                    can_occlude: full,
                    solid_render: full,
                    hardness: 1.,
                    requires_tool: false,
                    replaceable: !full,
                    random_ticks: false,
                    block_entity: i == CELL as usize,
                    render_shape: if i == 0 {
                        RenderShape::Invisible
                    } else {
                        RenderShape::Model
                    },
                    full_collision: full,
                    friction: 0.6,
                    explosion_resistance: 1.,
                    item: (*name).into(),
                    sounds: sounds.clone(),
                    fluid: Fluid::None,
                    fluid_level: 0,
                    fluid_falling: false,
                    waterlogged: false,
                    random_offset: false,
                    outline_offset: false,
                    legacy_solid: full,
                    sturdy_full: if full { 63 } else { 0 },
                    classes: if i == 6 {
                        vec![Arc::<str>::from("LeavesBlock")].into()
                    } else {
                        Arc::from([])
                    },
                    outline: Arc::from([]),
                    outline_edges: Arc::from([]),
                    map_color: 0,
                }
            })
            .collect();
        BlockFacts {
            states,
            air: BlockStateId(0),
            woods: HashMap::new(),
        }
    }
    fn visibility(assets: &Assets) -> FaceVisibility {
        let empty = Tag::compound([
            ("x", Tag::LongArray(vec![0])),
            ("y", Tag::LongArray(vec![0])),
            ("z", Tag::LongArray(vec![0])),
            ("occupied", Tag::ByteArray(vec![])),
            ("cube", Tag::Byte(0)),
            ("cube_axes", Tag::Byte(0)),
            ("discrete_min", Tag::IntArray(vec![0; 3])),
            ("discrete_max", Tag::IntArray(vec![0; 3])),
        ]);
        let axis = Tag::LongArray(vec![0, 1.0f64.to_bits() as i64]);
        let full = Tag::compound([
            ("x", axis.clone()),
            ("y", axis.clone()),
            ("z", axis),
            ("occupied", Tag::ByteArray(vec![1])),
            ("cube", Tag::Byte(1)),
            ("cube_axes", Tag::Byte(7)),
            ("discrete_min", Tag::IntArray(vec![0; 3])),
            ("discrete_max", Tag::IntArray(vec![1; 3])),
        ]);
        let table = Tag::compound([
            ("DataVersion", Tag::Int(horizon_world::DATA_VERSION)),
            (
                "states",
                Tag::List(
                    NAMES
                        .iter()
                        .map(|name| Tag::String((*name).into()))
                        .collect(),
                ),
            ),
            ("shapes", Tag::List(vec![empty, full])),
            ("cached_shapes", Tag::IntArray(vec![0, 1, 1, 1, 1, 1, 0, 0])),
        ]);
        let shapes = CachedCollisionRegistry::from_nbt(&table, assets.registry).unwrap();
        let definitions = NAMES
            .iter()
            .enumerate()
            .map(|(i, name)| {
                (
                    (*name).to_owned(),
                    RenderDefinition {
                        skip: SkipRendering::Never,
                        can_occlude: i != 0 && i != 6 && i != CELL as usize,
                        bars_tag: false,
                    },
                )
            })
            .collect();
        FaceVisibility::new(&shapes, &definitions).unwrap()
    }
    fn fixture(root: &Path, emission: u8) -> Fixture {
        // Actual native loaders/bakers. Not modeled owners or placeholder meshes.
        let assets = Arc::new(Assets::load(root).expect("original synthetic Assets must load"));
        assert_eq!(assets.registry.len(), NAMES.len());
        let schema: BTreeMap<String, Value> =
            serde_json::from_slice(&std::fs::read(root.join("model-states.json")).unwrap())
                .unwrap();
        let mut sprites = crate::sprites::Sprites::default();
        let catalog = crate::models::StateModels::from_facts(
            root,
            assets.registry,
            &["stone", "pluto_fixture:cell"],
            &mut sprites,
            &schema,
        )
        .unwrap();
        for name in [
            "water_still",
            "water_flow",
            "water_overlay",
            "lava_still",
            "lava_flow",
        ] {
            sprites
                .insert(root, &format!("minecraft:block/{name}"))
                .unwrap();
        }
        sprites.finish();
        let layers = u32::try_from(sprites.count()).unwrap();
        let render = Arc::new(RenderAssets {
            sprite_size: sprites.size,
            sprite_minimum_size: sprites.minimum_size,
            texture_layers: layers,
            pixels: SpritePixels::new(sprites.pixels),
            model_catalog: catalog,
            model_library: Err("original synthetic fixture uses only actual StateModels".into()),
            model_placements: Vec::new(),
            model_gallery: Vec::new(),
            animations: sprites.animations,
            sprite_layers: sprites.layers,
            emissive: Vec::new(),
            translucent_layers: sprites.translucent,
            sprite_mips: sprites.mips,
            pbr_normals: sprites.normals,
            pbr_specular: sprites.specular,
            pbr_size: sprites.pbr_size,
            connected: Default::default(),
            fusion: Default::default(),
            generation: 1,
        });
        let visibility = visibility(&assets);
        let mut facts = facts();
        facts.states[CELL as usize].emission = emission;
        let tables =
            Arc::new(MeshTables::new(&assets, &render, &facts, visibility.clone()).unwrap());
        assert!(
            tables.is_animated(render.sprite_layers["pluto_fixture:block/cell"] as u16),
            "real animated sprite must be prepared"
        );
        let biomes = Biomes::load(root).unwrap();
        assert!(!biomes.list.is_empty());
        let mut selected = Vec::new();
        render
            .model_catalog
            .models_at(BlockStateId(CELL), BlockPos::new(8, 8, 8), &mut selected)
            .unwrap();
        assert!(!selected.is_empty(), "must exercise actual baked geometry");
        assert!(selected.iter().any(|m| !m.faces.is_empty()));
        Fixture {
            assets,
            render,
            tables,
            visibility,
            biomes,
        }
    }
    fn prepare(f: &Fixture, variant: u32, stale_after_staging: bool) {
        let calls = Arc::new(AtomicUsize::new(0));
        // Controlled monotonic validity source: first check in prepare, second
        // before computation, third after computation. Not an impure provider.
        let current: binding::CurrentRevision = if stale_after_staging {
            Arc::new(move || calls.fetch_add(1, Ordering::SeqCst) < 2)
        } else {
            Arc::new(|| true)
        };
        super::super::pluto_terrain_binding_host::prepare(
            &f.assets,
            &f.render,
            &f.tables,
            Ok((
                vec![binding::Request {
                    group: GROUP.into(),
                    variant,
                    block: "pluto_fixture:cell".into(),
                    properties: BTreeMap::new(),
                    texture: "pluto_fixture:block/cell".into(),
                }],
                current,
            )),
        )
        .unwrap();
    }
    fn mesher(f: &Fixture, smooth: bool, pack: bool) -> Mesher<'_> {
        Mesher {
            nether_shading: false,
            shader_pack_lighting: pack,
            separate_ao: smooth,
            shader_pack: pack,
            assets: &f.assets,
            render: &f.render,
            tables: &f.tables,
            biomes: &f.biomes,
            visibility: f.visibility.clone(),
            zoom: BiomeZoom::new(0),
            blend_radius: 0,
            smooth_lighting: smooth,
            cutout_leaves: true,
            translucent_layers: &f.render.translucent_layers,
            leaf_culling: LeafCullingMode::Off,
        }
    }
    fn chunk(kind: &str) -> Chunk {
        let mut chunk = Chunk::with_layout(0, 0, crate::chunk::ColumnLayout::new(0, 32).unwrap());
        match kind {
            "single" => {
                chunk.set(8, 8, 8, CELL);
                chunk.set(9, 8, 8, STONE);
                chunk.set(2, 3, 4, STONE);
            }
            "leaf" => {
                for y in 4..12 {
                    for z in 4..12 {
                        for x in 4..12 {
                            chunk.set(x, y, z, 6);
                        }
                    }
                }
                chunk.set(1, 1, 1, CELL);
                chunk.set(2, 1, 1, STONE);
            }
            "empty" => {}
            "dense" => {
                for y in 0..16 {
                    for z in 0..16 {
                        for x in 0..16 {
                            chunk.set(x, y, z, CELL);
                        }
                    }
                }
            }
            "ordinary" | "mixed" => {
                for y in (0..16).step_by(4) {
                    for z in (0..16).step_by(2) {
                        for x in (0..16).step_by(2) {
                            chunk.set(x, y, z, if kind == "mixed" { CELL } else { STONE });
                        }
                    }
                }
            }
            _ => panic!("unknown fixture layout"),
        }
        let light = std::array::from_fn(|i| ((i % 16) as u8) << 4 | ((i / 16) % 16) as u8);
        Arc::make_mut(&mut chunk.sections[0]).light = crate::chunk::Light::Full(Box::new(light));
        chunk
    }
    fn region(chunk: &Chunk) -> [[Option<&Chunk>; 3]; 3] {
        let mut r = [[None; 3]; 3];
        r[1][1] = Some(chunk);
        r
    }
    fn equal(a: &SectionMesh, b: &SectionMesh) {
        assert_eq!(
            bytemuck::cast_slice::<TerrainVertex, u8>(&a.solid),
            bytemuck::cast_slice::<TerrainVertex, u8>(&b.solid)
        );
        assert_eq!(
            bytemuck::cast_slice::<TerrainVertex, u8>(&a.translucent),
            bytemuck::cast_slice::<TerrainVertex, u8>(&b.translucent)
        );
        assert_eq!(a.sides, b.sides);
        assert_eq!(a.packed, b.packed);
        assert_eq!(a.solid_tints, b.solid_tints);
        assert_eq!(a.visibility, b.visibility);
        assert_eq!(a.block_entities, b.block_entities);
        assert_eq!(a.lights, b.lights);
        assert_eq!(a.sprite_layers, b.sprite_layers);
        assert_eq!(a.bounds, b.bounds);
        assert_eq!(a.keep, b.keep);
    }
    fn scratch_equal(a: &MeshScratch, b: &MeshScratch, mode: LeafCullingMode) {
        assert_eq!(a.own.as_deref(), b.own.as_deref());
        assert_eq!(a.padded.blocks, b.padded.blocks);
        assert_eq!(a.padded.light, b.padded.light);
        assert_eq!(a.solid_rows, b.solid_rows);
        // LeafMask's private allocation/cache layout is not an API. Compare all
        // observable interior decisions rather than forging access to its fields.
        for y in 0..16 {
            for z in 0..16 {
                for x in 0..16 {
                    assert_eq!(
                        a.leaf.interior(mode, x, y, z),
                        b.leaf.interior(mode, x, y, z)
                    );
                }
            }
        }
    }
    fn claimed_half(a: &SectionMesh, b: &SectionMesh) {
        assert_eq!(a.visibility, b.visibility);
        assert_eq!(a.block_entities, b.block_entities);
        assert_eq!(a.lights, b.lights);
        let max_y = |mesh: &SectionMesh| {
            mesh.translucent
                .iter()
                .filter(|v| v.pos[0] < 8.99 && v.pos[0] >= 8.0)
                .map(|v| v.pos[1])
                .fold(f32::NEG_INFINITY, f32::max)
        };
        assert_eq!(max_y(&a), 9.0);
        assert_eq!(max_y(&b), 8.5);
        assert_eq!(
            a.translucent.len(),
            b.translucent.len(),
            "same retained native culling and pass"
        );
    }
    fn compare(f: &Fixture, chunk: &Chunk, smooth: bool, pack: bool, expect_native: bool) {
        compare_in_mode(f, chunk, smooth, pack, expect_native, LeafCullingMode::Off);
    }
    fn compare_in_mode(
        f: &Fixture,
        chunk: &Chunk,
        smooth: bool,
        pack: bool,
        expect_native: bool,
        mode: LeafCullingMode,
    ) {
        compare_with_replay(f, chunk, smooth, pack, expect_native, mode, false);
    }
    fn compare_with_replay(
        f: &Fixture,
        chunk: &Chunk,
        smooth: bool,
        pack: bool,
        expect_native: bool,
        mode: LeafCullingMode,
        resume_valid_claim: bool,
    ) {
        let region = region(chunk);
        let mut baseline = mesher(f, smooth, pack);
        let mut candidate = mesher(f, smooth, pack);
        baseline.leaf_culling = mode;
        candidate.leaf_culling = mode;
        let mut sa = MeshScratch::default();
        let mut sb = MeshScratch::default();
        let mut a = native_baseline(&mut baseline, &region, 0, SectionMesh::default(), &mut sa);
        let mut b = candidate.mesh_reusing(&region, 0, SectionMesh::default(), &mut sb);
        assert!(
            !a.solid.is_empty() || !a.translucent.is_empty(),
            "a non-skipping real mesh is required"
        );
        scratch_equal(&sa, &sb, mode);
        let sprite = f.render.sprite_layers["pluto_fixture:block/cell"] as u16;
        assert!(a.sprite_layers.contains(&sprite));
        assert!(b.sprite_layers.contains(&sprite));
        if expect_native {
            equal(&a, &b);
        } else {
            claimed_half(&a, &b);
        }
        a.sort_solid();
        b.sort_solid();
        if expect_native {
            equal(&a, &b);
        }
        if expect_native {
            // Replay after staged fallback, changing padding/light and then an
            // empty early-return section. Reuse the SAME Mesher/scratch/output.
            // Capacity/cache occupancy may differ; semantic output must not.
            for kind in ["ordinary", "empty", "single"] {
                let next = super_chunk(kind);
                let mut r = super_region(&next);
                let neighbor = super_chunk("ordinary");
                r[1][2] = Some(&neighbor);
                a = native_baseline(&mut baseline, &r, 0, a, &mut sa);
                b = candidate.mesh_reusing(&r, 0, b, &mut sb);
                let claimed_replay = resume_valid_claim && kind == "single";
                if claimed_replay {
                    claimed_half(&a, &b);
                } else {
                    equal(&a, &b);
                }
                scratch_equal(&sa, &sb, mode);
                a.sort_solid();
                b.sort_solid();
                if !claimed_replay {
                    equal(&a, &b);
                }
            }
            println!(
                "PLUTO_TERRAIN_REPLAY {}",
                json!({"baseline_capacity_bytes":a.capacity_bytes(),"adapter_capacity_bytes":b.capacity_bytes(),"native_replay_geometry_equal":!resume_valid_claim,"valid_claim_resumed":resume_valid_claim,"allocation_capacity_equality_required":false})
            );
        }
    }
    fn super_chunk(kind: &str) -> Chunk {
        chunk(kind)
    }
    fn super_region(chunk: &Chunk) -> [[Option<&Chunk>; 3]; 3] {
        region(chunk)
    }
    fn sample(f: &Fixture, chunk: &Chunk, iterations: usize, adapted: bool) -> u64 {
        let region = region(chunk);
        let mut m = mesher(f, true, false);
        let mut scratch = MeshScratch::default();
        let mut out = SectionMesh::default();
        for _ in 0..8 {
            out = if adapted {
                m.mesh_reusing(&region, 0, out, &mut scratch)
            } else {
                native_baseline(&mut m, &region, 0, out, &mut scratch)
            };
            std::hint::black_box(&out);
        }
        let start = Instant::now();
        for _ in 0..iterations {
            out = if adapted {
                m.mesh_reusing(&region, 0, out, &mut scratch)
            } else {
                native_baseline(&mut m, &region, 0, out, &mut scratch)
            };
            std::hint::black_box(&out);
        }
        u64::try_from(start.elapsed().as_nanos() / iterations as u128).unwrap()
    }
    fn timing(f: &Fixture, chunk: &Chunk, label: &str, iterations: usize, profile: &str) {
        let mut baseline = Vec::new();
        let mut adapted = Vec::new();
        for round in 0..7 {
            if round % 2 == 0 {
                baseline.push(sample(f, chunk, iterations, false));
                adapted.push(sample(f, chunk, iterations, true));
            } else {
                adapted.push(sample(f, chunk, iterations, true));
                baseline.push(sample(f, chunk, iterations, false));
            }
        }
        let mut a = baseline.clone();
        let mut b = adapted.clone();
        a.sort_unstable();
        b.sort_unstable();
        println!(
            "PLUTO_TERRAIN_CPU {}",
            json!({"case":label,"profile_label":profile,"debug_assertions":cfg!(debug_assertions),"iterations_per_sample":iterations,"warmup":8,"samples":7,"baseline_ns_per_section":baseline,"adapter_ns_per_section":adapted,"median_ratio":b[3] as f64/a[3] as f64,"scope":"CPU mesher, original synthetic assets; no GPU or production performance threshold"})
        );
    }

    #[test]
    fn real_native_mesh_differential_and_timing() {
        let root = PathBuf::from(
            std::env::var_os("PLUTO_TERRAIN_FIXTURE_RESOURCES")
                .expect("explicit generated resources are required"),
        );
        let mode = std::env::var("PLUTO_TERRAIN_FIXTURE_MODE")
            .expect("run no-provider and registered as separate processes");
        assert!(mode == "no-provider" || mode == "registered");
        let profile = std::env::var("PLUTO_TERRAIN_PROFILE_LABEL")
            .expect("record actual Cargo profile/optimization; do not infer performance");
        let iterations: usize = std::env::var("PLUTO_TERRAIN_TIMING_ITERS")
            .unwrap_or_else(|_| "64".into())
            .parse()
            .unwrap();
        assert!((1..=4096).contains(&iterations));
        if mode == "registered" {
            cells::register(GROUP, provider).unwrap();
        }
        let ordinary = root.join("ordinary");
        let ambiguous = root.join("ambiguous");
        if mode == "no-provider" {
            let f = fixture(&ordinary, 0);
            prepare(&f, 1, false);
            for smooth in [false, true] {
                for pack in [false, true] {
                    compare(&f, &chunk("single"), smooth, pack, true);
                }
            }
            timing(&f, &chunk("mixed"), "no-provider", iterations, &profile);
            assert_eq!(
                cells::register(GROUP, provider),
                Err(cells::RegisterError::Frozen)
            );
        } else {
            for (variant, stale, expected, label) in [
                (0, false, true, "no-claim"),
                (1, false, false, "claimed"),
                (2, false, true, "invalid-cell"),
                (1, true, true, "post-stale"),
            ] {
                for smooth in [false, true] {
                    for pack in [false, true] {
                        let f = fixture(&ordinary, if pack { 7 } else { 0 });
                        prepare(&f, variant, stale);
                        compare(&f, &chunk("single"), smooth, pack, expected);
                    }
                }
                println!("PLUTO_TERRAIN_CASE {label} passed 4 lighting combinations");
            }
            for mode in [LeafCullingMode::Solid, LeafCullingMode::SolidAggressive] {
                for stale in [false, true] {
                    let f = fixture(&ordinary, 0);
                    prepare(&f, if stale { 1 } else { 2 }, stale);
                    compare_in_mode(&f, &chunk("leaf"), true, false, true, mode);
                }
            }
            let f = fixture(&ordinary, 0);
            compare(&f, &chunk("single"), true, false, true); // distinct unprepared owner triple
            let f = fixture(&ambiguous, 0);
            prepare(&f, 1, false);
            compare(&f, &chunk("single"), true, false, true);
            let f = fixture(&ordinary, 0);
            prepare(&f, 1, false);
            // A budget failure is per section, not a permanent provider disable.
            // Later valid geometry must resume its claim, not equal native.
            compare_with_replay(
                &f,
                &chunk("dense"),
                true,
                false,
                true,
                LeafCullingMode::Off,
                true,
            );
            for (variant, layout, label) in [
                (0, "ordinary", "registered-ordinary"),
                (0, "mixed", "provider-no-claim"),
                (1, "mixed", "claimed-mixed"),
            ] {
                let f = fixture(&ordinary, 0);
                prepare(&f, variant, false);
                timing(&f, &chunk(layout), label, iterations, &profile);
            }
            assert_eq!(
                cells::register("pluto_fixture:late", provider),
                Err(cells::RegisterError::Frozen)
            );
        }
        println!(
            "PLUTO_TERRAIN_MODE {mode} complete; actual native CPU mesh, original synthetic resources only"
        );
    }

    // INSERT EXACT NATIVE BASELINE
}
