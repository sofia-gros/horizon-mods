// Live native trees with borrowed, tracked decorator callbacks.
use super::{Behaviours, LevelAccess, Ticks, feature_world::LevelFeatureWorld};
use super::pluto_block_access::{StateInfo, LevelView, LevelEdit, TreeCallback, RootBindings, RootEdit, RootCallback, TreeStep, PlannedTreeCallback, MAX_FEATURE_DEPTH};
use crate::registry::BlockStateId;
use crate::worldgen::{feature_graph::{FeatureDefinition, FeatureExecutionError}, placement::PlacementQueryError};
use crate::worldgen::live::pluto_tree_catalog;
use horizon_core::{position::BlockPos, random::{LegacyRandom, RandomSource}};
use serde_json::Value;

fn at([x, y, z]: [i32; 3]) -> BlockPos { BlockPos::new(x, y, z) }

pub(super) struct TreeEditor<'a, 's, 'l, L: LevelAccess + ?Sized> {
    world: &'a mut LevelFeatureWorld<'s, 'l, L>,
    random: &'a mut LegacyRandom,
    written: &'a mut Vec<[i32; 3]>,
    bound_roots: Option<&'a pluto_tree_catalog::BoundRoots>,
}

impl<'a,'s,'l,L: LevelAccess + ?Sized> TreeEditor<'a,'s,'l,L> {
    pub(super) fn for_markers(world:&'a mut LevelFeatureWorld<'s,'l,L>,random:&'a mut LegacyRandom,written:&'a mut Vec<[i32;3]>)->Self {
        Self {world,random,written,bound_roots:None}
    }
    pub(super) fn with_world<T>(&mut self,body:impl FnOnce(&mut LevelFeatureWorld<'s,'l,L>,&mut LegacyRandom)->T)->T {
        body(self.world,self.random)
    }
    fn info(&self, id: BlockStateId) -> Option<StateInfo> {
        let state = self.world.features.data().registry().state(id)?;
        Some(StateInfo { block: state.name.to_string(), properties: state.properties.iter().map(|(k,v)| (k.to_string(),v.to_string())).collect() })
    }
    pub(super) fn resolve(&self, wanted: &StateInfo) -> Option<BlockStateId> {
        let registry = self.world.features.data().registry();
        let mut state = registry.state(registry.default_state(&wanted.block)?)?.clone();
        for (key, value) in &wanted.properties { state.properties.insert(key.as_str().into(), value.as_str().into()); }
        registry.resolve(&state).ok()
    }
}

impl<L: LevelAccess + ?Sized> LevelView for TreeEditor<'_, '_, '_, L> {
    fn state(&self, pos: [i32; 3]) -> Option<StateInfo> {
        let level = &*self.world.sim.level;
        if !level.is_loaded(at(pos)) || !(level.min_y()..level.max_y()).contains(&pos[1]) { return None }
        self.info(self.world.sim.state(at(pos)))
    }
    fn is_air(&self, pos: [i32; 3]) -> bool { self.world.sim.b.is_air(self.world.sim.state(at(pos))) }
    fn face_sturdy(&self, pos: [i32; 3], face: u8) -> bool {
        face < 6 && self.world.sim.b.shape_info(self.world.sim.state(at(pos))).full_faces & (1 << face) != 0
    }
    fn in_tag(&self, pos: [i32; 3], tag: &str) -> bool {
        let b = self.world.sim.b;
        b.tag(tag).is_some_and(|set| set.contains(b.block(self.world.sim.state(at(pos)))))
    }
    fn brightness(&self, pos: [i32; 3]) -> u8 {
        let level = &*self.world.sim.level;
        super::raw_brightness(level, at(pos), level.sky_darken()).clamp(0,15) as u8
    }
    fn light_passes_up(&self, pos: [i32; 3]) -> bool {
        use super::data::state_flags::{CAN_OCCLUDE, SHAPE_OCCLUSION};
        use horizon_core::geometry::Direction;
        let b = self.world.sim.b;
        let state = self.world.sim.state(at(pos));
        let above = self.world.sim.state(at([pos[0], pos[1].wrapping_add(1), pos[2]]));
        let occludes = |s| b.facts(s).flags & (CAN_OCCLUDE | SHAPE_OCCLUSION) == (CAN_OCCLUDE | SHAPE_OCCLUSION);
        let full = (occludes(state) && b.shape_info(state).full_faces & (1 << Direction::Up as u8) != 0)
            || (occludes(above) && b.shape_info(above).full_faces & (1 << Direction::Down as u8) != 0);
        (if full { 16 } else { i32::from(b.facts(above).dampening) }) < 15
    }
    fn is_water(&self, pos: [i32; 3]) -> bool {
        self.state(pos).is_some_and(|s| s.block == "minecraft:water" || s.properties.get("waterlogged").is_some_and(|v| v == "true"))
    }
    fn fluid_full(&self, pos: [i32; 3]) -> bool {
        self.state(pos).is_some_and(|s| s.properties.get("waterlogged").is_some_and(|v| v == "true")
            || (matches!(s.block.as_str(), "minecraft:water" | "minecraft:lava") && s.properties.get("level").is_some_and(|v| v == "0")))
    }
    fn area_loaded(&self, pos: [i32; 3], radius: i32) -> bool {
        if radius < 0 { return false }
        [-radius, radius].iter().all(|dx| [-radius, radius].iter().all(|dz| {
            let (Some(x), Some(z)) = (pos[0].checked_add(*dx), pos[2].checked_add(*dz)) else { return false };
            self.world.sim.level.is_loaded(at([x, pos[1], z]))
        }))
    }
    fn full_block(&self, pos: [i32; 3]) -> bool { self.world.sim.b.shape_info(self.world.sim.state(at(pos))).full_faces & 0x3f == 0x3f }
    fn build_height(&self) -> [i32; 2] { [self.world.sim.level.min_y(), self.world.sim.level.max_y()] }
}

impl<L: LevelAccess + ?Sized> LevelEdit for TreeEditor<'_, '_, '_, L> {
    fn set_state(&mut self, pos: [i32; 3], state: &StateInfo, flags: i32) -> bool {
        let Some(id) = self.resolve(state) else { return false };
        let changed = self.world.sim.set_block(at(pos), id, flags);
        if changed { self.written.push(pos); }
        changed
    }
    fn random_int(&mut self, bound: i32) -> i32 { if bound <= 0 { 0 } else { self.random.next_int_bound(bound).unwrap_or(0) } }
    fn random_float(&mut self) -> f32 { self.random.next_float() }
    fn schedule_tick(&mut self, pos: [i32; 3], block: &str, delay: i32) -> bool {
        let Some(id) = self.world.features.data().registry().default_state(block) else { return false };
        self.world.sim.schedule_tick(at(pos), self.world.sim.b.block(id), delay);
        true
    }
    fn place_feature(&mut self, _feature: &str, _pos: [i32; 3]) -> bool {
        // A nested feature writes outside this tree's tracked sets. Until that
        // integration exists, reject it without writing rather than lose tracking
        // or invoke a second algorithm/RNG stream.
        false
    }
    fn place_tree(&mut self, _configuration: &Value, _pos: [i32; 3], _decorate: &mut TreeCallback<'_>) -> bool {
        // Nested trees also need to merge tracked writes into the outer tree.
        // Conservative documented stub, not a stale level-RNG clone.
        false
    }
}

impl<L: LevelAccess + ?Sized> RootEdit for TreeEditor<'_, '_, '_, L> {
    fn matches_block_set(&mut self, index: usize, pos: [i32; 3]) -> Result<bool, String> {
        let set = self.bound_roots.and_then(|b| b.block_sets.get(index)).ok_or_else(|| "root block-set binding unavailable".to_owned())?;
        if !self.world.sim.level.is_loaded(at(pos)) { return Err("root position not loaded".into()) }
        Ok(set.contains(&self.world.sim.state(at(pos))))
    }
    fn sample_state_provider(&mut self, index: usize, pos: [i32; 3]) -> Result<StateInfo, String> {
        let provider = self.bound_roots.and_then(|b| b.state_providers.get(index)).ok_or_else(|| "root provider binding unavailable".to_owned())?;
        let state = provider.get_optional_state(self.world, self.random, pos).map_err(|e| e.to_string())?
            .ok_or_else(|| "selected root provider produced no state".to_owned())?;
        self.info(state).ok_or_else(|| "sampled root state unavailable".to_owned())
    }
}

pub(super) fn on_world<L: LevelAccess + ?Sized>(
    world: &mut LevelFeatureWorld<'_, '_, L>, random: &mut LegacyRandom,
    configuration: &Value, pos: [i32; 3], mut roots: Option<(&RootBindings, &mut RootCallback<'_>)>, plan: Option<&[TreeStep]>, decorate: &mut PlannedTreeCallback<'_>,
) -> bool {
    let tree = match pluto_tree_catalog::decode_tree(world.features, configuration) {
        Ok(tree) => tree,
        Err(error) => { eprintln!("PlutoLib: tree configuration failed: {error}"); return false }
    };
    let bound = match roots.as_ref() {
        Some((bindings, _)) => match pluto_tree_catalog::bind_roots(world.features, bindings) {
            Ok(bound) => Some(bound),
            Err(error) => { eprintln!("PlutoLib: root binding failed: {error}"); return false }
        },
        None => None,
    };
    let has_roots = roots.is_some();
    let blocks = pluto_tree_catalog::feature_blocks(world.features);
    let mut nested = |feature: &FeatureDefinition, world: &mut LevelFeatureWorld<'_, '_, L>, random: &mut LegacyRandom, pos| {
        feature.place(world, random, pos, blocks)
    };
    let mut root_override = |world: &mut LevelFeatureWorld<'_, '_, L>, random: &mut LegacyRandom, pos, written: &mut Vec<[i32; 3]>| {
        let mut editor = TreeEditor { world, random, written, bound_roots: bound.as_ref() };
        let callback = &mut *roots.as_mut().expect("root callback exists").1;
        let selected = callback(&mut editor, pos).map_err(|error| FeatureExecutionError::Query(PlacementQueryError::Failed(error)))?;
        selected.map(|state| editor.resolve(&state).ok_or_else(|| FeatureExecutionError::Query(PlacementQueryError::Failed("root override state invalid".into())))).transpose()
    };
    let root_override: Option<&mut crate::worldgen::tree::pluto_tree_placement::RootOverride<'_, _, _>> = if has_roots { Some(&mut root_override) } else { None };
    let result = crate::worldgen::tree::pluto_tree_placement::place_with_plan(&tree, world, random, pos, &mut nested, root_override, plan,
        |index, world, random, positions, written| {
            let mut editor = TreeEditor { world, random, written, bound_roots: None };
            decorate(index, &mut editor, positions.logs, positions.leaves, positions.roots)
                .map_err(|error| FeatureExecutionError::Query(PlacementQueryError::Failed(error)))
        });
    match result {
        Ok(placed) => placed,
        Err(error) => { eprintln!("PlutoLib: tree placement failed: {error}"); false }
    }
}

/// Concrete live entry: preserve the copied native RNG even when a callback
/// panics and a consumer catches that panic. No world rollback is attempted.
pub fn place(
    behaviours: &Behaviours, level: &mut dyn LevelAccess, ticks: &mut Ticks,
    configuration: &Value, pos: [i32; 3], depth: u8, decorate: &mut TreeCallback<'_>,
) -> bool {
    place_bound(behaviours, level, ticks, configuration, pos, depth, None, decorate)
}

pub fn place_with_roots(
    behaviours: &Behaviours, level: &mut dyn LevelAccess, ticks: &mut Ticks,
    configuration: &Value, pos: [i32; 3], depth: u8, bindings: &RootBindings, root: &mut RootCallback<'_>, decorate: &mut TreeCallback<'_>,
) -> bool {
    place_bound(behaviours, level, ticks, configuration, pos, depth, Some((bindings, root)), decorate)
}

fn place_bound(
    behaviours: &Behaviours, level: &mut dyn LevelAccess, ticks: &mut Ticks,
    configuration: &Value, pos: [i32; 3], depth: u8, roots: Option<(&RootBindings, &mut RootCallback<'_>)>, decorate: &mut TreeCallback<'_>,
) -> bool {
    place_planned_bound(behaviours, level, ticks, configuration, pos, depth, roots, None,
        &mut |_, world, logs, leaves, roots| decorate(world, logs, leaves, roots))
}

pub fn place_with_plan(
    behaviours: &Behaviours, level: &mut dyn LevelAccess, ticks: &mut Ticks,
    configuration: &Value, pos: [i32; 3], depth: u8, roots: Option<(&RootBindings, &mut RootCallback<'_>)>, plan: &[TreeStep], decorate: &mut PlannedTreeCallback<'_>,
) -> bool {
    place_planned_bound(behaviours, level, ticks, configuration, pos, depth, roots, Some(plan), decorate)
}

fn place_planned_bound(
    behaviours: &Behaviours, level: &mut dyn LevelAccess, ticks: &mut Ticks,
    configuration: &Value, pos: [i32; 3], depth: u8, roots: Option<(&RootBindings, &mut RootCallback<'_>)>, plan: Option<&[TreeStep]>, decorate: &mut PlannedTreeCallback<'_>,
) -> bool {
    if depth == 0 || depth > MAX_FEATURE_DEPTH { return false }
    let Some(features) = behaviours.live_features() else { return false };
    let mut sim = behaviours.sim(level, ticks);
    super::pluto_block_access::with_random(&mut sim, |sim| sim.random(), |sim, random| {
        let mut world = LevelFeatureWorld { sim, features, entities: Vec::new() };
        on_world(&mut world, random, configuration, pos, roots, plan, decorate)
    })
}
