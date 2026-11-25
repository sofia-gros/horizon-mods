//! Shared, host-independent world-fluid services.
//!
//! This first vertical slice supports finite downward flow into air, source
//! pickup and ordinary block-state persistence. It is not a Forge FluidType API.
pub mod contact;

use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    sync::{Arc, OnceLock, RwLock},
};

pub type Position = [i32; 3];
/// Level 0 is a source, 1..=7 horizontal flow, and 8 a falling column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cell {
    pub fluid: String,
    pub level: u8,
}
impl Cell {
    pub fn source(fluid: impl Into<String>) -> Self {
        Self {
            fluid: fluid.into(),
            level: 0,
        }
    }
    pub fn is_source(&self) -> bool {
        self.level == 0
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Sample {
    Air,
    Fluid(Cell),
    Solid,
    Unloaded,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Definition {
    pub id: String,
    pub block: String,
    pub bucket: String,
    pub texture: String,
    pub tick_delay: u16,
    pub drop_off: u8,
    pub light: u8,
}
fn identifier(id: &str) -> bool {
    let Some((ns, path)) = id.split_once(':') else {
        return false;
    };
    !ns.is_empty()
        && !path.is_empty()
        && ns != "minecraft"
        && ns
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.-".contains(&b))
        && path
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_./-".contains(&b))
        && !path
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
}
impl Definition {
    pub fn validate(&self) -> Result<(), String> {
        if ![&self.id, &self.block, &self.bucket, &self.texture]
            .into_iter()
            .all(|s| identifier(s))
        {
            return Err(
                "fluid, block, bucket and texture require safe non-vanilla namespaced IDs".into(),
            );
        }
        if self.block == self.bucket
            || !(1..=1200).contains(&self.tick_delay)
            || !(1..=7).contains(&self.drop_off)
            || self.light > 15
        {
            return Err("invalid fluid identities or bounded flow/render policy".into());
        }
        Ok(())
    }
    pub fn from_json(text: &str) -> Result<Self, String> {
        let v: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let object = v.as_object().ok_or("fluid definition must be an object")?;
        let keys = [
            "id",
            "block",
            "bucket",
            "texture",
            "tick_delay",
            "drop_off",
            "light",
        ];
        if object.len() != keys.len() || object.keys().any(|k| !keys.contains(&k.as_str())) {
            return Err("fluid definition has missing or unknown fields".into());
        }
        let string = |key| {
            v.get(key)
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| format!("{key} must be text"))
        };
        let number = |key| {
            v.get(key)
                .and_then(Value::as_u64)
                .ok_or_else(|| format!("{key} must be a nonnegative integer"))
        };
        let out = Self {
            id: string("id")?,
            block: string("block")?,
            bucket: string("bucket")?,
            texture: string("texture")?,
            tick_delay: u16::try_from(number("tick_delay")?).map_err(|_| "tick_delay overflow")?,
            drop_off: u8::try_from(number("drop_off")?).map_err(|_| "drop_off overflow")?,
            light: u8::try_from(number("light")?).map_err(|_| "light overflow")?,
        };
        out.validate()?;
        Ok(out)
    }
}
/// Opaque, monotonic validity of a captured definition snapshot. A successful
/// addition invalidates the old revision; immutable snapshot contents remain
/// readable. This is a point-in-time check, not a lock or rendering lease.
#[derive(Clone, Debug)]
pub struct RegistryRevision(Arc<std::sync::atomic::AtomicBool>);
impl RegistryRevision {
    fn new() -> Self {
        Self(Arc::new(std::sync::atomic::AtomicBool::new(true)))
    }
    pub fn is_current(&self) -> bool {
        self.0.load(std::sync::atomic::Ordering::Acquire) && !registry().is_poisoned()
    }
    fn invalidate(&self) {
        self.0.store(false, std::sync::atomic::Ordering::Release);
    }
}
struct Registry {
    definitions: BTreeMap<String, Arc<Definition>>,
    revision: RegistryRevision,
}
impl Default for Registry {
    fn default() -> Self {
        Self {
            definitions: BTreeMap::new(),
            revision: RegistryRevision::new(),
        }
    }
}
static REGISTRY: OnceLock<RwLock<Registry>> = OnceLock::new();
fn registry() -> &'static RwLock<Registry> {
    REGISTRY.get_or_init(Default::default)
}
/// Repeating the same declaration is harmless. Conflicting IDs never replace a
/// live definition. Call during content registration, identically on both sides.
pub fn register(definition: Definition) -> Result<(), String> {
    register_all([definition])
}

/// Register a finite batch as one atomic change. Identical duplicates are
/// idempotent, including within the batch. Invalid definitions or conflicting
/// fluid/block/bucket identities leave every existing registration unchanged.
/// Validation finishes before acquiring the registry write lock; the candidate
/// map is published only after all conflicts have been checked.
pub fn register_all(definitions: impl IntoIterator<Item = Definition>) -> Result<(), String> {
    let definitions: Vec<_> = definitions
        .into_iter()
        .map(|definition| {
            definition.validate()?;
            Ok(definition)
        })
        .collect::<Result<_, String>>()?;
    if definitions.is_empty() {
        return Ok(());
    }
    let mut all = registry().write().map_err(|_| "fluid registry poisoned")?;
    register_validated(&mut all, definitions)
}

fn register_validated(all: &mut Registry, definitions: Vec<Definition>) -> Result<(), String> {
    let mut candidate = all.definitions.clone();
    for definition in definitions {
        if let Some(old) = candidate.get(&definition.id) {
            if **old == definition {
                continue;
            }
            return Err("fluid already has another definition".into());
        }
        if candidate
            .values()
            .any(|d| d.block == definition.block || d.bucket == definition.bucket)
        {
            return Err("fluid block or bucket already belongs to another fluid".into());
        }
        candidate.insert(definition.id.clone(), Arc::new(definition));
    }
    if candidate.len() != all.definitions.len() {
        // All validation finished. Publish the map and new revision under the
        // same write lock; old readers can only observe monotonic invalidation.
        all.revision.invalidate();
        all.revision = RegistryRevision::new();
        all.definitions = candidate;
    }
    Ok(())
}

/// Immutable, process-local registry view. Later registration does not change
/// this view. It contains stable namespaced definitions, never host state IDs or
/// world data; it is not a save format or proof that host content was installed.
#[derive(Clone, Debug)]
pub struct RegistrySnapshot {
    revision: RegistryRevision,
    by_id: BTreeMap<String, Arc<Definition>>,
    by_block: BTreeMap<String, Arc<Definition>>,
    by_bucket: BTreeMap<String, Arc<Definition>>,
}
impl RegistrySnapshot {
    pub fn revision(&self) -> RegistryRevision {
        self.revision.clone()
    }
    pub fn get(&self, id: &str) -> Option<&Definition> {
        self.by_id.get(id).map(Arc::as_ref)
    }
    pub fn for_block(&self, id: &str) -> Option<&Definition> {
        self.by_block.get(id).map(Arc::as_ref)
    }
    pub fn for_bucket(&self, id: &str) -> Option<&Definition> {
        self.by_bucket.get(id).map(Arc::as_ref)
    }
    pub fn len(&self) -> usize {
        self.by_id.len()
    }
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
    /// Deterministic order by fluid ID, without retaining a global lock.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &Definition> + DoubleEndedIterator + '_ {
        self.by_id.values().map(Arc::as_ref)
    }
}

/// Capture one complete registry revision. All lookups in this snapshot refer
/// to that same set, even if another startup participant registers later.
pub fn registry_snapshot() -> Result<RegistrySnapshot, String> {
    let (by_id, revision) = {
        let registry = registry().read().map_err(|_| "fluid registry poisoned")?;
        (registry.definitions.clone(), registry.revision.clone())
    };
    let by_block = by_id
        .values()
        .map(|d| (d.block.clone(), Arc::clone(d)))
        .collect();
    let by_bucket = by_id
        .values()
        .map(|d| (d.bucket.clone(), Arc::clone(d)))
        .collect();
    Ok(RegistrySnapshot {
        revision,
        by_id,
        by_block,
        by_bucket,
    })
}

pub fn get(id: &str) -> Option<Arc<Definition>> {
    registry().read().ok()?.definitions.get(id).cloned()
}
pub fn for_block(id: &str) -> Option<Arc<Definition>> {
    registry()
        .read()
        .ok()?
        .definitions
        .values()
        .find(|d| d.block == id)
        .cloned()
}
pub fn for_bucket(id: &str) -> Option<Arc<Definition>> {
    registry()
        .read()
        .ok()?
        .definitions
        .values()
        .find(|d| d.bucket == id)
        .cloned()
}

/// The adapter owns bounds, loaded chunks, permissions and normal block-update
/// effects. A refused write leaves both the block and inventory unchanged.
pub trait World {
    fn sample(&self, pos: Position) -> Sample;
    fn replace(&mut self, pos: Position, expected: &Sample, cell: Option<&Cell>) -> bool;
    fn schedule(&mut self, pos: Position, delay: u16);
}
const HORIZONTAL: [Position; 4] = [[-1, 0, 0], [1, 0, 0], [0, 0, -1], [0, 0, 1]];
const NEIGHBORS: [Position; 6] = [
    [-1, 0, 0],
    [1, 0, 0],
    [0, 0, -1],
    [0, 0, 1],
    [0, -1, 0],
    [0, 1, 0],
];
fn offset(p: Position, d: Position) -> Option<Position> {
    Some([
        p[0].checked_add(d[0])?,
        p[1].checked_add(d[1])?,
        p[2].checked_add(d[2])?,
    ])
}
fn sample(world: &impl World, pos: Option<Position>) -> Sample {
    pos.map_or(Sample::Solid, |p| world.sample(p))
}
fn same_level(sample: &Sample, id: &str) -> Option<u8> {
    match sample {
        Sample::Fluid(c) if c.fluid == id && c.level <= 8 => Some(c.level),
        _ => None,
    }
}
fn changed(world: &mut impl World, p: Position, definition: &Definition) {
    world.schedule(p, definition.tick_delay);
    for d in NEIGHBORS {
        if let Some(p) = offset(p, d) {
            world.schedule(p, definition.tick_delay);
        }
    }
}
/// Place only into air. No source conversion, replacement of plants, foreign
/// fluids or waterlogging is implied by this initial API.
pub fn place_source(world: &mut impl World, pos: Position, definition: &Definition) -> bool {
    if world.sample(pos) != Sample::Air {
        return false;
    }
    if world.replace(pos, &Sample::Air, Some(&Cell::source(&definition.id))) {
        changed(world, pos, definition);
        true
    } else {
        false
    }
}
/// Remove exactly a source of this family; flowing cells cannot fill buckets.
pub fn pickup_source(world: &mut impl World, pos: Position, definition: &Definition) -> bool {
    let expected = Sample::Fluid(Cell::source(&definition.id));
    if world.sample(pos) != expected {
        return false;
    }
    if world.replace(pos, &expected, None) {
        changed(world, pos, definition);
        true
    } else {
        false
    }
}
/// One due scheduled BLOCK tick. The native scheduler persists the queue. Reads
/// at unloaded boundaries are conservative and retry; they never load chunks.
pub fn tick(world: &mut impl World, pos: Position, definition: &Definition) {
    let old = world.sample(pos);
    let Some(mut level) = same_level(&old, &definition.id) else {
        return;
    };
    if level != 0 {
        let above = sample(world, offset(pos, [0, 1, 0]));
        if above == Sample::Unloaded {
            world.schedule(pos, definition.tick_delay);
            return;
        }
        let next = if same_level(&above, &definition.id).is_some() {
            Some(8)
        } else {
            let mut best = None;
            for d in HORIZONTAL {
                let neighbor = sample(world, offset(pos, d));
                if neighbor == Sample::Unloaded {
                    world.schedule(pos, definition.tick_delay);
                    return;
                }
                if let Some(n) = same_level(&neighbor, &definition.id) {
                    let n = if n == 8 { 0 } else { n };
                    best = Some(best.map_or(n, |v: u8| v.min(n)));
                }
            }
            best.and_then(|v| v.checked_add(definition.drop_off))
                .filter(|v| *v < 8)
        };
        if next != Some(level) {
            let cell = next.map(|level| Cell {
                fluid: definition.id.clone(),
                level,
            });
            if !world.replace(pos, &old, cell.as_ref()) {
                return;
            }
            changed(world, pos, definition);
            let Some(next) = next else { return };
            level = next;
        }
    }
    let below_pos = offset(pos, [0, -1, 0]);
    let below = sample(world, below_pos);
    if below == Sample::Unloaded {
        world.schedule(pos, definition.tick_delay);
        return;
    }
    let down_level = same_level(&below, &definition.id);
    if below == Sample::Air || down_level.is_some_and(|l| l != 0) {
        if down_level != Some(8) {
            if let Some(below_pos) = below_pos {
                let falling = Cell {
                    fluid: definition.id.clone(),
                    level: 8,
                };
                if world.replace(below_pos, &below, Some(&falling)) {
                    changed(world, below_pos, definition);
                }
            }
        }
        return;
    }
    let next = if level == 8 {
        definition.drop_off
    } else {
        level.saturating_add(definition.drop_off)
    };
    if next >= 8 {
        return;
    }
    for d in HORIZONTAL {
        let Some(p) = offset(pos, d) else { continue };
        let old = world.sample(p);
        if old == Sample::Unloaded {
            world.schedule(pos, definition.tick_delay);
            continue;
        }
        if old == Sample::Air
            || same_level(&old, &definition.id).is_some_and(|l| l > next && l != 8)
        {
            let cell = Cell {
                fluid: definition.id.clone(),
                level: next,
            };
            if world.replace(p, &old, Some(&cell)) {
                changed(world, p, definition);
            }
        }
    }
}

/// Preparation-time declarations and intentionally stepped block models. The
/// consumer supplies its textures and bucket item model. No host FluidKind is
/// forged, and no engine renderer source is replaced.
pub fn content(def: &Definition) -> Result<BTreeMap<String, Value>, String> {
    def.validate()?;
    let (namespace, path) = def.block.split_once(':').unwrap();
    let mut out = BTreeMap::new();
    let mut shapes = serde_json::Map::new();
    let mut variants = serde_json::Map::new();
    for level in 0..=8 {
        let height = if level == 8 {
            16.0
        } else {
            f64::from(8 - level) * 16.0 / 9.0
        };
        // Only sources have an outline: empty buckets ray past flowing cells.
        shapes.insert(
            level.to_string(),
            if level == 0 {
                json!([[0, 0, 0, 16, height, 16]])
            } else {
                json!([])
            },
        );
        let model = format!("{namespace}:block/{path}/level_{level}");
        variants.insert(format!("level={level}"), json!({"model": model}));
        let faces: serde_json::Map<String, Value> =
            ["down", "up", "north", "south", "west", "east"]
                .into_iter()
                .map(|d| (d.into(), json!({"texture":"#fluid"})))
                .collect();
        out.insert(
            format!("assets/{namespace}/models/block/{path}/level_{level}.json"),
            json!({"ambientocclusion":false,"textures":{"fluid":def.texture,"particle":def.texture},
                   "elements":[{"from":[0,0,0],"to":[16,height,16],"faces":faces}]}),
        );
    }
    out.insert(
        format!("assets/{namespace}/blockstates/{path}.json"),
        json!({"variants": variants}),
    );
    // Glass has no vanilla liquid behavior. Explicit shape/occlusion overrides
    // make the cell noncolliding. Heightmaps/pathfinding are not fluid-aware yet.
    out.insert(format!("data/{namespace}/horizon/block/{path}.json"), json!({
        "template":"minecraft:glass", "item":false,
        "properties":{"hardness":-1.0,"explosion_resistance":100.0,"light":def.light,"occludes":false},
        "state_properties":{"level":{"type":"enum","values":["0","1","2","3","4","5","6","7","8"]}},
        "default_state":{"level":"0"},
        "shapes":{"property":"level","values":shapes},"collision_shapes":{"default":[]}
    }));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    fn def() -> Definition {
        Definition {
            id: "demo:fluid".into(),
            block: "demo:fluid".into(),
            bucket: "demo:bucket".into(),
            texture: "demo:block/fluid".into(),
            tick_delay: 5,
            drop_off: 1,
            light: 0,
        }
    }
    #[derive(Default)]
    struct Grid {
        cells: BTreeMap<Position, Cell>,
        blocked: BTreeSet<Position>,
        unloaded: BTreeSet<Position>,
        due: BTreeSet<Position>,
        deny: bool,
    }
    impl World for Grid {
        fn sample(&self, p: Position) -> Sample {
            if self.unloaded.contains(&p) {
                Sample::Unloaded
            } else if p[1] < 0 || self.blocked.contains(&p) {
                Sample::Solid
            } else {
                self.cells
                    .get(&p)
                    .cloned()
                    .map_or(Sample::Air, Sample::Fluid)
            }
        }
        fn replace(&mut self, p: Position, expected: &Sample, cell: Option<&Cell>) -> bool {
            if self.deny || &self.sample(p) != expected {
                return false;
            }
            match cell {
                Some(c) => {
                    self.cells.insert(p, c.clone());
                }
                None => {
                    self.cells.remove(&p);
                }
            }
            true
        }
        fn schedule(&mut self, p: Position, _: u16) {
            if self.cells.contains_key(&p) {
                self.due.insert(p);
            }
        }
    }
    fn run(g: &mut Grid, d: &Definition, n: usize) {
        for _ in 0..n {
            let due = std::mem::take(&mut g.due);
            for p in due {
                tick(g, p, d);
            }
        }
    }
    #[test]
    fn source_spreads_and_entire_pool_decays_after_pickup() {
        let d = def();
        let mut g = Grid::default();
        assert!(place_source(&mut g, [0, 0, 0], &d));
        run(&mut g, &d, 50);
        assert_eq!(g.cells[&[7, 0, 0]].level, 7);
        assert!(!g.cells.contains_key(&[8, 0, 0]));
        assert!(!pickup_source(&mut g, [1, 0, 0], &d));
        assert!(pickup_source(&mut g, [0, 0, 0], &d));
        run(&mut g, &d, 100);
        assert!(g.cells.is_empty());
    }
    #[test]
    fn falls_before_spreading_and_preserves_obstacles() {
        let d = def();
        let mut g = Grid::default();
        g.blocked.insert([1, 0, 0]);
        place_source(&mut g, [0, 3, 0], &d);
        run(&mut g, &d, 30);
        assert_eq!(g.cells[&[0, 0, 0]].level, 8);
        assert!(!g.cells.contains_key(&[1, 0, 0]));
        assert!(!g.cells.contains_key(&[1, 3, 0]));
        pickup_source(&mut g, [0, 3, 0], &d);
        run(&mut g, &d, 150);
        assert!(g.cells.is_empty());
    }
    #[test]
    fn missing_chunk_is_not_air_and_refused_write_is_noop() {
        let d = def();
        let mut g = Grid::default();
        g.unloaded.insert([0, 0, 0]);
        assert!(!place_source(&mut g, [0, 0, 0], &d));
        g.unloaded.clear();
        g.deny = true;
        assert!(!place_source(&mut g, [0, 0, 0], &d));
        assert!(g.cells.is_empty());
        g.deny = false;
        place_source(&mut g, [0, 1, 0], &d);
        g.unloaded.insert([0, 0, 0]);
        run(&mut g, &d, 5);
        assert_eq!(g.cells.len(), 1);
        assert!(g.due.contains(&[0, 1, 0]));
    }
    #[test]
    fn registry_is_idempotent_and_rejects_identity_replacement() {
        let d = def();
        register(d.clone()).unwrap();
        register(d.clone()).unwrap();
        assert_eq!(for_bucket(&d.bucket).unwrap().id, d.id);
        assert_eq!(for_block(&d.block).unwrap().id, d.id);
        let mut conflict = d.clone();
        conflict.tick_delay += 1;
        assert!(register(conflict).is_err());
        let mut collision = d.clone();
        collision.id = "other:fluid".into();
        assert!(register(collision).is_err());
        assert_eq!(*get(&d.id).unwrap(), d);
    }

    fn named_definition(name: &str) -> Definition {
        Definition {
            id: format!("registry_test:{name}"),
            block: format!("registry_test:{name}_fluid"),
            bucket: format!("registry_test:{name}_bucket"),
            texture: "registry_test:fluid".into(),
            ..def()
        }
    }

    #[test]
    fn batch_conflict_preserves_existing_and_does_not_publish_valid_prefix() {
        let existing = named_definition("rollback_existing");
        register(existing.clone()).unwrap();
        let first = named_definition("rollback_first");
        let last = named_definition("rollback_last");
        let mut conflict = existing.clone();
        conflict.light += 1;
        assert!(register_all([first.clone(), conflict, last.clone()]).is_err());
        let snapshot = registry_snapshot().unwrap();
        assert_eq!(snapshot.get(&existing.id), Some(&existing));
        assert!(snapshot.get(&first.id).is_none());
        assert!(snapshot.get(&last.id).is_none());
    }

    #[test]
    fn batch_rejects_internal_identity_collisions_and_invalid_members_atomically() {
        let first = named_definition("internal_first");
        let mut second = named_definition("internal_second");
        second.block = first.block.clone();
        assert!(register_all([first.clone(), second.clone()]).is_err());
        assert!(get(&first.id).is_none());
        assert!(get(&second.id).is_none());
        second.block = "registry_test:internal_second_fluid".into();
        second.bucket = first.bucket.clone();
        assert!(register_all([first.clone(), second.clone()]).is_err());
        assert!(get(&first.id).is_none());
        second.bucket = "registry_test:internal_second_bucket".into();
        second.light = 16;
        assert!(register_all([first.clone(), second.clone()]).is_err());
        assert!(get(&first.id).is_none());
        assert!(get(&second.id).is_none());
    }

    #[test]
    fn successful_batch_is_idempotent_and_snapshot_indexes_are_consistent() {
        let a = named_definition("batch_z");
        let b = named_definition("batch_a");
        register_all([a.clone(), b.clone(), a.clone()]).unwrap();
        register_all([b.clone(), a.clone()]).unwrap();
        register_all([]).unwrap();
        let snapshot = registry_snapshot().unwrap();
        assert!(!snapshot.is_empty());
        assert_eq!(snapshot.len(), snapshot.iter().len());
        let names: Vec<_> = snapshot.iter().map(|d| &d.id).collect();
        assert!(names.windows(2).all(|pair| pair[0] < pair[1]));
        for d in [a, b] {
            assert_eq!(snapshot.get(&d.id), Some(&d));
            assert_eq!(snapshot.for_block(&d.block), Some(&d));
            assert_eq!(snapshot.for_bucket(&d.bucket), Some(&d));
            assert!(std::ptr::eq(
                snapshot.get(&d.id).unwrap(),
                snapshot.for_bucket(&d.bucket).unwrap()
            ));
        }
    }

    #[test]
    fn captured_snapshots_do_not_change_with_later_registration() {
        let early = named_definition("snapshot_early");
        let late = named_definition("snapshot_late");
        register(early.clone()).unwrap();
        let before = registry_snapshot().unwrap();
        let copy = before.clone();
        register(late.clone()).unwrap();
        for snapshot in [&before, &copy] {
            assert_eq!(snapshot.get(&early.id), Some(&early));
            assert!(snapshot.get(&late.id).is_none());
            assert!(snapshot.for_block(&late.block).is_none());
            assert!(snapshot.for_bucket(&late.bucket).is_none());
        }
        assert_eq!(registry_snapshot().unwrap().get(&late.id), Some(&late));
    }

    #[test]
    fn concurrent_snapshot_reader_never_observes_part_of_a_batch() {
        use std::sync::{
            atomic::{AtomicBool, Ordering},
            Barrier,
        };
        let done = AtomicBool::new(false);
        let start = Barrier::new(2);
        let definitions: Vec<_> = (0..32)
            .map(|n| named_definition(&format!("concurrent_{n}")))
            .collect();
        let count = |view: &RegistrySnapshot| {
            view.iter()
                .filter(|d| d.id.starts_with("registry_test:concurrent_"))
                .count()
        };
        assert_eq!(count(&registry_snapshot().unwrap()), 0);
        std::thread::scope(|scope| {
            scope.spawn(|| {
                start.wait();
                register_all(definitions).unwrap();
                done.store(true, Ordering::Release);
            });
            start.wait();
            loop {
                let observed = count(&registry_snapshot().unwrap());
                assert!(observed == 0 || observed == 32);
                if done.load(Ordering::Acquire) {
                    break;
                }
                std::thread::yield_now();
            }
        });
        assert_eq!(count(&registry_snapshot().unwrap()), 32);
    }

    #[test]
    fn validates_ids_bounds_and_generated_levels() {
        let d = def();
        let files = content(&d).unwrap();
        assert_eq!(files.len(), 11);
        assert_eq!(
            files["data/demo/horizon/block/fluid.json"]["collision_shapes"]["default"],
            json!([])
        );
        let mut bad = d;
        bad.block = "demo:../escape".into();
        assert!(content(&bad).is_err());
    }
}

#[cfg(test)]
mod revision_tests {
    use super::*;
    fn definition(id: &str) -> Definition {
        Definition {
            id: format!("revision:{id}"),
            block: format!("revision:{id}_block"),
            bucket: format!("revision:{id}_bucket"),
            texture: format!("revision:{id}_texture"),
            tick_delay: 5,
            drop_off: 1,
            light: 0,
        }
    }
    #[test]
    fn successful_addition_invalidates_only_previous_revision_and_keeps_old_values() {
        let mut registry = Registry::default();
        let empty = registry.revision.clone();
        register_validated(&mut registry, vec![definition("first")]).unwrap();
        assert!(!empty.is_current());
        let old = registry.definitions.clone();
        let previous = registry.revision.clone();
        register_validated(&mut registry, vec![definition("second")]).unwrap();
        assert!(!previous.is_current());
        assert!(registry.revision.is_current());
        assert_eq!(old.len(), 1);
        assert_eq!(registry.definitions.len(), 2);
    }
    #[test]
    fn idempotent_empty_and_failed_batches_keep_the_same_valid_revision() {
        let mut registry = Registry::default();
        let first = definition("one");
        register_validated(&mut registry, vec![first.clone()]).unwrap();
        let revision = registry.revision.clone();
        register_validated(&mut registry, vec![]).unwrap();
        register_validated(&mut registry, vec![first.clone(), first.clone()]).unwrap();
        let mut conflict = first.clone();
        conflict.light = 1;
        assert!(register_validated(&mut registry, vec![definition("two"), conflict]).is_err());
        assert!(revision.is_current());
        assert!(Arc::ptr_eq(&revision.0, &registry.revision.0));
        assert_eq!(registry.definitions.len(), 1);
        let mut invalid = first;
        invalid.drop_off = 0;
        assert!(invalid.validate().is_err());
        assert!(revision.is_current());
    }
}
