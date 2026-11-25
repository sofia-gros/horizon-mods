//! Mod config files. Fabric API has none (Fabric mods bring their own); NeoForge has `ModConfigSpec` and `ModConfigEvent`.
//! PlutoLib offers the common core: register a config file with its default text, and hear when its text is loaded, changed on disk
//! or dropped. Parsing is the mod's business (TOML by convention), so the API deals in text.

use std::sync::Mutex;

use super::base::Event;

/// Which side reads a config and when (NeoForge: `ModConfig.Type`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ConfigKind {
    /// The window only, when the game starts.
    Client,
    /// Once per process: the window, or a dedicated server.
    Common,
    /// The server, for each world it opens.
    Server,
}

/// A config the mod asks for: `config/<mod_id>-<kind>.toml` unless `file` names another, written from `default_text` when missing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigSpec {
    pub mod_id: String,
    pub kind: ConfigKind,
    pub file: Option<String>,
    pub default_text: String,
}

/// A config file as the game read it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModConfig {
    pub mod_id: String,
    pub kind: ConfigKind,
    pub file: String,
    pub text: String,
}

static REGISTERED: Mutex<Vec<ConfigSpec>> = Mutex::new(Vec::new());

/// NeoForge: `ModContainer.registerConfig`.
pub struct ConfigRegistry;

impl ConfigRegistry {
    /// Asks for `config/<mod_id>-<kind>.toml`, created from `default_text` when missing. Call it while the mod's hooks are built.
    pub fn register(mod_id: &str, kind: ConfigKind, default_text: &str) {
        Self::push(ConfigSpec { mod_id: mod_id.to_owned(), kind, file: None, default_text: default_text.to_owned() });
    }

    /// The same in another file of `config/`, such as `mymod/machines.toml`.
    pub fn register_file(mod_id: &str, kind: ConfigKind, file: &str, default_text: &str) {
        Self::push(ConfigSpec { mod_id: mod_id.to_owned(), kind, file: Some(file.to_owned()), default_text: default_text.to_owned() });
    }

    fn push(spec: ConfigSpec) {
        if let Ok(mut all) = REGISTERED.lock() {
            all.push(spec);
        }
    }
}

/// NeoForge: `ModConfigEvent`. Every mod hears every config, so a callback checks `config.mod_id`.
pub struct ConfigEvents;

static LOADING: Event<ModConfig, ()> = Event::void();
static RELOADING: Event<ModConfig, ()> = Event::void();
static UNLOADING: Event<ModConfig, ()> = Event::void();

impl ConfigEvents {
    /// `ModConfigEvent.Loading`: a registered config was read.
    pub const LOADING: &'static Event<ModConfig, ()> = &LOADING;
    /// `ModConfigEvent.Reloading`: a loaded config's file changed on disk (not fired for the same text saved again).
    pub const RELOADING: &'static Event<ModConfig, ()> = &RELOADING;
    /// `ModConfigEvent.Unloading`: a `Server` config was dropped as its server stopped.
    pub const UNLOADING: &'static Event<ModConfig, ()> = &UNLOADING;
}

/// Everything registered so far (read by `hooks.rs` when the game asks which configs exist).
pub(crate) fn registered() -> Vec<ConfigSpec> {
    REGISTERED.lock().map(|all| all.clone()).unwrap_or_default()
}

pub(crate) fn loading(config: ModConfig) {
    LOADING.invoke(&config);
}

pub(crate) fn reloading(config: ModConfig) {
    RELOADING.invoke(&config);
}

pub(crate) fn unloading(config: ModConfig) {
    UNLOADING.invoke(&config);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn registered_configs_are_listed_and_events_carry_the_text_to_callbacks_that_pick_their_mod() {
        ConfigRegistry::register("cfg_test", ConfigKind::Common, "a = 1\n");
        ConfigRegistry::register_file("cfg_test", ConfigKind::Server, "cfg_test/extra.toml", "");
        let specs: Vec<_> = registered().into_iter().filter(|s| s.mod_id == "cfg_test").collect();
        assert_eq!(specs.len(), 2);
        assert_eq!(specs[0].file, None);
        assert_eq!(specs[1].file.as_deref(), Some("cfg_test/extra.toml"));

        let seen = Arc::new(Mutex::new(Vec::new()));
        let sink = seen.clone();
        ConfigEvents::LOADING.register(move |c| {
            if c.mod_id == "cfg_test" {
                sink.lock().unwrap().push(c.text.clone());
            }
        });
        loading(ModConfig { mod_id: "other".into(), kind: ConfigKind::Common, file: "o.toml".into(), text: "no".into() });
        loading(ModConfig { mod_id: "cfg_test".into(), kind: ConfigKind::Common, file: "cfg_test-common.toml".into(), text: "a = 2".into() });
        assert_eq!(*seen.lock().unwrap(), ["a = 2"], "only the callback's own mod's text arrives");
    }
}
