//! Throwaway consumer: proves PlutoLib's events fire in a running game. Uses only the PlutoLib API.
use horizon_client::mod_api::Hooks;
use pluto_lib::{ServerChunkEvents, ServerLifecycleEvents, ServerPlayConnectionEvents, ServerTickEvents};
use std::sync::atomic::{AtomicBool, Ordering};

pub fn hooks() -> Box<dyn Hooks> {
    ServerLifecycleEvents::SERVER_STARTING.register(|s| println!("PLUTO-PROBE server_starting dedicated={}", s.dedicated));
    ServerPlayConnectionEvents::JOIN.register(|p| println!("PLUTO-PROBE join name={}", p.name));
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
    Box::new(Probe)
}

struct Probe;
impl Hooks for Probe {}
