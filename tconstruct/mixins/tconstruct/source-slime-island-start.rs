// Source island start; resource preparation must supply every required source
// block, template and configured feature before adding island structure sets.
use super::*;
use super::tconstruct_slime_island_piece::{IslandPiece, MarkerConfig};
use super::tconstruct_slime_tree_config::SlimeTreeSpec;
use crate::worldgen::structure::{Mirror, Rotation};

fn entries(value: &Value, key: &str) -> Option<Vec<(String, u32)>> {
    value.get(key)?.as_array()?.iter().map(|entry| {
        let name = entry.get("data")?.as_str()?.to_owned();
        let weight = u32::try_from(entry.get("weight")?.as_u64()?).ok()?;
        (weight > 0).then_some((name, weight))
    }).collect()
}

fn draw<'a>(choices: &'a [(String, u32)], random: &mut WorldgenRandom<LegacyRandom>) -> Option<&'a str> {
    if choices.is_empty() { return None; }
    let total = choices.iter().try_fold(0u32, |sum, (_, weight)| sum.checked_add(*weight))?;
    let value = random.next_int_bound(i32::try_from(total).ok()?).ok()? as u32;
    let weights: Vec<_> = choices.iter().map(|(_, weight)| *weight).collect();
    let selected = super::tconstruct_slime_island_markers::weighted_index(&weights, value)?;
    Some(&choices[selected].0)
}

fn source_entry(resources: &StructureResources, bundle: &str, id: &str) -> Option<Value> {
    if bundle == "features.json" {
        // Keep the original custom feature outside the host's vanilla feature
        // catalog, whose decoder does not understand tconstruct:slime_tree.
        let name = match id {
            "tconstruct:earth_slime_island_tree" => "earth_slime_island_tree.json",
            "tconstruct:sky_slime_island_tree" => "sky_slime_island_tree.json",
            _ => return None,
        };
        let root = resources.dir.parent()?.parent()?;
        let data = std::fs::read(root.join("data/tconstruct/island_feature").join(name)).ok()?;
        return serde_json::from_slice(&data).ok();
    }
    let dir = resources.dir.parent()?;
    let map: Value = serde_json::from_slice(&std::fs::read(dir.join(bundle)).ok()?).ok()?;
    serde_json::from_str(map.get(id)?.as_str()?).ok()
}

pub(crate) fn fungus_replaceable(bindings: &FeatureBlockBindings<'_>, resources: &StructureResources) -> Option<std::sync::Arc<Vec<bool>>> {
    let facts = bindings.tree_blocks().ok()?;
    Some(std::sync::Arc::new((0..resources.registry.len()).map(|id|
        facts.has(crate::registry::BlockStateId(id as u32), crate::worldgen::tree_blocks::REPLACEABLE)).collect()))
}

pub(crate) fn generate(registry: &StructureRegistry, resources: &StructureResources, bindings: &FeatureBlockBindings<'_>,
    context: &mut impl StartContext, structure: usize, chunk: ChunkPos, any_biome: bool) -> Option<StructureStart> {
    let def = registry.structures.get(structure)?;
    context.set_valid_biomes((!any_biome).then_some(&def.biomes));
    let surface = top_of_chunk_center(context, chunk, HeightmapType::WorldSurfaceWg)?;
    if !any_biome && !def.biomes.contains(&context.noise_biome(surface.map(|v| v >> 2))) { return None; }
    let source = source_entry(resources, "structures.json", &def.name)?;
    if source.get("type")?.as_str()? != "tconstruct:island" { return None; }
    let templates = entries(&source, "templates")?;
    let trees = entries(&source, "trees")?;
    let mut grasses = entries(&source, "grasses")?;
    // Vanilla renamed the short grass block after the source version.
    for (name, _) in &mut grasses {
        if name == "minecraft:grass" { *name = "minecraft:short_grass".into(); }
    }
    let mut random = WorldgenRandom::new(LegacyRandom::new(0));
    random.set_large_feature_seed(context.seed(), chunk.x, chunk.z);
    let template = draw(&templates, &mut random)?;
    let rotation = Rotation::random(&mut random);
    let height = match source.get("placement")?.as_str()? {
        "sea" => context.sea_level().checked_sub(7)?.max(0),
        "sky" => {
            let (dx, dz) = match rotation { Rotation::None => (5, 5), Rotation::Clockwise90 => (-5, 5),
                Rotation::Clockwise180 => (-5, -5), Rotation::Counterclockwise90 => (5, -5) };
            let x = chunk.x * 16 + 7;
            let z = chunk.z * 16 + 7;
            let lowest = *corner_heights(context, x, dx, z, dz).iter().min()?;
            lowest.checked_add(60)?.checked_add(random.next_int_bound(50).ok()?)?
                .min(context.generation().height.checked_sub(20)?)
        },
        _ => return None,
    };
    let mirror = match random.next_int_bound(3).ok()? { 0 => Mirror::None, 1 => Mirror::LeftRight, _ => Mirror::FrontBack };
    let (tree, deferred_tree) = if trees.is_empty() { (None, None) } else {
        let id = draw(&trees, &mut random)?;
        match id {
            "tconstruct:earth_slime_island_tree" | "tconstruct:sky_slime_island_tree" => {
                let feature = source_entry(resources, "features.json", id)?;
                (Some(SlimeTreeSpec::from_json(id, &feature)?), None)
            }
            "tconstruct:ender_slime_tree" | "tconstruct:ender_slime_tree_tall" |
            "tconstruct:blood_slime_island_fungus" | "minecraft:oak" | "minecraft:birch" |
            "minecraft:spruce" | "minecraft:acacia" | "minecraft:jungle_tree_no_vine" => (None, Some(id.to_owned())),
            _ => return None,
        }
    };
    let vines = source.get("vines").and_then(Value::as_str).map(str::to_owned);
    let markers = MarkerConfig { structure: def.name.clone(), tree, deferred_tree, grasses, vine: vines };
    let piece = IslandPiece::new(resources, template, [chunk.x * 16 + 8, height, chunk.z * 16 + 8],
        rotation, mirror, markers, 0, fungus_replaceable(bindings, resources), context.generation().height)?;
    Some(StructureStart::new(structure, def, chunk, vec![Piece::Legacy(Box::new(piece))]))
}
