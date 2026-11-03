//! Run against a prepared resource root containing TConstruct. This exercises
//! the host FaceVisibility implementation, not GPU rendering or gameplay.
use horizon_core::geometry::Direction;
use horizon_world::{
    block_render::{FaceVisibility, RenderDefinition, SkipRendering},
    collision::CachedCollisionRegistry,
    registry::{BlockRegistry, BlockStateId, BlockTableFile},
};
use std::{collections::BTreeMap, fs, path::Path};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args()
        .nth(1)
        .expect("prepared Horizon resources directory");
    let p = Path::new(&root);
    let registry = BlockRegistry::from_facts_json(fs::File::open(p.join("facts.json"))?)?;
    let table = BlockTableFile::open(&p.join("occlusion.nbt"), &registry)?;
    let occlusion = CachedCollisionRegistry::from_table(table.table("occlusion")?)?;
    let raw: BTreeMap<String, serde_json::Value> =
        serde_json::from_reader(fs::File::open(p.join("render-definitions.json"))?)?;
    let defs = raw
        .iter()
        .map(|(id, v)| {
            let skip = match v["rule"].as_str().unwrap() {
                "never" => SkipRendering::Never,
                "same_block" => SkipRendering::SameBlock,
                "bars" => SkipRendering::Bars,
                "leaves" => SkipRendering::Leaves,
                "liquid" => SkipRendering::Liquid,
                "mangrove_roots" => SkipRendering::MangroveRoots,
                _ => panic!(),
            };
            (
                id.clone(),
                RenderDefinition {
                    skip,
                    can_occlude: v["can_occlude"].as_bool().unwrap(),
                    bars_tag: v["bars_tag"].as_bool().unwrap(),
                },
            )
        })
        .collect();
    let mut v = FaceVisibility::new(&occlusion, &defs)?;
    assert_eq!(raw["tconstruct:enderbark_roots"]["rule"], "mangrove_roots");
    assert_eq!(raw["tconstruct:enderbark_roots"]["can_occlude"], false);
    let a = raw["tconstruct:enderbark_roots"]["states"]
        .as_array()
        .unwrap();
    let b = raw["minecraft:mangrove_roots"]["states"]
        .as_array()
        .unwrap();
    let c = raw["tconstruct:earth_enderbark_roots"]["states"]
        .as_array()
        .unwrap();
    assert_eq!(a.len(), 2, "expected dry and waterlogged root states");
    assert_eq!(b.len(), 2, "expected dry and waterlogged mangrove states");
    assert_eq!(c.len(), 1, "expected one slimy root state");
    let mut waterlogged: Vec<_> = a
        .iter()
        .map(|id| {
            let id = BlockStateId(id.as_u64().unwrap() as u32);
            let state = registry.state(id).expect("registered root state");
            assert_eq!(state.name, "tconstruct:enderbark_roots");
            state
                .properties
                .get("waterlogged")
                .expect("waterlogged property")
                .as_str()
        })
        .collect();
    waterlogged.sort();
    assert_eq!(waterlogged, ["false", "true"]);
    for cutout in [false, true] {
        for x in a {
            for y in a {
                for d in Direction::ALL {
                    let skip = v.skip_rendering(
                        BlockStateId(x.as_u64().unwrap() as u32),
                        BlockStateId(y.as_u64().unwrap() as u32),
                        d,
                        cutout,
                    )?;
                    assert_eq!(skip, matches!(d, Direction::Up | Direction::Down));
                    let vis = v.should_render_face(
                        BlockStateId(x.as_u64().unwrap() as u32),
                        BlockStateId(y.as_u64().unwrap() as u32),
                        d,
                        cutout,
                    )?;
                    assert_eq!(vis, !matches!(d, Direction::Up | Direction::Down));
                }
            }
        }
        for x in a {
            for group in [b, c] {
                for y in group {
                    for d in Direction::ALL {
                        assert!(!v.skip_rendering(
                            BlockStateId(x.as_u64().unwrap() as u32),
                            BlockStateId(y.as_u64().unwrap() as u32),
                            d,
                            cutout
                        )?);
                    }
                }
            }
        }
    }
    println!("PASS: root states {}, same-root 24 direction pairs per leaf mode, different mangrove/slimy roots 36 pairs per mode; vertical faces culled",a.len());
    Ok(())
}
