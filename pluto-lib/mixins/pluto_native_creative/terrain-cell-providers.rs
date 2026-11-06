// One startup registry of pure cell geometry policies. This is not a content,
// physics, texture or fluid registry; only prepared binding groups are eligible.
use super::pluto_terrain_bindings::Binding;
use super::pluto_terrain_cell_geometry::Batch;
use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

// Startup-only bounded map: enough for the complete 70-family consumer set
// and other mods, without changing per-section or per-cell output limits.
pub const MAX_PROVIDERS: usize = 256;
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
pub enum MaterialSlot {
    Primary,
    Secondary,
}
/// Fixed-size geometry plus one prepared-material choice per quad. No raw
/// sprite, alpha flag, render pass, color or native face can be supplied here.
pub struct PaintedBatch {
    geometry: Batch,
    slots: [MaterialSlot; 12],
}
impl PaintedBatch {
    pub fn new(geometry: Batch, slot: MaterialSlot) -> Self {
        Self {
            geometry,
            slots: [slot; 12],
        }
    }
    pub fn paint_face(
        &mut self,
        face: super::pluto_terrain_cell_geometry::Face,
        slot: MaterialSlot,
    ) {
        for (i, q) in self.geometry.as_slice().iter().enumerate() {
            if q.face == face {
                self.slots[i] = slot;
            }
        }
    }
    pub fn geometry(&self) -> &Batch {
        &self.geometry
    }
    pub fn slot(&self, index: usize) -> Option<MaterialSlot> {
        (index < self.geometry.as_slice().len()).then(|| self.slots[index])
    }
}
pub struct MaterialContext<'a> {
    pub cell: CellContext<'a>,
    pub appearance: &'a super::pluto_terrain_bindings::Appearance,
}
pub type MaterialProvider = for<'a> fn(&MaterialContext<'a>) -> Option<PaintedBatch>;
#[derive(Clone, Copy)]
pub(super) enum RegisteredProvider {
    Geometry(Provider),
    Material(MaterialProvider),
}
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
    providers: BTreeMap<String, RegisteredProvider>,
}
struct Registry {
    pending: Mutex<Pending>,
    frozen: OnceLock<Result<BTreeMap<String, RegisteredProvider>, RegisterError>>,
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
        self.register_all(&[(group, provider)])
    }
    fn register_all(&self, entries: &[(&str, Provider)]) -> Result<(), RegisterError> {
        if entries.len() > MAX_PROVIDERS {
            return Err(RegisterError::Limit);
        }
        let entries: Vec<_> = entries
            .iter()
            .map(|&(g, p)| (g, RegisteredProvider::Geometry(p)))
            .collect();
        self.register_entries(&entries)
    }
    fn register_entries(
        &self,
        entries: &[(&str, RegisteredProvider)],
    ) -> Result<(), RegisterError> {
        if entries.len() > MAX_PROVIDERS {
            return Err(RegisterError::Limit);
        }
        let mut staged = BTreeMap::new();
        for &(group, provider) in entries {
            if group.is_empty()
                || group.len() > MAX_GROUP_BYTES
                || group.chars().any(char::is_control)
            {
                return Err(RegisterError::InvalidGroup);
            }
            if staged.insert(group.to_owned(), provider).is_some() {
                return Err(RegisterError::Duplicate);
            }
        }
        let mut pending = self.pending.lock().map_err(|_| RegisterError::Poisoned)?;
        if pending.frozen {
            return Err(RegisterError::Frozen);
        }
        if staged
            .keys()
            .any(|group| pending.providers.contains_key(group))
        {
            return Err(RegisterError::Duplicate);
        }
        if pending
            .providers
            .len()
            .checked_add(staged.len())
            .is_none_or(|n| n > MAX_PROVIDERS)
        {
            return Err(RegisterError::Limit);
        }
        // All checks completed before publishing any member of the batch.
        pending.providers.extend(staged);
        Ok(())
    }
    fn snapshot(&self) -> Option<&BTreeMap<String, RegisteredProvider>> {
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
/// Register a bounded set atomically. Reusing a callback for different groups
/// is supported; duplicate groups, capacity errors or a frozen registry publish
/// nothing. Empty batches do not reopen a frozen registry.
pub fn register_all(entries: &[(&str, Provider)]) -> Result<(), RegisterError> {
    REGISTRY.register_all(entries)
}
/// Register material-aware callbacks in the same frozen registry and budget.
/// Geometry-only and material-aware registrations cannot own the same group.
pub fn register_materials(entries: &[(&str, MaterialProvider)]) -> Result<(), RegisterError> {
    if entries.len() > MAX_PROVIDERS {
        return Err(RegisterError::Limit);
    }
    let entries: Vec<_> = entries
        .iter()
        .map(|&(g, p)| (g, RegisteredProvider::Material(p)))
        .collect();
    REGISTRY.register_entries(&entries)
}
pub(super) fn snapshot() -> Option<&'static BTreeMap<String, RegisteredProvider>> {
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
pub(super) fn tint_color(original: Option<i32>, tint: u32) -> Option<i32> {
    if tint == 0xFF_FFFF {
        return original;
    }
    let original = original.map(|v| v as u32).unwrap_or(0xFFFF_FFFF);
    let c = |shift| ((original >> shift) & 255) * ((tint >> shift) & 255) / 255;
    Some(((original & 0xFF00_0000) | (c(16) << 16) | (c(8) << 8) | c(0)) as i32)
}
// Baked opacity proves only the original UV rectangle (in every sprite frame).
// Reusing it for a larger rectangle would silently skip alpha testing. No
// epsilon expansion is safe: the proof is an exact conservative subset check.
pub(super) fn opacity_allows(
    opaque: bool,
    explicit_pass: bool,
    source: &[[f32; 2]; 4],
    target: &[[f32; 2]; 4],
) -> bool {
    !opaque || explicit_pass || uv_subset(source, target)
}
pub(super) fn uv_subset(source: &[[f32; 2]; 4], target: &[[f32; 2]; 4]) -> bool {
    if source
        .iter()
        .chain(target)
        .flatten()
        .any(|v| !v.is_finite())
    {
        return false;
    }
    (0..2).all(|axis| {
        let lo = source.iter().map(|p| p[axis]).fold(f32::INFINITY, f32::min);
        let hi = source
            .iter()
            .map(|p| p[axis])
            .fold(f32::NEG_INFINITY, f32::max);
        target.iter().all(|p| p[axis] >= lo && p[axis] <= hi)
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
    #[test]
    fn seventy_groups_share_one_function_without_exceeding_budget() {
        let r = Registry::new();
        let names: Vec<_> = (0..70).map(|i| format!("family:{i}")).collect();
        let entries: Vec<(&str, Provider)> = names
            .iter()
            .map(|s| (s.as_str(), native as Provider))
            .collect();
        r.register_all(&entries).unwrap();
        assert_eq!(r.snapshot().unwrap().len(), 70);
        assert_eq!(r.register_all(&[]), Err(RegisterError::Frozen));
    }
    #[test]
    fn batch_duplicates_and_invalid_groups_publish_nothing() {
        let r = Registry::new();
        r.register("existing", native).unwrap();
        for (entries, error) in [
            (
                vec![("new", native as Provider), ("existing", native)],
                RegisterError::Duplicate,
            ),
            (
                vec![("new", native as Provider), ("new", native)],
                RegisterError::Duplicate,
            ),
            (
                vec![("new", native as Provider), ("", native)],
                RegisterError::InvalidGroup,
            ),
        ] {
            assert_eq!(r.register_all(&entries), Err(error));
        }
        assert_eq!(
            r.snapshot()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            vec!["existing"]
        );
    }
    #[test]
    fn exact_capacity_and_overflow_are_atomic() {
        let r = Registry::new();
        let names: Vec<_> = (0..=MAX_PROVIDERS).map(|i| format!("budget:{i}")).collect();
        let entries: Vec<(&str, Provider)> = names
            .iter()
            .map(|s| (s.as_str(), native as Provider))
            .collect();
        assert_eq!(r.register_all(&entries), Err(RegisterError::Limit));
        r.register_all(&entries[..MAX_PROVIDERS]).unwrap();
        assert_eq!(r.register("overflow", native), Err(RegisterError::Limit));
        assert_eq!(r.snapshot().unwrap().len(), MAX_PROVIDERS);
    }
    #[test]
    fn opacity_uv_envelope_handles_reverse_degenerate_and_nonfinite() {
        let source = [[0.75, 0.75], [0.25, 0.75], [0.25, 0.25], [0.75, 0.25]];
        assert!(opacity_allows(false, false, &source, &[[0.0, 1.0]; 4]));
        assert!(opacity_allows(true, true, &source, &[[0.0, 1.0]; 4]));
        assert!(!opacity_allows(true, false, &source, &[[0.0, 1.0]; 4]));
        assert!(uv_subset(&source, &source));
        assert!(uv_subset(&source, &[[0.5, 0.5]; 4]));
        assert!(!uv_subset(&source, &[[0.0, 0.5]; 4]));
        assert!(!uv_subset(
            &source,
            &[[f32::from_bits(0.75f32.to_bits() + 1), 0.5]; 4]
        ));
        assert!(uv_subset(&[[0.5, 0.5]; 4], &[[0.5, 0.5]; 4]));
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(!uv_subset(&source, &[[value, 0.5]; 4]));
            assert!(!uv_subset(&[[value, 0.5]; 4], &source));
        }
    }
    fn material_native(_: &MaterialContext<'_>) -> Option<PaintedBatch> {
        None
    }
    #[test]
    fn geometry_and_material_callbacks_share_capacity_identity_and_freeze() {
        let r = Registry::new();
        r.register("same", native).unwrap();
        assert_eq!(
            r.register_entries(&[
                ("new", RegisteredProvider::Material(material_native)),
                ("same", RegisteredProvider::Material(material_native))
            ]),
            Err(RegisterError::Duplicate)
        );
        r.register_entries(&[("material", RegisteredProvider::Material(material_native))])
            .unwrap();
        let map = r.snapshot().unwrap();
        assert_eq!(map.len(), 2);
        assert!(matches!(
            map.get("same"),
            Some(RegisteredProvider::Geometry(_))
        ));
        assert!(matches!(
            map.get("material"),
            Some(RegisteredProvider::Material(_))
        ));
        assert_eq!(
            r.register_entries(&[("late", RegisteredProvider::Material(material_native))]),
            Err(RegisterError::Frozen)
        );
    }
    #[test]
    fn tint_keeps_white_identity_and_multiplies_rgb_without_losing_alpha() {
        assert_eq!(tint_color(None, 0xFF_FFFF), None);
        assert_eq!(tint_color(Some(0x12804020), 0xFF_FFFF), Some(0x12804020));
        assert_eq!(tint_color(None, 0x123456), Some(0xFF123456u32 as i32));
        assert_eq!(tint_color(Some(0x12804020), 0x808080), Some(0x12402010));
        assert_eq!(tint_color(Some(0x12804020), 0), Some(0x12000000));
    }
    #[test]
    fn painting_preserves_fixed_geometry_and_paired_back_selection() {
        use super::super::pluto_terrain_cell_geometry::{
            self as g, Cell, Face, Heights, Surface, TopDiagonal,
        };
        let surface = Some(Surface {
            uv: [[0.0, 0.0]; 4],
            back: true,
        });
        let cell = Cell {
            origin: [0; 3],
            heights: Heights {
                north_west: 0.5,
                north_east: 0.5,
                south_west: 0.5,
                south_east: 0.5,
            },
            bottom: 0.0,
            side_inset: 0.0,
            top_diagonal: TopDiagonal::NorthWestSouthEast,
            top: surface,
            down: None,
            sides: [surface; 4],
        };
        let batch = g::build(&cell).unwrap();
        let mut painted = PaintedBatch::new(batch.clone(), MaterialSlot::Primary);
        painted.paint_face(Face::North, MaterialSlot::Secondary);
        assert_eq!(painted.geometry(), &batch);
        for (i, q) in batch.as_slice().iter().enumerate() {
            assert_eq!(
                painted.slot(i),
                Some(if q.face == Face::North {
                    MaterialSlot::Secondary
                } else {
                    MaterialSlot::Primary
                })
            );
        }
        assert_eq!(painted.slot(batch.as_slice().len()), None);
        assert_eq!(painted.slot(usize::MAX), None);
    }
}
