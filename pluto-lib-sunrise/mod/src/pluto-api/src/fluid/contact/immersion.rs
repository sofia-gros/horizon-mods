//! Synchronous height facts from explicit bounds and caller-owned surfaces.
//! Not a tick cache, entity lookup, movement integrator or contact-effect hook.
use super::{range, Bounds, Limits, QueryError};
use crate::fluid::Position;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub enum SurfaceSample {
    Dry,
    Unloaded,
    /// Absolute surface coordinate. The provider owns height/above-fluid rules
    /// and numeric precision; this kernel never infers them from a fluid ID.
    Fluid { id: String, surface_y: f64 },
}
#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    Query(QueryError),
    NonFiniteSurface { position: Position },
}
impl From<QueryError> for Error {
    fn from(error: QueryError) -> Self { Self::Query(error) }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Heights {
    /// Maximum surface minus the supplied lower Y, not intersection volume.
    /// A tangent surface is retained as zero; positive immersion requires > 0.
    pub fluids: BTreeMap<String, f64>,
    pub sampled_cells: u32,
    pub unloaded_cells: u32,
}
impl Heights {
    pub fn is_complete(&self) -> bool { self.unloaded_cells == 0 }
}

/// Query the supplied already-adjusted box synchronously in X/Y/Z order.
/// Coordinates and the existing contact budget are checked before sampling.
/// Providers must be read-only and distinguish unknown cells from known dry.
/// Incomplete results establish only known lower bounds, not absence of fluid.
/// No deflation, boat adjustment, fluid filter, queue or owner is selected here.
pub fn query_with(
    bounds: Bounds,
    limits: Limits,
    mut sample: impl FnMut(Position) -> Result<SurfaceSample, Error>,
) -> Result<Heights, Error> {
    let (start, end) = range(bounds, limits)?;
    let mut out = Heights::default();
    for x in start[0]..end[0] {
        for y in start[1]..end[1] {
            for z in start[2]..end[2] {
                let position = [x as i32, y as i32, z as i32];
                out.sampled_cells += 1;
                match sample(position)? {
                    SurfaceSample::Dry => {},
                    SurfaceSample::Unloaded => out.unloaded_cells += 1,
                    SurfaceSample::Fluid { id, surface_y } => {
                        if !surface_y.is_finite() {
                            return Err(Error::NonFiniteSurface { position });
                        }
                        if surface_y >= bounds.min[1] {
                            let height = surface_y - bounds.min[1];
                            let known = out.fluids.entry(id).or_insert(0.0);
                            if height > *known { *known = height; }
                        }
                    }
                }
            }
        }
    }
    Ok(out)
}

/// Reuse the currently declared stepped-cell geometry without changing the
/// existing decoder. This is not Forge getHeight's above-fluid/precision rule.
pub fn stepped_surface(position: Position, sample: crate::fluid::Sample) -> Result<SurfaceSample, Error> {
    use crate::fluid::Sample;
    Ok(match sample {
        Sample::Air | Sample::Solid => SurfaceSample::Dry,
        Sample::Unloaded => SurfaceSample::Unloaded,
        Sample::Fluid(cell) => SurfaceSample::Fluid {
            surface_y: f64::from(position[1]) + super::checked_height(&cell, position)?,
            id: cell.fluid,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn bounds() -> Bounds { Bounds { min: [0.0; 3], max: [2.0; 3] } }
    fn fluid(id: &str, y: f64) -> Result<SurfaceSample, Error> {
        Ok(SurfaceSample::Fluid { id: id.into(), surface_y: y })
    }
    #[test]
    fn max_height_is_not_sum_volume_and_keeps_fluid_identities() {
        let out = query_with(bounds(), Limits::default(), |p|
            fluid(if p[0] == 0 { "example:a" } else { "example:b" }, p[1] as f64 + 0.25)).unwrap();
        assert_eq!(out.sampled_cells, 8);
        assert_eq!(out.fluids.values().copied().collect::<Vec<_>>(), [1.25, 1.25]);
        assert!(out.is_complete());
    }
    #[test]
    fn tangent_and_unloaded_are_not_silently_positive_or_dry() {
        let out = query_with(bounds(), Limits::default(), |p|
            if p == [0, 0, 0] { fluid("example:tangent", 0.0) } else { Ok(SurfaceSample::Unloaded) }).unwrap();
        assert_eq!(out.fluids["example:tangent"], 0.0);
        assert_eq!(out.unloaded_cells, 7);
        assert!(!out.is_complete());
        let out = query_with(bounds(), Limits::default(), |_| fluid("example:below", -0.1)).unwrap();
        assert!(out.fluids.is_empty());
    }
    #[test]
    fn exact_provider_precision_and_adjusted_minimum_are_preserved() {
        let b = Bounds { min: [-0.999, 63.001, -0.999], max: [-0.001, 64.999, -0.001] };
        let mut order = Vec::new();
        let out = query_with(b, Limits::default(), |p| {
            order.push(p);
            fluid("example:a", f64::from(p[1] as f32 + 8.0f32 / 9.0))
        }).unwrap();
        assert_eq!(order, [[-1, 63, -1], [-1, 64, -1]]);
        assert_eq!(out.fluids["example:a"].to_bits(), (f64::from(64.0f32 + 8.0f32 / 9.0) - 63.001).to_bits());
    }
    #[test]
    fn x_y_z_order_and_preflight_budget_are_explicit() {
        let mut seen = Vec::new();
        query_with(bounds(), Limits::default(), |p| { seen.push(p); Ok(SurfaceSample::Dry) }).unwrap();
        assert_eq!(seen, [[0,0,0],[0,0,1],[0,1,0],[0,1,1],[1,0,0],[1,0,1],[1,1,0],[1,1,1]]);
        let mut calls = 0;
        assert!(query_with(bounds(), Limits { max_cells: 7 }, |_| { calls += 1; Ok(SurfaceSample::Dry) }).is_err());
        assert_eq!(calls, 0);
    }
    #[test]
    fn invalid_bounds_and_nonfinite_surfaces_fail_closed() {
        for b in [Bounds { min: [f64::NAN,0.0,0.0], ..bounds() },
                  Bounds { max: [0.0,2.0,2.0], ..bounds() },
                  Bounds { min: [f64::from(i32::MIN)-1.0,0.0,0.0], ..bounds() }] {
            assert!(query_with(b, Limits::default(), |_| panic!("invalid bounds sampled")).is_err());
        }
        for y in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(query_with(bounds(), Limits::default(), |_| fluid("example:a", y)),
                Err(Error::NonFiniteSurface { position: [0,0,0] }));
        }
    }
    #[test]
    fn stepped_provider_reuses_current_level_validation_and_geometry() {
        use crate::fluid::{Cell, Sample};
        assert_eq!(stepped_surface([0,4,0], Sample::Unloaded), Ok(SurfaceSample::Unloaded));
        for level in 0..=8 {
            assert_eq!(stepped_surface([0,4,0], Sample::Fluid(Cell { fluid: "example:a".into(), level })),
                fluid("example:a", 4.0 + super::super::fill_height(level).unwrap()));
        }
        assert!(matches!(stepped_surface([0,4,0], Sample::Fluid(Cell { fluid: "example:a".into(), level: 9 })),
            Err(Error::Query(QueryError::InvalidLevel { .. }))));
    }
}
