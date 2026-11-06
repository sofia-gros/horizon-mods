// One startup registry of pure cell geometry policies. This is not a content,
// physics, texture or fluid registry; only prepared binding groups are eligible.
use super::pluto_terrain_bindings::Binding;
use super::pluto_terrain_cell_geometry::Batch;
use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

pub const MAX_PROVIDERS: usize = 64;
pub const MAX_GROUP_BYTES: usize = 256;
pub const MAX_SECTION_QUADS: usize = 8192;
pub const MAX_SECTION_VERTICES: usize = 262_144;

/// The already prepared render neighborhood, not a loaded-world capability.
/// Offsets outside the immediate 3x3x3 neighborhood return None. A missing
/// binding means only that the render state has no entry in this catalog.
pub trait Neighborhood {
    fn binding(&self, offset: [i8; 3]) -> Option<&Binding>;
}
pub struct CellContext<'a> {
    pub binding: &'a Binding,
    pub world: [i32; 3],
    pub neighborhood: &'a dyn Neighborhood,
}
/// Must be pure, bounded and safe on meshing worker threads. Return None to keep
/// the native model. A returned Batch uses cell-local coordinates [0,1], not
/// section coordinates. Empty geometry explicitly suppresses an eligible model.
/// No shader/material/light, mutation, send, asset load or retained host handle
/// capability is provided. Arbitrary callback effects cannot be rolled back.
pub type Provider = for<'a> fn(&CellContext<'a>) -> Option<Batch>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegisterError {
    InvalidGroup,
    Duplicate,
    Limit,
    Frozen,
    Poisoned,
}
struct Pending {
    frozen: bool,
    providers: BTreeMap<String, Provider>,
}
struct Registry {
    pending: Mutex<Pending>,
    frozen: OnceLock<Result<BTreeMap<String, Provider>, RegisterError>>,
}
impl Registry {
    const fn new() -> Self {
        Self {
            pending: Mutex::new(Pending {
                frozen: false,
                providers: BTreeMap::new(),
            }),
            frozen: OnceLock::new(),
        }
    }
    fn register(&self, group: &str, provider: Provider) -> Result<(), RegisterError> {
        if group.is_empty() || group.len() > MAX_GROUP_BYTES || group.chars().any(char::is_control)
        {
            return Err(RegisterError::InvalidGroup);
        }
        let mut pending = self.pending.lock().map_err(|_| RegisterError::Poisoned)?;
        if pending.frozen {
            return Err(RegisterError::Frozen);
        }
        if pending.providers.contains_key(group) {
            return Err(RegisterError::Duplicate);
        }
        if pending.providers.len() >= MAX_PROVIDERS {
            return Err(RegisterError::Limit);
        }
        pending.providers.insert(group.to_owned(), provider);
        Ok(())
    }
    fn snapshot(&self) -> Option<&BTreeMap<String, Provider>> {
        self.frozen
            .get_or_init(|| {
                let mut pending = self.pending.lock().map_err(|_| RegisterError::Poisoned)?;
                pending.frozen = true;
                Ok(std::mem::take(&mut pending.providers))
            })
            .as_ref()
            .ok()
    }
}
static REGISTRY: Registry = Registry::new();
/// Register before the FIRST mesh_reusing call, even when no provider was
/// previously registered. Late changes are rejected rather than silently mixing
/// old cached meshes and new policy. There is intentionally no runtime removal.
pub fn register(group: &str, provider: Provider) -> Result<(), RegisterError> {
    REGISTRY.register(group, provider)
}
pub(super) fn snapshot() -> Option<&'static BTreeMap<String, Provider>> {
    REGISTRY.snapshot()
}
pub(super) fn cell_local(batch: &Batch) -> bool {
    batch.as_slice().iter().all(|quad| {
        quad.vertices.iter().all(|vertex| {
            vertex
                .position
                .into_iter()
                .chain(vertex.uv)
                .all(|value| value.is_finite() && (0.0..=1.0).contains(&value))
        })
    })
}
#[derive(Default)]
pub(super) struct Budget {
    quads: usize,
    vertices: usize,
}
impl Budget {
    pub(super) fn quads(&mut self, count: usize) -> bool {
        match self.quads.checked_add(count) {
            Some(n) if n <= MAX_SECTION_QUADS => {
                self.quads = n;
                true
            }
            _ => false,
        }
    }
    pub(super) fn vertices(&mut self, count: usize) -> bool {
        match self.vertices.checked_add(count) {
            Some(n) if n <= MAX_SECTION_VERTICES => {
                self.vertices = n;
                true
            }
            _ => false,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn native(_: &CellContext<'_>) -> Option<Batch> {
        None
    }
    #[test]
    fn first_empty_snapshot_freezes_registration() {
        let r = Registry::new();
        assert!(r.snapshot().unwrap().is_empty());
        assert_eq!(r.register("demo", native), Err(RegisterError::Frozen));
    }
    #[test]
    fn duplicates_bounds_and_freeze_are_atomic() {
        let r = Registry::new();
        assert_eq!(r.register("", native), Err(RegisterError::InvalidGroup));
        assert_eq!(r.register("a", native), Ok(()));
        assert_eq!(r.register("a", native), Err(RegisterError::Duplicate));
        for i in 1..MAX_PROVIDERS {
            r.register(&i.to_string(), native).unwrap();
        }
        assert_eq!(r.register("overflow", native), Err(RegisterError::Limit));
        assert_eq!(r.snapshot().unwrap().len(), MAX_PROVIDERS);
        assert_eq!(r.register("later", native), Err(RegisterError::Frozen));
    }
    #[test]
    fn poison_cannot_publish_a_partial_registry() {
        let r = Registry::new();
        let _ = std::panic::catch_unwind(|| {
            let _lock = r.pending.lock().unwrap();
            panic!("poison");
        });
        assert!(r.snapshot().is_none());
        assert_eq!(r.register("x", native), Err(RegisterError::Poisoned));
    }
    #[test]
    fn section_relative_geometry_is_not_admitted_as_cell_local() {
        use super::super::pluto_terrain_cell_geometry::{
            self as geometry, Cell, Heights, Surface, TopDiagonal,
        };
        let mut cell = Cell {
            origin: [0; 3],
            heights: Heights {
                north_west: 0.75,
                north_east: 0.75,
                south_west: 0.75,
                south_east: 0.75,
            },
            bottom: 0.0,
            side_inset: 0.0,
            top_diagonal: TopDiagonal::NorthWestSouthEast,
            top: Some(Surface {
                uv: [[0.0, 0.0]; 4],
                back: true,
            }),
            down: None,
            sides: [None; 4],
        };
        assert!(cell_local(&geometry::build(&cell).unwrap()));
        cell.origin = [1, 0, 0];
        assert!(!cell_local(&geometry::build(&cell).unwrap()));
    }
    #[test]
    fn output_budget_does_not_wrap_or_mutate_on_failure() {
        let mut b = Budget::default();
        assert!(b.quads(MAX_SECTION_QUADS));
        assert!(!b.quads(1));
        assert!(!b.quads(usize::MAX));
        assert!(b.quads(0));
        assert!(b.vertices(MAX_SECTION_VERTICES));
        assert!(!b.vertices(1));
        assert!(!b.vertices(usize::MAX));
        assert!(b.vertices(0));
    }
}
