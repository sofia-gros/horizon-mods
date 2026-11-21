//! The Tinkers creative tabs, from the CreativeModeTab registrations of TinkerCommons (general), TinkerTools, TinkerToolParts,
//! TinkerSmeltery, TinkerWorld, TinkerTables and TinkerFluids and the `displayItems` they run (SlimeKnights, MIT; see LICENSE).
//! The tabs are in the original's order (general, tools, tool parts, smeltery, world, [tables], tanks) with its titles. A PlutoLib
//! tab is fixed when the mod starts, before materials load, so the part, tool and crystal lists are the shipped data baked into
//! creative_parts.rs, creative_tools.rs and creative_fluids.rs; a datapack's extra materials are not listed. A tool stack carries only
//! its material list: its stats need the loaded registries, so they are rebuilt when the stack is used.
//! Only items this port declares are listed. Not here: the Deco tab (it holds only retextured variants of tables, controllers and
//! drains, and anvils, none of which exist yet), the books, food, clear tinted glass, platforms and gadgets of the general tab,
//! tank and proxy tank of the smeltery tab, and the geodes, heads, trees and foliage
//! of the world tab.
use crate::{creative_fluids, creative_parts::{CRYSTALS, PARTS}, creative_tools::TOOLS};
use pluto_lib::{Identifier, ItemGroup, ItemGroupRegistry, ItemStack};

fn item(name: &str) -> ItemStack {
    ItemStack::new(Identifier::new("tconstruct", name).expect("valid item id"), 1)
}

fn part(name: &str, material: &str) -> ItemStack {
    item(name).with_components(&format!("{{\"minecraft:custom_data\":{{Material:\"tconstruct:{material}\"}},\"minecraft:custom_model_data\":{{strings:[\"{material}\"]}}}}"))
}

fn group(name: &str, title: &str, icon: ItemStack) -> ItemGroup {
    ItemGroup::new(Identifier::new("tconstruct", name).expect("valid group id"), title, icon)
}

/// Tool Parts: one stack per part and visible material, the pairs in original order, then a modifier crystal per modifier a recipe makes.
pub(crate) fn tool_parts_group() -> ItemGroup {
    let entries = PARTS.iter().flat_map(|(name, materials)| materials.iter().map(move |material| part(name, material)));
    let crystals = CRYSTALS.iter().map(|modifier| item("modifier_crystal").with_components(&format!("{{\"minecraft:custom_data\":{{modifier:\"{modifier}\"}}}}")));
    group("tool_parts", "Tinkers' Parts", part("pick_head", "wood")).entries(entries).entries(crystals)
}

/// A tool stack made of the given materials, one per material slot, uninitialized like ToolStack.createTool before the registries load;
/// a tool with no materials is a plain stack.
fn tool(name: &str, materials: &[&str]) -> ItemStack {
    if materials.is_empty() { return item(name); }
    let list = materials.iter().map(|material| format!("\"tconstruct:{material}\"")).collect::<Vec<_>>().join(",");
    // The item models select their sprites on custom_model_data.strings, one material path per slot, as tool_materials::sync_model keeps them.
    let sprites = materials.iter().map(|material| format!("\"{material}\"")).collect::<Vec<_>>().join(",");
    item(name).with_components(&format!("{{\"minecraft:custom_data\":{{tic_materials:[{list}]}},\"minecraft:custom_model_data\":{{strings:[{sprites}]}}}}"))
}

/// Tools: every tool in the original order, each once per visible material.
pub(crate) fn tools_group() -> ItemGroup {
    let entries = TOOLS.iter().flat_map(|(name, variants)| variants.iter().map(move |materials| tool(name, materials)));
    group("tools", "Tinkers' Tools", item("pickaxe")).entries(entries)
}

/// The tables, then the crafting items of the general tab that this port declares (TinkerTables.addTabItems, the pattern last),
/// then TinkerCommons' glass section and bars (the clear and stained glass in GlassColor order, the soul glass, the panes in the
/// same order, gold bars, obsidian pane; the clear tinted glass is not declared), then the metals of TinkerMaterials.addTabItems,
/// which the general tab calls (its non-metal materials are not declared): the loose nuggets where the original puts them and,
/// per metal, the block, the ingot and the nugget.
pub(crate) const GENERAL: &[&str] = &["part_builder", "tinker_station", "modifier_worktable", "pattern",
    "clear_glass", "white_clear_stained_glass", "orange_clear_stained_glass", "magenta_clear_stained_glass", "light_blue_clear_stained_glass", "yellow_clear_stained_glass", "lime_clear_stained_glass", "pink_clear_stained_glass", "gray_clear_stained_glass", "light_gray_clear_stained_glass", "cyan_clear_stained_glass", "purple_clear_stained_glass", "blue_clear_stained_glass", "brown_clear_stained_glass", "green_clear_stained_glass", "red_clear_stained_glass", "black_clear_stained_glass", "soul_glass",
    "clear_glass_pane", "white_clear_stained_glass_pane", "orange_clear_stained_glass_pane", "magenta_clear_stained_glass_pane", "light_blue_clear_stained_glass_pane", "yellow_clear_stained_glass_pane", "lime_clear_stained_glass_pane", "pink_clear_stained_glass_pane", "gray_clear_stained_glass_pane", "light_gray_clear_stained_glass_pane", "cyan_clear_stained_glass_pane", "purple_clear_stained_glass_pane", "blue_clear_stained_glass_pane", "brown_clear_stained_glass_pane", "green_clear_stained_glass_pane", "red_clear_stained_glass_pane", "black_clear_stained_glass_pane", "soul_glass_pane",
    "gold_bars", "obsidian_pane",
    "copper_nugget", "cobalt_block", "cobalt_ingot", "cobalt_nugget", "steel_block", "steel_ingot", "steel_nugget", "debris_nugget", "slimesteel_block", "slimesteel_ingot", "slimesteel_nugget", "amethyst_bronze_block", "amethyst_bronze_ingot", "amethyst_bronze_nugget", "rose_gold_block", "rose_gold_ingot", "rose_gold_nugget", "pig_iron_block", "pig_iron_ingot", "pig_iron_nugget", "cinderslime_block", "cinderslime_ingot", "cinderslime_nugget", "queens_slime_block", "queens_slime_ingot", "queens_slime_nugget", "manyullyn_block", "manyullyn_ingot", "manyullyn_nugget", "hepatizon_block", "hepatizon_ingot", "hepatizon_nugget", "netherite_nugget", "knightmetal_block", "knightmetal_ingot", "knightmetal_nugget", "knightslime_block", "knightslime_ingot", "knightslime_nugget"];

pub(crate) fn general_group() -> ItemGroup {
    group("general", "Tinkers' General", item("tinker_station")).entries(GENERAL.iter().map(|name| item(name)))
}

/// A tank item holding a fluid (`TankItem.fillTank`): the fluid saved under `tank` in the item's custom data, as the block entity saves it.
fn filled_tank(name: &str, fluid: &str, amount: i32) -> ItemStack {
    item(name).with_components(&format!("{{\"minecraft:custom_data\":{{tank:{{FluidName:\"{fluid}\",Amount:{amount}}}}}}}"))
}

/// The tank type names in `TankType` order and their capacities, from the tank block data.
fn capacity(kind: &str) -> i32 {
    crate::tank::TYPES.iter().find(|(name, _)| *name == kind).map(|(_, capacity)| *capacity).expect("a tank type")
}

/// What a `BuildingBlockObject` adds: the block, then slab, stairs and, for the wall and fence kinds, that.
fn building(name: &str, extra: &[&str]) -> Vec<String> {
    let mut names = vec![name.to_owned(), format!("{name}_slab"), format!("{name}_stairs")];
    names.extend(extra.iter().map(|extra| format!("{name}_{extra}")));
    names
}

/// The casts in `TinkerSmeltery.addCasts` order for one family (`""`, `"sand_"` or `"red_sand_"`); the arrow cast is a single item.
const CAST_ORDER: &[&str] = &[
    "ingot", "nugget", "gem", "rod", "repair_kit", "plate", "gear", "coin", "wire", "pick_head", "small_axe_head", "small_blade", "adze_head",
    "hammer_head", "broad_axe_head", "broad_blade", "large_plate", "tool_handle", "tool_binding", "tough_handle", "tough_binding",
    "bow_limb", "bow_grip", "ARROW", "helmet_plating", "chestplate_plating", "leggings_plating", "boots_plating", "maille",
];

fn casts(family: &str) -> Vec<String> {
    CAST_ORDER.iter().filter(|name| **name != "ARROW" || family.is_empty()).map(|name| match *name {
        "ARROW" => "arrow_cast".to_owned(),
        name => format!("{name}_{family}cast"),
    }).collect()
}

/// TinkerSmeltery.addTabItems for the declared items: crafting items, controllers, IO blocks, tanks, fluid transfer, casting blocks,
/// the seared and scorched building blocks and the casts; the buckets and bottles of TinkerFluids.addTabItems close it, as the smeltery
/// calls it last. The soulsteel bucket is in no tab: the original registers it but leaves it for the future.
pub(crate) fn smeltery_group() -> ItemGroup {
    let mut names: Vec<String> = Vec::new();
    let mut add = |list: &[&str]| names.extend(list.iter().map(|name| (*name).to_owned()));
    add(&["grout", "seared_brick", "nether_grout", "scorched_brick", "copper_can"]);
    add(&["seared_melter", "seared_heater", "scorched_alloyer", "smeltery_controller", "foundry_controller"]);
    add(&["seared_drain", "seared_duct", "seared_chute", "scorched_drain", "scorched_duct", "scorched_chute"]);
    let smeltery = group("smeltery", "Tinkers' Smeltery", item("smeltery_controller"));
    let tank = |kind: &str| -> Vec<ItemStack> {
        let mut stacks: Vec<ItemStack> = crate::tank::TYPES.iter().map(|(name, _)| item(&format!("{kind}_{name}"))).collect();
        // toss in some pre filled fuel tanks
        for fluid in ["minecraft:lava", "tconstruct:blazing_blood"] { stacks.push(filled_tank(&format!("{kind}_fuel_tank"), fluid, capacity("fuel_tank"))); }
        stacks
    };
    let mut entries: Vec<ItemStack> = names.drain(..).map(|name| item(&name)).collect();
    entries.extend(["copper_gauge", "obsidian_gauge"].iter().map(|name| item(name)));
    entries.extend(tank("seared"));
    entries.push(item("seared_lantern"));
    entries.extend(tank("scorched"));
    entries.push(item("scorched_lantern"));
    let mut rest: Vec<String> = Vec::new();
    rest.extend(["seared_faucet", "scorched_faucet", "seared_channel", "scorched_channel"].map(str::to_owned));
    rest.extend(["seared_table", "scorched_table", "seared_basin", "scorched_basin"].map(str::to_owned));
    rest.extend(["seared_fluid_cannon", "scorched_fluid_cannon", "end_fluid_cannon"].map(str::to_owned));
    rest.extend(["seared_casting_tank", "scorched_proxy_tank"].map(str::to_owned));
    rest.extend(building("seared_bricks", &["wall"]));
    rest.extend(building("seared_stone", &[]));
    rest.extend(["seared_cracked_bricks", "seared_fancy_bricks", "seared_triangle_bricks"].map(str::to_owned));
    rest.extend(building("seared_cobble", &["wall"]));
    rest.extend(building("seared_paver", &[]));
    rest.extend(["seared_lamp", "seared_ladder", "seared_glass", "seared_tinted_glass", "seared_soul_glass", "seared_glass_pane", "seared_soul_glass_pane"].map(str::to_owned));
    rest.extend(building("scorched_bricks", &["fence"]));
    rest.extend(["chiseled_scorched_bricks", "scorched_stone", "polished_scorched_stone"].map(str::to_owned));
    rest.extend(building("scorched_road", &[]));
    rest.extend(["scorched_lamp", "scorched_ladder", "scorched_glass", "scorched_tinted_glass", "scorched_soul_glass", "scorched_glass_pane", "scorched_soul_glass_pane"].map(str::to_owned));
    rest.extend(casts(""));
    rest.push("blank_sand_cast".to_owned());
    rest.extend(casts("sand_"));
    rest.push("blank_red_sand_cast".to_owned());
    rest.extend(casts("red_sand_"));
    // TinkerFluids.addTabItems, which the smeltery calls after the casts: the slime fluids and their bottles, the food fluids and the
    // meat soup bowl, the stone fluids, the ore, metal and alloy fluids, then the compat metals and alloys. The splash and lingering
    // bottles are not declared in this port, and the soulsteel bucket is in no tab in the original.
    rest.extend(["earth_slime_bucket", "sky_slime_bucket", "ichor_bucket", "ender_slime_bucket", "earth_slime_bottle", "sky_slime_bottle", "ichor_slime_bottle", "ender_slime_bottle", "magma_bucket", "magma_bottle", "venom_bucket", "venom_bottle"].map(str::to_owned));
    rest.extend(["honey_bucket", "beetroot_soup_bucket", "mushroom_stew_bucket", "rabbit_stew_bucket", "meat_soup_bucket", "meat_soup"].map(str::to_owned));
    rest.extend(["seared_stone_bucket", "scorched_stone_bucket", "molten_clay_bucket", "molten_porcelain_bucket", "molten_glass_bucket", "molten_obsidian_bucket", "liquid_soul_bucket", "molten_ender_bucket", "blazing_blood_bucket"].map(str::to_owned));
    rest.extend(["molten_emerald_bucket", "molten_quartz_bucket", "molten_amethyst_bucket", "molten_diamond_bucket", "molten_debris_bucket"].map(str::to_owned));
    rest.extend(["molten_copper_bucket", "molten_iron_bucket", "molten_gold_bucket", "molten_cobalt_bucket", "molten_steel_bucket"].map(str::to_owned));
    rest.extend(["molten_slimesteel_bucket", "molten_amethyst_bronze_bucket", "molten_rose_gold_bucket", "molten_pig_iron_bucket"].map(str::to_owned));
    rest.extend(["molten_cinderslime_bucket", "molten_queens_slime_bucket", "molten_manyullyn_bucket", "molten_hepatizon_bucket", "molten_netherite_bucket", "molten_knightmetal_bucket", "molten_knightslime_bucket"].map(str::to_owned));
    rest.extend(["molten_tin_bucket", "molten_aluminum_bucket", "molten_lead_bucket", "molten_silver_bucket", "molten_nickel_bucket", "molten_zinc_bucket", "molten_platinum_bucket", "molten_tungsten_bucket", "molten_osmium_bucket", "molten_uranium_bucket", "molten_chromium_bucket", "molten_cadmium_bucket"].map(str::to_owned));
    rest.extend(["molten_bronze_bucket", "molten_brass_bucket", "molten_electrum_bucket", "molten_invar_bucket", "molten_constantan_bucket", "molten_pewter_bucket", "molten_nicrosil_bucket", "molten_enderium_bucket", "molten_lumium_bucket", "molten_signalum_bucket", "molten_refined_glowstone_bucket", "molten_refined_obsidian_bucket", "molten_duralumin_bucket", "molten_bendalloy_bucket", "molten_steeleaf_bucket", "fiery_liquid_bucket"].map(str::to_owned));
    entries.extend(rest.iter().map(|name| item(name)));
    smeltery.entries(entries)
}

/// TinkerWorld.addTabItems for the declared items: ores, then each slime (ball, block, congealed), slime dirt and the shards.
pub(crate) const WORLD: &[&str] = &[
    "cobalt_ore", "raw_cobalt", "raw_cobalt_block",
    "earth_congealed_slime",
    "sky_slime_ball", "sky_slime", "sky_congealed_slime", "ichor_slime_ball", "ichor_slime", "ichor_congealed_slime", "ender_slime_ball", "ender_slime", "ender_congealed_slime",
    "earth_slime_dirt", "sky_slime_dirt", "ichor_slime_dirt", "ender_slime_dirt",
    "steel_shard", "cobalt_shard", "knightmetal_shard",
];

pub(crate) fn world_group() -> ItemGroup {
    group("world", "Tinkers' World", item("cobalt_ore")).entries(WORLD.iter().map(|name| item(name)))
}

/// Tanks (TinkerFluids.tabFluids, TankItem.addFilledVariants): for every source fluid in registry order, the seared and scorched tank and
/// gauge filled to capacity; metals use the ingot kinds, the rest the fuel kinds.
pub(crate) fn fluids_group() -> ItemGroup {
    let vanilla = [("minecraft:water", false), ("minecraft:lava", false)].into_iter().map(|(name, metal)| (name.to_owned(), metal));
    let tinkers = creative_fluids::FLUIDS.iter().map(|(name, metal)| (format!("tconstruct:{name}"), *metal));
    let mut entries = Vec::new();
    for (fluid, metal) in vanilla.chain(tinkers) {
        let (tank, gauge) = if metal { ("ingot_tank", "ingot_gauge") } else { ("fuel_tank", "fuel_gauge") };
        for kind in ["seared", "scorched"] {
            entries.push(filled_tank(&format!("{kind}_{tank}"), &fluid, capacity(tank)));
            entries.push(filled_tank(&format!("{kind}_{gauge}"), &fluid, capacity(gauge)));
        }
    }
    group("fluids", "Tinkers' Tanks", filled_tank("seared_fuel_gauge", "tconstruct:molten_cobalt", capacity("fuel_gauge"))).entries(entries)
}

/// Every tab, in the original's order. The Deco tab (`tables`) is not among them: it lists only retextured variants.
pub(crate) fn groups() -> Vec<ItemGroup> {
    vec![general_group(), tools_group(), tool_parts_group(), smeltery_group(), world_group(), fluids_group()]
}

pub(crate) fn register() {
    for group in groups() { ItemGroupRegistry::register(group); }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_part_lists_each_material_once_in_original_order_and_stacks_carry_the_material() {
        let group = tool_parts_group();
        let expected: usize = PARTS.iter().map(|(_, materials)| materials.len()).sum::<usize>() + CRYSTALS.len();
        assert_eq!(group.entries.len(), expected);
        assert!(expected > 500, "the shipped data gives a large catalog");
        let first = &group.entries[0];
        assert_eq!(first.item.to_string(), "tconstruct:pick_head");
        assert_eq!(first.to_snbt(), "{count:1,id:\"tconstruct:pick_head\",components:{\"minecraft:custom_data\":{Material:\"tconstruct:wood\"},\"minecraft:custom_model_data\":{strings:[\"wood\"]}}}");
        // No part repeats a material.
        for (item, materials) in PARTS {
            let mut seen = std::collections::BTreeSet::new();
            assert!(materials.iter().all(|material| seen.insert(*material)), "{item} repeats a material");
        }
        // Parts of one stat type share a list, and a stat type only some materials have gives a shorter one.
        let materials = |name: &str| PARTS.iter().find(|(item, _)| *item == name).map(|(_, m)| *m).unwrap();
        assert_eq!(materials("pick_head"), materials("hammer_head"));
        assert!(materials("helmet_plating").len() < materials("pick_head").len(), "plating is not on every material");
        assert!(materials("ribcage").len() < materials("pick_head").len(), "slimesuit repair parts are rarer");
        // The crystals close the tab, one per modifier, carrying it where the table reads it.
        let last = group.entries.last().unwrap();
        assert_eq!(last.item.to_string(), "tconstruct:modifier_crystal");
        assert!(last.to_snbt().contains(&format!("modifier:\"{}\"", CRYSTALS.last().unwrap())), "{}", last.to_snbt());
    }

    #[test]
    fn tools_tab_lists_each_tool_per_material_with_slots_filled_like_create_single_material() {
        let group = tools_group();
        let expected: usize = TOOLS.iter().map(|(_, variants)| variants.len()).sum();
        assert_eq!(group.entries.len(), expected);
        let find = |item: &str| TOOLS.iter().find(|(name, _)| *name == item).map(|(_, v)| *v).unwrap();
        // tools that lack materials come first, as plain stacks
        assert_eq!(group.entries[0].to_snbt(), "{count:1,id:\"tconstruct:flint_and_brick\"}");
        assert_eq!(find("sky_staff").len(), 1);
        let pickaxe = find("pickaxe");
        assert!(pickaxe.len() > 20 && pickaxe.iter().all(|slots| slots.len() == 3), "a pickaxe has head, handle and binding");
        assert!(find("broad_axe").iter().all(|slots| slots.len() == 4), "the broad axe lists four part slots");
        let armor = group.entries.iter().find(|stack| stack.item.path() == "travelers_helmet").unwrap();
        assert_eq!(armor.to_snbt(), "{count:1,id:\"tconstruct:travelers_helmet\",components:{\"minecraft:custom_data\":{tic_materials:[\"tconstruct:copper\",\"tconstruct:leather\"]},\"minecraft:custom_model_data\":{strings:[\"copper\",\"leather\"]}}}");
        // Wings have one slot; plate takes plating and maille, so a plating-only material gets the first maille material.
        assert!(find("slime_wings").iter().all(|m| m.len() == 1));
        assert!(find("plate_helmet").len() == find("plate_chestplate").len() && find("slime_helmet").len() > find("slime_chestplate").len());
        let plating: Vec<&str> = PARTS.iter().find(|(item, _)| *item == "helmet_plating").unwrap().1.to_vec();
        assert!(plating.iter().all(|material| find("plate_helmet").iter().any(|slots| slots[0] == *material)));
        let order: Vec<&str> = TOOLS.iter().map(|(name, _)| *name).collect();
        assert_eq!(order.first(), Some(&"flint_and_brick"));
        assert_eq!(order.last(), Some(&"slime_wings"));
    }

    #[test]
    fn the_tabs_come_in_the_original_order_with_its_titles_and_the_tank_tab_fills_every_fluid() {
        let all = groups();
        let ids: Vec<String> = all.iter().map(|group| group.id.path().to_owned()).collect();
        assert_eq!(ids, ["general", "tools", "tool_parts", "smeltery", "world", "fluids"]);
        let titles: Vec<&str> = all.iter().map(|group| group.display_name.as_str()).collect();
        assert_eq!(titles, ["Tinkers' General", "Tinkers' Tools", "Tinkers' Parts", "Tinkers' Smeltery", "Tinkers' World", "Tinkers' Tanks"]);
        let fluids = fluids_group();
        // water, lava and the 71 fluids that are not hidden (the potion): four tanks each
        assert_eq!(fluids.entries.len(), 4 * (2 + creative_fluids::FLUIDS.len()));
        let snbt: Vec<String> = fluids.entries.iter().map(ItemStack::to_snbt).collect();
        assert!(snbt.contains(&"{count:1,id:\"tconstruct:seared_ingot_tank\",components:{\"minecraft:custom_data\":{tank:{FluidName:\"tconstruct:molten_iron\",Amount:4320}}}}".to_owned()), "metals fill ingot tanks");
        assert!(snbt.contains(&"{count:1,id:\"tconstruct:scorched_fuel_gauge\",components:{\"minecraft:custom_data\":{tank:{FluidName:\"minecraft:lava\",Amount:4000}}}}".to_owned()), "other fluids fill fuel gauges");
        let smeltery = smeltery_group();
        assert!(smeltery.entries.iter().any(|stack| stack.to_snbt().contains("tconstruct:blazing_blood")), "pre filled fuel tanks");
        let position = |name: &str| smeltery.entries.iter().position(|stack| stack.item.path() == name && stack.components.is_none()).unwrap();
        assert!(position("grout") < position("seared_melter") && position("seared_table") < position("seared_bricks") && position("seared_bricks") < position("ingot_cast") && position("ingot_cast") < position("blank_sand_cast") && position("blank_sand_cast") < position("ingot_sand_cast") && position("ingot_red_sand_cast") < position("venom_bottle"));
    }
}
