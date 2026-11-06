// Synthetic minimal resources through real official loaders. This fixture does
// not reproduce production gameplay assets, native water policy, or game actors.
use super::query::View;
use horizon_client::{chunk::{Chunk, LevelShape}, world::World};
use horizon_world::{chunk::BlockState, fluid::FluidKind, level::Behaviours,
    nbt::{self, NamedTag, Tag}, registry::{BlockRegistry, BlockStateId}};
use pluto_api::fluid::{self, contact::{Bounds, Limits, PointContact, QueryError}, Cell, Definition, Sample};
use serde_json::json;
use std::{collections::BTreeMap, sync::{Arc, OnceLock}};

struct Fixture { registry: BlockRegistry, behaviours: Behaviours }
fn definition(name: &str) -> Definition {
    Definition { id: format!("pluto_view_fixture:{name}"), block: format!("pluto_view_fixture:{name}_block"),
        bucket: format!("pluto_view_fixture:{name}_bucket"), texture: format!("pluto_view_fixture:{name}_texture"),
        tick_delay: 5, drop_off: 1, light: 0 }
}
fn fixture() -> &'static Fixture {
    static FIXTURE: OnceLock<Fixture> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let mut names: Vec<String> = serde_json::from_str(include_str!("required-blocks.json")).unwrap();
        assert_eq!(names.first().map(String::as_str), Some("minecraft:air"));
        for name in ["first", "second"] { let d = definition(name); fluid::register(d.clone()).unwrap(); names.push(d.block); }
        names.push("pluto_view_fixture:unknown_block".into());
        let mut states = Vec::new(); let mut defaults = BTreeMap::new(); let mut rows = Vec::new();
        for name in &names {
            defaults.insert(name.as_str().into(), Tag::Int(states.len() as i32));
            let levels = if matches!(name.as_str(), "minecraft:water" | "minecraft:lava") { 16 }
                         else if name.starts_with("pluto_view_fixture:") && !name.ends_with("unknown_block") { 9 } else { 1 };
            for level in 0..levels {
                let mut state = BlockState { name: name.clone(), properties: Default::default() };
                if levels > 1 { state.properties.insert("level".into(), level.to_string()); }
                states.push(state.to_nbt());
                let air = matches!(name.as_str(), "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air");
                rows.push(json!([0, if air {2} else {0}, 0, 0, false, 0, 0, null, "HARP"]));
            }
        }
        let registry = BlockRegistry::from_nbt(&Tag::compound([
            ("DataVersion", Tag::Int(horizon_world::DATA_VERSION)),
            ("states", Tag::List(states.clone())), ("defaults", Tag::Compound(defaults)),
        ])).unwrap();
        // Same empty-grid schema as the official collision_data fixture, with
        // identities/counts validated by BlockTableFile and native collision loader.
        let empty = Tag::compound([
            ("x", Tag::LongArray(vec![0])), ("y", Tag::LongArray(vec![0])), ("z", Tag::LongArray(vec![0])),
            ("occupied", Tag::ByteArray(vec![])), ("cube", Tag::Byte(0)), ("cube_axes", Tag::Byte(0)),
            ("discrete_min", Tag::IntArray(vec![0; 3])), ("discrete_max", Tag::IntArray(vec![0; 3])),
        ]);
        let collision = Tag::compound([
            ("DataVersion", Tag::Int(horizon_world::DATA_VERSION)), ("states", Tag::List(states)),
            ("shapes", Tag::List(vec![empty])), ("cached_shapes", Tag::IntArray(vec![0; registry.len()])),
        ]);
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("minimal-world-resources-{}", std::process::id()));
        std::fs::create_dir(&root).expect("fresh fixture resource directory; never reuse production assets");
        std::fs::create_dir_all(root.join("data/minecraft/tags/fluid")).unwrap();
        std::fs::write(root.join("block-tags.json"), b"{}").unwrap();
        let bytes = nbt::write_named(&NamedTag { name: "".into(), tag: collision }, nbt::Limits::tables()).unwrap();
        std::fs::write(root.join("cached-collision.nbt"), bytes).unwrap();
        let behaviour = json!({"blocks": names.iter().map(|name| json!({"name":name})).collect::<Vec<_>>(),
            "states": rows, "fluids": FluidKind::ALL.map(|kind| kind.name()), "items": {}, "fire": {}});
        let gameplay = vec![json!({}); registry.len()];
        // Persist the exact synthetic inputs for audit; invoke the real loaders.
        std::fs::write(root.join("block-behaviours.json"), serde_json::to_vec(&behaviour).unwrap()).unwrap();
        std::fs::write(root.join("gameplay-facts.json"), serde_json::to_vec(&json!({"states":gameplay})).unwrap()).unwrap();
        let behaviours = Behaviours::load(&registry, &root).expect("native minimal fixture resource validation");
        Fixture { registry, behaviours }
    })
}
fn world(shape: LevelShape, chunks: &[(i32, i32)]) -> World {
    let f = fixture(); let mut world = World::new(f.registry.default_state("minecraft:air").unwrap(), 1);
    assert_eq!(world.air, BlockStateId(0)); world.shape = shape;
    for &(x, z) in chunks { world.chunks.insert((x, z), Arc::new(Chunk::with_layout(x, z, world.layout()))); }
    world
}
fn state(name: &str, level: Option<u8>) -> BlockStateId {
    let registry = &fixture().registry;
    let id = registry.default_state(name).unwrap();
    if let Some(level) = level {
        let mut value = registry.state(id).unwrap().clone(); value.properties.insert("level".into(), level.to_string());
        registry.id(&value).unwrap()
    } else { id }
}
fn view(world: &World) -> View<'_> { let f = fixture(); View::new(world, &f.registry, &f.behaviours) }

#[test]
fn actual_view_preserves_shape_bounds_not_widened_chunk_storage() {
    let world = world(LevelShape::NETHER, &[(0, 0)]);
    assert_eq!((world.bottom_y(), world.top_y()), (-64, 320));
    for y in [-64, -1, 256, 319] { assert_eq!(view(&world).sample([0, y, 0]), Sample::Solid); }
    for y in [0, 255] { assert_eq!(view(&world).sample([0, y, 0]), Sample::Air); }
    // Bounds precede loaded state, including absent columns.
    assert_eq!(view(&world).sample([16, -1, 0]), Sample::Solid);
    assert_eq!(view(&world).sample([16, 0, 0]), Sample::Unloaded);
}
#[test]
fn actual_view_keeps_unloaded_unknown_and_recognizes_nondefault_air() {
    let mut world = world(LevelShape::OVERWORLD, &[(-1, -1)]);
    assert_eq!(world.block(0, 64, 0), world.air);
    assert_eq!(view(&world).sample([0, 64, 0]), Sample::Unloaded);
    for name in ["minecraft:cave_air", "minecraft:void_air"] {
        let id = state(name, None); assert_ne!(id, world.air);
        world.set(-1, 64, -1, id).unwrap();
        assert_eq!(view(&world).sample([-1, 64, -1]), Sample::Air);
    }
}
#[test]
fn actual_view_recognizes_only_registered_custom_blocks_and_all_levels() {
    let mut world = world(LevelShape::OVERWORLD, &[(0, 0)]); let d = definition("first");
    for level in 0..=8 {
        world.set(0, 64, 0, state(&d.block, Some(level))).unwrap();
        assert_eq!(view(&world).sample([0, 64, 0]), Sample::Fluid(Cell {fluid:d.id.clone(), level}));
    }
    for name in ["minecraft:stone", "minecraft:water", "minecraft:lava", "pluto_view_fixture:unknown_block"] {
        world.set(0, 64, 0, state(name, None)).unwrap();
        assert_eq!(view(&world).sample([0, 64, 0]), Sample::Solid);
    }
}
#[test]
fn actual_view_queries_preserve_surface_tangency_volumes_unknown_and_budgets() {
    let mut world = world(LevelShape::OVERWORLD, &[(0, 0)]);
    let a = definition("first"); let b = definition("second");
    world.set(0, 64, 0, state(&a.block, Some(0))).unwrap();
    world.set(1, 64, 0, state(&b.block, Some(8))).unwrap();
    let v = view(&world);
    assert!(matches!(v.query_point([0.5, 64.5, 0.5]).unwrap(), PointContact::Fluid {..}));
    assert_eq!(v.query_point([0.5, 64.0 + 8.0 / 9.0, 0.5]).unwrap(), PointContact::Dry);
    let contacts = v.query_box(Bounds {min:[0.0,64.0,0.0], max:[2.0,65.0,1.0]}, Limits::default()).unwrap();
    assert!(contacts.is_complete()); assert_eq!(contacts.sampled_cells, 2);
    assert!((contacts.fluids[&a.id].volume - 8.0 / 9.0).abs() < 1e-12);
    assert_eq!(contacts.fluids[&b.id].volume, 1.0);
    let unknown = v.query_box(Bounds {min:[15.0,64.0,0.0], max:[17.0,65.0,1.0]}, Limits::default()).unwrap();
    assert!(!unknown.is_complete()); assert_eq!(unknown.unloaded_cells, 1); assert!(unknown.fluids.is_empty());
    assert!(matches!(v.query_box(Bounds {min:[0.0;3], max:[2.0;3]}, Limits {max_cells:1}), Err(QueryError::BudgetExceeded {..})));
}
#[test]
fn actual_view_queries_leave_world_storage_and_pending_markers_unchanged() {
    let mut world = world(LevelShape::OVERWORLD, &[(0,0)]);
    world.set(0,64,0,state(&definition("first").block,Some(0))).unwrap();
    world.game_time = 123; world.day_time = 456;
    let before = (world.revision, world.chunks.membership_revision(), world.game_time, world.day_time,
        world.dirty_sections.len(), world.dirty_light.len(), world.light_edits.len(), world.modified.len(),
        world.loaded_entities.len(), world.loaded_chunks.len(), world.block(0,64,0));
    let v = view(&world);
    for _ in 0..4 { v.query_point([0.5,64.5,0.5]).unwrap(); v.query_box(Bounds {min:[0.0,64.0,0.0],max:[2.0,65.0,1.0]},Limits::default()).unwrap(); }
    assert_eq!(before, (world.revision, world.chunks.membership_revision(), world.game_time, world.day_time,
        world.dirty_sections.len(), world.dirty_light.len(), world.light_edits.len(), world.modified.len(),
        world.loaded_entities.len(), world.loaded_chunks.len(), world.block(0,64,0)));
}

#[test]
fn actual_view_height_query_reuses_loaded_decoder_and_supplied_box() {
    let mut world = world(LevelShape::OVERWORLD, &[(0, 0)]);
    let a = definition("first"); let b = definition("second");
    world.set(0, 64, 0, state(&a.block, Some(0))).unwrap();
    world.set(0, 65, 0, state(&a.block, Some(7))).unwrap();
    world.set(1, 64, 0, state(&b.block, Some(8))).unwrap();
    let bounds = Bounds { min: [0.0, 64.001, 0.0], max: [2.0, 65.999, 1.0] };
    let before = (world.revision, world.chunks.membership_revision(), world.modified.len());
    let heights = view(&world).query_heights(bounds, Limits::default()).unwrap();
    assert_eq!(heights.sampled_cells, 4);
    assert_eq!(heights.fluids[&a.id].to_bits(), (65.0f64 + 1.0 / 9.0 - 64.001).to_bits());
    assert_eq!(heights.fluids[&b.id].to_bits(), (65.0f64 - 64.001).to_bits());
    assert!(heights.is_complete());
    let unknown = view(&world).query_heights(Bounds {min:[15.0,64.0,0.0],max:[17.0,65.0,1.0]}, Limits::default()).unwrap();
    assert_eq!(unknown.unloaded_cells, 1); assert!(!unknown.is_complete());
    assert!(view(&world).query_heights(bounds, Limits {max_cells:1}).is_err());
    assert_eq!(before, (world.revision, world.chunks.membership_revision(), world.modified.len()));
}
