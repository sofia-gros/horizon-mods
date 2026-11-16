use pluto_lib::{ConfigEvents, ConfigKind, Identifier, ModConfig};
use serde_json::json;

#[test]
fn registered_material_loader_observes_config_and_each_real_reload_snapshot() {
    let state = crate::common::register();
    assert!(std::sync::Arc::ptr_eq(state, crate::common::register()));
    let notifications = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let observed = notifications.clone();
    let published = state.clone();
    crate::materials::MATERIALS_LOADED.register(move |event| {
        // Callbacks must be able to read the already-published state without deadlocking.
        let generation = match event.side {
            crate::materials::MaterialSide::Server => {
                let state = published.try_read().expect("notification holds no server write lock");
                assert!(state.server_loaded);
                state.generation
            }
            crate::materials::MaterialSide::RemoteClient => {
                let state = crate::material_sync::client_state().unwrap().try_read().expect("notification holds no client write lock");
                assert!(state.as_ref().unwrap().server_loaded);
                state.as_ref().unwrap().generation
            }
        };
        assert_eq!(event.generation, generation);
        observed.lock().unwrap().push(*event);
    });
    let listener = "tconstruct:material_definitions";
    let folder = crate::materials::FOLDER;
    let files = vec![
        ("tconstruct:wood".into(), json!({"craftable":true})),
        ("tconstruct:alias".into(), json!({"redirect":[{"id":"tconstruct:wood"}]})),
        ("tconstruct:conditional".into(), json!({"craftable":true,"condition":{"type":"forge:item_exists","item":"minecraft:stick"}})),
        ("tconstruct:tagged".into(), json!({"craftable":true,"condition":{"type":"mantle:tag_filled","tag":"forge:ingots/test"}})),
        ("tconstruct:forced".into(), json!({"craftable":true,"condition":{"type":"tconstruct:config","prop":"force_integration_materials"}})),
    ];
    let entries = vec!["minecraft:stick".into()];
    let tags = vec![("forge:ingots/test".into(), entries.clone())];
    let captured_tags = || [(crate::materials::TAG_FOLDER.into(), vec![pluto_lib::JsonResourceStack {
        id: Identifier::parse("tconstruct:test").unwrap(),
        layers: vec![Ok(json!({"values":["tconstruct:wood", {"id":"tconstruct:conditional","required":false}]}))],
    }]), (crate::material_traits::FOLDER.into(), vec![pluto_lib::JsonResourceStack {
        id: Identifier::parse("tconstruct:wood").unwrap(),
        layers: vec![Ok(json!({"default":[{"name":"tconstruct:cultivated"}],
            "perStat":{"tconstruct:melee_harvest":[{"name":"tconstruct:sturdy","level":2}]}}))],
    }]), (crate::material_stats::FOLDER.into(), vec![pluto_lib::JsonResourceStack {
        id: Identifier::parse("tconstruct:wood").unwrap(),
        layers: vec![Ok(json!({"stats":{"tconstruct:head":{"durability":20}}}))],
    }])].into_iter().collect();
    let apply = |items| pluto_lib::fixtures::server_json_reload_folders(listener, folder, &files, items, captured_tags());
    assert!(apply(Some((&entries, &tags))).is_err(), "config not loaded is not a default-valued context");
    let config = |text: &str| ModConfig { mod_id: "tconstruct".into(), kind: ConfigKind::Common,
        file: "tconstruct-common.toml".into(), text: text.into() };
    ConfigEvents::LOADING.invoke(&config(&crate::config::default_text()));
    assert!(apply(Some((&entries, &tags))).unwrap());
    let first = state.read().unwrap().clone();
    assert_eq!(first.materials.len(), 3);
    assert!(first.problems.is_empty());
    assert!(first.server_loaded);
    crate::common::with_client_materials(|view| {
        assert_eq!(view.unwrap().generation, first.generation, "integrated client shares loaded server materials without loopback");
    });
    let part = crate::tool_materials::ToolPart {
        item: Identifier::parse("tconstruct:pick_head").unwrap(), stat: Identifier::parse("tconstruct:head").unwrap(),
    };
    let local_variants = part.client_variants("");
    assert_eq!(local_variants.len(), 1);
    assert_eq!(crate::tool_materials::part_material(&local_variants[0].custom_data).to_string(), "tconstruct:wood");
    assert_eq!(crate::tool_materials::part_material(&part.client_variants("tconstruct:wood#grain")[0].custom_data).to_string(), "tconstruct:wood#grain");
    assert_eq!(part.client_variants("invalid filter").len(), 1, "invalid filter chooses first eligible visible material");
    let mut unchanged = local_variants[0].clone();
    assert!(!part.set_material(&mut unchanged, &crate::tool_materials::MaterialVariant::parse("tconstruct:alias").unwrap(), &first));
    assert_eq!(unchanged, local_variants[0], "part validation uses actual stats, not aliases/default stats");
    let deliver = |snapshot: &crate::materials::Definitions| {
        let bytes = snapshot.snapshot.as_ref().unwrap();
        let before = notifications.lock().unwrap().clone();
        let cut = bytes.len() / 2;
        for (offset, data) in [(0, &bytes[..cut]), (cut, &bytes[cut..])] {
            let mut packet = Vec::new();
            packet.extend_from_slice(&snapshot.generation.to_be_bytes());
            packet.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
            packet.extend_from_slice(&(offset as u32).to_be_bytes());
            packet.extend_from_slice(data);
            pluto_lib::fixtures::client_payload("tconstruct:materials/v1", &packet);
            if offset == 0 {
                assert!(crate::material_sync::client_state().unwrap().read().unwrap().is_none());
                assert!(part.client_variants("").is_empty(), "part listing cannot read local materials during remote transfer");
                assert_eq!(*notifications.lock().unwrap(), before, "partial transfers cannot announce completion");
            }
        }
    };
    pluto_lib::ClientPlayConnectionEvents::JOIN.invoke(&());
    crate::common::with_client_materials(|view| assert!(view.is_none(), "remote join cannot expose local materials"));
    deliver(&first);
    crate::common::with_client_materials(|view| assert_eq!(view.unwrap().generation, first.generation));
    assert_eq!(*notifications.lock().unwrap(), vec![
        crate::materials::MaterialsLoaded { side: crate::materials::MaterialSide::Server, generation: first.generation },
        crate::materials::MaterialsLoaded { side: crate::materials::MaterialSide::RemoteClient, generation: first.generation },
    ]);
    let client = crate::material_sync::client_state().unwrap();
    assert_eq!(client.read().unwrap().as_ref().unwrap().stats, first.stats);
    assert_eq!(part.client_variants(""), local_variants, "remote part list uses the published material/stat snapshot");
    crate::common::with_client_materials(|view| {
        let materials = crate::tool_materials::MaterialList::from_nbt(Some(&pluto_lib::Nbt::List(vec![
            local_variants[0].custom_data["Material"].clone(),
        ])));
        let mut stats = crate::tool_stats::Builder::default();
        let parts = [(part.item.clone(), part.clone())].into_iter().collect();
        let module = crate::tool_stats::StatModule::decode(&json!({"type":"tconstruct:part_stats", "parts":[{"item":"tconstruct:pick_head", "scale":2}]}), &parts).unwrap();
        module.apply(&mut stats, &materials, view.unwrap());
        assert_eq!(stats.build()[&crate::tool_stats::Stat::Durability], 40.0,
            "decoded part module, part NBT and received stats feed scaled material calculation");
        for module in [
            json!({"type":"tconstruct:base_stats", "stats":{"tconstruct:durability":10}}),
            json!({"type":"tconstruct:multiply_stats", "multipliers":{"tconstruct:durability":2}}),
        ] {
            crate::tool_stats::StatModule::decode(&module, &parts).unwrap().apply(&mut stats, &materials, view.unwrap());
        }
        assert_eq!(stats.build()[&crate::tool_stats::Stat::Durability], 100.0, "base stats sum with materials before global multiplication");
    });
    pluto_lib::ClientPlayConnectionEvents::JOIN.invoke(&());
    assert!(client.read().unwrap().is_none(), "reconfiguration login also clears the previous connection generation");
    deliver(&first);
    let wood = Identifier::parse("tconstruct:wood").unwrap();
    let head = Identifier::parse("tconstruct:head").unwrap();
    let alias = Identifier::parse("tconstruct:alias").unwrap();
    assert!(first.get_stats(&wood, &head).is_some());
    assert!(first.get_stats(&alias, &head).is_none(), "stats do not resolve redirects implicitly");
    assert!(first.get_traits(&alias, &head).is_empty());
    assert_eq!(first.first_with_stat(&head).unwrap().id, wood);
    assert!(matches!(first.stats_or_default(&wood, &head), Some(std::borrow::Cow::Borrowed(_))));
    assert!(matches!(first.stats_or_default(&alias, &head).as_deref(), Some(crate::material_stats::Stats::Head { durability:1, .. })));
    assert!(first.stats_or_default(&wood, &Identifier::parse("test:unknown").unwrap()).is_none());
    assert_eq!(first.traits[&wood].get(&Identifier::parse("tconstruct:head").unwrap())[0].as_ref().unwrap().level, 2);
    let tag_id = Identifier::parse("tconstruct:test").unwrap();
    assert_eq!(first.tags[&tag_id].len(), 2);
    assert!(first.reverse_tags[&Identifier::parse("tconstruct:wood").unwrap()].contains(&tag_id));
    ConfigEvents::RELOADING.invoke(&config("[debug]\nforceIntegrationMaterials = true\n"));
    assert!(apply(Some((&[], &[]))).unwrap());
    let second = state.read().unwrap().clone();
    crate::common::with_client_materials(|view| assert_eq!(view.unwrap().generation, first.generation, "remote view does not read a different local generation"));
    assert_eq!(second.materials.len(), 2);
    assert_eq!(second.tags[&tag_id].len(), 1, "tag membership uses the same generation as definitions");
    assert!(second.materials.contains_key(&Identifier::parse("tconstruct:forced").unwrap()));
    assert!(first.materials.contains_key(&Identifier::parse("tconstruct:conditional").unwrap()), "owned old snapshot is unchanged");
    assert!(apply(None).unwrap());
    assert_eq!(state.read().unwrap().problems.len(), 2, "missing registry is an error, not false membership");
    let before_failed_reload = notifications.lock().unwrap().clone();
    ConfigEvents::RELOADING.invoke(&config("[debug]\nforceIntegrationMaterials = 'bad'\n"));
    assert!(apply(Some((&entries, &tags))).is_err());
    assert!(state.read().unwrap().materials.is_empty(), "failed context cannot retain earlier materials");
    deliver(&state.read().unwrap());
    assert!(!client.read().unwrap().as_ref().unwrap().server_loaded, "failed server reload invalidates the remote snapshot too");
    crate::common::with_client_materials(|view| assert!(view.is_none(), "failed remote generation is unavailable"));
    ConfigEvents::RELOADING.invoke(&config(""));
    assert!(pluto_lib::fixtures::server_json_reload(listener, folder, &files, Some((&entries, &tags))).is_err(),
        "uncaptured folder is not an empty folder");
    assert!(state.read().unwrap().tags.is_empty());
    assert!(pluto_lib::fixtures::server_json_reload_folders(listener, folder, &files, Some((&entries, &tags)),
        [(crate::materials::TAG_FOLDER.into(), vec![])].into_iter().collect()).is_err(), "missing traits capture fails closed");
    assert!(state.read().unwrap().traits.is_empty());
    assert!(pluto_lib::fixtures::server_json_reload_folders(listener, folder, &files, Some((&entries, &tags)),
        [(crate::materials::TAG_FOLDER.into(), vec![]), (crate::material_traits::FOLDER.into(), vec![])].into_iter().collect()).is_err(), "stats capture is also mandatory");
    assert!(!state.read().unwrap().server_loaded);
    assert!(state.read().unwrap().stats.is_empty());
    assert_eq!(*notifications.lock().unwrap(), before_failed_reload, "invalid generations never announce completion on either side");
    assert!(pluto_lib::fixtures::server_json_reload_folders(listener, folder, &[], Some((&entries, &tags)),
        [(crate::materials::TAG_FOLDER.into(), vec![]), (crate::material_traits::FOLDER.into(), vec![]),
         (crate::material_stats::FOLDER.into(), vec![])].into_iter().collect()).unwrap());
    assert!(state.read().unwrap().server_loaded, "empty captured folders are a completed reload");
    assert!(state.read().unwrap().first_with_stat(&head).is_none(), "first material lookup cannot leak an old generation");
    assert!(state.read().unwrap().stats.is_empty());
    assert!(state.read().unwrap().traits.is_empty(), "removed traits clear their generation");
    assert!(!first.traits.is_empty(), "owned old snapshot stays immutable");
    assert!(state.read().unwrap().materials.is_empty(), "removed resources clear their generation");
    deliver(&state.read().unwrap());
    assert!(client.read().unwrap().as_ref().unwrap().server_loaded);
    assert!(client.read().unwrap().as_ref().unwrap().materials.is_empty());
    pluto_lib::ClientPlayConnectionEvents::DISCONNECT.invoke(&());
    assert!(client.read().unwrap().is_none());
    assert!(state.read().unwrap().tags.is_empty());
    crate::common::with_client_materials(|view| assert!(view.is_some(), "local view uses the completed empty server generation"));
    let tools = crate::tool_definitions::register();
    let tool = Identifier::parse("tconstruct:pickaxe").unwrap();
    let tool_reload = |files: &[(String, serde_json::Value)]| {
        pluto_lib::fixtures::server_json_reload("tconstruct:tool_definitions", crate::tool_definitions::FOLDER, files, None).unwrap()
    };
    assert!(tool_reload(&[(tool.to_string(), json!({"modules":[
        {"type":"tconstruct:base_stats","stats":{"tconstruct:durability":12}},
        {"type":"tconstruct:multiply_stats","multipliers":{"tconstruct:durability":3}},
        {"type":"tconstruct:tool_actions","tool_actions":["axe_strip"]},
        {"type":"tconstruct:modifier_slots","slots":{"upgrades":3}},
        {"type":"tconstruct:traits","traits":[{"name":"tconstruct:sturdy"}]},
        {"type":"tconstruct:material_traits","stat_type":"tconstruct:head","material_index":0},
        {"type":"tconstruct:volatile_flag","flag":"test:loaded"},
        {"type":"tconstruct:volatile_int","flag":"test:loaded","value":2}
    ]})), ("test:unregistered".into(), json!({}))]));
    let original_tools = tools.read().unwrap().clone();
    assert_eq!(original_tools.data.len(), 1, "unregistered JSON cannot create a tool definition");
    assert!(original_tools.data[&tool].allows_action("axe_strip"));
    assert_eq!(original_tools.data[&tool].build_volatile()["test:loaded"], pluto_lib::Nbt::Int(3));
    assert_eq!(original_tools.data[&tool].build_volatile()["upgrades"], pluto_lib::Nbt::Int(3));
    assert_eq!(original_tools.data[&tool].build_stats(&crate::tool_materials::MaterialList::default(), &first)
        .get(crate::tool_stats::Stat::Durability), 36.0);
    assert!(tool_reload(&[(tool.to_string(), json!({"modules":[
        {"type":"tconstruct:base_stats","stats":{"tconstruct:durability":99}},
        {"type":"tconstruct:harvest_logic"}
    ]}))]));
    assert!(!tools.read().unwrap().data.contains_key(&tool), "incomplete hooks invalidate the entire definition, not only one module");
    assert!(original_tools.data.contains_key(&tool), "owned earlier generation remains immutable");
    assert!(tool_reload(&[(tool.to_string(), json!({}))]));
    assert!(tools.read().unwrap().data.contains_key(&tool), "loaded empty definition differs from missing data");
    assert!(!tools.read().unwrap().data[&tool].allows_action("axe_strip"), "reload removes old action grants");
    assert!(tool_reload(&[]));
    assert!(tools.read().unwrap().data.is_empty(), "removed resources clear previous definitions");
    assert!(tool_reload(&[(tool.to_string(), json!({}))]));
    let modifiers = crate::common::server_modifiers().unwrap();
    let modifier_reload = |files: &[(String, serde_json::Value)]| {
        pluto_lib::fixtures::server_json_reload_folders("tconstruct:modifiers", crate::modifiers::FOLDER, files, Some((&entries, &tags)),
            [(crate::modifiers::TAG_FOLDER.into(), vec![pluto_lib::JsonResourceStack {
                id: Identifier::parse("test:traits").unwrap(), layers: vec![Ok(json!({"values":[{"id":"tconstruct:sturdy","required":false}]}))],
            }])].into_iter().collect())
    };
    assert!(modifier_reload(&[("tconstruct:sturdy".into(), json!({"modules":[
        {"type":"tconstruct:volatile_flag","flag":"test:material_trait","modifier_level":{"min":2}}
    ]}))]).unwrap());
    assert_eq!(modifiers.read().unwrap().tags[&Identifier::parse("test:traits").unwrap()], vec![Identifier::parse("tconstruct:sturdy").unwrap()]);
    let tool_materials = crate::tool_materials::MaterialList::from_nbt(Some(&pluto_lib::Nbt::List(vec![
        local_variants[0].custom_data["Material"].clone(),
    ])));
    let traits = original_tools.data[&tool].traits(&tool_materials, &first).unwrap();
    let upgrades = crate::modifier_entries::List::default().add_amount(Identifier::parse("tconstruct:sturdy").unwrap(), 1, 4);
    let resolved = modifiers.read().unwrap().resolve_with_upgrades(&upgrades, &traits).unwrap();
    assert_eq!(resolved[0].0.level, 4);
    assert_eq!(resolved[0].0.effective_level(), 3.25, "saved partial upgrades merge with definition and captured material traits");
    let mut volatile = original_tools.data[&tool].build_volatile();
    for (entry, modifier) in &resolved { modifier.apply_volatile(entry, &mut volatile); }
    assert_eq!(volatile["test:material_trait"], pluto_lib::Nbt::Byte(1));
    assert_eq!(volatile["upgrades"], pluto_lib::Nbt::Int(3), "modifier contributions follow definition contributions");
    assert!(modifier_reload(&[("tconstruct:sturdy".into(), json!({"modules":[{"type":"test:unsupported"}]}))]).unwrap());
    assert!(modifiers.read().unwrap().resolve(&traits).is_err(), "unimplemented required effects cannot become empty modifiers");
    assert!(modifier_reload(&[]).unwrap());
    assert!(modifiers.read().unwrap().definitions_loaded, "an empty reload still completes definitions");
    assert!(modifiers.read().unwrap().tags[&Identifier::parse("test:traits").unwrap()].is_empty());
    assert!(modifiers.read().unwrap().reverse_tags.is_empty());
    assert!(pluto_lib::fixtures::server_json_reload("tconstruct:modifiers", crate::modifiers::FOLDER, &[], Some((&entries, &tags))).is_err());
    assert!(!modifiers.read().unwrap().definitions_loaded, "missing tag capture is not an empty tag folder");
    assert!(modifiers.read().unwrap().tags.is_empty());
    ConfigEvents::RELOADING.invoke(&config("[debug]\nforceIntegrationMaterials = 'bad'\n"));
    assert!(modifier_reload(&[]).is_err());
    assert!(!modifiers.read().unwrap().definitions_loaded, "failed context clears readiness and old data");
    ConfigEvents::RELOADING.invoke(&config(""));
    assert!(modifier_reload(&[]).unwrap());
    assert!(crate::common::item_tags_loaded(), "a reload that captured the item registry vouches for item tags");
    let before_stop = notifications.lock().unwrap().clone();
    pluto_lib::ServerLifecycleEvents::SERVER_STOPPED.invoke(&pluto_lib::ServerInfo { dedicated: false });
    crate::common::with_client_materials(|view| assert!(view.is_none(), "world teardown cannot leak materials into the next world"));
    assert!(!state.read().unwrap().server_loaded);
    assert!(state.read().unwrap().snapshot.is_none());
    assert_eq!(*notifications.lock().unwrap(), before_stop, "teardown is not completion");
    assert!(tools.read().unwrap().data.is_empty(), "server teardown also clears tool definitions");
    assert!(!modifiers.read().unwrap().definitions_loaded, "server teardown clears modifier readiness");
    assert!(!crate::common::item_tags_loaded(), "the item snapshot is gone after teardown");
}
