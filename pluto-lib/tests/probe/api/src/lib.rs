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
            // a slime vine hangs on a stone to its north, and goes when the stone does
            world.set_block_state([40, 81, 40], &stone);
            let vine = pluto_lib::BlockState::parse("tconstruct:sky_slime_vine[north=true]").unwrap();
            world.set_block_state([40, 81, 41], &vine);
            let kept = world.get_block_state([40, 81, 41]).is_some_and(|s| s.block.path() == "sky_slime_vine" && s.property("north") == Some("true"));
            world.set_block_state([40, 81, 40], &pluto_lib::BlockState::parse("minecraft:air").unwrap());
            println!("PLUTO-PROBE vine kept_on_stone={kept}");
            world.set_block_state([34, 80, 34], &pluto_lib::BlockState::parse("tconstruct:budding_sky_slime_crystal").unwrap());
            // seeds used on slime dirt make grass (the event is invoked here; the game's hook calls it the same way)
            world.set_block_state([26, 80, 26], &pluto_lib::BlockState::parse("tconstruct:earth_slime_dirt").unwrap());
            let used = pluto_lib::UseItemOnBlockContext {
                item: "tconstruct:sky_slime_grass_seeds".into(), count: 1, pos: [26, 80, 26], face: [0, 1, 0],
                dimension: "minecraft:overworld".into(), player: pluto_lib::ServerPlayer { uuid: 1, name: "probe".into() }, creative: true,
            };
            let answer = pluto_lib::UseItemOnBlockCallback::EVENT.invoke(&used);
            let seeded = world.get_block_state([26, 80, 26]).map(|s| s.block.to_string());
            println!("PLUTO-PROBE seeds answer={answer:?} block={seeded:?}");
            // grass under a stone gets no light and turns back into dirt on a random tick
            world.set_block_state([20, 80, 20], &pluto_lib::BlockState::parse("tconstruct:earth_earth_slime_grass").unwrap());
            world.set_block_state([20, 81, 20], &stone);
            // a sapling in its second stage on slime dirt grows into a tree on a random tick
            world.set_block_state([24, 79, 24], &pluto_lib::BlockState::parse("tconstruct:earth_slime_dirt").unwrap());
            world.set_block_state([24, 80, 24], &pluto_lib::BlockState::parse("tconstruct:earth_slime_sapling[stage=1]").unwrap());
            // leaves count their distance from a slime log, and from leaves next to it
            let log = pluto_lib::BlockState::parse("tconstruct:skyroot_log").unwrap();
            let leaves = pluto_lib::BlockState::parse("tconstruct:sky_slime_leaves").unwrap();
            world.set_block_state([44, 81, 44], &log);
            world.set_block_state([45, 81, 44], &leaves);
            world.set_block_state([46, 81, 44], &leaves);
            let distance = |x: i32| world.get_block_state([x, 81, 44]).and_then(|s| s.property("distance").map(str::to_owned));
            println!("PLUTO-PROBE leaves near={:?} next={:?}", distance(45), distance(46));
            world.set_block_state([30, 96, 30], &pluto_lib::BlockState::parse("tconstruct:enderbark_roots").unwrap());
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
        registrar.register("pluto_probe check", |ctx| {
            let later = pluto_lib::World::new("minecraft:overworld").get_block_state([5, 81, 5]).map(|s| s.block.to_string());
            println!("PLUTO-PROBE punji after_support_removed_later={later:?}");
            let vine = pluto_lib::World::new("minecraft:overworld").get_block_state([40, 81, 41]).map(|s| s.block.to_string());
            println!("PLUTO-PROBE vine after_support_removed={vine:?}");
            let grass = pluto_lib::World::new("minecraft:overworld").get_block_state([20, 80, 20]).map(|s| s.block.to_string());
            println!("PLUTO-PROBE grass covered={grass:?}");
            let around = pluto_lib::World::new("minecraft:overworld");
            let buds = pluto_lib::Direction::ALL
                .iter()
                .filter_map(|d| around.get_block_state(d.relative([34, 80, 34])))
                .filter(|s| s.block.path().ends_with("sky_slime_crystal_bud") || s.block.path().ends_with("sky_slime_crystal_cluster"))
                .count();
            println!("PLUTO-PROBE crystal buds_found={}", buds > 0);
            let chime = around.play_sound("tconstruct:block.sky_crystal.chime", [34.5, 80.5, 34.5], 1.0, 1.0);
            println!("PLUTO-PROBE chime played={chime}");
            let tree = pluto_lib::World::new("minecraft:overworld").get_block_state([24, 80, 24]).map(|s| s.block.to_string());
            println!("PLUTO-PROBE sapling grew={tree:?}");
            let waxed = pluto_lib::World::new("minecraft:overworld").get_block_state([30, 79, 29]).map(|s| s.block.to_string());
            let weathered = pluto_lib::World::new("minecraft:overworld").get_block_state([27, 79, 29]).is_some_and(|s| s.block.path() != "copper_platform");
            println!("PLUTO-PROBE copper waxed={waxed:?} weathered={weathered}");
            let mut drunk: Vec<String> = ctx.source.effects().into_iter().map(|e| e.id).filter(|id| id != "minecraft:speed").collect();
            drunk.sort();
            println!("PLUTO-PROBE bottle effects={drunk:?}");
            // what real clicks did: seeds on dirt, bone meal on grass, on a sapling and on a fungus
            let w = pluto_lib::World::new("minecraft:overworld");
            let name = |pos| w.get_block_state(pos).map(|s| s.block.to_string()).unwrap_or_default();
            let count = |from: [i32; 3], to: [i32; 3], pred: &dyn Fn(&str) -> bool| {
                let mut n = 0;
                for x in from[0]..=to[0] {
                    for y in from[1]..=to[1] {
                        for z in from[2]..=to[2] {
                            if pred(&name([x, y, z])) {
                                n += 1;
                            }
                        }
                    }
                }
                n
            };
            let seeds_grass = name([28, 79, 30]) == "tconstruct:sky_earth_slime_grass";
            let plants = count([27, 80, 28], [33, 81, 32], &|b| b.ends_with("_slime_fern") || b.ends_with("_slime_tall_grass")) > 0;
            let tree = count([30, 80, 28], [34, 95, 32], &|b| b == "tconstruct:greenheart_log") > 0;
            let fungus = count([34, 80, 26], [44, 100, 36], &|b| b == "tconstruct:bloodshroom_log") > 0;
            println!("PLUTO-PROBE real seeds_grass={seeds_grass} bonemeal_plants={plants} sapling_tree={tree} fungus={fungus}");
            1
        });
    });
    // picking a block and digging one in survival reach the callbacks
    pluto_lib::BlockBehaviors::register(pluto_lib::BlockBehavior::new(Identifier::new("tconstruct", "earth_slime_dirt").unwrap()).pick_stack(|pick| {
        println!("PLUTO-PROBE pick called block={}", pick.state.block);
        Some(ItemStack::new(Identifier::new("minecraft", "bone_meal").unwrap(), 1))
    }));
    pluto_lib::BreakSpeedCallback::EVENT.register(|dig| {
        static ONCE: AtomicBool = AtomicBool::new(false);
        if !ONCE.swap(true, Ordering::Relaxed) {
            println!("PLUTO-PROBE break_speed block={} original={}", dig.block.block, dig.original);
        }
    });
    // a random tick on a block the game ticks at random: the callback may write blocks and roll the level's dice
    pluto_lib::BlockBehaviors::register(pluto_lib::BlockBehavior::new(Identifier::new("tconstruct", "enderbark_roots").unwrap()).random_tick(|c| {
        static ONCE: AtomicBool = AtomicBool::new(false);
        let marker = pluto_lib::BlockState::parse("minecraft:glass").unwrap();
        let rolled = c.world.random_int(6);
        let set = c.world.set_block_state([c.pos[0], c.pos[1] + 2, c.pos[2]], &marker);
        if !ONCE.swap(true, Ordering::Relaxed) {
            println!("PLUTO-PROBE random_tick pos={:?} rolled_in_range={} set={set}", c.pos, (0..6).contains(&rolled));
        }
    }));
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
