//! Counterpart of Fabric's `fabric-resource-loader-v0`: be told when resources reload.
//!
//! Fabric: `ResourceManagerHelper.get(ResourceType.CLIENT_RESOURCES).registerReloadListener(listener)`.
//! Here: `ResourceManagerHelper::get(ResourceType::ClientResources).register_reload_listener(id, listener)`.

use super::base::{Event, Identifier};

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
