//! Read-only, bounded geometry queries for PlutoLib's bottom-filled fluid cells.
//!
//! These return facts, not entity effects or movement decisions. Unloaded cells
//! are unknown, never dry. Flow direction does not imply an inverted shape.
pub mod immersion;

use super::{Cell, Position, Sample, World};
use std::{collections::BTreeMap, fmt};

/// Absolute ceiling even when a caller requests a larger sampling budget.
pub const MAX_QUERY_CELLS: u32 = 65_536;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_cells: u32,
}
impl Default for Limits {
    fn default() -> Self {
        Self { max_cells: 4096 }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueryError {
    NonFiniteCoordinates,
    NonPositiveBounds,
    OutsideCoordinateRange,
    InvalidBudget,
    BudgetExceeded { required_cells: u128, limit: u32 },
    InvalidLevel { position: Position, level: u8 },
}
impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteCoordinates => write!(f, "fluid query requires finite coordinates"),
            Self::NonPositiveBounds => write!(
                f,
                "fluid query bounds require positive extent on every axis"
            ),
            Self::OutsideCoordinateRange => {
                write!(f, "fluid query exceeds signed 32-bit cell coordinates")
            }
            Self::InvalidBudget => write!(f, "fluid query budget must be 1..={MAX_QUERY_CELLS}"),
            Self::BudgetExceeded {
                required_cells,
                limit,
            } => write!(
                f,
                "fluid query needs {required_cells} cells, budget is {limit}"
            ),
            Self::InvalidLevel { position, level } => {
                write!(f, "invalid fluid level {level} at {position:?}")
            }
        }
    }
}
impl std::error::Error for QueryError {}

/// Fluid volume intersecting a box, in cubic blocks. Each positive intersection
/// contributes one touched cell. Tangent faces do not count as contact.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Contact {
    pub volume: f64,
    pub touched_cells: u32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Contacts {
    /// Known contacts grouped by stable namespaced fluid ID, in sorted order.
    pub fluids: BTreeMap<String, Contact>,
    pub sampled_cells: u32,
    pub unloaded_cells: u32,
}
impl Contacts {
    /// Only a complete query can establish that an empty contact map is dry.
    pub fn is_complete(&self) -> bool {
        self.unloaded_cells == 0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PointContact {
    Dry,
    Unloaded,
    Fluid {
        cell: Cell,
        position: Position,
        /// Height of the upper fluid surface in world coordinates.
        surface_y: f64,
        /// Positive vertical distance from the point to that surface.
        depth: f64,
    },
}

/// The same bottom-filled height used by the current stepped block models:
/// source 8/9, horizontal levels (8-level)/9, falling level 8 is full height.
/// This is not a promise of vanilla/Forge corner smoothing or a density rule.
pub fn fill_height(level: u8) -> Option<f64> {
    match level {
        0..=7 => Some(f64::from(8 - level) / 9.0),
        8 => Some(1.0),
        _ => None,
    }
}

fn checked_height(cell: &Cell, position: Position) -> Result<f64, QueryError> {
    fill_height(cell.level).ok_or(QueryError::InvalidLevel {
        position,
        level: cell.level,
    })
}

fn point_cell(point: [f64; 3]) -> Result<Position, QueryError> {
    if point.iter().any(|n| !n.is_finite()) {
        return Err(QueryError::NonFiniteCoordinates);
    }
    let mut out = [0; 3];
    for axis in 0..3 {
        let p = point[axis].floor();
        if p < f64::from(i32::MIN) || p > f64::from(i32::MAX) {
            return Err(QueryError::OutsideCoordinateRange);
        }
        out[axis] = p as i32;
    }
    Ok(out)
}

/// Read a point using an existing world adapter. The query never writes or
/// schedules. The upper face of a fluid is excluded from the filled volume.
pub fn query_point(world: &impl World, point: [f64; 3]) -> Result<PointContact, QueryError> {
    query_point_with(point, |p| world.sample(p))
}

/// Read-only integration seam; the provider receives exactly one valid cell
/// coordinate, or is never called when coordinates are invalid.
pub fn query_point_with(
    point: [f64; 3],
    mut sample: impl FnMut(Position) -> Sample,
) -> Result<PointContact, QueryError> {
    let position = point_cell(point)?;
    match sample(position) {
        Sample::Air | Sample::Solid => Ok(PointContact::Dry),
        Sample::Unloaded => Ok(PointContact::Unloaded),
        Sample::Fluid(cell) => {
            let surface_y = f64::from(position[1]) + checked_height(&cell, position)?;
            if point[1] >= surface_y {
                return Ok(PointContact::Dry);
            }
            Ok(PointContact::Fluid {
                cell,
                position,
                surface_y,
                depth: surface_y - point[1],
            })
        }
    }
}

fn range(bounds: Bounds, limits: Limits) -> Result<([i64; 3], [i64; 3]), QueryError> {
    if limits.max_cells == 0 || limits.max_cells > MAX_QUERY_CELLS {
        return Err(QueryError::InvalidBudget);
    }
    if bounds.min.iter().chain(&bounds.max).any(|n| !n.is_finite()) {
        return Err(QueryError::NonFiniteCoordinates);
    }
    let mut start = [0; 3];
    let mut end = [0; 3];
    let mut count = 1u128;
    for axis in 0..3 {
        if bounds.max[axis] <= bounds.min[axis] {
            return Err(QueryError::NonPositiveBounds);
        }
        let lo = bounds.min[axis].floor();
        let hi = bounds.max[axis].ceil();
        if lo < f64::from(i32::MIN) || hi > f64::from(i32::MAX) + 1.0 {
            return Err(QueryError::OutsideCoordinateRange);
        }
        start[axis] = lo as i64;
        end[axis] = hi as i64;
        count *= (end[axis] - start[axis]) as u128;
    }
    if count > u128::from(limits.max_cells) {
        return Err(QueryError::BudgetExceeded {
            required_cells: count,
            limit: limits.max_cells,
        });
    }
    Ok((start, end))
}

/// Intersect a box against known fluid cells. Budget and coordinates are checked
/// before any world access; iteration is deterministic (Y, Z, X).
pub fn query_box(
    world: &impl World,
    bounds: Bounds,
    limits: Limits,
) -> Result<Contacts, QueryError> {
    query_box_with(bounds, limits, |p| world.sample(p))
}

/// Generic provider form. Providers must distinguish unloaded cells from air.
/// A partial result reports known contacts and its unloaded count; it must not
/// be interpreted as an exhaustive dry/submerged decision.
pub fn query_box_with(
    bounds: Bounds,
    limits: Limits,
    mut sample: impl FnMut(Position) -> Sample,
) -> Result<Contacts, QueryError> {
    let (start, end) = range(bounds, limits)?;
    let mut out = Contacts::default();
    for y in start[1]..end[1] {
        for z in start[2]..end[2] {
            for x in start[0]..end[0] {
                let p = [x as i32, y as i32, z as i32];
                out.sampled_cells += 1;
                match sample(p) {
                    Sample::Air | Sample::Solid => {}
                    Sample::Unloaded => out.unloaded_cells += 1,
                    Sample::Fluid(cell) => {
                        let h = checked_height(&cell, p)?;
                        let low = [x as f64, y as f64, z as f64];
                        let high = [low[0] + 1.0, low[1] + h, low[2] + 1.0];
                        let mut volume = 1.0;
                        for axis in 0..3 {
                            volume *= (bounds.max[axis].min(high[axis])
                                - bounds.min[axis].max(low[axis]))
                            .max(0.0);
                        }
                        if volume > 0.0 {
                            let contact = out.fluids.entry(cell.fluid).or_default();
                            contact.volume += volume;
                            contact.touched_cells += 1;
                        }
                    }
                }
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn block() -> Bounds {
        Bounds {
            min: [0.0; 3],
            max: [1.0; 3],
        }
    }
    fn fluid(id: &str, level: u8) -> Sample {
        Sample::Fluid(Cell {
            fluid: id.into(),
            level,
        })
    }
    fn close(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-12, "{a} != {b}");
    }

    #[test]
    fn source_volume_and_tangent_surface_are_distinct() {
        let out = query_box_with(block(), Limits::default(), |_| fluid("example:a", 0)).unwrap();
        close(out.fluids["example:a"].volume, 8.0 / 9.0);
        assert_eq!(out.fluids["example:a"].touched_cells, 1);
        assert!(out.is_complete());
        let bounds = Bounds {
            min: [0.0, 8.0 / 9.0, 0.0],
            ..block()
        };
        assert!(
            query_box_with(bounds, Limits::default(), |_| fluid("example:a", 0))
                .unwrap()
                .fluids
                .is_empty()
        );
        assert_eq!(
            query_point_with([0.5, 8.0 / 9.0, 0.5], |_| fluid("example:a", 0)).unwrap(),
            PointContact::Dry
        );
    }
    #[test]
    fn different_families_are_separate_sorted_facts() {
        let bounds = Bounds {
            max: [2.0, 1.0, 1.0],
            ..block()
        };
        let mut visited = vec![];
        let out = query_box_with(bounds, Limits::default(), |p| {
            visited.push(p);
            if p[0] == 0 {
                fluid("example:z", 0)
            } else {
                fluid("example:a", 8)
            }
        })
        .unwrap();
        assert_eq!(visited, vec![[0, 0, 0], [1, 0, 0]]);
        assert_eq!(
            out.fluids.keys().map(String::as_str).collect::<Vec<_>>(),
            vec!["example:a", "example:z"]
        );
        close(out.fluids["example:a"].volume, 1.0);
        close(out.fluids["example:z"].volume, 8.0 / 9.0);
    }
    #[test]
    fn negative_coordinates_use_floor_and_clip_fractional_bounds() {
        let bounds = Bounds {
            min: [-0.75; 3],
            max: [-0.25; 3],
        };
        let out = query_box_with(bounds, Limits::default(), |p| {
            assert_eq!(p, [-1, -1, -1]);
            fluid("example:a", 8)
        })
        .unwrap();
        close(out.fluids["example:a"].volume, 0.125);
        let point = query_point_with([-0.5, -0.75, -0.5], |p| {
            assert_eq!(p, [-1, -1, -1]);
            fluid("example:a", 4)
        })
        .unwrap();
        match point {
            PointContact::Fluid {
                position, depth, ..
            } => {
                assert_eq!(position, [-1, -1, -1]);
                close(depth, 4.0 / 9.0 - 0.25);
            }
            _ => panic!("expected contact"),
        }
    }
    #[test]
    fn unloaded_is_unknown_even_alongside_known_contact() {
        let bounds = Bounds {
            max: [2.0, 1.0, 1.0],
            ..block()
        };
        let out = query_box_with(bounds, Limits::default(), |p| {
            if p[0] == 0 {
                Sample::Unloaded
            } else {
                fluid("example:a", 8)
            }
        })
        .unwrap();
        assert_eq!(out.sampled_cells, 2);
        assert_eq!(out.unloaded_cells, 1);
        assert!(!out.is_complete());
        close(out.fluids["example:a"].volume, 1.0);
        assert_eq!(
            query_point_with([0.0; 3], |_| Sample::Unloaded).unwrap(),
            PointContact::Unloaded
        );
    }
    #[test]
    fn budget_is_checked_before_sampling_and_large_products_do_not_overflow() {
        let bounds = Bounds {
            max: [65.0, 1.0, 1.0],
            ..block()
        };
        assert_eq!(
            query_box_with(bounds, Limits { max_cells: 64 }, |_| panic!(
                "must not sample"
            )),
            Err(QueryError::BudgetExceeded {
                required_cells: 65,
                limit: 64
            })
        );
        let bounds = Bounds {
            min: [f64::from(i32::MIN); 3],
            max: [f64::from(i32::MAX) + 1.0; 3],
        };
        assert_eq!(
            query_box_with(bounds, Limits::default(), |_| panic!("must not sample")),
            Err(QueryError::BudgetExceeded {
                required_cells: 1u128 << 96,
                limit: 4096
            })
        );
    }
    #[test]
    fn invalid_bounds_and_budgets_never_sample() {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                query_box_with(
                    Bounds {
                        min: [bad, 0.0, 0.0],
                        ..block()
                    },
                    Limits::default(),
                    |_| panic!("sample")
                ),
                Err(QueryError::NonFiniteCoordinates)
            );
            assert_eq!(
                query_point_with([bad, 0.0, 0.0], |_| panic!("sample")),
                Err(QueryError::NonFiniteCoordinates)
            );
        }
        for max in [0.0, -1.0] {
            assert_eq!(
                query_box_with(
                    Bounds {
                        max: [max, 1.0, 1.0],
                        ..block()
                    },
                    Limits::default(),
                    |_| panic!("sample")
                ),
                Err(QueryError::NonPositiveBounds)
            );
        }
        for budget in [0, MAX_QUERY_CELLS + 1] {
            assert_eq!(
                query_box_with(block(), Limits { max_cells: budget }, |_| panic!("sample")),
                Err(QueryError::InvalidBudget)
            );
        }
    }
    #[test]
    fn coordinate_edges_are_explicit_and_do_not_wrap() {
        let edge = f64::from(i32::MAX);
        let bounds = Bounds {
            min: [edge; 3],
            max: [edge + 1.0; 3],
        };
        let out = query_box_with(bounds, Limits::default(), |p| {
            assert_eq!(p, [i32::MAX; 3]);
            fluid("example:a", 8)
        })
        .unwrap();
        close(out.fluids["example:a"].volume, 1.0);
        assert_eq!(
            query_point_with([edge + 1.0, 0.0, 0.0], |_| panic!("sample")),
            Err(QueryError::OutsideCoordinateRange)
        );
        assert_eq!(
            query_box_with(
                Bounds {
                    min: [f64::from(i32::MIN) - 0.5, 0.0, 0.0],
                    ..block()
                },
                Limits::default(),
                |_| panic!("sample")
            ),
            Err(QueryError::OutsideCoordinateRange)
        );
    }
    #[test]
    fn malformed_levels_are_errors_not_dry_results() {
        let error = QueryError::InvalidLevel {
            position: [0; 3],
            level: 9,
        };
        assert_eq!(
            query_box_with(block(), Limits::default(), |_| fluid("example:a", 9)),
            Err(error.clone())
        );
        assert_eq!(
            query_point_with([0.5; 3], |_| fluid("example:a", 9)),
            Err(error)
        );
    }
    #[test]
    fn current_query_geometry_matches_every_generated_model_level() {
        let d = super::super::Definition {
            id: "example:a".into(),
            block: "example:a".into(),
            bucket: "example:bucket".into(),
            texture: "example:fluid/a".into(),
            tick_delay: 1,
            drop_off: 1,
            light: 0,
        };
        let content = super::super::content(&d).unwrap();
        for level in 0..=8 {
            let key = format!("assets/example/models/block/a/level_{level}.json");
            let model_height = content[&key]["elements"][0]["to"][1].as_f64().unwrap() / 16.0;
            close(fill_height(level).unwrap(), model_height);
            let out =
                query_box_with(block(), Limits::default(), |_| fluid("example:a", level)).unwrap();
            close(out.fluids["example:a"].volume, model_height);
        }
        assert_eq!(fill_height(9), None);
    }
    #[test]
    fn world_adapter_queries_never_write_or_schedule() {
        struct ReadOnly;
        impl World for ReadOnly {
            fn sample(&self, _: Position) -> Sample {
                fluid("example:a", 8)
            }
            fn replace(&mut self, _: Position, _: &Sample, _: Option<&Cell>) -> bool {
                panic!("write")
            }
            fn schedule(&mut self, _: Position, _: u16) {
                panic!("schedule")
            }
        }
        assert!(query_box(&ReadOnly, block(), Limits::default())
            .unwrap()
            .is_complete());
        assert!(matches!(
            query_point(&ReadOnly, [0.5; 3]).unwrap(),
            PointContact::Fluid { .. }
        ));
    }
}
