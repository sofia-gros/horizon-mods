//! The host-independent half of PlutoLib, built without Horizon, plus the compiled documentation examples.
#[path = "../../../mod/api/src/pure/mod.rs"]
pub mod pure;
#[path = "../../../mod/api/src/exports.rs"]
mod exports;

pub use exports::*;

/// Every page of `pluto-lib/docs/api` is included here, so each ```rust block in it is compiled and run by `cargo test --doc`.
#[cfg(doctest)]
mod api_pages {
    macro_rules! page {
        ($name:ident, $file:literal) => {
            #[doc = include_str!(concat!("../../../docs/api/", $file))]
            pub struct $name;
        };
    }
    page!(Index, "index.md");
    page!(Base, "base.md");
    page!(Lifecycle, "lifecycle.md");
    page!(Networking, "networking.md");
    page!(Interaction, "interaction.md");
    page!(Entities, "entities.md");
    page!(Loot, "loot.md");
    page!(Resources, "resources.md");
    page!(Items, "items.md");
    page!(Creative, "creative.md");
    page!(BlockEntity, "block-entity.md");
    page!(Screen, "screen.md");
    page!(Renderer, "renderer.md");
    page!(Player, "player.md");
    page!(Attachments, "attachments.md");
    page!(Config, "config.md");
    page!(Commands, "commands.md");
}
