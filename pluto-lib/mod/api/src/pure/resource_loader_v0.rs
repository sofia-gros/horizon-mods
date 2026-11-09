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
