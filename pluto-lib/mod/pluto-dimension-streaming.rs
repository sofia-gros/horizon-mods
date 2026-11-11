// Adapted from MIT Horizon; covered by pluto-lib/LICENSE.horizon.
// Native constructor retained; only dimension saved-start reads added.
use super::*;

struct SavedStarts(horizon_client::old_world::SavedOldWorld);
impl horizon_world::worldgen::blending::OldWorld for SavedStarts {
    fn is_old_chunk_around(&self, _: horizon_core::position::ChunkPos, _: i32) -> bool { false }
    fn blending(&self, _: i32, _: i32) -> Option<Arc<horizon_world::worldgen::blending::BlendingData>> { None }
    fn old_biomes(&self, _: i32, _: i32) -> Option<Arc<horizon_world::worldgen::biome_chunk::BiomeChunk<horizon_world::worldgen::biome::BiomeId>>> { None }
    fn saved_starts(&self, x: i32, z: i32) -> Vec<horizon_world::worldgen::blending::SavedStart> {
        horizon_world::worldgen::blending::OldWorld::saved_starts(&self.0, x, z)
    }
}

pub(super) fn make_streaming(
    res: &Resources,
    pack: &PackResources,
    options: &Options,
    settings: &Settings,
    world_root: &std::path::Path,
    dim: Dim,
    shape: horizon_client::chunk::LevelShape,
    seed: i64,
    enabled_features: &[String],
) -> Result<(Streaming, Arc<SaveDir>, Option<Arc<VanillaTerrain>>)> {
    let save = Arc::new(SaveDir::new(
        horizon_client::save::dimension_dir(world_root, dim.id()),
        shape,
    ));
    if dim == Dim::End {
        // Migrate before any chunk reader can cache the old block entities. A side file it
        // can't read stays where it is and costs only its old exits: the End still opens.
        if let Err(e) = save.migrate_end_gateway_exits() {
            log::warn!("Horizon: End gateway exits not migrated: {e}");
        }
    }
    // The world's packs over the game's biomes (`RegistryDataLoader`
    // layering): the generator names the ids and the client draws them.
    let biomes = packed_biomes(
        world_root,
        &options.resources,
        res.biomes(),
        enabled_features,
    );
    let mut locator = None;
    let flat = (dim == Dim::Overworld)
        .then(|| horizon_client::save::world_gen_settings(world_root))
        .flatten()
        .and_then(|s| {
            horizon_client::flatgen::FlatTerrain::from_settings(
                &s,
                pack.assets.registry,
                res.biomes(),
                shape,
            )
        });
    // `DebugLevelSource`: the overworld of the Debug preset.
    let debug = (dim == Dim::Overworld)
        .then(|| horizon_client::save::world_gen_settings(world_root))
        .flatten()
        .and_then(|s| {
            horizon_client::debugworld::DebugTerrain::from_settings(
                &s,
                pack.assets.registry,
                res.biomes(),
                shape,
            )
        });
    // PlaceCommand.placeJigsaw/placeStructure generate directly, independently
    // of WorldOptions.generateStructures. Keep their flat locator available
    // even when automatic structure starts are disabled.
    let automatic_structures =
        horizon_client::save::WorldGen::read(world_root).is_none_or(|w| w.structures);
    let flat_settings = (dim == Dim::Overworld)
        .then(|| horizon_client::save::world_gen_settings(world_root))
        .flatten()
        .and_then(|s| horizon_client::flatgen::FlatTerrain::pipeline_settings(&s));
    // `adjustGenerationSettings`: a superflat world with features or lakes is decorated
    // whether or not its structures generate.
    let flat_decorates = flat_settings
        .as_ref()
        .is_some_and(|s| s.features || s.lakes);
    // A generator that cannot be built (a layer block the registry lacks, a level shorter
    // than the noise) leaves the world on its flat ground, as it was before.
    let flat_vanilla = flat_settings.and_then(|settings| {
        let overrides =
            world_worldgen_overrides(world_root, &options.resources, &biomes, enabled_features);
        match VanillaTerrain::for_flat(
            &options.resources.join("worldgen"),
            &overrides,
            pack.assets.registry,
            &biomes,
            seed,
            &settings,
            automatic_structures,
        ) {
            Ok(mut terrain) => {
                terrain.add_mod_signs(res.facts());
                Some(Arc::new(terrain))
            }
            Err(e) => {
                log::warn!("Horizon: the superflat structures and features are unavailable: {e}");
                None
            }
        }
    });
    let mut streaming = if let Some(inner) = debug {
        Streaming::new(
            Arc::new(Persistent {
                inner: Arc::new(inner),
                save: save.clone(),
                registry: pack.assets.clone(),
                biomes: biomes.clone(),
            }),
            pack.context_with(biomes.clone()),
            res.threads(),
        )
    } else if let Some(inner) = flat_vanilla
        .as_ref()
        .filter(|_| automatic_structures || flat_decorates)
        .cloned()
    {
        if automatic_structures {
            inner.search_rings_behind();
        }
        inner.set_reference_file(world_root.join("data").join(format!(
            "horizon_structure_references_{}.json",
            dim.id().replace(':', "_")
        )));
        {
            let save = save.clone();
            inner.set_saved_references(Arc::new(move |name, x, z| {
                save.start_references(x, z, name)
            }));
        }
        locator = Some(inner.clone());
        Streaming::new(
            Arc::new(Persistent {
                inner,
                save: save.clone(),
                registry: pack.assets.clone(),
                biomes: biomes.clone(),
            }),
            pack.context_with(biomes.clone()),
            res.threads(),
        )
    } else if let Some(inner) = flat {
        // A superflat world that neither decorates nor generates structures: its
        // layers alone. Only manual commands use the locator.
        locator = flat_vanilla;
        Streaming::new(
            Arc::new(Persistent {
                inner: Arc::new(inner),
                save: save.clone(),
                registry: pack.assets.clone(),
                biomes: biomes.clone(),
            }),
            pack.context_with(biomes.clone()),
            res.threads(),
        )
    } else if options.test_terrain && dim == Dim::Overworld {
        let inner = Arc::new(TestTerrain::new(pack.assets.registry, res.biomes())?);
        Streaming::new(
            Arc::new(Persistent {
                inner,
                save: save.clone(),
                registry: pack.assets.clone(),
                biomes: biomes.clone(),
            }),
            pack.context_with(biomes.clone()),
            res.threads(),
        )
    } else {
        // WorldOptions.generateStructures, from the world's settings.
        let structures =
            horizon_client::save::WorldGen::read(world_root).is_none_or(|w| w.structures);
        let preset = horizon_client::save::WorldGen::preset(world_root);
        let overrides =
            world_worldgen_overrides(world_root, &options.resources, &biomes, enabled_features);
        let saved_stem = horizon_client::save::world_gen_settings(world_root)
            .and_then(|s| s.get("dimensions")?.get(dim.id()).cloned())
            .map(|stem| horizon_client::scoreboard::tag_to_json(&stem));
        let mut inner = VanillaTerrain::for_world_with_settings(&options.resources.join("worldgen"), &overrides, pack.assets.registry, &biomes, seed, preset, dim.id(), structures, saved_stem.as_ref()).map_err(|e| {
                if overrides.is_empty() {
                    format!("{e}\nPrepare the worldgen bundle with the launcher, or: horizon-prepare --prepare")
                } else {
                    format!("{e}\nThe world's data packs change its world generation: one of them may not be made for this version")
                }
            })?;
        inner.add_mod_signs(res.facts());
        let inner = Arc::new(inner);
        if dim == Dim::Overworld && structures {
            inner.search_rings_behind();
        }
        // Chunks an older version saved: the new chunks around them blend in.
        if dim == Dim::Overworld {
            inner.set_old_world(Arc::new(horizon_client::old_world::SavedOldWorld::new(
                save.clone(),
                pack.assets.clone(),
                res.biomes().clone(),
                inner.generator_data().clone(),
            )));
        }
        inner.set_reference_file(world_root.join("data").join(format!(
            "horizon_structure_references_{}.json",
            dim.id().replace(':', "_")
        )));
        {
            let save = save.clone();
            inner.set_saved_references(Arc::new(move |name, x, z| {
                save.start_references(x, z, name)
            }));
        }
        locator = Some(inner.clone());
        Streaming::new(
            Arc::new(Persistent {
                inner,
                save: save.clone(),
                registry: pack.assets.clone(),
                biomes: biomes.clone(),
            }),
            pack.context_with(biomes.clone()),
            res.threads(),
        )
    };
    streaming.forced = save.load_forced_chunks();
    streaming.radius = settings.render_distance;
    streaming.biome_blend = settings.biome_blend;
    streaming.smooth_lighting = settings.smooth_lighting;
    streaming.cutout_leaves = settings.cutout_leaves;
    streaming.leaf_culling = settings
        .horizon
        .leaf_culling_effective(&settings.resource_packs);
    streaming.biome_zoom = BiomeZoom::new(obfuscate_seed(seed));
    streaming.sky_light = dim != Dim::Nether;
    streaming.nether_shading = dim == Dim::Nether;
    // A server draws nothing: neither the integrated server nor the
    // dedicated one meshes sections.
    streaming.mesh = options.execution_role.is_client();
    // Only saved starts are exposed in other dimensions: no terrain blending.
    if dim != Dim::Overworld {
        if let Some(inner) = locator.as_ref() {
            inner.set_old_world(Arc::new(SavedStarts(horizon_client::old_world::SavedOldWorld::new(
                save.clone(), pack.assets.clone(), res.biomes().clone(), inner.generator_data().clone(),
            ))));
        }
    }
    Ok((streaming, save, locator))
}
