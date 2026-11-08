//! Throwaway consumer: proves PlutoLib's events fire in a running game. Uses only the PlutoLib API.
use horizon_client::mod_api::Hooks;
use pluto_lib::{
    BlockEntityType, BlockEntityTypeRegistry, Nbt, NbtCompoundExt, AttachmentRegistry, ItemGroup, ItemGroupRegistry, ItemStack, CommandRegistrationCallback, ItemUse, ItemUseRegistry, UseAction, ConfigEvents, ConfigKind, ConfigRegistry, Identifier, LootTableEvents, ResourceManagerHelper, ResourceType, ServerChunkEvents, ServerLifecycleEvents, ServerPlayConnectionEvents,
    ServerTickEvents,
};
use std::sync::atomic::{AtomicBool, Ordering};

pub fn hooks() -> Box<dyn Hooks> {
    BlockEntityTypeRegistry::register(
        BlockEntityType::new(Identifier::new("tconstruct", "seared_stone").unwrap())
            .with_id(Identifier::new("pluto_probe", "stone_entity").unwrap())
            .ticker(|tick| {
                let ticks = tick.data.get_int("ticks", 0) + 1;
                tick.data.insert("ticks".to_owned(), Nbt::Int(ticks as i32));
                if ticks == 5 {
                    println!("PLUTO-PROBE block_entity ticks={ticks} pos={:?}", tick.pos);
                }
            })
            .sync_data(),
    );
    ItemGroupRegistry::register(
        ItemGroup::new(Identifier::new("pluto_probe", "tab").unwrap(), "Probe Tab", ItemStack::new(Identifier::new("minecraft", "stick").unwrap(), 1))
            .entry(ItemStack::new(Identifier::new("minecraft", "stick").unwrap(), 3).with_components("{\"minecraft:custom_name\":'\"Probe\"'}")),
    );
    ItemUseRegistry::register(
        Identifier::new("pluto_probe", "snack").unwrap(),
        ItemUse::new(16, UseAction::Eat).food(4, 0.5).remainder(Identifier::new("minecraft", "stick").unwrap(), 2),
    );
    CommandRegistrationCallback::EVENT.register(|registrar| {
        registrar.register("pluto_probe ping <word:word>", |ctx| {
            println!("PLUTO-PROBE command ping arg={} source={}", ctx.args[0], ctx.source.name);
            1
        });
    });
    ConfigRegistry::register("pluto_probe", ConfigKind::Common, "probe = true
");
    ConfigEvents::LOADING.register(|c| {
        if c.mod_id == "pluto_probe" {
            println!("PLUTO-PROBE config_loading {} text={:?}", c.file, c.text);
        }
    });
    ServerLifecycleEvents::SERVER_STARTING.register(|s| println!("PLUTO-PROBE server_starting dedicated={}", s.dedicated));
    ServerPlayConnectionEvents::JOIN.register(|p| {
        println!("PLUTO-PROBE join name={}", p.name);
        // what the game side holds for the item registered through PlutoLib
        let spec = horizon_client::entry::pluto_item_use::get("pluto_probe:snack");
        println!("PLUTO-PROBE item_use game_side={spec:?}");
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
