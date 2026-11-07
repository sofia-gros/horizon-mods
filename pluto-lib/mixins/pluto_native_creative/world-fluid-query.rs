// Read-only official-host view. No ticks, writes or actor/effect policy.
use horizon_client::world::World as NativeWorld;
use horizon_world::{level::Behaviours, registry::{BlockRegistry, BlockStateId}};
use pluto_api::fluid::{self, contact, Cell, Definition, Position, Sample};
use std::sync::Arc;

pub(super) fn cell(registry: &BlockRegistry, state: BlockStateId) -> Option<(Arc<Definition>, Cell)> {
    let state = registry.state(state)?;
    let def = fluid::for_block(&state.name)?;
    let value = state
        .properties
        .get("level")?
        .parse::<u8>()
        .ok()
        .filter(|n| *n <= 8)?;
    let c = Cell {
        fluid: def.id.clone(),
        level: value,
    };
    Some((def, c))
}
/// All three references must come from the same host world/resource context.
/// Definitions retain the existing per-sample registry lookup semantics.
pub(super) struct View<'a> {
    world: &'a NativeWorld,
    registry: &'a BlockRegistry,
    behaviours: &'a Behaviours,
}
impl<'a> View<'a> {
    pub(super) fn new(world: &'a NativeWorld, registry: &'a BlockRegistry, behaviours: &'a Behaviours) -> Self {
        Self { world, registry, behaviours }
    }
    pub(super) fn sample(&self, p: Position) -> Sample {
        if p[1] < self.world.shape.min_y || p[1] >= self.world.shape.max_y() {
            return Sample::Solid;
        }
        if !self.world.loaded(p[0], p[2]) {
            return Sample::Unloaded;
        }
        let state = self.world.block(p[0], p[1], p[2]);
        if self.behaviours.is_air(state) {
            return Sample::Air;
        }
        cell(self.registry, state).map_or(Sample::Solid, |(_, c)| Sample::Fluid(c))
    }
    pub(super) fn query_point(&self, point: [f64; 3]) -> Result<contact::PointContact, contact::QueryError> {
        contact::query_point_with(point, |p| self.sample(p))
    }
    pub(super) fn query_box(&self, bounds: contact::Bounds, limits: contact::Limits) -> Result<contact::Contacts, contact::QueryError> {
        contact::query_box_with(bounds, limits, |p| self.sample(p))
    }
    /// Current stepped-cell surface facts, not a source engine's cached fluid heights.
    pub(super) fn query_heights(&self, bounds: contact::Bounds, limits: contact::Limits)
        -> Result<contact::immersion::Heights, contact::immersion::Error> {
        contact::immersion::query_with(bounds, limits, |p|
            contact::immersion::stepped_surface(p, self.sample(p)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_world::{chunk::BlockState, nbt::Tag};
    use std::collections::BTreeMap;

    fn definition(name: &str) -> Definition {
        Definition {
            id: format!("pluto_query_fixture:{name}"),
            block: format!("pluto_query_fixture:{name}_block"),
            bucket: format!("pluto_query_fixture:{name}_bucket"),
            texture: format!("pluto_query_fixture:{name}_texture"),
            tick_delay: 5, drop_off: 1, light: 0,
        }
    }
    fn registry(entries: &[(&str, Option<&str>)]) -> BlockRegistry {
        let mut defaults = BTreeMap::new();
        let states = entries.iter().enumerate().map(|(i, (name, level))| {
            defaults.entry((*name).into()).or_insert(Tag::Int(i as i32));
            let mut state = BlockState { name: (*name).into(), properties: Default::default() };
            if let Some(level) = level { state.properties.insert("level".into(), (*level).into()); }
            state.to_nbt()
        }).collect();
        BlockRegistry::from_nbt(&Tag::Compound([
            ("DataVersion".into(), Tag::Int(horizon_world::DATA_VERSION)),
            ("states".into(), Tag::List(states)),
            ("defaults".into(), Tag::Compound(defaults)),
        ].into_iter().collect())).unwrap()
    }

    #[test]
    fn actual_registry_retains_all_levels_and_existing_lexical_parse() {
        let d = definition("levels"); fluid::register(d.clone()).unwrap();
        let values = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "+1", "01"];
        let entries: Vec<_> = values.iter().map(|v| (d.block.as_str(), Some(*v))).collect();
        let registry = registry(&entries);
        for (i, value) in values.iter().enumerate() {
            let (actual_definition, cell) = cell(&registry, BlockStateId(i as u32)).unwrap();
            assert_eq!(*actual_definition, d);
            assert_eq!(cell, Cell { fluid: d.id.clone(), level: value.parse().unwrap() });
        }
    }

    #[test]
    fn actual_registry_missing_malformed_unknown_and_invalid_ids_stay_unrecognized() {
        let d = definition("invalid"); fluid::register(d.clone()).unwrap();
        let values = [None, Some("9"), Some("-1"), Some("256"), Some("water")];
        let mut entries: Vec<_> = values.iter().map(|v| (d.block.as_str(), *v)).collect();
        entries.push(("minecraft:water", Some("0")));
        let registry = registry(&entries);
        for i in 0..entries.len() { assert!(cell(&registry, BlockStateId(i as u32)).is_none()); }
        assert!(cell(&registry, BlockStateId(u32::MAX)).is_none());
    }

    #[test]
    fn per_sample_registration_visibility_is_not_silently_snapshotted() {
        let d = definition("later_registration");
        let registry = registry(&[(d.block.as_str(), Some("0"))]);
        assert!(cell(&registry, BlockStateId(0)).is_none());
        fluid::register(d.clone()).unwrap();
        assert_eq!(cell(&registry, BlockStateId(0)).unwrap().1, Cell::source(d.id));
    }
}
