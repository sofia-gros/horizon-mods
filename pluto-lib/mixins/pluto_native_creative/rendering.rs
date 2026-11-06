// Shared, unshaded drawing mechanics. Content, layout and render hooks belong to consumers.
use horizon_client::hud::{HudAtlas, HudBatch, Rect};
use horizon_client::mesher::Vertex;

/// Borrowed atlas region. Do not retain it across an atlas/resource reload.
#[derive(Clone, Copy)]
pub struct Sprite<'a>(&'a Rect);

pub fn sprite<'a>(atlas: &'a HudAtlas, key: &str) -> Option<Sprite<'a>> {
    atlas.sprites.get(key).map(Sprite)
}

impl Sprite<'_> {
    pub fn logical_size(self) -> [f32; 2] {
        [self.0.w as f32, self.0.h as f32]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawError {
    InvalidInput,
    QuadBudgetExceeded,
}
impl std::fmt::Display for DrawError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidInput => "nonfinite, inverted or unsupported drawing input",
            Self::QuadBudgetExceeded => "drawing exceeds its quad budget",
        })
    }
}
impl std::error::Error for DrawError {}

/// Hard upper bound for one tiled draw, regardless of the requested budget.
pub const MAX_TILE_QUADS: usize = 4096;

fn finite(values: &[f32]) -> Result<(), DrawError> {
    if values.iter().all(|n| n.is_finite()) {
        Ok(())
    } else {
        Err(DrawError::InvalidInput)
    }
}
fn extent(size: [f32; 2]) -> Result<(), DrawError> {
    finite(&size)?;
    if size.into_iter().all(|n| n > 0.0) {
        Ok(())
    } else {
        Err(DrawError::InvalidInput)
    }
}
fn rectangle([x, y, w, h]: [f32; 4]) -> Result<(), DrawError> {
    finite(&[x, y, w, h, x + w, y + h])?;
    if w >= 0.0 && h >= 0.0 {
        Ok(())
    } else {
        Err(DrawError::InvalidInput)
    }
}
fn sprite_valid(sprite: Sprite<'_>) -> Result<(), DrawError> {
    let r = sprite.0;
    finite(&[r.u0, r.v0, r.u1, r.v1])
}

/// Append a tinted source crop. Source extent is explicit: it need not be the
/// sprite's logical size. Color is packed 0xAABBGGRR and is not shaded/repacked.
/// Zero destination area is a no-op. Errors never append partial output.
pub fn crop(
    batch: &mut HudBatch,
    sprite: Sprite<'_>,
    source: [f32; 4],
    source_extent: [f32; 2],
    destination: [f32; 4],
    color: u32,
) -> Result<(), DrawError> {
    sprite_valid(sprite)?;
    extent(source_extent)?;
    rectangle(source)?;
    rectangle(destination)?;
    let [sx, sy, sw, sh] = source;
    let mut r = *sprite.0;
    let du = (r.u1 - r.u0) / source_extent[0];
    let dv = (r.v1 - r.v0) / source_extent[1];
    r.u0 += sx * du;
    r.v0 += sy * dv;
    r.u1 = r.u0 + sw * du;
    r.v1 = r.v0 + sh * dv;
    finite(&[r.u0, r.v0, r.u1, r.v1])?;
    let [x, y, w, h] = destination;
    if w > 0.0 && h > 0.0 {
        batch.rect_uv(x, y, w, h, r, color);
    }
    Ok(())
}

/// Tile from the bottom upwards, left to right. Partial tiles take the top-left
/// source pixels. Destination is [left, bottom, width, height] in GUI pixels;
/// tile size uses the same units as the sprite's logical size. Arithmetic and
/// emission order retain the original float32 loop, including partial rows.
/// Requests are fully preflighted within a bounded budget before appending.
pub fn tiled_bottom_up(
    batch: &mut HudBatch,
    sprite: Sprite<'_>,
    destination: [f32; 4],
    tile: [f32; 2],
    color: u32,
    quad_budget: usize,
) -> Result<(), DrawError> {
    sprite_valid(sprite)?;
    extent(sprite.logical_size())?;
    extent(tile)?;
    let [left, bottom, width, height] = destination;
    rectangle([left, bottom - height, width, height])?;
    finite(&destination)?;
    let du = (sprite.0.u1 - sprite.0.u0) / sprite.0.w as f32;
    let dv = (sprite.0.v1 - sprite.0.v0) / sprite.0.h as f32;
    finite(&[du, dv])?;
    if width == 0.0 || height == 0.0 {
        return Ok(());
    }
    let budget = quad_budget.min(MAX_TILE_QUADS);
    let mut prepared = Vec::new();
    let mut y = 0.0;
    while y < height {
        let h = (height - y).min(tile[1]);
        let next_y = y + h;
        if next_y <= y {
            return Err(DrawError::InvalidInput);
        }
        let mut x = 0.0;
        while x < width {
            if prepared.len() == budget {
                return Err(DrawError::QuadBudgetExceeded);
            }
            let w = (width - x).min(tile[0]);
            let next_x = x + w;
            if next_x <= x {
                return Err(DrawError::InvalidInput);
            }
            let mut r = *sprite.0;
            r.u1 = r.u0 + w * du;
            r.v1 = r.v0 + h * dv;
            let at = [left + x, bottom - y - h, w, h];
            rectangle(at)?;
            finite(&[r.u1, r.v1])?;
            prepared.push((at, r));
            x = next_x;
        }
        y = next_y;
    }
    for ([x, y, w, h], r) in prepared {
        batch.rect_uv(x, y, w, h, r, color);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    Top,
    West,
    East,
    North,
    South,
    Bottom,
}
pub const TOP: &[Face] = &[Face::Top];
pub const TOP_AND_SIDES: &[Face] = &[Face::Top, Face::West, Face::East, Face::North, Face::South];
pub const ALL_FACES: &[Face] = &[
    Face::Top,
    Face::West,
    Face::East,
    Face::North,
    Face::South,
    Face::Bottom,
];

/// Append at most six ordered, unshaded entity quads. Bounds are final block
/// units, origin is already camera-relative, and UV span is in logical pixels.
/// Faces retain caller order; duplicate selections are permitted within six.
/// Layer, normal and glint coordinates are zero. Zero-height bounds are valid.
/// No culling, tank discovery, transforms, shading or alpha policy is imposed.
pub fn cuboid(
    out: &mut Vec<Vertex>,
    sprite: Sprite<'_>,
    origin: [f32; 3],
    bounds: [f32; 6],
    faces: &[Face],
    uv_span: [f32; 2],
    color: u32,
    light: u32,
) -> Result<(), DrawError> {
    sprite_valid(sprite)?;
    extent(sprite.logical_size())?;
    extent(uv_span)?;
    finite(&origin)?;
    finite(&bounds)?;
    if faces.len() > 6 {
        return Err(DrawError::QuadBudgetExceeded);
    }
    let [x0, y0, z0, x1, y1, z1] = bounds;
    if x0 > x1 || y0 > y1 || z0 > z1 {
        return Err(DrawError::InvalidInput);
    }
    let r = sprite.0;
    // Keep multiplication after division, rather than reusing GUI crop's du/dv.
    let u1 = r.u0 + (r.u1 - r.u0) * (uv_span[0] / r.w as f32);
    let v1 = r.v0 + (r.v1 - r.v0) * (uv_span[1] / r.h as f32);
    finite(&[u1, v1])?;
    let uv = [[r.u0, r.v0], [u1, r.v0], [u1, v1], [r.u0, v1]];
    let mut prepared = Vec::with_capacity(faces.len() * 4);
    for face in faces {
        let corners = match face {
            Face::Top => [[x0, y1, z0], [x1, y1, z0], [x1, y1, z1], [x0, y1, z1]],
            Face::West => [[x0, y1, z0], [x0, y1, z1], [x0, y0, z1], [x0, y0, z0]],
            Face::East => [[x1, y1, z1], [x1, y1, z0], [x1, y0, z0], [x1, y0, z1]],
            Face::North => [[x1, y1, z0], [x0, y1, z0], [x0, y0, z0], [x1, y0, z0]],
            Face::South => [[x0, y1, z1], [x1, y1, z1], [x1, y0, z1], [x0, y0, z1]],
            Face::Bottom => [[x0, y0, z1], [x1, y0, z1], [x1, y0, z0], [x0, y0, z0]],
        };
        for (p, uv) in corners.into_iter().zip(uv) {
            let pos = [origin[0] + p[0], origin[1] + p[1], origin[2] + p[2]];
            finite(&pos)?;
            prepared.push(Vertex {
                pos,
                uv,
                layer: 0,
                color,
                light,
                normal: [0; 4],
                glint_uv: [0.0; 2],
            });
        }
    }
    out.extend(prepared);
    Ok(())
}

/// Preserve packed world skylight and raise block light to a caller's minimum.
/// The caller owns luminosity policy; legal minimums are 0..=15.
pub fn minimum_block_light(world_light: u8, minimum: u8) -> Result<u32, DrawError> {
    if minimum > 15 {
        return Err(DrawError::InvalidInput);
    }
    let packed = (world_light & 0xf0) | (world_light & 15).max(minimum);
    Ok(horizon_client::block_entity_render::vertex_light(packed))
}

// BEGIN ADDITIVE TERRAIN AUTHOR FACADE
/// Mod-facing entry points for prepared terrain cells. The implementation and
/// registry remain in the renderer; these aliases and methods add no state.
///
/// Register during the mod's explicit client startup, before the first mesh.
/// This is not an automatic initialization event or a content/physics registry.
pub mod terrain {
    use horizon_client::mesher::pluto_terrain_cells as implementation;
    pub use horizon_client::mesher::pluto_terrain_bindings::{Appearance, Binding};
    pub use horizon_client::mesher::pluto_terrain_cell_geometry::{
        build, build_oriented, Batch, Cell, Error as GeometryError, Face, Heights,
        Quad, Surface, TopDiagonal, Vertex, VerticalAnchor,
    };
    pub use horizon_client::mesher::pluto_terrain_cells::{
        CellContext, MaterialContext, MaterialProvider, MaterialSlot, Neighborhood,
        PaintedBatch, Provider, RegisterError, MAX_GROUP_BYTES, MAX_PROVIDERS,
    };

    /// One short entry to the existing startup-only renderer registry.
    /// Copies share the same registry; dropping this value unregisters nothing.
    pub const CELLS: CellRenderers = CellRenderers { _private: () };

    #[derive(Clone, Copy, Debug)]
    pub struct CellRenderers {
        _private: (),
    }
    impl CellRenderers {
        /// Register one pure material-aware renderer for a prepared group with
        /// explicit appearance data. Default/no-appearance groups use geometry.
        /// Missing preparation, incompatible native metadata or invalid output
        /// still falls back to native rendering. Registration is not draw proof.
        pub fn register(self, group: &str, renderer: MaterialProvider) -> Result<(), RegisterError> {
            implementation::register_materials(&[(group, renderer)])
        }
        /// Atomically register material-aware groups in the same bounded map.
        /// A duplicate, invalid group, full or frozen map publishes no member.
        pub fn register_all(self, entries: &[(&str, MaterialProvider)]) -> Result<(), RegisterError> {
            implementation::register_materials(entries)
        }
        /// Preserve the legacy geometry-only mode explicitly. It cannot claim
        /// cells with explicit appearance data; use register for those cells.
        pub fn register_geometry(self, group: &str, renderer: Provider) -> Result<(), RegisterError> {
            implementation::register(group, renderer)
        }
        /// The original geometry-only batch API, without a new registry or limit.
        pub fn register_geometry_all(self, entries: &[(&str, Provider)]) -> Result<(), RegisterError> {
            implementation::register_all(entries)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        fn native_material(_: &MaterialContext<'_>) -> Option<PaintedBatch> { None }
        fn native_geometry(_: &CellContext<'_>) -> Option<Batch> { None }
        #[test]
        fn facade_types_are_the_implementation_types() {
            let material: implementation::MaterialProvider = native_material;
            let geometry: implementation::Provider = native_geometry;
            let _: MaterialProvider = material;
            let _: Provider = geometry;
            let _: fn(&Cell, VerticalAnchor) -> Result<Batch, GeometryError> = build_oriented;
            assert_eq!(std::mem::size_of_val(&CELLS), 0);
        }
        #[test]
        fn invalid_registration_retains_existing_validation_without_publishing() {
            // Invalid selectors are rejected before locking/freezing, so this
            // test neither changes global registration nor requires an empty map.
            assert_eq!(CELLS.register("", native_material), Err(RegisterError::InvalidGroup));
            assert_eq!(CELLS.register_all(&[("", native_material)]), Err(RegisterError::InvalidGroup));
            assert_eq!(CELLS.register_geometry("", native_geometry), Err(RegisterError::InvalidGroup));
            assert_eq!(CELLS.register_geometry_all(&[("", native_geometry)]), Err(RegisterError::InvalidGroup));
        }
    }
}
// END ADDITIVE TERRAIN AUTHOR FACADE
