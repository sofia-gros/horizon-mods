// Adapted from pinned MIT Horizon game/native_reload_listeners.rs. License:
// pluto-lib/LICENSE.horizon. One pack stack supplies both normal top JSON and
// ordered complete resource stacks. Prepare owned bytes before callbacks, so a
// nested reload cannot replace the outer callback's registry/resource snapshot.
fn pluto_reload_registry(game:&mut Game<'_>)->std::ops::ControlFlow<()> {
    use sunrise_client::registries::pluto_reload_registry as access;
    let mut listeners=Vec::new();
    game.native_add_server_reload_listeners(&mut listeners);
    if listeners.is_empty() { return std::ops::ControlFlow::Break(()); }
    let authoritative=game.native_authoritative();
    let registries=if authoritative {
        crate::game::chat::pluto_reload_registry_source::snapshot(game).unwrap_or_default()
    } else { Vec::new() };
    let stack=functions::pack_stack(&game.world_root,&game.options.resources,game.cmds.vanilla_disabled,&game.level.enabled_features);
    let packs:Vec<&DataPack>=stack.builtin.iter().chain(stack.enabled_packs()).collect();
    let mut prepared=Vec::new();
    let mut folders=access::ResourceFolders::new();
    // Overlapping registrations (e.g. a root and one child directory) must use
    // the same captured bytes for a file, not reread it between preparations.
    let mut bytes_by_pack_file:std::collections::BTreeMap<(usize,String),Result<Vec<u8>,String>>=Default::default();
    for listener in listeners {
        if !folder_ok(&listener.folder) {
            log::warn!("Horizon: reload listener {} folder {:?} is not a folder under data/<namespace>/",listener.id,listener.folder);
            continue;
        }
        let resources=folders.entry(listener.folder.clone()).or_insert_with(|| {
            let mut layers:std::collections::BTreeMap<String,Vec<Result<Vec<u8>,String>>>=Default::default();
            for (pack_index,pack) in packs.iter().enumerate() {
                for file in pack.list_folder(&listener.folder) {
                    if let Some(id)=file_id(&file,&listener.folder) {
                        let bytes=bytes_by_pack_file.entry((pack_index,file.to_string())).or_insert_with(||
                            pack.read(&file).ok_or_else(||format!("unreadable {file} in pack {}",pack.id))).clone();
                        layers.entry(id).or_default().push(bytes);
                    }
                }
            }
            layers.into_iter().map(|(id,layers)|access::Resource { id,layers }).collect()
        });
        let (files,errors)=access::effective_json(resources);
        for error in errors { log::error!("Horizon: couldn't prepare JSON for {}: {error}",listener.id); }
        prepared.push((listener,files));
    }
    drop(packs);
    drop(stack);
    let registries=std::sync::Arc::new(registries);
    let resources=authoritative.then(||std::sync::Arc::new(folders));
    for (listener,files) in prepared {
        access::with_folders(registries.clone(),resources.clone(),|| {
            game.native_server_reload_listener_apply(&listener,&files);
        });
    }
    std::ops::ControlFlow::Break(())
}
