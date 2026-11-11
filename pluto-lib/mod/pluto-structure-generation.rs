// Generation-world orchestration adapted from the pinned MIT Horizon source.
// Keep native terrain swaps, feature ordering, RNG seeds, entities and clipping.
// Only registered piece dispatch differs; the official input is hash-guarded.
use super::*;

use super::pluto_structure_generation_placement as placement;

pub(super) fn has_registered(region:&[ProtoChunk;9])->bool {
    region[4].structure_starts.iter().any(|start|crate::level::pluto_structure_level::has_registered(start))
}

impl Decoration {
    pub(super) fn pluto_decorate(
        &self,
        data: &Arc<GeneratorData>,
        seed: i64,
        generation: WorldGenerationContext,
        worker: &mut WorkerState,
        region: &mut [ProtoChunk; 9],
        level_random: &mut dyn crate::worldgen::structure::legacy::PieceRandom,
        structures: Option<&crate::worldgen::generator_structures::StructureState>,
    ) -> Result<DecorateStats, GeneratorError> {
        let center = region[4].position();
        for (i, chunk) in region.iter().enumerate() {
            let expected = ChunkPos::new(
                center.x.wrapping_add(i as i32 % 3 - 1),
                center.z.wrapping_add(i as i32 / 3 - 1),
            );
            if chunk.position() != expected {
                return Err(resource("decoration region is not a 3×3 neighborhood"));
            }
            if (chunk.terrain.generation_stage() as u8) < PreLightChunkStage::Terrain as u8 {
                return Err(resource("decoration requires terrain-stage chunks"));
            }
        }
        if region[4].is_decorated() {
            return Ok(DecorateStats::default());
        }
        let mut chunks = Vec::with_capacity(9);
        let mut biomes = Vec::with_capacity(9);
        let (min_y, height) = (region[4].terrain.min_y(), region[4].terrain.height());
        for chunk in region.iter_mut() {
            // A one-section stand-in while the region holds the terrain: it
            // is swapped back below, and allocating the full height for nine
            // stand-ins per region cost more than some features.
            let empty = crate::worldgen::terrain::TerrainChunk::new(
                chunk.position(),
                min_y,
                16,
                data.registry(),
                data.block_states().clone(),
            )
            .map_err(|e| resource(e.to_string()))?;
            chunks.push(std::mem::replace(&mut chunk.terrain, empty));
            biomes.push(chunk.biomes().clone());
        }
        let terrain = TerrainRegion::new(chunks, generation)
            .and_then(|region| region.with_block_bindings(&self.bindings))
            .map(|region| region.with_world_seed(seed));
        let terrain = match terrain {
            Ok(terrain) => terrain,
            Err(e) => return Err(resource(e.to_string())),
        };
        let structures_only = worker.structures_only || !region[4].features;
        let references = region[4].structure_starts.clone();
        let mut block_entities = Vec::new();
        for chunk in region.iter_mut() {
            block_entities.append(&mut chunk.block_entities);
        }
        let mut world = World {
            block_entities,
            entities: Vec::new(),
            level_random,
            structure_mobs: worker.structure_mobs.clone(),
            starts: references.clone(),
            biome_names: data.biome_names(),
            blocks: data
                .feature_resources
                .as_ref()
                .and_then(|r| r.blocks.as_ref()),
            region: terrain,
            biomes: BiomeRegion::new(biomes, obfuscate_seed(seed))
                .map_err(|e| resource(e.to_string()))?,
            zone: GenerationWriteZone::new(center, 1),
            center,
            uniform_biomes: vec![UNKNOWN_BIOME; 9 * (height / 16).max(0) as usize],
            biome_quarts: region[4].biomes().quart_range(),
            decoration: self,
            temperatures: &mut worker.temperatures,
            behaviours: data.behaviours.as_deref(),
            // WorldGenRegion.isOutsideBuildHeight: the level's range, not the noise's.
            build: (min_y, min_y + height),
            shape_ticks: std::cell::RefCell::new(crate::level::Ticks::new(0)),
        };
        let mut stats = DecorateStats::default();
        let blocks = self.blocks;
        // The blocks' facts: `BlockState.isSolidRender` and the rest, the
        // structure pieces ask them of the blocks around them.
        let tree_blocks = self
            .bindings
            .tree_blocks()
            .map_err(|e| resource(e.to_string()))?;
        // ChunkGenerator.getWritableArea: the chunk's minimum Y + 1 to its maximum Y.
        let chunk_box = BoundingBox::new(
            [center.x * 16, min_y + 1, center.z * 16],
            [center.x * 16 + 15, min_y + height - 1, center.z * 16 + 15],
        );
        let mut structure_errors = 0;
        let result = self.plan.decorate(
            &mut world,
            center,
            seed,
            STEPS.len(),
            |world, random, decoration_seed, step| {
                let Some(structures) = structures else {
                    return Ok(());
                };
                let Some(in_step) = structures.by_step.get(step) else {
                    return Ok(());
                };
                for (index, &structure) in in_step.iter().enumerate() {
                    random.set_feature_seed(decoration_seed, index as i32, step as i32);
                    for start in references.iter().filter(|s| s.structure == structure) {
                        let mut place_feature =
                            |world: &mut World<'_>,
                             random: &mut WorldgenRandom<XoroshiroRandom>,
                             name: &str,
                             pos: [i32; 3]| {
                                match self.named_feature(name) {
                                    Some(feature) => feature
                                        .place(world, random, pos, blocks)
                                        .map_err(|e| PlacementQueryError::Failed(e.to_string())),
                                    None => Ok(false),
                                }
                            };
                        if placement::place_in_chunk(
                            data,
                            start,
                            world,
                            &structures.resources,
                            &tree_blocks,
                            random,
                            chunk_box,
                            &mut place_feature,
                        )
                        .is_err()
                        {
                            structure_errors += 1;
                        }
                    }
                }
                Ok(())
            },
            |world, placer, random, id, origin| {
                if structures_only {
                    return Ok(false);
                }
                let Ok(feature) = &self.features[id.0 as usize] else {
                    stats.unsupported += 1;
                    stats.skipped.insert(self.names[id.0 as usize].clone());
                    return Ok(false);
                };
                stats.placed += 1;
                let placed = placer.place_with_biome_check(
                    &feature.placement,
                    world,
                    random,
                    origin,
                    id,
                    |world, random, p| feature.feature.place(world, random, p, blocks),
                );
                if placed.is_err() {
                    stats.failed += 1;
                    stats.skipped.insert(self.names[id.0 as usize].clone());
                }
                Ok(placed.unwrap_or(false))
            },
        );
        // `FillLayerFeature.place` for the chunk: air at the layer's Y becomes the layer.
        if result.is_ok() && !structures_only {
            for &(y, state) in &self.top_layers {
                for dz in 0..16 {
                    for dx in 0..16 {
                        let at = [center.x * 16 + dx, y, center.z * 16 + dz];
                        let current = world
                            .block(at)
                            .map_err(|e| GeneratorError::Stage("features", e.to_string()))?;
                        if data.block_states().get(current).is_air() {
                            world
                                .set_block(at, state, 2)
                                .map_err(|e| GeneratorError::Stage("features", e.to_string()))?;
                        }
                    }
                }
            }
        }
        for (pos, nbt) in std::mem::take(&mut world.block_entities) {
            let index = ((pos[2] >> 4).wrapping_sub(center.z) + 1) * 3
                + (pos[0] >> 4).wrapping_sub(center.x)
                + 1;
            if let Some(chunk) = usize::try_from(index).ok().and_then(|i| region.get_mut(i)) {
                chunk.block_entities.push((pos, nbt));
            }
        }
        // WorldGenRegion.addFreshEntity: into the chunk holding the position.
        for nbt in std::mem::take(&mut world.entities) {
            let Some(at) = block_of(&nbt) else {
                continue;
            };
            let index = ((at[2] >> 4).wrapping_sub(center.z) + 1) * 3
                + (at[0] >> 4).wrapping_sub(center.x)
                + 1;
            if let Some(chunk) = usize::try_from(index).ok().and_then(|i| region.get_mut(i)) {
                chunk.entities.push(nbt);
            }
        }
        for terrain in world.region.into_chunks() {
            let p = terrain.position();
            let index = (p.z.wrapping_sub(center.z) + 1) * 3 + p.x.wrapping_sub(center.x) + 1;
            region[index as usize].terrain = terrain;
        }
        result.map_err(|e| GeneratorError::Stage("features", e.to_string()))?;
        stats.failed += structure_errors;
        region[4]
            .terrain
            .set_generation_stage(PreLightChunkStage::Features);
        region[4].set_decorated();
        Ok(stats)
    }
}
