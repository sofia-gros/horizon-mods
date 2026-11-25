#[path = "../../../mod/src/pluto-reload-registry-access.rs"]
#[allow(dead_code)]
mod access;

use access::{Resource, ResourceFolders};
use std::sync::Arc;

#[test]
fn effective_json_never_revives_lower_layers() {
    let lower = || Ok(br#"{"layer":1}"#.to_vec());
    let resources = vec![
        Resource { id: "test:valid".into(), layers: vec![lower(), Ok(br#"{"layer":2}"#.to_vec())] },
        Resource { id: "test:malformed".into(), layers: vec![lower(), Ok(b"{".to_vec())] },
        Resource { id: "test:unreadable".into(), layers: vec![lower(), Err("read failed".into())] },
        Resource { id: "test:empty".into(), layers: vec![] },
    ];
    let (files, errors) = access::effective_json(&resources);
    assert_eq!(files, vec![("test:valid".into(), serde_json::json!({"layer":2}))]);
    assert_eq!(errors.len(), 3);
    for id in ["test:malformed", "test:unreadable", "test:empty"] {
        assert!(errors.iter().any(|error| error.starts_with(id)));
    }
    // Effective selection must not consume the full stack needed by tag loaders.
    assert_eq!(resources[1].layers[0].as_ref().unwrap(), br#"{"layer":1}"#);
    assert!(resources[2].layers[1].is_err());
}

#[test]
fn nested_reload_unwind_restores_outer_owned_snapshot() {
    assert!(access::current().is_none());
    assert!(access::current_resources().is_none());
    let registries = Arc::new(vec![access::Registry {
        id: "minecraft:item".into(), entries: vec!["test:outer".into()],
        tags: vec![], item_defaults: Some(vec![]),
    }]);
    let folders = Arc::new(ResourceFolders::from([
        ("rules".into(), vec![Resource { id: "test:outer".into(), layers: vec![Ok(b"1".to_vec())] }]),
        ("empty".into(), vec![]),
    ]));
    let retained = access::with_folders(registries.clone(), Some(folders.clone()), || {
        let captured = access::current_resources().unwrap();
        assert!(captured["empty"].is_empty());
        assert!(!captured.contains_key("uncaptured"));
        let nested = std::panic::catch_unwind(|| {
            access::with_folders(Arc::new(vec![]), None, || {
                assert!(access::current().unwrap().is_empty());
                assert!(access::current_resources().is_none());
                panic!("listener failure");
            });
        });
        assert!(nested.is_err());
        assert!(Arc::ptr_eq(&access::current().unwrap(), &registries));
        assert!(Arc::ptr_eq(&access::current_resources().unwrap(), &folders));
        captured
    });
    assert!(access::current().is_none());
    assert!(access::current_resources().is_none());
    assert_eq!(retained["rules"][0].layers[0].as_ref().unwrap(), b"1");
}

