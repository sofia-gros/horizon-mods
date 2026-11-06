// Excluded test-only extension to the exact successful CPU fixture's tests module.
// Real native resource owners; original synthetic assets. Not the Game reload UI.
mod reload {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use super::super::super::pluto_terrain_binding_host as host;

    fn snapshot(f: &Fixture) -> Result<binding::Status, binding::CacheError> {
        host::with_owners(&f.assets, &f.render, &f.tables, Clone::clone)
    }

    fn publish(f: &Fixture, variant: u32, texture: &str, current: binding::CurrentRevision) {
        host::prepare(&f.assets, &f.render, &f.tables, Ok((vec![binding::Request {
            group: GROUP.into(), variant, block: "pluto_fixture:cell".into(),
            properties: BTreeMap::new(), texture: texture.into(),
        }], current))).unwrap();
    }

    fn variant(f: &Fixture) -> u32 {
        let binding::Status::Ready(catalog) = snapshot(f).unwrap() else {
            panic!("Expected an actual prepared catalog, not native fallback");
        };
        assert_eq!(catalog.by_state.len(), 1);
        assert_eq!(catalog.resource_generation, f.render.generation);
        catalog.by_state.get(&CELL).unwrap().variant
    }

    // stage_pack shares Assets but creates RenderAssets and MeshTables anew.
    // This is an actual-owner contract fixture, not execution of stage_pack:
    // RenderAssets still uses the existing fixture's actual baker/Sprites output.
    fn next_owners(root: &Path, old: &Fixture) -> Fixture {
        let mut next = fixture(root, 0);
        for name in NAMES {
            let state = horizon_world::chunk::BlockState {
                name: name.into(), properties: BTreeMap::new().into(),
            };
            assert_eq!(next.assets.registry.id(&state), old.assets.registry.id(&state));
        }
        next.assets = old.assets.clone();
        next.visibility = visibility(&next.assets);
        next.tables = Arc::new(MeshTables::new(
            &next.assets, &next.render, &facts(), next.visibility.clone(),
        ).unwrap());
        assert!(Arc::ptr_eq(&next.assets, &old.assets));
        assert!(!Arc::ptr_eq(&next.render, &old.render));
        assert!(!Arc::ptr_eq(&next.tables, &old.tables));
        // Deliberately collide diagnostic generations: identity must not use it.
        assert_eq!(next.render.generation, old.render.generation);
        next
    }

    #[test]
    fn exact_resource_owner_reload_contract() {
        let root = PathBuf::from(std::env::var_os("PLUTO_TERRAIN_FIXTURE_RESOURCES")
            .expect("Explicit generated original resources are required")).join("ordinary");
        let base = fixture(&root, 0);
        publish(&base, 1, "pluto_fixture:block/cell", Arc::new(|| true));
        let next = next_owners(&root, &base);
        assert_eq!(snapshot(&next), Err(binding::CacheError::Unbound));
        publish(&next, 2, "pluto_fixture:block/cell", Arc::new(|| true));
        assert_eq!(variant(&base), 1);
        assert_eq!(variant(&next), 2);
        assert_eq!(variant(&base), 1); // Old retained worker owners remain readable.
        println!("RELOAD_CASE same_assets_new_render_tables_generation_collision ok");

        assert_eq!(host::with_owners(&base.assets, &next.render, &base.tables, Clone::clone),
            Err(binding::CacheError::Unbound));
        assert_eq!(host::with_owners(&base.assets, &base.render, &next.tables, Clone::clone),
            Err(binding::CacheError::Unbound));
        let (a, r, t) = (base.assets.clone(), base.render.clone(), base.tables.clone());
        assert_eq!(host::with_owners(&a, &r, &t, Clone::clone), snapshot(&base));
        println!("RELOAD_CASE exact_triple_and_clones_only ok");

        let failed = next_owners(&root, &base);
        publish(&failed, 3, "pluto_fixture:absent", Arc::new(|| true));
        assert!(matches!(snapshot(&failed), Ok(binding::Status::Unavailable(
            binding::Error::MissingTexture(ref name))) if name == "pluto_fixture:absent"));
        assert_eq!(variant(&base), 1);
        assert_eq!(variant(&next), 2);
        println!("RELOAD_CASE unavailable_candidate_does_not_replace_old_owners ok");

        let abandoned = next_owners(&root, &base);
        publish(&abandoned, 4, "pluto_fixture:block/cell", Arc::new(|| true));
        let (render, tables) = (Arc::downgrade(&abandoned.render), Arc::downgrade(&abandoned.tables));
        drop(abandoned);
        assert!(render.upgrade().is_none());
        assert!(tables.upgrade().is_none());
        assert_eq!(variant(&base), 1);
        println!("RELOAD_CASE abandoned_candidate_does_not_keep_resource_values_alive ok");

        let stale = next_owners(&root, &base);
        let live = Arc::new(AtomicBool::new(true));
        let read = live.clone();
        publish(&stale, 5, "pluto_fixture:block/cell",
            Arc::new(move || read.load(Ordering::Acquire)));
        assert_eq!(variant(&stale), 5);
        live.store(false, Ordering::Release);
        assert_eq!(snapshot(&stale), Ok(binding::Status::Unavailable(binding::Error::StaleRegistry)));
        // Same owners + equal catalog cannot replace their first revision token.
        publish(&stale, 5, "pluto_fixture:block/cell", Arc::new(|| true));
        assert_eq!(snapshot(&stale), Ok(binding::Status::Unavailable(binding::Error::StaleRegistry)));
        let renewed = next_owners(&root, &stale);
        publish(&renewed, 5, "pluto_fixture:block/cell", Arc::new(|| true));
        assert_eq!(variant(&renewed), 5);
        println!("RELOAD_CASE stale_registry_requires_new_owners ok");

        let rejected = host::prepare(&base.assets, &base.render, &base.tables,
            Ok((vec![binding::Request { group: GROUP.into(), variant: 6,
                block: "pluto_fixture:cell".into(), properties: BTreeMap::new(),
                texture: "pluto_fixture:block/cell".into() }], Arc::new(|| true))));
        assert_eq!(rejected, Err(binding::CacheError::ConflictingSnapshot));
        assert_eq!(variant(&base), 1);
        println!("RELOAD_CASE same_owner_conflict_retains_original_snapshot ok");
    }
}
