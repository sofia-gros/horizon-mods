//! Throwaway consumer: proves PlutoLib's events fire in a running game. Uses only the PlutoLib API.
use horizon_client::mod_api::Hooks;
use pluto_lib::{
    AttachmentRegistry, Identifier, LootTableEvents, ResourceManagerHelper, ResourceType, ServerChunkEvents, ServerLifecycleEvents, ServerPlayConnectionEvents,
    ServerTickEvents,
};
use std::sync::atomic::{AtomicBool, Ordering};

pub fn hooks() -> Box<dyn Hooks> {
    ServerLifecycleEvents::SERVER_STARTING.register(|s| println!("PLUTO-PROBE server_starting dedicated={}", s.dedicated));
    ServerPlayConnectionEvents::JOIN.register(|p| {
        println!("PLUTO-PROBE join name={}", p.name);
        let said = p.player.send_message("PlutoLib says hello");
        let left = p.player.give_item("minecraft:stick", 1);
        println!("PLUTO-PROBE player send_message={said} give_left={left}");
        let visits = AttachmentRegistry::create(Identifier::new("pluto_probe", "visits").unwrap(), || 0u32);
        let before = p.player.get_attached(&visits);
        let stored = p.player.set_attached(&visits, before + 1);
        println!("PLUTO-PROBE attachment stored={stored} now={}", p.player.get_attached(&visits));
    });
    ServerChunkEvents::CHUNK_LOAD.register(|c| {
        static ONCE: AtomicBool = AtomicBool::new(false);
        if !ONCE.swap(true, Ordering::Relaxed) {
            println!("PLUTO-PROBE chunk_load {} {} {} new={}", c.dimension, c.x, c.z, c.newly_generated);
        }
    });
    ServerTickEvents::END_SERVER_TICK.register(|_| {
        static ONCE: AtomicBool = AtomicBool::new(false);
        if !ONCE.swap(true, Ordering::Relaxed) {
            println!("PLUTO-PROBE end_server_tick");
        }
    });
    LootTableEvents::MODIFY.register(|c| {
        static ONCE: AtomicBool = AtomicBool::new(false);
        if !ONCE.swap(true, Ordering::Relaxed) {
            println!("PLUTO-PROBE loot_modify id={}", c.id);
        }
    });
    ResourceManagerHelper::get(ResourceType::ClientResources).register_reload_listener(
        Identifier::new("pluto_probe", "reload").expect("valid id"),
        || {
            static ONCE: AtomicBool = AtomicBool::new(false);
            if !ONCE.swap(true, Ordering::Relaxed) {
                println!("PLUTO-PROBE client_resources_reloaded");
            }
        },
    );
    Box::new(Probe)
}

struct Probe;
impl Hooks for Probe {}
