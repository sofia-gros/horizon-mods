//! Pipes for Sunrise: item and fluid transport pipes that work against Tinkers' Construct's
//! furnaces, tanks, casting blocks and chests (and the vanilla chest) over PlutoLib's public
//! neighbour reach. A clean-room design in the classic extraction-pipe style; no other mod's
//! code or assets are used, and nothing here injects — every reach into the game goes through
//! PlutoLib, exactly as Tinkers' Construct itself does.
//!
//! Entry point, and how it differs from the Horizon edition of this crate: there, the root
//! exposes `pub fn hooks() -> pluto_lib::ModHooks`, whose body runs the five `register()`
//! calls below and returns `pluto_lib::no_hooks()`. On Sunrise the loader's entry is the one
//! marked `impl mod_api::Hooks` at the crate root — the `#[mod_api::sunrise_mod]` macro
//! generates the `hooks()` factory, and a hand-written factory must not be supplied (Sunrise
//! modding book, "Writing hooks" / "Using an AI agent"). The `register()` list therefore
//! runs inside the `register` hook, the registration-phase moment PlutoLib's own Sunrise
//! entry uses the same way; mods run in dependency order, so PlutoLib's registration
//! precedes this one. The five `register()` bodies are shared with the Horizon edition
//! unchanged — only this entry block is converted, per the master-sync discipline for the
//! Sunrise tree (seed by conversion, then maintain in place).
mod common;
mod creative;
mod connections;
mod fluid_pipe;
mod item_pipe;
mod render;

use mod_api::RegistrationContext;

/// Pipes' loader entry on Sunrise. The type carries no state: every behaviour is
/// registered through PlutoLib's public registries from the `register` hook, and the
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
        creative::register();
        item_pipe::register();
        fluid_pipe::register();
        connections::register();
        render::register();
    }
}
