// Upper-layer projection from the single authoritative fluid definition store.
// It adds no meshing hook and does not change contact/flow lookup semantics.
use horizon_client::{
    assets::{Assets, RenderAssets},
    mesher::{
        MeshTables, pluto_terrain_binding_host,
        pluto_terrain_bindings::{CurrentRevision, Error, Request},
    },
};
use std::sync::Arc;

fn requests() -> Result<(Vec<Request>, CurrentRevision), Error> {
    let snapshot = pluto_api::fluid::registry_snapshot().map_err(|_| Error::RegistryUnavailable)?;
    if snapshot.len() > horizon_client::mesher::pluto_terrain_bindings::MAX_BINDINGS / 9 {
        return Err(Error::LimitExceeded);
    }
    // Explicit display facts must not be silently projected as a single still
    // texture. A material-aware adapter is a separate gate; until installed,
    // nondefault facts make this prepared catalog explicitly unavailable.
    for definition in snapshot.iter() {
        let render = snapshot
            .render(&definition.id)
            .ok_or(Error::RegistryUnavailable)?;
        if render != &pluto_api::fluid::RenderDefinition::default_for(definition) {
            return Err(Error::UnsupportedAppearance(definition.id.clone()));
        }
    }
    // Bound projected text before multiplying it into nine owned descriptors.
    let mut text_bytes = 0usize;
    for definition in snapshot.iter() {
        let per_state = definition
            .id
            .len()
            .checked_add(definition.block.len())
            .and_then(|n| n.checked_add(definition.texture.len()))
            .and_then(|n| n.checked_add(6))
            .ok_or(Error::LimitExceeded)?;
        text_bytes = per_state
            .checked_mul(9)
            .and_then(|n| text_bytes.checked_add(n))
            .ok_or(Error::LimitExceeded)?;
        if text_bytes > horizon_client::mesher::pluto_terrain_bindings::MAX_TEXT_BYTES {
            return Err(Error::LimitExceeded);
        }
    }
    let revision = snapshot.revision();
    let current: CurrentRevision = Arc::new(move || revision.is_current());
    let requests = snapshot
        .iter()
        .flat_map(|definition| {
            (0..=8).map(move |level| Request {
                group: definition.id.clone(),
                variant: level,
                block: definition.block.clone(),
                properties: [("level".into(), level.to_string())].into_iter().collect(),
                texture: definition.texture.clone(),
            })
        })
        .collect();
    Ok((requests, current))
}

pub(super) fn prepare(assets: &Arc<Assets>, render: &Arc<RenderAssets>, tables: &Arc<MeshTables>) {
    if let Err(error) = pluto_terrain_binding_host::prepare(assets, render, tables, requests()) {
        log::warn!("PlutoLib: custom terrain binding publication unavailable: {error:?}");
    }
}
