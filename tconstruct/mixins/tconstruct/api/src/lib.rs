//! Tinkers' Construct, the part that uses only PlutoLib's API (Horizon's Fabric API). Everything that cannot yet be
//! expressed through PlutoLib stays in the manifest's direct injections.
use std::sync::atomic::{AtomicBool, Ordering};

use pluto_lib::{AttachmentRegistry, CommandContext, ItemUse, ItemUseRegistry, UseAction, CommandRegistrationCallback, ConfigEvents, ConfigKind, ConfigRegistry, Identifier, ServerPlayConnectionEvents};

/// `config/tconstruct-common.toml`, `[gameplay] shouldSpawnWithTinkersBook` (default true), as in the original mod.
static SPAWN_WITH_BOOK: AtomicBool = AtomicBool::new(true);

mod fluids;
mod gadgets;
mod shared;
mod trees;
mod tinkering_data;
mod station;
mod tools;
mod world;
mod fluids_table;

pub fn hooks() -> pluto_lib::ModHooks {
    tools::register();
    station::register();
    fluids::register();
    gadgets::register();
    shared::register();
    shared::register_glow();
    shared::register_cheese();
    world::register();
    world::register_leaves();
    world::register_grass();
    world::register_seeds();
    world::register_crystals();
    world::register_crystal_chimes();
    trees::register();
    trees::register_fungus();
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
    // a cheese block: eaten over 32 ticks, feeds 3 (0.4 saturation), leaves 3 cheese ingots
    ItemUseRegistry::register(
        Identifier::new("tconstruct", "cheese_block").expect("valid id"),
        ItemUse::new(32, UseAction::Eat).food(3, 0.4).remainder(Identifier::new("tconstruct", "cheese_ingot").expect("valid id"), 3),
    );
    CommandRegistrationCallback::EVENT.register(|registrar| {
        registrar.register("tconstruct parts", parts);
        registrar.register("tconstruct preview <tool:word> <material:word>", tools::preview_command);
        registrar.register("tconstruct part <part:word> <material:word>", tools::part_command);
        registrar.register("tconstruct craft", tools::craft_command);
        registrar.register("tconstruct station", station::station_command);
        registrar.register("tconstruct upgrade <modifier:word>", tools::upgrade_command);
        registrar.register("tconstruct modifiers add <modifier:word> [level:integer]", |ctx| tools::modifiers_command(ctx, true));
        registrar.register("tconstruct modifiers remove <modifier:word> [level:integer]", |ctx| tools::modifiers_command(ctx, false));
        registrar.register("tconstruct durability add <amount:integer>", |ctx| tools::durability_command(ctx, true));
        registrar.register("tconstruct durability set <amount:integer>", |ctx| tools::durability_command(ctx, false));
        registrar.register("tconstruct slots add <slot_type:word> [count:integer]", |ctx| tools::slots_command(ctx, true));
        registrar.register("tconstruct slots set <slot_type:word> <count:integer>", |ctx| tools::slots_command(ctx, false));
        registrar.register("tconstruct build <tool:word> <material:word> [second:word] [third:word] [fourth:word]", tools::build_command);
        registrar.register("tconstruct assemble", assemble);
    });
    pluto_lib::no_hooks()
}

const PARTS: [&str; 3] = ["tconstruct:flint_pick_head", "tconstruct:wood_binding", "tconstruct:wood_handle"];

/// `/tconstruct parts`: one flint and two sticks become the three parts of a flint pickaxe, if all three fit.
fn parts(ctx: &CommandContext) -> i32 {
    let player = &ctx.source;
    if player.count_item("minecraft:flint") < 1 || player.count_item("minecraft:stick") < 2 || player.empty_slots() < PARTS.len() as u32 {
        return 0;
    }
    player.take_item("minecraft:flint", 1);
    player.take_item("minecraft:stick", 2);
    for part in PARTS {
        player.give_item(part, 1);
    }
    println!("tconstruct: parts crafted for {}", player.name);
    1
}

/// `/tconstruct assemble`: the three parts become a flint pickaxe (the parts come back if it does not fit).
fn assemble(ctx: &CommandContext) -> i32 {
    let player = &ctx.source;
    if PARTS.iter().any(|part| player.count_item(part) < 1) {
        return 0;
    }
    for part in PARTS {
        player.take_item(part, 1);
    }
    if player.give_item("tconstruct:flint_pickaxe", 1) > 0 {
        for part in PARTS {
            player.give_item(part, 1);
        }
        return 0;
    }
    println!("tconstruct: pickaxe assembled for {}", player.name);
    1
}

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
