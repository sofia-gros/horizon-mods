use crate::{
    assets::{Assets, RenderAssets},
    mesher::{
        self, pluto_terrain_binding_host as host, pluto_terrain_bindings as api, Info, MeshTables,
        Mesher,
    },
};
use horizon_world::{chunk::BlockState, fluid::FluidState, nbt::Tag, registry::BlockRegistry};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
fn registry(name: &str) -> BlockRegistry {
    let states = (0..=8)
        .map(|level| {
            BlockState {
                name: name.into(),
                properties: [("level".into(), level.to_string())]
                    .into_iter()
                    .collect::<BTreeMap<_, _>>()
                    .into(),
            }
            .to_nbt()
        })
        .collect();
    BlockRegistry::from_nbt(&Tag::Compound(
        [
            ("DataVersion".into(), Tag::Int(horizon_world::DATA_VERSION)),
            ("states".into(), Tag::List(states)),
            (
                "defaults".into(),
                Tag::Compound([(name.into(), Tag::Int(0))].into_iter().collect()),
            ),
        ]
        .into_iter()
        .collect(),
    ))
    .unwrap()
}
fn owners() -> (Arc<Assets>, Arc<RenderAssets>, Arc<MeshTables>) {
    (
        Arc::new(Assets {
            registry: Box::leak(Box::new(registry("binding:fluid"))),
        }),
        Arc::new(RenderAssets {
            texture_layers: 3,
            sprite_layers: [("binding:texture".into(), 2)].into_iter().collect(),
            generation: 42,
        }),
        Arc::new(MeshTables {
            info: (0..9)
                .map(|_| Info {
                    fluid: FluidState::EMPTY,
                    model: true,
                })
                .collect(),
            animated_layers: vec![false, false, true],
        }),
    )
}
fn requests() -> Vec<api::Request> {
    (0..=8)
        .map(|level| api::Request {
            group: "binding:family".into(),
            variant: level,
            block: "binding:fluid".into(),
            properties: [("level".into(), level.to_string())].into_iter().collect(),
            texture: "binding:texture".into(),
        })
        .collect()
}
fn ready() -> api::CurrentRevision {
    Arc::new(|| true)
}
fn read(a: &Assets, r: &RenderAssets, t: &MeshTables) -> api::Status {
    host::with_frame(
        &Mesher {
            assets: a,
            render: r,
            tables: t,
        },
        Clone::clone,
    )
    .unwrap()
}
#[test]
fn exact_registry_resolves_all_nine_states_and_retained_resource_owners() {
    let (a, r, t) = owners();
    host::prepare(&a, &r, &t, Ok((requests(), ready()))).unwrap();
    let api::Status::Ready(catalog) = read(&a, &r, &t) else {
        panic!()
    };
    assert_eq!(catalog.by_state.len(), 9);
    for (id, binding) in &catalog.by_state {
        assert_eq!(*id, binding.variant);
        assert_eq!(binding.sprite, 2);
        assert!(binding.animated);
    }
    assert_eq!(
        read(&a.clone(), &r.clone(), &t.clone()),
        api::Status::Ready(catalog)
    );
    let (_, r2, t2) = owners();
    assert_eq!(
        host::with_frame(
            &Mesher {
                assets: &a,
                render: &r2,
                tables: &t2
            },
            |_| ()
        ),
        Err(api::CacheError::Unbound)
    );
    // Same asset registry and generation do not alias a reload of render/tables.
    host::prepare(&a, &r2, &t2, Ok((requests(), ready()))).unwrap();
    assert!(matches!(read(&a, &r2, &t2), api::Status::Ready(_)));
}
#[test]
fn native_lookup_failures_publish_only_explicit_unavailable() {
    for variant in 0..8 {
        let (a, mut r, mut t) = owners();
        let mut req = requests();
        let expected = match variant {
            0 => {
                req[8].properties.insert("level".into(), "9".into());
                api::Error::MissingState {
                    group: "binding:family".into(),
                    variant: 8,
                }
            }
            1 => {
                Arc::get_mut(&mut r).unwrap().sprite_layers.clear();
                api::Error::MissingTexture("binding:texture".into())
            }
            2 => {
                Arc::get_mut(&mut t).unwrap().info[4].fluid = FluidState::WATER;
                api::Error::NativeFluid(4)
            }
            3 => {
                Arc::get_mut(&mut t).unwrap().info[4].model = false;
                api::Error::NotModel(4)
            }
            4 => {
                Arc::get_mut(&mut r).unwrap().texture_layers = 2;
                api::Error::InvalidLayer("binding:texture".into())
            }
            5 => {
                req[8].properties = req[0].properties.clone();
                api::Error::ConflictingState(0)
            }
            6 => {
                Arc::get_mut(&mut t).unwrap().info.pop();
                api::Error::MissingFacts(8)
            }
            _ => {
                let r = Arc::get_mut(&mut r).unwrap();
                r.texture_layers = u32::MAX;
                r.sprite_layers
                    .insert("binding:texture".into(), u32::from(mesher::OPAQUE_LAYER));
                api::Error::InvalidLayer("binding:texture".into())
            }
        };
        host::prepare(&a, &r, &t, Ok((req, ready()))).unwrap();
        assert_eq!(read(&a, &r, &t), api::Status::Unavailable(expected));
    }
    let (a, r, t) = owners();
    host::prepare(&a, &r, &t, Err(api::Error::RegistryUnavailable)).unwrap();
    assert_eq!(
        read(&a, &r, &t),
        api::Status::Unavailable(api::Error::RegistryUnavailable)
    );
}
#[test]
fn callback_can_reenter_without_cache_lock_and_invalidation_is_checked_twice() {
    let (a, r, t) = owners();
    let token = Arc::new(AtomicBool::new(true));
    let current = token.clone();
    host::prepare(
        &a,
        &r,
        &t,
        Ok((
            requests(),
            Arc::new(move || current.load(Ordering::Acquire)),
        )),
    )
    .unwrap();
    let frame = Mesher {
        assets: &a,
        render: &r,
        tables: &t,
    };
    host::with_frame(&frame, |_| host::with_frame(&frame, |_| ()).unwrap()).unwrap();
    assert_eq!(
        host::with_frame(&frame, |_| token.store(false, Ordering::Release)),
        Err(api::CacheError::StaleRegistry)
    );
    assert_eq!(
        read(&a, &r, &t),
        api::Status::Unavailable(api::Error::StaleRegistry)
    );
    // Republishing identical values must not reactivate the expired revision.
    host::prepare(&a, &r, &t, Ok((requests(), ready()))).unwrap();
    assert_eq!(
        read(&a, &r, &t),
        api::Status::Unavailable(api::Error::StaleRegistry)
    );
}
#[test]
fn actual_fluid_registry_revision_is_atomic_and_late_registration_requires_new_owners() {
    use pluto_api::fluid::{self, Definition};
    let first = Definition {
        id: "binding:family".into(),
        block: "binding:fluid".into(),
        bucket: "binding:bucket".into(),
        texture: "binding:texture".into(),
        tick_delay: 5,
        drop_off: 1,
        light: 0,
    };
    fluid::register(first.clone()).unwrap();
    let old = fluid::registry_snapshot().unwrap();
    let lease = old.revision();
    let (a, r, t) = owners();
    crate::game::prepare(&a, &r, &t);
    assert!(matches!(read(&a, &r, &t), api::Status::Ready(_)));
    fluid::register_all([first.clone(), first.clone()]).unwrap();
    assert!(lease.is_current());
    let mut bad = first.clone();
    bad.light = 1;
    assert!(fluid::register_all([bad]).is_err());
    assert!(lease.is_current());
    let second = Definition {
        id: "binding:later".into(),
        block: "binding:later_block".into(),
        bucket: "binding:later_bucket".into(),
        texture: "binding:later_texture".into(),
        ..first
    };
    fluid::register(second).unwrap();
    assert!(!lease.is_current());
    assert_eq!(old.len(), 1);
    assert_eq!(fluid::registry_snapshot().unwrap().len(), 2);
    assert_eq!(
        read(&a, &r, &t),
        api::Status::Unavailable(api::Error::StaleRegistry)
    );
    let (a2, r2, t2) = owners();
    crate::game::prepare(&a2, &r2, &t2);
    assert!(matches!(
        read(&a2, &r2, &t2),
        api::Status::Unavailable(api::Error::MissingState { .. })
    ));
}
