// Trace-only host double: executes the production adapter, not a world generator.
// It checks saved-source routing and candidate ownership, not gameplay parity.
extern crate self as sunrise_core;
use std::{collections::BTreeMap, sync::{Arc, Mutex, OnceLock, TryLockError}};

pub mod random {
    pub struct LegacyRandom;
    impl LegacyRandom { pub fn new(_: i64) -> Self { Self } }
    pub trait RandomSource { fn next_int_bound(&mut self, _: i32) -> Result<i32, ()>; }
    pub struct WorldgenRandom;
    impl WorldgenRandom {
        pub fn new(_: LegacyRandom) -> Self { Self }
        pub fn set_large_feature_seed(&mut self, _: i64, _: i32, _: i32) {}
    }
    impl RandomSource for WorldgenRandom {
        fn next_int_bound(&mut self, _: i32) -> Result<i32, ()> { Ok(0) }
    }
}
pub mod level { pub mod pluto_structure_access {
    pub fn is_registered(kind: &str) -> Result<bool, String> { Ok(kind == "test:custom") }
} }
pub mod worldgen { pub mod structure { pub mod start {
    pub enum StructureKind { Unsupported(String), Native }
} } }
use worldgen::structure::start::StructureKind;
#[derive(Clone, Copy)]
struct ChunkPos { x: i32, z: i32 }
#[derive(Clone, Debug)]
struct StructureStart { structure: usize, references: i32 }
type Starts = Arc<Vec<Arc<StructureStart>>>;
struct Def { name: String, kind: StructureKind }
struct Set { name: String, structures: Vec<(usize, i32)> }
struct Registry { structures: Vec<Def>, sets: Vec<Set> }
impl Registry {
    fn is_structure_chunk(&self, _: &Set, _: i64, _: i32, _: i32, _: &Rings) -> bool { true }
}
struct Rings;
#[derive(Default)]
struct Cache(Mutex<Option<Starts>>);
impl Cache {
    fn get(&self, _: (i32, i32)) -> Option<Starts> { self.0.lock().unwrap().clone() }
    fn insert_first(&self, _: (i32, i32), value: Starts) -> Starts {
        self.0.lock().unwrap().get_or_insert(value).clone()
    }
}
#[derive(Default)]
struct Work(Mutex<()>);
impl Work { fn at(&self, _: (i32, i32)) -> &Mutex<()> { &self.0 } }
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq)]
struct Name(String);
impl Name { fn to_string_lossy(&self) -> String { self.0.clone() } }
#[derive(Clone)]
struct Saved { valid: bool, references: i32 }
type Source = dyn Fn(ChunkPos) -> Option<BTreeMap<Name, Saved>> + Send + Sync;
struct StructureState {
    registry: Registry, resources: (), bindings: (), possible_sets: Vec<usize>,
    starts: Cache, start_work: Work, saved: OnceLock<Arc<Source>>,
}
impl StructureState {
    fn rings_for(&self, _: &mut impl StartContext, _: ChunkPos) -> &Rings { &Rings }
    fn saved_starts_at(&self, _: &mut impl StartContext, _: ChunkPos) -> Starts { Arc::new(vec![]) }
}
trait StartContext {
    fn seed(&self) -> i64 { 7 }
    fn saved_starts(&self, _: ChunkPos) -> Vec<()> { vec![] }
    fn is_old_chunk(&self, _: ChunkPos) -> bool { false }
    fn generated(&mut self, index: usize);
}
#[derive(Default)]
struct Context(Vec<usize>);
impl StartContext for Context { fn generated(&mut self, index: usize) { self.0.push(index); } }
fn try_generate(_: &Registry, _: &(), _: &(), c: &mut impl StartContext, index: usize, _: ChunkPos, _: bool) -> Option<StructureStart> {
    c.generated(index);
    Some(StructureStart { structure: index, references: 0 })
}
fn create_starts(r: &Registry, a: &(), b: &(), c: &mut impl StartContext, sets: &[usize], _: &Rings, p: ChunkPos) -> Vec<StructureStart> {
    sets.iter().filter_map(|i| try_generate(r, a, b, c, r.sets[*i].structures[0].0, p, false)).collect()
}
fn read_saved_start(index: usize, _: &Def, _: ChunkPos, saved: &Saved, _: &(), _: &()) -> Option<StructureStart> {
    saved.valid.then_some(StructureStart { structure: index, references: saved.references })
}
mod pluto_structure_context {
    use super::*;
    pub fn natural_start(_: &StructureState, _: &mut impl StartContext, _: &str, _: ChunkPos) -> Result<Option<Option<StructureStart>>, String> { Ok(None) }
    pub fn natural_set_supported(_: &StructureState, _: &str) -> Result<bool, String> { Ok(true) }
}
mod production { include!(PRODUCTION_HELPER); }
fn state(saved: Vec<(&str, bool, i32)>) -> StructureState {
    let state = StructureState {
        registry: Registry {
            structures: vec![
                Def { name: "test:native".into(), kind: StructureKind::Native },
                Def { name: "test:custom".into(), kind: StructureKind::Unsupported("test:custom".into()) },
                Def { name: "test:orphan".into(), kind: StructureKind::Native },
            ],
            sets: vec![Set { name: "test:mixed".into(), structures: vec![(0, 1), (1, 1)] }],
        }, resources: (), bindings: (), possible_sets: vec![0], starts: Cache::default(), start_work: Work::default(), saved: OnceLock::new(),
    };
    let rows: BTreeMap<_, _> = saved.into_iter().map(|(name, valid, references)| (Name(name.into()), Saved { valid, references })).collect();
    state.saved.set(Arc::new(move |_| Some(rows.clone()))).ok().unwrap();
    state
}
#[test]
fn current_chunk_saved_start_wins_without_regeneration_and_keeps_references() {
    let state = state(vec![("test:native", true, 7), ("test:orphan", true, 3)]);
    let mut context = Context::default();
    let std::ops::ControlFlow::Break(Some(starts)) = production::build(&state, &mut context, ChunkPos { x: 2, z: -3 }, true) else { panic!("adapter did not handle registered types") };
    assert!(context.0.is_empty(), "saved starts must not call any generation attempt: {:?}", context.0);
    assert_eq!(starts.iter().map(|s| (s.structure, s.references)).collect::<Vec<_>>(), vec![(0, 7), (2, 3)]);
}
#[test]
fn unreadable_saved_start_falls_back_to_generation() {
    let state = state(vec![("test:native", false, 7)]);
    let mut context = Context::default();
    let std::ops::ControlFlow::Break(Some(starts)) = production::build(&state, &mut context, ChunkPos { x: 2, z: -3 }, true) else { panic!("no result") };
    assert_eq!(context.0, vec![0]);
    assert_eq!(starts[0].references, 0);
}
