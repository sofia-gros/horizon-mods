// Lower-renderer adapter. Only a derived cache; no gameplay/API dependency.
use super::pluto_terrain_bindings::{
    self as api, Binding, CacheError, Catalog, CurrentRevision, Error, OwnerCache, Request, Status,
};
use super::{MeshTables, Mesher, OPAQUE_LAYER};
use crate::assets::{Assets, RenderAssets};
use horizon_world::{chunk::BlockState, fluid::FluidKind};
use std::sync::{Arc, Mutex};

struct Prepared {
    status: Status,
    revision: Option<CurrentRevision>,
}
impl PartialEq for Prepared {
    fn eq(&self, other: &Self) -> bool {
        self.status == other.status
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

/// Publish once for these exact resource owners. All resolution runs before the
/// cache lock. Failure is recorded explicitly; an empty successful set is Ready.
/// Repeated identical publication is harmless; conflicting snapshots never win.
pub fn prepare(
    assets: &Arc<Assets>,
    render: &Arc<RenderAssets>,
    tables: &Arc<MeshTables>,
    requests: Result<(Vec<Request>, CurrentRevision), Error>,
) -> Result<(), CacheError> {
    let prepared = match requests {
        Ok((requests, revision)) => {
            let status = if !revision() {
                Status::Unavailable(Error::StaleRegistry)
            } else {
                match resolve(assets, render, tables, &requests) {
                    Ok(catalog) => Status::Ready(catalog),
                    Err(error) => Status::Unavailable(error),
                }
            };
            Prepared {
                status,
                revision: Some(revision),
            }
        }
        Err(error) => Prepared {
            status: Status::Unavailable(error),
            revision: None,
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
