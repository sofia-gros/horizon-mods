// Lower-renderer adapter. Only a derived cache; no gameplay/API dependency.
use super::pluto_terrain_bindings::{
    self as api, Appearance, AppearanceRequest, Binding, CacheError, Catalog, CurrentRevision,
    Error, OwnerCache, Request, Status,
};
use super::{MeshTables, Mesher, OPAQUE_LAYER};
use crate::assets::{Assets, RenderAssets};
use crate::models::catalog::BakedFace;
use horizon_assets::{bake::ModelQuad, model::Material as ModelMaterial};
use horizon_world::{chunk::BlockState, fluid::FluidKind};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

pub(super) struct Material {
    pub facts: Appearance,
    pub ambient_occlusion: bool,
    pub faces: [Option<BakedFace>; 6],
    model: String,
    texture: String,
}
impl PartialEq for Material {
    fn eq(&self, other: &Self) -> bool {
        // These resource owners are immutable. Exact selectors and facts refer
        // to the same baked templates, so no deep per-frame mesh comparison.
        self.facts == other.facts && self.model == other.model && self.texture == other.texture
    }
}
pub(super) type Materials = BTreeMap<u32, Arc<Material>>;
struct Prepared {
    status: Status,
    revision: Option<CurrentRevision>,
    materials: Materials,
}
impl PartialEq for Prepared {
    fn eq(&self, other: &Self) -> bool {
        self.status == other.status && self.materials == other.materials
    }
}
// Identical repeated preparation keeps the FIRST validity check and snapshot;
// it cannot reactivate an old owner set after a registry change.
static CACHE: Mutex<OwnerCache<Assets, RenderAssets, MeshTables, Prepared>> =
    Mutex::new(OwnerCache::new());

fn resolve(
    assets: &Assets,
    render: &RenderAssets,
    tables: &MeshTables,
    requests: &[Request],
) -> Result<Catalog, Error> {
    api::validate(requests)?;
    let mut by_state = std::collections::BTreeMap::new();
    for request in requests {
        let state = assets
            .registry
            .id(&BlockState {
                name: request.block.clone(),
                properties: request.properties.clone().into(),
            })
            .ok_or_else(|| Error::MissingState {
                group: request.group.clone(),
                variant: request.variant,
            })?;
        let facts = tables
            .info
            .get(state.0 as usize)
            .ok_or(Error::MissingFacts(state.0))?;
        if facts.fluid.kind() != FluidKind::Empty {
            return Err(Error::NativeFluid(state.0));
        }
        if !facts.model {
            return Err(Error::NotModel(state.0));
        }
        let layer = *render
            .sprite_layers
            .get(&request.texture)
            .ok_or_else(|| Error::MissingTexture(request.texture.clone()))?;
        if layer >= render.texture_layers || layer >= u32::from(OPAQUE_LAYER) {
            return Err(Error::InvalidLayer(request.texture.clone()));
        }
        let sprite =
            u16::try_from(layer).map_err(|_| Error::InvalidLayer(request.texture.clone()))?;
        let binding = Binding {
            group: request.group.clone(),
            variant: request.variant,
            state: state.0,
            sprite,
            animated: tables.is_animated(sprite),
        };
        if by_state.insert(state.0, binding).is_some() {
            return Err(Error::ConflictingState(state.0));
        }
    }
    Ok(Catalog {
        resource_generation: render.generation,
        by_state,
    })
}

fn copy_template(source: &BakedFace) -> BakedFace {
    let q = &source.quad;
    BakedFace {
        quad: ModelQuad {
            positions: q.positions,
            uv: q.uv,
            facing: q.facing,
            cull: q.cull,
            material: ModelMaterial {
                sprite: String::new(),
                force_translucent: q.material.force_translucent,
            },
            tint_index: q.tint_index,
            shade_direction_override: q.shade_direction_override,
            light_emission: q.light_emission,
            light: q.light,
            color: q.color,
            ambient_occlusion: q.ambient_occlusion,
            render_type: q.render_type,
            shade: q.shade,
        },
        layer: source.layer,
        opaque: source.opaque,
        translucent: source.translucent,
        fusion: source.fusion,
    }
}
fn materials(
    render: &RenderAssets,
    catalog: &Catalog,
    requests: &[AppearanceRequest],
) -> Result<Materials, Error> {
    let states: BTreeMap<_, _> = catalog
        .by_state
        .values()
        .map(|b| ((b.group.as_str(), b.variant), b.state))
        .collect();
    let mut prepared = Materials::new();
    let mut reused = BTreeMap::new();
    for request in requests {
        let state = *states
            .get(&(request.group.as_str(), request.variant))
            .ok_or(Error::InvalidRequest)?;
        let key = (
            request.model.as_str(),
            request.texture.as_str(),
            request.tint,
            request.anchor == super::pluto_terrain_cell_geometry::VerticalAnchor::Top,
        );
        let material = if let Some(value) = reused.get(&key) {
            Arc::clone(value)
        } else {
            if reused.len() >= api::MAX_MATERIAL_TEMPLATES {
                return Err(Error::LimitExceeded);
            }
            let layer = *render
                .sprite_layers
                .get(&request.texture)
                .ok_or_else(|| Error::MissingTexture(request.texture.clone()))?;
            if layer >= render.texture_layers || layer >= u32::from(OPAQUE_LAYER) {
                return Err(Error::InvalidLayer(request.texture.clone()));
            }
            let model = render
                .model_catalog
                .standalone(&request.model)
                .ok_or_else(|| Error::MissingMaterialModel(request.model.clone()))?;
            if model.faces.is_empty() || model.faces.len() > 6 {
                return Err(Error::InvalidMaterialModel(request.model.clone()));
            }
            let mut faces: [Option<BakedFace>; 6] = std::array::from_fn(|_| None);
            for source in &model.faces {
                let direction = source.quad.facing as usize;
                if source.layer != layer
                    || faces[direction].is_some()
                    || source.quad.uv.iter().flatten().any(|v| !v.is_finite())
                {
                    return Err(Error::InvalidMaterialModel(request.model.clone()));
                }
                // Fixed-size metadata only: no source material String clone.
                faces[direction] = Some(copy_template(source));
            }
            let value = Arc::new(Material {
                facts: Appearance {
                    secondary_sprite: layer as u16,
                    tint: request.tint,
                    anchor: request.anchor,
                },
                ambient_occlusion: model.ambient_occlusion,
                faces,
                model: request.model.clone(),
                texture: request.texture.clone(),
            });
            reused.insert(key, Arc::clone(&value));
            value
        };
        if prepared.insert(state, material).is_some() {
            return Err(Error::ConflictingState(state));
        }
    }
    Ok(prepared)
}

/// Legacy single-texture preparation retains its original contract.
pub fn prepare(
    assets: &Arc<Assets>,
    render: &Arc<RenderAssets>,
    tables: &Arc<MeshTables>,
    requests: Result<(Vec<Request>, CurrentRevision), Error>,
) -> Result<(), CacheError> {
    prepare_with_materials(
        assets,
        render,
        tables,
        requests.map(|(r, v)| (r, Vec::new(), v)),
    )
}
/// Resolve all appearance models/sprites before publication in the SAME owner
/// cache/revision as the base catalog. No material/model lookup occurs per cell.
pub fn prepare_with_materials(
    assets: &Arc<Assets>,
    render: &Arc<RenderAssets>,
    tables: &Arc<MeshTables>,
    requests: Result<(Vec<Request>, Vec<AppearanceRequest>, CurrentRevision), Error>,
) -> Result<(), CacheError> {
    let prepared = match requests {
        Ok((requests, appearance, revision)) => {
            let resolved = (|| {
                if !revision() {
                    return Err(Error::StaleRegistry);
                }
                api::validate_appearances(&requests, &appearance)?;
                let catalog = resolve(assets, render, tables, &requests)?;
                let materials = materials(render, &catalog, &appearance)?;
                Ok((catalog, materials))
            })();
            let (status, materials) = match resolved {
                Ok((catalog, materials)) => (Status::Ready(catalog), materials),
                Err(error) => (Status::Unavailable(error), Materials::new()),
            };
            Prepared {
                status,
                materials,
                revision: Some(revision),
            }
        }
        Err(error) => Prepared {
            status: Status::Unavailable(error),
            revision: None,
            materials: Materials::new(),
        },
    };
    CACHE
        .lock()
        .map_err(|_| CacheError::Poisoned)?
        .publish(assets, render, tables, prepared)
}

/// Borrow facts only when all three live resource owners match this mesher.
/// Callback runs without the cache lock. No mesh, chunk or texture is changed.
/// Numeric IDs describe only these resources; they are not persistent handles.
pub fn with_frame<R>(
    mesher: &Mesher<'_>,
    callback: impl FnOnce(&Status) -> R,
) -> Result<R, CacheError> {
    with_owners(mesher.assets, mesher.render, mesher.tables, callback)
}

// Internal staged mesher adapter: the exact owner borrows are split from the
// mutable mesher borrow. Same lookup and before/after validity contract.
pub(super) fn with_owners<R>(
    assets: &Assets,
    render: &RenderAssets,
    tables: &MeshTables,
    callback: impl FnOnce(&Status) -> R,
) -> Result<R, CacheError> {
    let status = CACHE
        .lock()
        .map_err(|_| CacheError::Poisoned)?
        .get(assets, render, tables)
        .ok_or(CacheError::Unbound)?;
    api::inspect(&status.status, status.revision.as_ref(), callback)
}

pub(super) fn with_material_owners<R>(
    assets: &Assets,
    render: &RenderAssets,
    tables: &MeshTables,
    callback: impl FnOnce(&Status, &Materials) -> R,
) -> Result<R, CacheError> {
    let prepared = CACHE
        .lock()
        .map_err(|_| CacheError::Poisoned)?
        .get(assets, render, tables)
        .ok_or(CacheError::Unbound)?;
    api::inspect(&prepared.status, prepared.revision.as_ref(), |status| {
        callback(status, &prepared.materials)
    })
}
