//! Tinkers' Construct, the part that uses only PlutoLib's API (Horizon's Fabric API). Everything that cannot yet be
//! expressed through PlutoLib stays in the manifest's direct injections.
use std::sync::atomic::{AtomicBool, Ordering};

use horizon_client::mod_api::Hooks;
use pluto_lib::{AttachmentRegistry, ConfigEvents, ConfigKind, ConfigRegistry, Identifier, ServerPlayConnectionEvents};

/// `config/tconstruct-common.toml`, `[gameplay] shouldSpawnWithTinkersBook` (default true), as in the original mod.
static SPAWN_WITH_BOOK: AtomicBool = AtomicBool::new(true);

pub fn hooks() -> Box<dyn Hooks> {
    ConfigRegistry::register("tconstruct", ConfigKind::Common, "");
    ConfigEvents::LOADING.register(|config| {
        if config.mod_id == "tconstruct" {
            SPAWN_WITH_BOOK.store(spawn_with_book(&config.text), Ordering::Relaxed);
        }
    });
    ServerPlayConnectionEvents::JOIN.register(|join| {
        if !SPAWN_WITH_BOOK.load(Ordering::Relaxed) {
            return;
        }
        let given = AttachmentRegistry::create(Identifier::new("tconstruct", "starting_book_given").expect("valid id"), || false);
        if !join.player.get_attached(&given) && join.player.give_item("tconstruct:materials_and_you", 1) == 0 {
            join.player.set_attached(&given, true);
            println!("tconstruct: gave the starting book to {}", join.name);
        }
    });
    Box::new(Nothing)
}

/// Everything is registered through PlutoLib's events; the trait object only has to exist.
struct Nothing;
impl Hooks for Nothing {}

fn spawn_with_book(text: &str) -> bool {
    text.parse::<toml::Table>()
        .ok()
        .and_then(|t| t.get("gameplay")?.get("shouldSpawnWithTinkersBook")?.as_bool())
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::spawn_with_book;

    #[test]
    fn the_book_is_on_unless_the_config_turns_it_off() {
        assert!(spawn_with_book(""));
        assert!(spawn_with_book("[gameplay]\nshouldSpawnWithTinkersBook = true\n"));
        assert!(!spawn_with_book("[gameplay]\nshouldSpawnWithTinkersBook = false\n"));
        assert!(spawn_with_book("[gameplay\n"), "a broken file keeps the default");
    }
}
