//! Counterpart of Fabric's `fabric-resource-loader-v0`: be told when resources reload.
//!
//! Fabric: `ResourceManagerHelper.get(ResourceType.CLIENT_RESOURCES).registerReloadListener(listener)`.
//! Here: `ResourceManagerHelper::get(ResourceType::ClientResources).register_reload_listener(id, listener)`.

use super::base::{Event, Identifier};
use std::sync::{Arc, Mutex};

/// A decoded client resource image: top-first, gamma-encoded, non-premultiplied RGBA pixels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceImage {
    /// Width in pixels, greater than zero.
    pub width: u32,
    /// Height in pixels, greater than zero.
    pub height: u32,
    /// Exactly width * height * 4 bytes, rows top first, RGBA without premultiplication.
    pub rgba: Vec<u8>,
}

pub(crate) struct ClientResourceBackend {
    pub read: Arc<dyn Fn(&str) -> Option<Vec<u8>> + Send + Sync>,
    pub image: Arc<dyn Fn(&str) -> Option<ResourceImage> + Send + Sync>,
}

static CLIENT_BACKEND: Mutex<Option<Arc<ClientResourceBackend>>> = Mutex::new(None);

pub(crate) fn set_client_resource_backend(backend: Option<ClientResourceBackend>) {
    let replacement = backend.map(Arc::new);
    let previous = match CLIENT_BACKEND.lock() {
        Ok(mut current) => std::mem::replace(&mut *current, replacement),
        Err(_) => return,
    };
    // Captured backend values may have their own destructors; drop them outside the mutex too.
    drop(previous);
}

fn client_backend() -> Option<Arc<ClientResourceBackend>> {
    CLIENT_BACKEND.lock().ok()?.clone()
}

fn plain_resource_id(id: &Identifier) -> bool {
    !matches!(id.namespace(), "." | "..")
        && id.path().split('/').all(|part| !part.is_empty() && part != "." && part != "..")
}

/// Which kind of resources a listener follows (Fabric: `ResourceType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceType {
    /// Assets: models, textures, language files. Reloaded on the client.
    ClientResources,
    /// Data: recipes, loot tables, tags. Called after the server's initial data load and subsequent reloads.
    ServerData,
}

static CLIENT_RELOAD: Event<(), ()> = Event::void();
static SERVER_RELOAD: Event<(), ()> = Event::void();

type JsonApply = Arc<dyn Fn(&[(Identifier, serde_json::Value)]) -> Result<(), String> + Send + Sync>;
#[derive(Clone)]
struct JsonListener { kind: ResourceType, id: Identifier, directory: String, apply: JsonApply }
static JSON_LISTENERS: Mutex<Vec<JsonListener>> = Mutex::new(Vec::new());

fn plain_directory(directory: &str) -> bool {
    !directory.is_empty() && !directory.contains(['\\', ':'])
        && directory.split('/').all(|p| !matches!(p, "" | "." | ".."))
}

pub(crate) fn json_reload_listeners(kind: ResourceType) -> Result<Vec<(Identifier,String)>,String> {
    let listeners = JSON_LISTENERS.lock().map_err(|_| "JSON reload registry poisoned")?;
    Ok(listeners.iter().filter(|l| l.kind==kind).map(|l| (l.id.clone(),l.directory.clone())).collect())
}

/// Native files already have pack precedence/JSON parsing applied. Validate the
/// WHOLE batch before invoking a copied callback, with no registry lock held.
pub(crate) fn apply_json_reload_listener(kind: ResourceType, id: &str, directory: &str,
    files: &[(String,serde_json::Value)]) -> Result<bool,String> {
    let listener = JSON_LISTENERS.lock().map_err(|_| "JSON reload registry poisoned")?
        .iter().find(|l| l.kind==kind && l.id.to_string()==id).cloned();
    let Some(listener)=listener else { return Ok(false) };
    if listener.directory!=directory { return Err("JSON listener directory mismatch".into()); }
    let mut seen=std::collections::BTreeSet::new();
    let files=files.iter().map(|(name,json)| {
        let name=Identifier::parse(name).map_err(|e| format!("invalid JSON resource ID: {e}"))?;
        if !plain_resource_id(&name) { return Err("non-plain JSON resource ID".into()); }
        if !seen.insert(name.clone()) { return Err("duplicate effective JSON resource ID".into()); }
        Ok((name,json.clone()))
    }).collect::<Result<Vec<_>,String>>()?;
    (listener.apply)(&files)?;
    Ok(true)
}

/// Owned registry membership and tags from the current server reload.
/// Sets deliberately make no registry-order or random-selection guarantees.
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct RegistryLookup {
    entries: std::collections::BTreeSet<Identifier>,
    tags: std::collections::BTreeMap<Identifier,std::collections::BTreeSet<Identifier>>,
}
impl RegistryLookup {
    /// Whether the ID exists in this captured registry (not just a render catalog).
    pub fn contains(&self,id:&Identifier)->bool { self.entries.contains(id) }
    pub(crate) fn entry_count(&self)->usize { self.entries.len() }
    /// A copy with extra tag members added: each `(tag, members)` joins the captured tag of that id (or makes a new tag). Members that
    /// are not entries of this registry are left out, as an optional tag entry is. The snapshot itself is not changed.
    pub fn with_extra_tags(&self,extra:&[(Identifier,Vec<Identifier>)])->Self {
        let mut result=self.clone();
        for (tag,members) in extra {
            let present:Vec<Identifier>=members.iter().filter(|id|self.entries.contains(id)).cloned().collect();
            if present.is_empty() { continue; }
            result.tags.entry(tag.clone()).or_default().extend(present);
        }
        result
    }
    /// Members of a captured tag; None means absent, Some(empty) means present but empty.
    pub fn get_tag(&self,tag:&Identifier)->Option<&std::collections::BTreeSet<Identifier>> { self.tags.get(tag) }
    pub(crate) fn from_parts(entries:&[String],tags:&[(String,Vec<String>)])->Result<Self,String> {
        let parse=|id:&str| Identifier::parse(id).map_err(|e|format!("invalid registry snapshot ID: {e}"));
        let entries=entries.iter().map(|id|parse(id)).collect::<Result<std::collections::BTreeSet<_>,_>>()?;
        let mut parsed=std::collections::BTreeMap::new();
        for (tag,members) in tags {
            let tag=parse(tag)?;
            let members=members.iter().map(|id|parse(id)).collect::<Result<std::collections::BTreeSet<_>,_>>()?;
            if !members.is_subset(&entries) { return Err("registry tag contains unknown members".into()); }
            if parsed.insert(tag,members).is_some() { return Err("duplicate registry tag".into()); }
        }
        Ok(Self { entries,tags:parsed })
    }
}
type ReloadRegistries=std::collections::BTreeMap<Identifier,RegistryLookup>;
/// Registry-specific invalid data makes ONLY that registry unavailable. Broken
/// registry IDs or duplicate registries reject the whole ambiguous context.
pub(crate) fn decode_reload_registries<'a>(records:impl IntoIterator<Item=(&'a str,&'a [String],&'a [(String,Vec<String>)])>)
    ->Result<(ReloadRegistries,Vec<String>),String> {
    let mut seen=std::collections::BTreeSet::new();
    let mut views=ReloadRegistries::new();
    let mut errors=Vec::new();
    for (registry,entries,tags) in records {
        let id=Identifier::parse(registry).map_err(|e|e.to_string())?;
        if !seen.insert(id.clone()) { return Err("duplicate reload registry".into()); }
        match RegistryLookup::from_parts(entries,tags) {
            Ok(lookup)=>{ views.insert(id,lookup); },
            Err(error)=>errors.push(format!("{registry}: {error}")),
        }
    }
    Ok((views,errors))
}
/// One resource ID's complete JSON layers, in bottom-to-top pack order.
/// Errors retain their layer position; no lower layer silently replaces them.
#[derive(Clone,Debug,PartialEq)]
pub struct JsonResourceStack {
    pub id:Identifier,
    pub layers:Vec<Result<serde_json::Value,String>>,
}
pub(crate) fn decode_json_stacks<'a>(records:impl IntoIterator<Item=(&'a str,&'a [Result<Vec<u8>,String>])>)
    ->Result<Vec<JsonResourceStack>,String> {
    let mut seen=std::collections::BTreeSet::new();
    let mut stacks=Vec::new();
    for (name,layers) in records {
        let id=Identifier::parse(name).map_err(|e|e.to_string())?;
        if !plain_resource_id(&id) || !seen.insert(id.clone()) || layers.is_empty() { return Err("invalid/duplicate resource ID or empty layer stack".into()); }
        let layers=layers.iter().enumerate().map(|(index,bytes)|match bytes {
            Ok(bytes)=>serde_json::from_slice(bytes).map_err(|e|format!("{name} layer {index}: {e}")),
            Err(error)=>Err(format!("{name} layer {index}: {error}")),
        }).collect();
        stacks.push(JsonResourceStack { id,layers });
    }
    Ok(stacks)
}
type ReloadFolders=std::collections::BTreeMap<String,Vec<JsonResourceStack>>;
struct ReloadContext {
    registries:Arc<ReloadRegistries>,
    resources:Arc<ReloadFolders>,
}
thread_local! { static RELOAD_REGISTRIES: std::cell::RefCell<Vec<ReloadContext>> = const { std::cell::RefCell::new(Vec::new()) }; }
pub(crate) fn with_reload_folders<T>(registries:ReloadRegistries,resources:ReloadFolders,action:impl FnOnce()->T)->T {
    struct Scope;
    impl Drop for Scope { fn drop(&mut self) { RELOAD_REGISTRIES.with(|v| { v.borrow_mut().pop(); }); } }
    RELOAD_REGISTRIES.with(|v| v.borrow_mut().push(ReloadContext { registries:Arc::new(registries),resources:Arc::new(resources) }));
    let _scope=Scope;
    action()
}
pub(crate) fn with_reload_context<T>(registries:ReloadRegistries,resources:Option<(String,Vec<JsonResourceStack>)>,action:impl FnOnce()->T)->T {
    with_reload_folders(registries,resources.into_iter().collect(),action)
}
pub(crate) fn with_reload_registries<T>(registries:ReloadRegistries,action:impl FnOnce()->T)->T {
    with_reload_context(registries,None,action)
}

/// Fabric: `ResourceManagerHelper`.
pub struct ResourceManagerHelper {
    resource_type: ResourceType,
}

impl ResourceManagerHelper {
    /// Read the effective `assets/<namespace>/<path>` resource, honoring the host's active pack/mod overlay.
    /// None before client initialization, for a non-plain resource path, or when no file can be read.
    pub fn read_client_resource(id: &Identifier) -> Option<Vec<u8>> {
        if !plain_resource_id(id) { return None; }
        let backend = client_backend()?;
        (backend.read)(&format!("assets/{}/{}", id.namespace(), id.path()))
    }

    /// Decode `namespace:textures/...png` from the effective client resources. The path includes `textures/` and `.png`.
    /// Rows remain top-first and colors non-premultiplied; missing/malformed images or inconsistent dimensions return None.
    pub fn read_client_image(id: &Identifier) -> Option<ResourceImage> {
        // The native texture decoder reserves this namespace for OptiFine CIT paths,
        // not assets/horizon_cit/textures. Never silently read a different resource.
        if !plain_resource_id(id) || id.namespace() == "horizon_cit" { return None; }
        let path = id.path().strip_prefix("textures/")?.strip_suffix(".png")?;
        if path.is_empty() { return None; }
        let backend = client_backend()?;
        let image = (backend.image)(&format!("{}:{path}", id.namespace()))?;
        if image.width == 0 || image.height == 0 { return None; }
        let bytes = usize::try_from(image.width).ok()?
            .checked_mul(usize::try_from(image.height).ok()?)?.checked_mul(4)?;
        (image.rgba.len() == bytes).then_some(image)
    }

    /// Fabric: `ResourceManagerHelper.get(type)`.
    pub fn get(resource_type: ResourceType) -> Self {
        Self { resource_type }
    }

    /// Capture registry/tag membership during a server JSON reload callback.
    /// None outside the callback or for an unsupported/unavailable registry.
    /// Native source supplies minecraft:entity_type and minecraft:item; native
    /// plan/build/game validation remains pending the native attachment gate.
    pub fn reload_registry(registry:&Identifier)->Option<RegistryLookup> {
        RELOAD_REGISTRIES.with(|v| v.borrow().last()?.registries.get(registry).cloned())
    }

    /// Complete JSON stacks for an exact directory captured in this server
    /// reload pass. All registered folders are prepared before any callback;
    /// reading another captured folder does not depend on listener order. None
    /// outside a supported callback, for uncaptured/unsafe/unavailable folders;
    /// Some(empty) is a genuinely captured empty folder.
    /// Layers are owned, bottom pack first, including explicit per-layer errors.
    /// Native source transport exists but is not plan/build/game validated.
    pub fn reload_json_stacks(directory:&str)->Option<Vec<JsonResourceStack>> {
        if !plain_directory(directory) { return None; }
        RELOAD_REGISTRIES.with(|v| {
            let context=v.borrow();
            context.last()?.resources.get(directory).cloned()
        })
    }

    /// Register a SimpleJsonResourceReloadListener-style folder reader.
    /// Each effective ID appears once; callbacks receive owned decoded JSON in
    /// backend order, including an empty batch when all resources disappear.
    /// Duplicate IDs within one resource type and unsafe directories are errors.
    /// Server registration/application forwards existing native API 400 in source;
    /// native build/game validation is pending. Client declarations are currently
    /// inert (no native client folder-discovery adapter), not server fallbacks.
    pub fn register_json_reload_listener(&self, id: Identifier, directory: &str,
        apply: impl Fn(&[(Identifier,serde_json::Value)]) -> Result<(),String> + Send + Sync + 'static) -> Result<(),String> {
        if !plain_resource_id(&id) || !plain_directory(directory) { return Err("invalid JSON listener ID/directory".into()); }
        let mut listeners=JSON_LISTENERS.lock().map_err(|_| "JSON reload registry poisoned")?;
        if listeners.iter().any(|l| l.kind==self.resource_type && l.id==id) { return Err(format!("duplicate JSON reload listener {id}")); }
        listeners.push(JsonListener { kind:self.resource_type, id, directory:directory.into(), apply:Arc::new(apply) });
        Ok(())
    }

    /// Fabric: `registerReloadListener`. The listener runs after every reload of this kind of resources. The identifier
    /// names the listener's phase, so listeners can be ordered with `Event::add_phase_ordering` on [`reload_event`].
    pub fn register_reload_listener(&self, id: Identifier, listener: impl Fn() + Send + Sync + 'static) {
        reload_event(self.resource_type).register_with_phase(id, move |_| listener());
    }
}

/// The event behind the listeners of a kind of resources.
pub fn reload_event(resource_type: ResourceType) -> &'static Event<(), ()> {
    match resource_type {
        ResourceType::ClientResources => &CLIENT_RELOAD,
        ResourceType::ServerData => &SERVER_RELOAD,
    }
}

pub(crate) fn client_resources_reloaded() {
    CLIENT_RELOAD.invoke(&());
}

pub(crate) fn server_data_reloaded() {
    SERVER_RELOAD.invoke(&());
}

#[cfg(test)]
#[path = "../../../pluto-reload-registry-access.rs"]
mod native_registry_scope;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn client_resource_reads_validate_paths_images_and_refresh_without_locked_io() {
        let file = Identifier::parse("example:notes/test.txt").unwrap();
        let image = Identifier::parse("example:textures/colormap/test.png").unwrap();
        set_client_resource_backend(None);
        assert_eq!(ResourceManagerHelper::read_client_resource(&file), None);
        assert_eq!(ResourceManagerHelper::read_client_image(&image), None);

        set_client_resource_backend(Some(ClientResourceBackend {
            read: Arc::new(|path| {
                assert_eq!(path, "assets/example/notes/test.txt");
                // Replacing the backend inside I/O would deadlock under a held global lock.
                set_client_resource_backend(Some(ClientResourceBackend {
                    read: Arc::new(|_| Some(vec![])),
                    image: Arc::new(|_| None),
                }));
                Some(vec![1, 2, 3])
            }),
            image: Arc::new(|_| None),
        }));
        assert_eq!(ResourceManagerHelper::read_client_resource(&file), Some(vec![1, 2, 3]));
        assert_eq!(ResourceManagerHelper::read_client_resource(&file), Some(vec![]), "new reads use the refreshed backend; an empty file is valid");
        assert_eq!(ResourceManagerHelper::read_client_image(&image), None, "missing/undecodable textures are not fabricated");

        set_client_resource_backend(Some(ClientResourceBackend {
            read: Arc::new(|_| panic!("invalid path reached resource I/O")),
            image: Arc::new(|_| panic!("invalid image id reached decoder")),
        }));
        for path in ["../secret", "/secret", "notes//test.txt", "notes/./test.txt"] {
            let id = Identifier::new("example", path).unwrap();
            assert_eq!(ResourceManagerHelper::read_client_resource(&id), None);
            assert_eq!(ResourceManagerHelper::read_client_image(&id), None);
        }
        assert_eq!(ResourceManagerHelper::read_client_resource(&Identifier::new("..", "secret").unwrap()), None);
        assert_eq!(ResourceManagerHelper::read_client_image(&Identifier::new("horizon_cit", "textures/test.png").unwrap()), None);
        for path in ["colormap/test.png", "textures/test.jpg", "textures/.png"] {
            assert_eq!(ResourceManagerHelper::read_client_image(&Identifier::new("example", path).unwrap()), None);
        }

        let pixels = ResourceImage { width: 1, height: 2, rgba: vec![200, 100, 50, 128, 0, 0, 255, 255] };
        for candidate in [
            pixels.clone(),
            ResourceImage { width: 0, height: 2, rgba: vec![] },
            ResourceImage { width: 1, height: 0, rgba: vec![] },
            ResourceImage { width: 1, height: 2, rgba: vec![0; 7] },
            ResourceImage { width: 1, height: 2, rgba: vec![0; 9] },
            ResourceImage { width: u32::MAX, height: u32::MAX, rgba: vec![] },
        ] {
            let expected = (candidate == pixels).then(|| pixels.clone());
            set_client_resource_backend(Some(ClientResourceBackend {
                read: Arc::new(|_| None),
                image: Arc::new(move |name| {
                    assert_eq!(name, "example:colormap/test");
                    set_client_resource_backend(None);
                    Some(candidate.clone())
                }),
            }));
            assert_eq!(ResourceManagerHelper::read_client_image(&image), expected, "top-first non-premultiplied pixels are kept; inconsistent dimensions are rejected");
            assert_eq!(ResourceManagerHelper::read_client_image(&image), None, "the decoder may refresh/remove the backend without a lock deadlock");
        }
        set_client_resource_backend(None);
    }

    #[test]
    fn json_folder_listeners_validate_batches_keep_snapshots_and_allow_reentrant_registration() {
        use serde_json::json;
        let server=ResourceManagerHelper::get(ResourceType::ServerData);
        let id=Identifier::parse("test:json_folder").unwrap();
        let seen=Arc::new(Mutex::new(Vec::new()));
        let captured=seen.clone();
        server.register_json_reload_listener(id.clone(),"custom/data",move |files| {
            let mut seen=captured.lock().unwrap();
            if seen.is_empty() {
                ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
                    Identifier::parse("test:json_nested").unwrap(),"nested", |_| Ok(()))?;
            }
            seen.push(files.to_vec()); Ok(())
        }).unwrap();
        let before=json_reload_listeners(ResourceType::ServerData).unwrap();
        assert!(before.contains(&(id.clone(),"custom/data".into())));
        assert!(!before.iter().any(|(id,_)| id.to_string()=="test:json_nested"));
        assert!(server.register_json_reload_listener(id.clone(),"wrong", |_| panic!("must not replace")).is_err());
        ResourceManagerHelper::get(ResourceType::ClientResources).register_json_reload_listener(
            id.clone(),"custom/data", |_| panic!("server batches must not call client listener")).unwrap();
        let original=vec![("example:first".into(),json!({"value":1})),("other:sub/second".into(),json!([1,2]))];
        assert!(apply_json_reload_listener(ResourceType::ServerData,"test:json_folder","custom/data",&original).unwrap());
        assert_eq!(seen.lock().unwrap()[0][0].1,json!({"value":1}));
        assert!(json_reload_listeners(ResourceType::ServerData).unwrap().iter().any(|(id,_)|id.to_string()=="test:json_nested"));
        assert!(!before.iter().any(|(id,_)|id.to_string()=="test:json_nested"),"enumerations are owned snapshots");
        for files in [vec![("example:first".into(),json!(1)),("bad space:id".into(),json!(2))],
            vec![("example:first".into(),json!(1)),("example:first".into(),json!(2))],
            vec![("example:../escape".into(),json!(1))]] {
            assert!(apply_json_reload_listener(ResourceType::ServerData,"test:json_folder","custom/data",&files).is_err());
        }
        assert_eq!(seen.lock().unwrap().len(),1,"invalid batch must not deliver a valid prefix");
        assert!(apply_json_reload_listener(ResourceType::ServerData,"test:json_folder","wrong",&[]).is_err());
        assert!(!apply_json_reload_listener(ResourceType::ServerData,"test:foreign_listener","custom/data",&original).unwrap());
        assert!(apply_json_reload_listener(ResourceType::ServerData,"test:json_folder","custom/data",&[]).unwrap());
        let seen=seen.lock().unwrap();
        assert_eq!(seen.len(),2); assert!(seen[1].is_empty()); assert_eq!(seen[0][0].1,json!({"value":1}));
        drop(seen);
        for directory in ["","/absolute","../escape","sub/../escape","sub//file","sub\\file","C:/file"] {
            assert!(server.register_json_reload_listener(Identifier::parse("test:invalid_json_directory").unwrap(),directory, |_| Ok(())).is_err());
        }
        server.register_json_reload_listener(Identifier::parse("test:json_error").unwrap(),"errors", |_| Err("construction failed".into())).unwrap();
        assert_eq!(apply_json_reload_listener(ResourceType::ServerData,"test:json_error","errors",&[]).unwrap_err(),"construction failed");
    }

    #[test]
    fn resource_stacks_keep_all_layers_top_errors_owned_snapshots_and_nested_scope_cleanup() {
        use super::native_registry_scope as native;
        use serde_json::json;
        let raw=vec![
            native::Resource { id:"test:layered".into(),layers:vec![Ok(br#"{"values":["low"]}"#.to_vec()),Ok(br#"{"replace":true,"values":["high"]}"#.to_vec())] },
            native::Resource { id:"test:broken_top".into(),layers:vec![Ok(br#"{"lower":1}"#.to_vec()),Ok(b"{".to_vec())] },
            native::Resource { id:"test:unreadable".into(),layers:vec![Ok(b"true".to_vec()),Err("read failed".into())] },
            native::Resource { id:"test:broken_lower".into(),layers:vec![Ok(b"{".to_vec()),Ok(b"null".to_vec())] },
        ];
        let (effective,errors)=native::effective_json(&raw);
        assert_eq!(errors.len(),2);
        assert_eq!(effective,vec![("test:layered".into(),json!({"replace":true,"values":["high"]})),("test:broken_lower".into(),json!(null))]);
        let parsed=decode_json_stacks(raw.iter().map(|r|(r.id.as_str(),r.layers.as_slice()))).unwrap();
        assert_eq!(parsed[0].layers,[Ok(json!({"values":["low"]})),Ok(json!({"replace":true,"values":["high"]}))]);
        assert!(parsed[1].layers[0].is_ok()); assert!(parsed[1].layers[1].is_err());
        assert!(parsed[2].layers[1].is_err()); assert!(parsed[3].layers[0].is_err()); assert_eq!(parsed[3].layers[1],Ok(json!(null)));
        let seen=Arc::new(AtomicUsize::new(0)); let captured=seen.clone();
        ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
            Identifier::parse("test:resource_layers").unwrap(),"tags/materials",move |files| {
                assert_eq!(files.len(),2,"broken top resources never fall back in the normal JSON batch");
                assert_eq!(ResourceManagerHelper::reload_json_stacks("tags/materials").unwrap().len(),4);
                assert_eq!(ResourceManagerHelper::reload_json_stacks("tool_definitions").unwrap()[0].layers[0],Ok(json!({"generation":1})),
                    "a callback sees other captured folders before their listener runs");
                captured.fetch_add(1,Ordering::SeqCst); Ok(())
            }).unwrap();
        assert!(ResourceManagerHelper::reload_json_stacks("tags/materials").is_none());
        let kept=with_reload_folders(Default::default(),[
            ("tags/materials".into(),parsed.clone()),
            ("tool_definitions".into(),vec![JsonResourceStack { id:Identifier::parse("test:tool").unwrap(),layers:vec![Ok(json!({"generation":1}))] }]),
            ("empty/folder".into(),vec![]),
        ].into_iter().collect(),|| {
            assert_eq!(ResourceManagerHelper::reload_json_stacks("tool_definitions").unwrap()[0].layers[0],Ok(json!({"generation":1})));
            assert_eq!(ResourceManagerHelper::reload_json_stacks("empty/folder"),Some(vec![]));
            apply_json_reload_listener(ResourceType::ServerData,"test:resource_layers","tags/materials",&effective).unwrap();
            assert!(ResourceManagerHelper::reload_json_stacks("another/folder").is_none());
            assert!(ResourceManagerHelper::reload_json_stacks("../escape").is_none());
            let mut copied=ResourceManagerHelper::reload_json_stacks("tags/materials").unwrap();
            copied[0].layers[0]=Ok(json!(99));
            assert_eq!(ResourceManagerHelper::reload_json_stacks("tags/materials").unwrap(),parsed);
            with_reload_context(Default::default(),Some(("tags/materials".into(),vec![])),|| {
                assert_eq!(ResourceManagerHelper::reload_json_stacks("tags/materials"),Some(vec![]));
                assert!(ResourceManagerHelper::reload_json_stacks("tool_definitions").is_none(),"inner reload must not borrow an outer folder");
            });
            assert_eq!(ResourceManagerHelper::reload_json_stacks("tool_definitions").unwrap()[0].layers[0],Ok(json!({"generation":1})));
            let failed=std::panic::catch_unwind(||with_reload_registries(Default::default(),|| {
                assert!(ResourceManagerHelper::reload_json_stacks("tags/materials").is_none()); panic!("inner reload failed");
            }));
            assert!(failed.is_err()); assert_eq!(ResourceManagerHelper::reload_json_stacks("tags/materials").unwrap(),parsed);
            ResourceManagerHelper::reload_json_stacks("tags/materials").unwrap()
        });
        assert_eq!(seen.load(Ordering::SeqCst),1); assert_eq!(kept,parsed);
        assert!(ResourceManagerHelper::reload_json_stacks("tags/materials").is_none());
        native::with_context(Vec::new(),Some(("tags/materials".into(),raw.clone())),|| {
            assert_eq!(native::current_resources().unwrap()["tags/materials"].len(),4);
            native::with_registries(Vec::new(),||assert!(native::current_resources().is_none()));
            assert_eq!(native::current_resources().unwrap()["tags/materials"].len(),4);
        });
        assert!(native::current_resources().is_none());
        let folders:Arc<native::ResourceFolders>=Arc::new([("tags/materials".into(),raw.clone()),("tool_definitions".into(),vec![])].into_iter().collect());
        let retained=native::with_folders(Arc::new(vec![]),Some(folders.clone()),|| {
            let outer=native::current_resources().unwrap();
            assert!(Arc::ptr_eq(&outer,&folders),"all callbacks share the captured pass, not reread files");
            assert!(outer["tool_definitions"].is_empty());
            let failed=std::panic::catch_unwind(||native::with_folders(Arc::new(vec![]),Some(Arc::new(Default::default())),|| {
                assert!(native::current_resources().unwrap().get("tags/materials").is_none());
                panic!("nested native reload failed");
            }));
            assert!(failed.is_err());
            assert!(Arc::ptr_eq(&native::current_resources().unwrap(),&outer));
            outer
        });
        assert!(native::current_resources().is_none());
        assert_eq!(retained["tags/materials"].len(),4,"retained raw bytes outlive the callback");
        for invalid in [vec![native::Resource { id:"test:../escape".into(),layers:vec![Ok(b"null".to_vec())] }],
            vec![raw[0].clone(),raw[0].clone()],vec![native::Resource { id:"test:empty".into(),layers:vec![] }]] {
            assert!(decode_json_stacks(invalid.iter().map(|r|(r.id.as_str(),r.layers.as_slice()))).is_err());
        }
    }

    #[test]
    fn reload_registry_batches_supply_items_and_isolate_invalid_registries() {
        use super::native_registry_scope::Registry;
        use serde_json::json;
        let mut records=vec![Registry { id:"minecraft:entity_type".into(),entries:vec!["minecraft:player".into()],tags:vec![],item_defaults:None },
            Registry { id:"minecraft:item".into(),entries:vec!["minecraft:stick".into()],tags:vec![
                ("test:items".into(),vec!["minecraft:stick".into()]),("test:empty".into(),vec![])],item_defaults:None }];
        let decode=|records:&[Registry]|decode_reload_registries(records.iter().map(|r|(r.id.as_str(),r.entries.as_slice(),r.tags.as_slice())));
        let (views,errors)=decode(&records).unwrap(); assert!(errors.is_empty());
        let seen=Arc::new(Mutex::new(None)); let captured=seen.clone();
        ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
            Identifier::parse("test:item_conditions_snapshot").unwrap(),"rules",move |files| {
                assert_eq!(files[0].1,json!({"rule":1}));
                let items=ResourceManagerHelper::reload_registry(&Identifier::parse("minecraft:item").unwrap()).unwrap();
                *captured.lock().unwrap()=Some([
                    items.contains(&Identifier::parse("minecraft:stick").unwrap()),
                    items.contains(&Identifier::parse("test:missing").unwrap()),
                    !items.get_tag(&Identifier::parse("test:items").unwrap()).unwrap().is_empty(),
                    !items.get_tag(&Identifier::parse("test:empty").unwrap()).unwrap().is_empty(),
                    items.get_tag(&Identifier::parse("test:absent").unwrap()).is_some(),
                ]); Ok(())
            }).unwrap();
        with_reload_registries(views,||apply_json_reload_listener(ResourceType::ServerData,"test:item_conditions_snapshot","rules",
            &[("test:rule".into(),json!({"rule":1}))])).unwrap();
        assert_eq!(*seen.lock().unwrap(),Some([true,false,true,false,false]));
        assert!(ResourceManagerHelper::reload_registry(&Identifier::parse("minecraft:item").unwrap()).is_none());
        records[1].tags.push(("test:bad".into(),vec!["test:unregistered".into()]));
        let (views,errors)=decode(&records).unwrap(); assert_eq!(errors.len(),1);
        assert!(views.contains_key(&Identifier::parse("minecraft:entity_type").unwrap()));
        assert!(!views.contains_key(&Identifier::parse("minecraft:item").unwrap()),"bad item data must not become a false empty item registry");
        records.push(records[1].clone()); assert!(decode(&records).unwrap_err().contains("duplicate"));
        records.pop(); records[1].id="bad space:registry".into(); assert!(decode(&records).is_err());
    }

    #[test]
    fn reload_registry_membership_snapshots_restore_nested_scopes_and_unwind() {
        use std::collections::BTreeMap;
        use super::native_registry_scope as native;
        let registry=Identifier::parse("minecraft:entity_type").unwrap();
        let player=Identifier::parse("minecraft:player").unwrap();
        let custom=Identifier::parse("example:slime").unwrap();
        let empty=Identifier::parse("example:empty").unwrap();
        let lookup=RegistryLookup::from_parts(&[player.to_string(),custom.to_string()],&[
            (empty.to_string(),vec![]),("example:slimes".into(),vec![custom.to_string()])]).unwrap();
        assert!(lookup.contains(&player)); assert!(lookup.contains(&custom));
        assert!(!lookup.contains(&Identifier::parse("example:missing").unwrap()));
        assert!(lookup.get_tag(&empty).unwrap().is_empty());
        assert!(lookup.get_tag(&Identifier::parse("example:missing").unwrap()).is_none());
        assert!(ResourceManagerHelper::reload_registry(&registry).is_none());
        let kept=with_reload_registries(BTreeMap::from([(registry.clone(),lookup.clone())]),|| {
            let kept=ResourceManagerHelper::reload_registry(&registry).unwrap();
            assert_eq!(kept,lookup);
            let thread_registry=registry.clone();
            assert!(std::thread::spawn(move ||ResourceManagerHelper::reload_registry(&thread_registry).is_none()).join().unwrap());
            with_reload_registries(BTreeMap::new(),|| assert!(ResourceManagerHelper::reload_registry(&registry).is_none()));
            assert_eq!(ResourceManagerHelper::reload_registry(&registry),Some(lookup.clone()));
            let failed=std::panic::catch_unwind(|| with_reload_registries(BTreeMap::new(),|| panic!("inner reload failed")));
            assert!(failed.is_err()); assert_eq!(ResourceManagerHelper::reload_registry(&registry),Some(lookup.clone()));
            kept
        });
        assert_eq!(kept,lookup); assert!(ResourceManagerHelper::reload_registry(&registry).is_none());
        for (entries,tags) in [
            (vec!["bad space:id".into()],vec![]),
            (vec![player.to_string()],vec![("example:bad".into(),vec![custom.to_string()])]),
            (vec![player.to_string()],vec![(empty.to_string(),vec![]),(empty.to_string(),vec![])]),
        ] { assert!(RegistryLookup::from_parts(&entries,&tags).is_err()); }
        let snapshot=vec![native::Registry { id:registry.to_string(),entries:vec![player.to_string()],tags:vec![],item_defaults:None }];
        assert!(native::current().is_none());
        let kept=native::with_registries(snapshot,|| {
            let kept=native::current().unwrap();
            assert_eq!(kept[0].id,registry.to_string()); assert_eq!(kept[0].entries,[player.to_string()]); assert!(kept[0].tags.is_empty());
            let failed=std::panic::catch_unwind(||native::with_registries(Vec::new(),||panic!("native nested reload failed")));
            assert!(failed.is_err()); assert_eq!(native::current().unwrap()[0].entries,[player.to_string()]);
            kept
        });
        assert_eq!(kept[0].entries,[player.to_string()]); assert!(native::current().is_none());
    }

    #[test]
    fn reload_types_notify_only_their_own_listeners_on_every_load() {
        let count = Arc::new(AtomicUsize::new(0));
        let seen = Arc::clone(&count);
        ResourceManagerHelper::get(ResourceType::ClientResources)
            .register_reload_listener(Identifier::new("test", "counter").unwrap(), move || {
                seen.fetch_add(1, Ordering::SeqCst);
            });
        client_resources_reloaded();
        client_resources_reloaded();
        assert_eq!(count.load(Ordering::SeqCst), 2);
        let server_count = Arc::new(AtomicUsize::new(0));
        let seen = server_count.clone();
        ResourceManagerHelper::get(ResourceType::ServerData)
            .register_reload_listener(Identifier::new("test", "server_counter").unwrap(), move || {
                seen.fetch_add(1, Ordering::SeqCst);
            });
        assert_eq!(server_count.load(Ordering::SeqCst), 0);
        server_data_reloaded();
        server_data_reloaded();
        assert_eq!(server_count.load(Ordering::SeqCst), 2);
        assert_eq!(count.load(Ordering::SeqCst), 2);
        client_resources_reloaded();
        assert_eq!(count.load(Ordering::SeqCst), 3);
        assert_eq!(server_count.load(Ordering::SeqCst), 2);
    }
}
