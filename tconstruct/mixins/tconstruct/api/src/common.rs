//! Shared registration, called while the product's hooks are constructed.
use pluto_lib::{Identifier, ResourceManagerHelper};
use std::sync::{atomic::{AtomicBool, Ordering}, Arc, OnceLock, RwLock};
use crate::{conditions::Context, materials::{self, Definitions}};

static MATERIALS: OnceLock<Arc<RwLock<Definitions>>> = OnceLock::new();
static MODIFIERS: OnceLock<Arc<RwLock<crate::modifiers::Registry>>> = OnceLock::new();
/// TinkerTags.isTagsLoaded for the server: true once a reload callback captured the native
/// item registry/tag snapshot, which is where item-tag membership comes from here.
/// Unlike Forge's never-reset flag it is cleared when the server stops, because the
/// snapshot it vouches for no longer exists.
static ITEM_TAGS_LOADED: AtomicBool = AtomicBool::new(false);
pub(crate) fn item_tags_loaded() -> bool { ITEM_TAGS_LOADED.load(Ordering::Acquire) }
fn capture_items() -> Option<pluto_lib::RegistryLookup> {
    let items = ResourceManagerHelper::reload_registry(&Identifier::new("minecraft", "item").unwrap());
    if items.is_some() { ITEM_TAGS_LOADED.store(true, Ordering::Release); }
    items
}
pub(crate) fn server_modifiers() -> Option<&'static Arc<RwLock<crate::modifiers::Registry>>> { MODIFIERS.get() }

/// Short read-only client operation. Do not reload or invoke lifecycle events inside it.
/// Local presentation shares server materials; remote presentation never falls back
/// to a local generation while its own transfer is pending or invalid.
pub(crate) fn with_client_materials<R>(read: impl FnOnce(Option<&Definitions>) -> R) -> R {
    if crate::material_sync::remote_connection() {
        let Some(client) = crate::material_sync::client_state() else { return read(None); };
        let state = client.read().expect("client material lock poisoned");
        read(state.as_ref().filter(|state| state.server_loaded))
    } else {
        let Some(server) = MATERIALS.get() else { return read(None); };
        let state = server.read().expect("server material lock poisoned");
        read(state.server_loaded.then_some(&*state))
    }
}

pub(crate) fn register() -> &'static Arc<RwLock<Definitions>> {
    MATERIALS.get_or_init(|| {
        let config = crate::config::register();
        let modifier_config = config.clone();
        let modifiers = crate::modifiers::register(move || {
            let config = modifier_config.read().map_err(|_| "Tinkers config lock poisoned")?.clone()?;
            let items = capture_items();
            Ok(Context { items, config })
        }).expect("Tinkers modifier listener registration failed");
        MODIFIERS.set(modifiers).expect("Tinkers modifier registry already initialized");
        let materials = materials::register(move || {
            let config = config.read().map_err(|_| "Tinkers config lock poisoned")?.clone()?;
            let items = capture_items();
            Ok(Context { items, config })
        }).expect("Tinkers material listener registration failed");
        crate::material_sync::register(materials.clone());
        crate::tool_definitions::register();
        let stopped = materials.clone();
        pluto_lib::ServerLifecycleEvents::SERVER_STOPPED.register(move |_| {
            ITEM_TAGS_LOADED.store(false, Ordering::Release);
            let mut state = stopped.write().expect("material registry lock poisoned");
            let generation = state.generation.wrapping_add(1);
            *state = Definitions { generation, ..Definitions::default() };
        });
        materials
    })
}
