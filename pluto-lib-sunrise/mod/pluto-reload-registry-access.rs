// Owned callback-local registry/resource snapshots. No Game pointers or world ID.
use std::cell::RefCell;
use std::sync::Arc;
#[derive(Clone,Debug)]
pub struct ItemPrototype {
    pub id:String,
    pub max_damage:Option<i32>,
    // Item-level crafting remainder, independent of use_remainder.
    pub crafting_remainder:Option<String>,
    // Network NBT compound, retaining numeric/array tag types.
    pub custom_data:Vec<u8>,
}
#[derive(Clone,Debug)]
pub struct Registry {
    pub id:String,
    pub entries:Vec<String>,
    pub tags:Vec<(String,Vec<String>)>,
    // None is unavailable, Some(empty) is a captured empty item registry.
    pub item_defaults:Option<Vec<ItemPrototype>>,
}
#[derive(Clone,Debug)]
pub struct Resource {
    pub id:String,
    // Bottom pack first. A failed read is retained, never silently omitted.
    pub layers:Vec<Result<Vec<u8>,String>>,
}
pub type ResourceFolders=std::collections::BTreeMap<String,Vec<Resource>>;
struct Frame {
    registries:Arc<Vec<Registry>>,
    resources:Option<Arc<ResourceFolders>>,
}
thread_local! { static CURRENT: RefCell<Vec<Frame>> = const { RefCell::new(Vec::new()) }; }
pub fn current()->Option<Arc<Vec<Registry>>> { CURRENT.with(|v|v.borrow().last().map(|f|f.registries.clone())) }
pub fn current_resources()->Option<Arc<ResourceFolders>> {
    CURRENT.with(|v|v.borrow().last()?.resources.clone())
}
pub fn with_folders<T>(registries:Arc<Vec<Registry>>,resources:Option<Arc<ResourceFolders>>,action:impl FnOnce()->T)->T {
    struct Scope;
    impl Drop for Scope { fn drop(&mut self) { CURRENT.with(|v| { v.borrow_mut().pop(); }); } }
    CURRENT.with(|v|v.borrow_mut().push(Frame { registries,resources }));
    let _scope=Scope;
    action()
}
pub fn with_context<T>(registries:Vec<Registry>,resources:Option<(String,Vec<Resource>)>,action:impl FnOnce()->T)->T {
    with_folders(Arc::new(registries),resources.map(|entry|Arc::new([entry].into_iter().collect())),action)
}
pub fn with_registries<T>(registries:Vec<Registry>,action:impl FnOnce()->T)->T {
    with_context(registries,None,action)
}
// The normal native JSON listener receives ONLY the top layer, like the pinned
// read_folder: broken/missing top bytes never revive a lower pack's JSON.
pub fn effective_json(resources:&[Resource])->(Vec<(String,serde_json::Value)>,Vec<String>) {
    let mut files=Vec::new();
    let mut errors=Vec::new();
    for resource in resources {
        match resource.layers.last() {
            Some(Ok(bytes))=>match serde_json::from_slice(bytes) {
                Ok(value)=>files.push((resource.id.clone(),value)),
                Err(error)=>errors.push(format!("{}: {error}",resource.id)),
            },
            Some(Err(error))=>errors.push(format!("{}: {error}",resource.id)),
            None=>errors.push(format!("{}: empty resource stack",resource.id)),
        }
    }
    (files,errors)
}
