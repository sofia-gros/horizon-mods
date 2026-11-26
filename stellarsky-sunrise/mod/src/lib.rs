//! Stellarsky for Sunrise: a real night sky over the game. The bright end
//! cannot be a texture, because a texture cannot twinkle and cannot darken
//! what another layer drew, so this native half registers it as a dynamic
//! sky layer through PlutoLib's public sky surface: the 523 stars of
//! magnitude 4 and brighter, each with its spectral colour and its own
//! scintillation, the dark patch of Sagittarius A* over the galactic
//! centre, and no vanilla stars — this field replaces them. The faint end —
//! stars of magnitude 4 to 7 and the Milky Way — is a texture, and it
//! registers through the same surface as textured layers (static_sky), so
//! the whole sky is data this native half hands to PlutoLib and no resource
//! pack is needed for it. Nothing here injects, exactly as Tinkers'
//! Construct and Pipes hand PlutoLib their declarations.
//!
//! Entry point, and how it differs from the Horizon edition of this crate: there, the root
//! exposes `pub fn hooks() -> pluto_lib::ModHooks`, whose body runs the two `register()`
//! calls below and returns `pluto_lib::no_hooks()`. On Sunrise the loader's entry is the one
//! marked `impl mod_api::Hooks` at the crate root — the `#[mod_api::sunrise_mod]` macro
//! generates the `hooks()` factory, and a hand-written factory must not be supplied (Sunrise
//! modding book, "Writing hooks" / "Using an AI agent"). The `register()` list therefore
//! runs inside the `register` hook, the registration-phase moment PlutoLib's own Sunrise
//! entry uses the same way; mods run in dependency order, so PlutoLib's registration
//! precedes this one. The `register()` bodies are shared with the Horizon edition
//! unchanged — only this entry block is converted, per the master-sync discipline for the
//! Sunrise tree (seed by conversion, then maintain in place).
mod stars;
mod stars_table;
mod static_sky;

use mod_api::RegistrationContext;

/// Stellarsky's loader entry on Sunrise. The type carries no state: the sky layers are
/// registered through PlutoLib's public sky registries from the `register` hook, and the
/// mod subscribes to no game hook of its own — the Sunrise form of the Horizon
/// edition's `pluto_lib::no_hooks()` return.
#[derive(Default)]
pub struct Mod;

#[mod_api::sunrise_mod]
impl mod_api::Hooks for Mod {
    fn register(&self, registrar: &mut dyn mod_api::Registrar, cx: &RegistrationContext) {
        // The registrations go into PlutoLib's own registries, not the game's
        // registrar; PlutoLib consumes them from its side of the bridge. The two
        // parameters are the hook's contract and are intentionally unused here.
        let _ = registrar;
        let _ = cx;
        // The registries validate each layer whole before taking it, and the
        // crate's tests pin that these exact layers pass; a refusal would leave
        // that half of the sky to whatever else draws it.
        let _ = crate::register();
        let _ = static_sky::register();
    }
}

/// Register the Overworld's dynamic sky layer (world 0, the world PlutoLib's
/// sky transport serves). Re-registering replaces the layer.
pub fn register() -> Result<(), String> {
    pluto_lib::SkyRegistry::register(0, stars::layer())
}
