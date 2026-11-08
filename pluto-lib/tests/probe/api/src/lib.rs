//! Throwaway consumer: proves PlutoLib's events fire in a running game. Uses only the PlutoLib API.
use horizon_client::mod_api::Hooks;
use pluto_lib::{
    BlockEntityRendererRegistry, BlockEntityType, BlockEntityTypeRegistry, FluidStack, ScreenHandlerRegistry, ScreenHandlerType, ScreenLayout, Nbt, NbtCompoundExt, AttachmentRegistry, ItemGroup, ItemGroupRegistry, ItemStack, CommandRegistrationCallback, ItemUse, ItemUseRegistry, UseAction, ConfigEvents, ConfigKind, ConfigRegistry, Identifier, LootTableEvents, ResourceManagerHelper, ResourceType, ServerChunkEvents, ServerLifecycleEvents, ServerPlayConnectionEvents,
    ServerTickEvents,
};
use std::sync::atomic::{AtomicBool, Ordering};

pub fn hooks() -> Box<dyn Hooks> {
    pluto_lib::ProjectileEvents::IMPACT.register(|hit| {
        static ONCE: AtomicBool = AtomicBool::new(false);
        if !ONCE.swap(true, Ordering::Relaxed) {
            println!("PLUTO-PROBE projectile kind={} dimension={}", hit.kind, hit.dimension);
        }
        true
    });
    pluto_lib::Translations::register("en_us", [("pluto_probe.hello".to_owned(), "Hello from the probe".to_owned())]);
    pluto_lib::Translations::register("ja_jp", [("pluto_probe.hello".to_owned(), "プローブからこんにちは".to_owned())]);
    BlockEntityRendererRegistry::register(Identifier::new("tconstruct", "seared_melter").unwrap(), |render| {
        static ONCE: AtomicBool = AtomicBool::new(false);
        if !ONCE.swap(true, Ordering::Relaxed) {
            println!("PLUTO-PROBE renderer pos={:?} data_keys={}", render.pos, render.data.len());
        }
        render.canvas.keep();
    });
    ScreenHandlerRegistry::register(
        ScreenHandlerType::new(Identifier::new("tconstruct", "seared_melter").unwrap(), ScreenLayout::new(176.0, 166.0).slot(56.0, 17.0).slot(56.0, 53.0).slot(116.0, 35.0))
            .on_draw(|d| {
                static ONCE: AtomicBool = AtomicBool::new(false);
                if !ONCE.swap(true, Ordering::Relaxed) {
                    println!("PLUTO-PROBE screen_draw origin_set={}", d.origin[0] >= 0.0);
                }
            }),
    );
    BlockEntityTypeRegistry::register(
        BlockEntityType::new(Identifier::new("tconstruct", "seared_melter").unwrap())
            .with_id(Identifier::new("pluto_probe", "stone_entity").unwrap())
            .item_slots(3)
            .menu("minecraft:furnace")
            .fluid_tanks([2000])
            .ticker(|tick| {
                let ticks = tick.data.get_int("ticks", 0) + 1;
                tick.data.insert("ticks".to_owned(), Nbt::Int(ticks as i32));
                if ticks == 1 {
                    tick.set_fluid(0, Some(FluidStack { fluid: Identifier::new("minecraft", "water").unwrap(), amount: 500 }));
                }
                if ticks == 5 {
                    println!("PLUTO-PROBE block_entity ticks={ticks} pos={:?} slots={} water={}", tick.pos, tick.items.len(), tick.fluid(0).map_or(0, |f| f.amount));
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
            let world = pluto_lib::World::new("minecraft:overworld");
            let glass = pluto_lib::BlockState::new(Identifier::new("minecraft", "glass").unwrap());
            let set = world.set_block_state([3, 81, 3], &glass);
            println!("PLUTO-PROBE world set={set} read={:?}", world.get_block_state([3, 81, 3]).map(|s| s.to_string()));
            // two punji sticks side by side on stone: each learns of the other; then the stone under one goes and so does the stick
            let stone = pluto_lib::BlockState::parse("minecraft:stone").unwrap();
            let punji = pluto_lib::BlockState::parse("tconstruct:punji[facing=down]").unwrap();
            for x in [5, 6] {
                world.set_block_state([x, 80, 5], &stone);
                world.set_block_state([x, 81, 5], &punji);
            }
            let east = world.get_block_state([5, 81, 5]).and_then(|s| s.property("east").map(str::to_owned));
            world.set_block_state([5, 80, 5], &pluto_lib::BlockState::parse("minecraft:air").unwrap());
            println!("PLUTO-PROBE punji east={east:?}");
            let signs: Vec<bool> = ["skyroot_sign", "skyroot_wall_sign", "greenheart_hanging_sign", "enderbark_wall_hanging_sign"]
                .iter()
                .enumerate()
                .map(|(i, name)| {
                    let state = pluto_lib::BlockState::parse(&format!("tconstruct:{name}")).unwrap();
                    world.set_block_state([8 + i as i32 * 2, 81, 8], &state)
                        && world.get_block_state([8 + i as i32 * 2, 81, 8]).is_some_and(|s| s.block.path() == *name)
                })
                .collect();
            println!("PLUTO-PROBE signs placed={signs:?}");
            // a platform learns of the stone below it and of the platform above it
            let platform = pluto_lib::BlockState::parse("tconstruct:iron_platform").unwrap();
            world.set_block_state([12, 81, 12], &platform);
            world.set_block_state([12, 80, 12], &stone);
            world.set_block_state([12, 82, 12], &platform);
            let read = |key: &str| world.get_block_state([12, 81, 12]).and_then(|s| s.property(key).map(str::to_owned));
            let pots: Vec<bool> = ["potted_blood_slime_fern", "potted_sky_slime_sapling"]
                .iter()
                .enumerate()
                .map(|(i, name)| {
                    let state = pluto_lib::BlockState::parse(&format!("tconstruct:{name}")).unwrap();
                    world.set_block_state([16 + i as i32 * 2, 81, 16], &state) && world.get_block_state([16 + i as i32 * 2, 81, 16]).is_some_and(|s| s.block.path() == *name)
                })
                .collect();
            println!("PLUTO-PROBE potted placed={pots:?}");
            println!("PLUTO-PROBE platform down={:?} up={:?}", read("down"), read("up"));
            println!("PLUTO-PROBE command ping arg={} source={}", ctx.args[0], ctx.source.name);
            1
        });
    });
    CommandRegistrationCallback::EVENT.register(|registrar| {
        registrar.register("pluto_probe check", |_| {
            let later = pluto_lib::World::new("minecraft:overworld").get_block_state([5, 81, 5]).map(|s| s.block.to_string());
            println!("PLUTO-PROBE punji after_support_removed_later={later:?}");
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
        println!(
            "PLUTO-PROBE i18n language={} text={} ja={} player_language={}",
            pluto_lib::I18n::language(),
            pluto_lib::I18n::translate("pluto_probe.hello"),
            pluto_lib::I18n::translate_in("ja_jp", "pluto_probe.hello"),
            p.player.language()
        );
        println!("PLUTO-PROBE fluids tconstruct={}", pluto_lib::FluidRegistry::registered().iter().filter(|f| f.id.namespace() == "tconstruct").count());
        // what the game side holds for the item registered through PlutoLib
        let spec = horizon_client::entry::pluto_item_use::get("pluto_probe:snack");
        println!("PLUTO-PROBE item_use game_side={spec:?}");
        let said = p.player.send_message("PlutoLib says hello");
        let left = p.player.give_item("minecraft:stick", 1);
        println!("PLUTO-PROBE player send_message={said} give_left={left}");
        let gave = p.player.add_effect(&pluto_lib::EffectInstance::new("minecraft:speed", 200).amplifier(1));
        let have: Vec<String> = p.player.effects().iter().map(|e| format!("{}:{}", e.id, e.amplifier)).collect();
        println!("PLUTO-PROBE effects gave={gave} have={have:?} food={:?} add_food={}", p.player.food(), p.player.add_food(4, 0.5));
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
