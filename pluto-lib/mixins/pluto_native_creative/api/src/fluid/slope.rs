//! Optional nearest-drop routing for the existing finite, air-only fluid kernel.
//! Native wall shapes, source conversion, mixing and actor physics are not implied.
use super::{Definition, HORIZONTAL, Position, Sample, World, offset, same_level, sample};

/// Maximum horizontal steps after the first adjacent cell (native convention).
/// Four means at most five cells from the origin. No implicit water policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlopeDistance(u8);
impl SlopeDistance {
    pub fn new(distance: u8) -> Result<Self, String> {
        if !(1..=4).contains(&distance) {
            return Err("slope distance must be in 1..=4".into());
        }
        Ok(Self(distance))
    }
    pub fn get(self) -> u8 { self.0 }
}

const NO_DROP: u8 = u8::MAX;

fn passable(value: &Sample, id: &str) -> bool {
    value == &Sample::Air || same_level(value, id).is_some_and(|level| level != 0)
}
fn read(world: &impl World, at: Option<Position>) -> Option<Sample> {
    match sample(world, at) {
        Sample::Unloaded => None,
        value => Some(value),
    }
}
fn hole(world: &impl World, at: Position, id: &str) -> Option<bool> {
    let below = read(world, offset(at, [0, -1, 0]))?;
    // Like native is_water_hole, an existing same-family source is a hole for
    // routing, not permission to replace it. The commit kernel preserves it.
    Some(below == Sample::Air || same_level(&below, id).is_some())
}
fn distance(
    world: &impl World, at: Position, id: &str, pass: u8, from: usize, limit: u8,
) -> Option<u8> {
    let mut lowest = NO_DROP;
    for (index, direction) in HORIZONTAL.into_iter().enumerate() {
        if index == from { continue; }
        let Some(next) = offset(at, direction) else { continue; };
        if !passable(&read(world, Some(next))?, id) { continue; }
        if hole(world, next, id)? { return Some(pass); }
        if pass < limit {
            lowest = lowest.min(distance(world, next, id, pass + 1, index ^ 1, limit)?);
        }
    }
    Some(lowest)
}

/// Complete selection precedes horizontal writes. Any encountered unloaded
/// sample defers this horizontal pass; it never loads a chunk or grants writes.
/// At most 4*(1+3+9+27+81) cell visits, two reads each: <=968 reads. The
/// existing tick's vertical/current-cell work is outside this selection bound.
pub(super) fn select(
    world: &impl World, at: Position, definition: &Definition, next_level: u8,
    limit: SlopeDistance,
) -> Option<[bool; 4]> {
    let mut selected = [false; 4];
    let mut lowest = NO_DROP;
    for (index, direction) in HORIZONTAL.into_iter().enumerate() {
        let Some(next) = offset(at, direction) else { continue; };
        let value = read(world, Some(next))?;
        // Preserve the original finite kernel's exact write eligibility.
        if value != Sample::Air && !same_level(&value, &definition.id)
            .is_some_and(|level| level > next_level && level != 8) { continue; }
        let cost = if hole(world, next, &definition.id)? { 0 } else {
            distance(world, next, &definition.id, 1, index ^ 1, limit.0)?
        };
        if cost < lowest { selected = [false; 4]; }
        if cost <= lowest {
            selected[index] = true;
            lowest = cost;
        }
    }
    Some(selected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::{Cell, Declaration, RenderDefinition, Registry, register_entries_with_slope,
        register_validated, tick, tick_with_slope};
    use std::{cell::Cell as Counter, collections::{BTreeMap, BTreeSet}, sync::Arc};

    fn definition() -> Definition {
        Definition { id: "slope:fluid".into(), block: "slope:block".into(),
            bucket: "slope:bucket".into(), texture: "slope:still".into(),
            tick_delay: 30, drop_off: 3, light: 12 }
    }
    #[derive(Default)]
    struct Grid {
        cells: BTreeMap<Position, Cell>, walls: BTreeSet<Position>,
        holes: BTreeSet<Position>, unloaded: BTreeSet<Position>,
        due: Vec<(Position, u16)>, writes: Vec<Position>, deny: bool,
        reads: Counter<usize>,
    }
    impl World for Grid {
        fn sample(&self, p: Position) -> Sample {
            self.reads.set(self.reads.get() + 1);
            if self.unloaded.contains(&p) { return Sample::Unloaded; }
            if self.walls.contains(&p) || (p[1] < 0 && !self.holes.contains(&p)) {
                return Sample::Solid;
            }
            self.cells.get(&p).cloned().map_or(Sample::Air, Sample::Fluid)
        }
        fn replace(&mut self, p: Position, expected: &Sample, cell: Option<&Cell>) -> bool {
            if self.deny || self.sample(p) != *expected { return false; }
            self.writes.push(p);
            if let Some(cell) = cell { self.cells.insert(p, cell.clone()); }
            else { self.cells.remove(&p); }
            true
        }
        fn schedule(&mut self, p: Position, delay: u16) { self.due.push((p, delay)); }
    }
    fn source() -> Grid {
        let mut g = Grid::default();
        g.cells.insert([0, 0, 0], Cell::source("slope:fluid"));
        g
    }
    fn choose(g: &Grid, limit: u8) -> Option<[bool; 4]> {
        select(g, [0, 0, 0], &definition(), 3, SlopeDistance::new(limit).unwrap())
    }

    #[test]
    fn distance_is_explicit_bounded_and_read_only() {
        for n in 1..=4 { assert_eq!(SlopeDistance::new(n).unwrap().get(), n); }
        for n in [0, 5, 255] { assert!(SlopeDistance::new(n).is_err()); }
        let d = definition();
        let entry = Declaration::new(d.clone(), RenderDefinition::default_for(&d)).unwrap();
        assert_eq!(entry.slope_distance(), None);
        assert_eq!(entry.with_slope_distance(SlopeDistance::new(3).unwrap()).slope_distance().unwrap().get(), 3);
    }
    #[test]
    fn direct_drop_wins_and_tied_drops_keep_native_iteration_order() {
        let mut g = source();
        g.holes.extend([[1, -1, 0], [0, -1, -1]]);
        assert_eq!(choose(&g, 3), Some([false, true, true, false]));
        tick_with_slope(&mut g, [0,0,0], &definition(), SlopeDistance::new(3).unwrap());
        assert_eq!(g.writes, vec![[1,0,0], [0,0,-1]]);
        assert!(g.due.iter().all(|(_, delay)| *delay == 30));
    }
    #[test]
    fn shorter_route_wins_but_equal_shortest_routes_are_kept() {
        let mut g = source();
        g.holes.extend([[2, -1, 0], [0, -1, -2]]);
        assert_eq!(choose(&g, 3), Some([false, true, true, false]));
    }
    #[test]
    fn distance_counts_steps_after_the_first_neighbor() {
        let mut g = source();
        g.holes.insert([4, -1, 0]);
        assert_eq!(choose(&g, 2), Some([true; 4]));
        assert_eq!(choose(&g, 3), Some([false, true, false, false]));
    }
    #[test]
    fn no_reachable_drop_preserves_all_eligible_directions() {
        let mut g = source();
        g.walls.insert([0,0,-1]);
        assert_eq!(choose(&g, 4), Some([true, true, false, true]));
    }
    #[test]
    fn sources_foreign_fluid_falling_and_stronger_neighbors_are_not_written() {
        let mut g = source();
        g.cells.insert([-1,0,0], Cell::source("slope:fluid"));
        g.cells.insert([1,0,0], Cell { fluid: "slope:other".into(), level: 7 });
        g.cells.insert([0,0,-1], Cell { fluid: "slope:fluid".into(), level: 8 });
        g.cells.insert([0,0,1], Cell { fluid: "slope:fluid".into(), level: 2 });
        assert_eq!(choose(&g, 3), Some([false; 4]));
        tick_with_slope(&mut g, [0,0,0], &definition(), SlopeDistance::new(3).unwrap());
        assert!(g.writes.is_empty());
    }
    #[test]
    fn unloaded_route_or_floor_defers_horizontal_writes_and_schedules_retry() {
        for p in [[1,0,0], [2,-1,0]] {
            let mut g = source();
            g.unloaded.insert(p);
            assert_eq!(choose(&g, 3), None);
            tick_with_slope(&mut g, [0,0,0], &definition(), SlopeDistance::new(3).unwrap());
            assert!(g.writes.is_empty());
            assert_eq!(g.due, vec![([0,0,0],30)]);
        }
    }
    #[test]
    fn coordinates_at_the_integer_boundary_never_wrap() {
        let g = Grid::default();
        assert_eq!(select(&g, [i32::MAX,0,0], &definition(), 3, SlopeDistance::new(4).unwrap()), Some([true,false,true,true]));
        assert_eq!(select(&g, [i32::MIN,0,0], &definition(), 3, SlopeDistance::new(4).unwrap()), Some([false,true,true,true]));
    }
    #[test]
    fn bounded_search_has_no_writes_and_at_most_968_reads() {
        let g = source();
        assert_eq!(choose(&g, 4), Some([true; 4]));
        assert!(g.reads.get() <= 968);
        assert!(g.writes.is_empty());
        assert!(g.due.is_empty());
    }
    #[test]
    fn denied_commit_cannot_publish_selected_fluid() {
        let mut g = source(); g.deny = true;
        g.holes.insert([1,-1,0]);
        tick_with_slope(&mut g, [0,0,0], &definition(), SlopeDistance::new(3).unwrap());
        assert_eq!(g.cells.len(), 1); assert!(g.writes.is_empty()); assert!(g.due.is_empty());
    }
    #[test]
    fn legacy_tick_still_spreads_all_sides_while_explicit_tick_selects() {
        let mut legacy = source(); legacy.holes.insert([1,-1,0]);
        let mut explicit = source(); explicit.holes.insert([1,-1,0]);
        tick(&mut legacy, [0,0,0], &definition());
        tick_with_slope(&mut explicit, [0,0,0], &definition(), SlopeDistance::new(3).unwrap());
        assert_eq!(legacy.writes, HORIZONTAL.to_vec());
        assert_eq!(explicit.writes, vec![[1,0,0]]);
    }
    #[test]
    fn vertical_priority_is_unchanged() {
        let mut g = source(); g.holes.insert([0,-1,0]);
        tick_with_slope(&mut g, [0,0,0], &definition(), SlopeDistance::new(3).unwrap());
        assert_eq!(g.writes, vec![[0,-1,0]]);
        assert_eq!(g.cells[&[0,-1,0]].level, 8);
    }
    #[test]
    fn atomic_registry_owns_slope_and_preserves_it_on_legacy_repeat() {
        let mut r = Registry::default(); let d = definition();
        let old = r.revision.clone(); let slope = SlopeDistance::new(3).unwrap();
        register_entries_with_slope(&mut r, vec![(d.clone(), None, Some(slope))]).unwrap();
        assert!(!old.is_current()); let rev = r.revision.clone();
        register_validated(&mut r, vec![d.clone()]).unwrap();
        register_entries_with_slope(&mut r, vec![(d.clone(), None, Some(slope))]).unwrap();
        assert!(Arc::ptr_eq(&rev.0, &r.revision.0)); assert_eq!(r.slopes.get(&d.id), Some(&slope));
        let mut fresh = d.clone(); fresh.id = "slope:new".into(); fresh.block = "slope:new_block".into(); fresh.bucket = "slope:new_bucket".into();
        assert!(register_entries_with_slope(&mut r, vec![(fresh.clone(), None, Some(slope)), (d, None, Some(SlopeDistance::new(4).unwrap()))]).is_err());
        assert!(!r.definitions.contains_key(&fresh.id)); assert!(!r.slopes.contains_key(&fresh.id));
        assert!(Arc::ptr_eq(&rev.0, &r.revision.0));
    }
    #[test]
    fn adding_policy_to_existing_legacy_definition_is_rejected() {
        let mut r = Registry::default(); let d = definition();
        register_validated(&mut r, vec![d.clone()]).unwrap(); let rev = r.revision.clone();
        assert!(register_entries_with_slope(&mut r, vec![(d, None, Some(SlopeDistance::new(3).unwrap()))]).is_err());
        assert!(r.slopes.is_empty()); assert!(Arc::ptr_eq(&rev.0, &r.revision.0));
    }
}
