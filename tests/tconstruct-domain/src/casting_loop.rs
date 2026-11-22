//! The casting loop over the shipped data, as the game would run it: molten material fluid + cast on a table or basin makes the right part
//! with its material (and the sprite keys that show it), a part melts back into the fluid that casts it, and cast patterns, tool station and
//! creative stacks agree. The item registry here is built the way the host builds it: the declared items and blocks, the shipped item tags
//! (`data/*/tags/item`) and Forge's convention tags that the importer lists (`convention_tags.rs`) added through `with_convention_tags`.
use crate::{
    casting::{Live, State, INPUT, OUTPUT},
    casting_recipes::{CastingKind, MaterialCasting, Station},
    fluid_values::calc_cooling_time,
    material_recipes::Ingredient,
    materials::Definitions,
    original_data::{fluid_tag_files, recursive_files, world},
    smeltery_data::Data,
};
use pluto_lib::{FluidStack, Identifier, ItemSlot, RegistryLookup};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
fn content() -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tconstruct/content/data") }

/// Every item and block id the port declares.
pub(crate) fn declared() -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for kind in ["item", "block"] {
        for entry in std::fs::read_dir(content().join("tconstruct/horizon").join(kind)).expect("declarations") {
            let path = entry.unwrap().path();
            found.insert(format!("tconstruct:{}", path.file_stem().unwrap().to_str().unwrap()));
        }
    }
    found
}

/// The shipped item tags by id, as written: `data/<namespace>/tags/item/<path>.json`.
fn item_tag_files() -> BTreeMap<String, Vec<String>> {
    let mut found = BTreeMap::new();
    for namespace in std::fs::read_dir(content()).unwrap() {
        let namespace = namespace.unwrap().path();
        let space = namespace.file_name().unwrap().to_str().unwrap().to_owned();
        let base = namespace.join("tags/item");
        if !base.is_dir() { continue; }
        let mut stack = vec![base.clone()];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() { stack.push(path); continue; }
                let relative = path.strip_prefix(&base).unwrap().with_extension("");
                let tag = format!("{space}:{}", relative.to_str().unwrap().replace('\\', "/"));
                let json: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
                let values = json["values"].as_array().unwrap().iter().map(|entry| entry.as_str().or_else(|| entry["id"].as_str()).unwrap().to_owned()).collect();
                found.insert(tag, values);
            }
        }
    }
    found
}

/// The item registry the game gives Tinkers: declared items, the vanilla ones the tags and conventions name, flattened tags, conventions.
pub(crate) fn item_registry() -> RegistryLookup {
    let declared = declared();
    let files = item_tag_files();
    fn flatten(tag: &str, files: &BTreeMap<String, Vec<String>>, seen: &mut Vec<String>, out: &mut BTreeSet<String>) {
        if seen.iter().any(|s| s == tag) { return; }
        seen.push(tag.to_owned());
        for entry in files.get(tag).into_iter().flatten() {
            match entry.strip_prefix('#') { Some(inner) => flatten(inner, files, seen, out), None => { out.insert(entry.clone()); } }
        }
    }
    let mut entries: BTreeSet<String> = declared.clone();
    let mut tags: Vec<(String, Vec<String>)> = Vec::new();
    for tag in files.keys() {
        let mut members = BTreeSet::new();
        flatten(tag, &files, &mut Vec::new(), &mut members);
        members.retain(|member| member.starts_with("minecraft:") || declared.contains(member));
        entries.extend(members.iter().cloned());
        tags.push((tag.clone(), members.into_iter().collect()));
    }
    for (_, members) in crate::convention_tags::TAGS { entries.extend(members.iter().filter(|m| m.starts_with("minecraft:")).map(|m| (*m).to_owned())); }
    let entries: Vec<&str> = entries.iter().map(String::as_str).collect();
    let tags: Vec<(&str, Vec<&str>)> = tags.iter().map(|(tag, members)| (tag.as_str(), members.iter().map(String::as_str).collect())).collect();
    let tag_refs: Vec<(&str, &[&str])> = tags.iter().map(|(tag, members)| (*tag, members.as_slice())).collect();
    crate::common::with_convention_tags(pluto_lib::fixtures::registry_with_tags(&entries, &tag_refs).unwrap())
}

pub(crate) struct Game { pub data: Data, pub items: RegistryLookup, pub materials: Definitions, pub tools: BTreeMap<Identifier, crate::tool_definitions::Data>, pub registry: crate::modifiers::Registry }

pub(crate) fn game() -> &'static Game {
    static GAME: std::sync::OnceLock<Game> = std::sync::OnceLock::new();
    GAME.get_or_init(|| {
        let items = item_registry();
        let context = crate::conditions::Context { items: Some(items.clone()), config: crate::config::parse("").unwrap() };
        let recipes = crate::fluid_recipes::Recipes::load(&recursive_files("fluids", "fluids"), &context);
        let tags = crate::fluid_ingredient::FluidTags::load(&fluid_tag_files());
        let data = Data { recipes, tags, recipes_loaded: true, tags_loaded: true };
        let world = world();
        Game { data, items, materials: world.materials, tools: world.tools.into_iter().collect(), registry: world.registry }
    })
}

impl Game {
    fn live(&self, station: Station) -> Live<'_> { Live { data: &self.data, item_tags: Some(&self.items), station, materials: Some(&self.materials), tool_defs: Some(&self.tools), modifiers: Some(&self.registry) } }
    fn in_fluid_tag(&self, fluid: &Identifier, tag: &Identifier) -> bool { self.data.tags.contains(fluid, tag) }
    /// The first concrete fluid a fluid ingredient names.
    fn fluid_of(&self, recipe: &crate::casting_recipes::MaterialFluid) -> Identifier {
        match &recipe.fluid.options[0].0 {
            crate::fluid_ingredient::Source::Id(fluid) => fluid.clone(),
            crate::fluid_ingredient::Source::Tag(tag) => self.data.tags.members(tag).and_then(|members| members.first().cloned()).unwrap_or_else(|| panic!("{}: fluid tag {tag} has no members", recipe.id)),
        }
    }
    /// Items an ingredient can be (the cast of a recipe).
    fn items_of(&self, ingredient: &Ingredient) -> Vec<Identifier> {
        match ingredient {
            Ingredient::Item(item) => vec![item.clone()],
            Ingredient::Tag(tag) => self.items.get_tag(tag).map(|members| members.iter().cloned().collect()).unwrap_or_default(),
            Ingredient::Any(list) => list.iter().flat_map(|inner| self.items_of(inner)).collect(),
            Ingredient::Difference { base, subtracted } => { let taken = self.items_of(subtracted); self.items_of(base).into_iter().filter(|item| !taken.contains(item)).collect() }
            _ => Vec::new(),
        }
    }
}

fn pour_and_cool(live: &Live, cast: &ItemSlot, fluid: &Identifier) -> (State, Vec<Option<ItemSlot>>, u32) {
    let (mut state, mut items) = (State::default(), vec![Some(cast.clone()), None]);
    let taken = state.fill(live, &mut items, &FluidStack { fluid: fluid.clone(), amount: 100_000, tag: None }, false);
    let cooling = state.cooling.max(0);
    for _ in 0..cooling { state.tick(live, &mut items); }
    (state, items, taken)
}

/// A pour of a fluid stack as it is (tag included), cooled to the end: the items afterwards and how much was taken.
fn pour_stack(live: &Live, cast: &ItemSlot, fluid: &FluidStack) -> (Vec<Option<ItemSlot>>, u32) {
    let (mut state, mut items) = (State::default(), vec![Some(cast.clone()), None]);
    let taken = state.fill(live, &mut items, fluid, false);
    for _ in 0..state.cooling.max(0) { state.tick(live, &mut items); }
    (items, taken)
}

fn material_of(stack: &ItemSlot) -> String { crate::tool_materials::part_material(&stack.custom_data).to_string() }

#[test]
fn the_item_registry_the_game_gives_has_the_convention_tags_the_casting_data_names() {
    let items = &game().items;
    for (tag, member) in [("forge:ingots/iron", "minecraft:iron_ingot"), ("forge:ingots/gold", "minecraft:gold_ingot"), ("forge:ingots/copper", "minecraft:copper_ingot"),
        ("forge:nuggets/iron", "minecraft:iron_nugget"), ("forge:storage_blocks/iron", "minecraft:iron_block"), ("forge:gems/diamond", "minecraft:diamond"),
        ("forge:gems/quartz", "minecraft:quartz"), ("forge:glass/colorless", "minecraft:glass"), ("forge:rods/wooden", "minecraft:stick"), ("forge:ingots/netherite", "minecraft:netherite_ingot")] {
        assert!(items.get_tag(&id(tag)).is_some_and(|members| members.contains(&id(member))), "{tag} lacks {member}");
    }
}

#[test]
fn every_part_cast_pours_every_castable_material_into_the_right_part_with_its_material_and_sprite_key() {
    let game = game();
    let in_fluid = |fluid: &Identifier, tag: &Identifier| game.in_fluid_tag(fluid, tag);
    let declared = declared();
    let (mut poured, mut refused, mut parts_seen, mut casts_seen, mut skipped) = (0, 0, BTreeSet::new(), BTreeSet::new(), BTreeSet::<String>::new());
    for station in [Station::Table, Station::Basin] {
        let live = game.live(station);
        for recipe in game.data.recipes.casting.iter().filter(|recipe| recipe.station == station) {
            let CastingKind::Material { result, item_cost } = &recipe.kind else { continue };
            // The fake ingot and fake storage block show a material as an ingot or block. They are declared
            // as plain items, but their material-item registration (a repair kit for the compatibility
            // materials, none of which this port ships) is not ported, so they are not material items here.
            if !declared.contains(&result.to_string()) || result.path() == "fake_ingot" || result.path() == "fake_storage_block" { skipped.insert(result.to_string()); continue; }
            let part = crate::tool_parts::material_items().get(result).cloned().unwrap_or_else(|| panic!("{result} is not a material item"));
            let casts = recipe.cast.as_ref().map(|cast| game.items_of(cast)).unwrap_or_default();
            assert!(!casts.is_empty(), "{}: the cast ingredient names no item the game has", recipe.id);
            for cast in casts {
                casts_seen.insert(cast.to_string());
                for fluid_recipe in &game.data.recipes.material_casting.casting {
                    let fluid = game.fluid_of(fluid_recipe);
                    let (state, items, taken) = pour_and_cool(&live, &ItemSlot::new(cast.clone(), 1), &fluid);
                    let material = &fluid_recipe.output;
                    if !part.can_use(material, &game.materials) {
                        assert_eq!(taken, 0, "{}: {fluid} must not cast {result} of {material}", recipe.id);
                        refused += 1;
                        continue;
                    }
                    let amount = MaterialCasting::fluid_amount(fluid_recipe, *item_cost, &fluid, &in_fluid);
                    assert_eq!(taken as i32, amount, "{}: {fluid} into {result} takes the fluid amount times the part cost", recipe.id);
                    assert_eq!(MaterialCasting::cooling_time(fluid_recipe, *item_cost, &fluid, &in_fluid), calc_cooling_time(fluid_recipe.temperature, amount));
                    let slot = if recipe.switch_slots { INPUT } else { OUTPUT };
                    let made = items[slot].as_ref().unwrap_or_else(|| panic!("{}: {fluid} on {cast} made nothing in slot {slot}: {items:?}", recipe.id));
                    assert_eq!(&made.item, result, "{}", recipe.id);
                    assert_eq!(material_of(made), material.to_string(), "{}: the part carries the fluid's material", recipe.id);
                    let strings = made.custom_model_data.as_ref().map(|data| data.strings.clone()).unwrap_or_default();
                    assert_eq!(strings, vec![material.material_id().unwrap().path().to_owned()], "{}: the sprite key follows the material", recipe.id);
                    // the cast: kept for a gold cast, used up for sand, red sand and clay casts
                    let other = if slot == INPUT { OUTPUT } else { INPUT };
                    assert_eq!(items[other].is_none(), recipe.cast_consumed, "{}", recipe.id);
                    assert!(state.fluid.is_none() && state.recipe.is_none(), "the table is reset after a casting");
                    poured += 1;
                    parts_seen.insert(result.to_string());
                }
            }
        }
    }
    assert!(poured > 400 && refused > 0, "{poured} castings, {refused} refused");
    assert_eq!(skipped.into_iter().collect::<Vec<_>>(), ["tconstruct:fake_ingot", "tconstruct:fake_storage_block"], "only the fake parts are skipped");
    // every tool part and the repair kit has a cast that makes it, of every kind: gold (multi use), sand and red sand
    for (part, _) in crate::creative_parts::PARTS {
        assert!(parts_seen.contains(&format!("tconstruct:{part}")) || ["bowstring", "arrow_head", "arrow_shaft", "fletching", "shield_core", "ribcage", "shell", "laces", "fake_ingot"].contains(part),
            "{part} is never cast: {parts_seen:?}");
    }
    for family in ["", "sand_", "red_sand_"] {
        assert!(casts_seen.contains(&format!("tconstruct:pick_head_{family}cast")), "the {family}pick head cast makes parts");
    }
}

#[test]
fn composite_casting_coats_a_part_of_one_material_in_another_and_uses_the_part_up() {
    let game = game();
    let in_fluid = |fluid: &Identifier, tag: &Identifier| game.in_fluid_tag(fluid, tag);
    let declared = declared();
    let (mut coated, mut preempted) = (0, 0);
    for station in [Station::Table, Station::Basin] {
        let live = game.live(station);
        for recipe in game.data.recipes.casting.iter().filter(|recipe| recipe.station == station) {
            let CastingKind::Composite { result, item_cost, stat_conflict } = &recipe.kind else { continue };
            // The fake parts are declared as plain items; their material-item registration is not
            // ported (see the material casting test), so they are not material items here either.
            if !declared.contains(&result.to_string()) || result.path() == "fake_ingot" || result.path() == "fake_storage_block" { continue; }
            let part = crate::tool_parts::material_items().get(result).cloned().unwrap_or_else(|| panic!("{result} is not a material item"));
            for fluid_recipe in &game.data.recipes.material_casting.composite {
                let Some(input) = &fluid_recipe.input else { continue };
                let fluid = game.fluid_of(fluid_recipe);
                let mut cast = ItemSlot::new(result.clone(), 1);
                crate::tool_materials::set_part_material(&mut cast, input);
                let (_, items, taken) = pour_and_cool(&live, &cast, &fluid);
                // ToolCastingRecipe shares the station, and its recipes sort before the parts' composite recipes: for the
                // parts that are also tool casts (shield core, laces, ribcage, shell), a fluid casting a material the tool's
                // fluid slot can use builds the tool instead of coating the part.
                if let Some(tool_recipe) = game.data.recipes.casting.iter().find(|candidate| candidate.station == station
                    && matches!(&candidate.kind, CastingKind::Tool { .. }) && candidate.cast.as_ref().is_some_and(|cast| game.items_of(cast).contains(result))) {
                    let CastingKind::Tool { result: tool_result, item_cost: tool_cost, cast_purpose, .. } = &tool_recipe.kind else { unreachable!() };
                    let stats = game.tools[tool_result].material_stat_types().unwrap();
                    let stat_index = if *cast_purpose == crate::casting_recipes::CastPurpose::MaybeMaterial { stats.len() - 1 } else { cast_purpose.fluid_index() as usize };
                    if let Some(casting) = game.data.recipes.material_casting.casting_fluid(&fluid, &in_fluid) {
                        if casting.output.material_id().is_some_and(|material| game.materials.get_stats(&material, &stats[stat_index].0).is_some()) {
                            assert_eq!(taken as i32, MaterialCasting::fluid_amount(casting, *tool_cost, &fluid, &in_fluid), "{}: the tool recipe takes the pour", tool_recipe.id);
                            let made = items[OUTPUT].as_ref().unwrap_or_else(|| panic!("{}: {fluid} on {input} {result} built nothing", tool_recipe.id));
                            assert_eq!(&made.item, tool_result, "{}", tool_recipe.id);
                            assert_eq!(materials_of(made), [input.to_string(), casting.output.to_string()], "{}: the part's material first, the fluid's second", tool_recipe.id);
                            preempted += 1;
                            continue;
                        }
                    }
                }
                // CompositeCastingRecipe.getFluidRecipe: a fluid that already casts a material the stat uses is left to tool casting
                let conflicts = stat_conflict.as_ref().is_some_and(|stat| game.data.recipes.material_casting.casting_fluid(&fluid, &in_fluid)
                    .and_then(|casting| casting.output.material_id()).is_some_and(|material| game.materials.get_stats(&material, stat).is_some()));
                if conflicts || !part.can_use(&fluid_recipe.output, &game.materials) || &fluid_recipe.output == input {
                    assert_eq!(taken, 0, "{}: {fluid} on {input} {result}", recipe.id);
                    continue;
                }
                assert_eq!(taken as i32, MaterialCasting::fluid_amount(fluid_recipe, *item_cost, &fluid, &in_fluid), "{}", recipe.id);
                let made = items[OUTPUT].as_ref().unwrap_or_else(|| panic!("{}: {fluid} on {input} {result} made nothing: {items:?}", recipe.id));
                assert_eq!((&made.item, material_of(made)), (result, fluid_recipe.output.to_string()), "{}", recipe.id);
                assert!(items[INPUT].is_none(), "the coated part is used up");
                let strings = made.custom_model_data.as_ref().map(|data| data.strings.clone()).unwrap_or_default();
                assert_eq!(strings, vec![fluid_recipe.output.material_id().unwrap().path().to_owned()]);
                coated += 1;
            }
        }
    }
    assert!(coated > 0, "some composite casting happened");
    assert!(preempted > 0, "some pours built tools instead of coating the part");
}

#[test]
fn gold_casts_are_made_from_the_item_they_shape_and_molten_gold_and_sand_casts_from_a_part_and_a_blank_one() {
    let game = game();
    let gold = id("tconstruct:molten_gold");
    let live = game.live(Station::Table);
    let (mut made, mut unobtainable) = (BTreeSet::new(), BTreeSet::new());
    for recipe in game.data.recipes.casting.iter().filter(|recipe| recipe.station == Station::Table) {
        let CastingKind::Item { result } = &recipe.kind else { continue };
        let crate::fluid_ingredient::Source::Id(result) = &result.source else { continue };
        if !result.path().ends_with("_cast") || result.path().contains("sand") { continue; }
        let casts = recipe.cast.as_ref().map(|cast| game.items_of(cast)).unwrap_or_default();
        if casts.is_empty() { unobtainable.insert(result.to_string()); continue; }
        for cast in casts {
            let (state, items, taken) = pour_and_cool(&live, &ItemSlot::new(cast.clone(), 1), &gold);
            assert_eq!(taken, 90, "{}: molten gold on {cast}", recipe.id);
            assert!(state.fluid.is_none());
            // the shape is used up and the new cast is in the table's first slot (switch_slots)
            assert_eq!(items[INPUT].as_ref().map(|stack| &stack.item), Some(result), "{}: {cast} and gold make {result}", recipe.id);
            assert!(items[OUTPUT].is_none(), "{}: the shape is used up", recipe.id);
            made.insert(result.to_string());
        }
    }
    // every gold cast the port declares is made, except the ones whose shape is another mod's item (plate, gear, coin, wire)
    let declared = declared();
    let mut unmade = BTreeSet::new();
    for name in &declared {
        let Some(path) = name.strip_prefix("tconstruct:") else { continue };
        if path.ends_with("_cast") && !path.contains("sand") && path != "arrow_cast" {
            if !made.contains(name) { unmade.insert(name.clone()); }
        }
    }
    // the gold casts shaped on other mods' plates, gears, coins and wires have no recipe without those mods' items (the original's recipes are conditional)
    assert_eq!(unmade.into_iter().collect::<Vec<_>>(), ["tconstruct:coin_cast", "tconstruct:gear_cast", "tconstruct:plate_cast", "tconstruct:wire_cast"]);
    let _ = unobtainable;
    assert!(made.len() >= 24, "{made:?}");
    // molding: a blank sand cast pressed with a part (or a gold cast) becomes that part's sand cast
    let mut pressed = 0;
    for kind in ["sand", "red_sand"] {
        let blank = ItemSlot::new(id(&format!("tconstruct:blank_{kind}_cast")), 1);
        for part in ["pick_head", "tool_handle", "helmet_plating", "repair_kit", "bow_limb", "large_plate"] {
            let held = ItemSlot::new(id(&format!("tconstruct:{part}")), 1);
            let molded = crate::casting::Env::find_molding(&live, &blank, Some(&held)).unwrap_or_else(|| panic!("no molding for {part} into a {kind} cast"));
            assert_eq!(molded.result.item.to_string(), format!("tconstruct:{part}_{kind}_cast"));
            assert!(!molded.pattern_consumed, "the part is kept");
            pressed += 1;
        }
    }
    assert_eq!(pressed, 12);
}

#[test]
fn the_part_builder_presses_sand_and_red_sand_casts_of_every_shape_from_a_blank_cast_or_four_from_sand() {
    use crate::part_builder_recipes::{Context, Inventory, Recipes};
    let game = game();
    let context = crate::conditions::Context { items: Some(game.items.clone()), config: crate::config::parse("").unwrap() };
    let recipes = Recipes::load(&recursive_files("fluids", "fluids"), &context);
    assert!(recipes.problems.is_empty(), "{:#?}", recipes.problems);
    let parts = crate::tool_parts::material_items();
    let in_tag = |item: &Identifier, tag: &Identifier| game.items.get_tag(tag).is_some_and(|members| members.contains(item));
    let ctx = Context { materials: &game.materials, parts: &parts, in_tag: &in_tag };
    let order = crate::part_builder_menu::pattern_order();
    for (pattern_item, family, count) in [("tconstruct:blank_sand_cast", "sand", 1), ("tconstruct:blank_red_sand_cast", "red_sand", 1), ("minecraft:sand", "sand", 4), ("minecraft:red_sand", "red_sand", 4)] {
        let pattern_item = id(pattern_item);
        let inv = Inventory { pattern_item: Some(&pattern_item), stack: None, material: None };
        let buttons = recipes.patterns(&inv, &ctx);
        assert_eq!(buttons.len(), 24, "{pattern_item}: every part, the repair kit and the ingot, nugget, gem and rod shapes (plate, gear, coin and wire shapes need other mods): {buttons:?}");
        assert!(buttons.iter().all(|button| order.contains(button)), "{pattern_item}: every button can be named by the window");
        for button in buttons {
            let recipe = recipes.find(&button, &inv, &ctx).unwrap_or_else(|| panic!("{pattern_item}: no recipe for {button}"));
            let made = recipe.assemble(&inv);
            assert_eq!((made.item.to_string(), made.count), (format!("tconstruct:{}_{family}_cast", button.path()), count), "{pattern_item} {button}");
            assert!(game.items.contains(&made.item), "the result is a declared item");
        }
    }
}

#[test]
fn a_cast_part_melts_back_into_the_fluid_that_cast_it_and_a_cast_melts_into_gold() {
    let game = game();
    let in_fluid = |fluid: &Identifier, tag: &Identifier| game.in_fluid_tag(fluid, tag);
    let in_item = |item: &Identifier, tag: &Identifier| game.items.get_tag(tag).is_some_and(|members| members.contains(item));
    let (mut melted, mut without_recipe) = (0, BTreeSet::new());
    for part in ["pick_head", "tool_binding", "large_plate", "helmet_plating", "repair_kit", "bow_limb"] {
        let item = id(&format!("tconstruct:{part}"));
        let cost = game.data.recipes.material_casting.item_cost(&item);
        assert!(cost > 0, "{part} has a casting cost");
        for fluid_recipe in &game.data.recipes.material_casting.casting {
            let material = &fluid_recipe.output;
            let Some((melting, found_cost)) = game.data.recipes.find_material_melting(&item, material) else { without_recipe.insert(material.to_string()); continue };
            assert_eq!(found_cost, cost);
            let fluid = game.fluid_of(fluid_recipe);
            assert_eq!(melting.result.resolve(&game.data.tags), Some(fluid.clone()), "{part} of {material} melts into the fluid that casts it");
            // the same amount comes out as went in: the casting fluid's amount per ingot times the cost
            let cast_amount = MaterialCasting::fluid_amount(fluid_recipe, cost, &fluid, &in_fluid);
            assert_eq!(melting.output_amount(cost), cast_amount, "{part} of {material}");
            assert_eq!(melting.temperature, fluid_recipe.temperature, "{part} of {material}");
            melted += 1;
        }
    }
    assert!(melted > 100, "{melted}");
    assert!(without_recipe.is_empty(), "casting materials that cannot be melted back: {without_recipe:?}");
    // a gold cast melts into gold
    let melting = game.data.recipes.find_melting(&id("tconstruct:ingot_cast"), &in_item, false).expect("a gold cast melts");
    assert_eq!((melting.output.resolve(&game.data.tags), melting.output.amount), (Some(id("tconstruct:molten_gold")), 90));
}

#[test]
fn parts_cast_from_molten_iron_and_copper_build_a_tool_in_the_station_with_their_sprite_keys() {
    use crate::tool_station::{lookup, Context, Input, Outcome};
    let game = game();
    let live = game.live(Station::Table);
    let cast_part = |cast: &str, fluid: &str| -> ItemSlot {
        let (_, items, _) = pour_and_cool(&live, &ItemSlot::new(id(cast), 1), &id(fluid));
        items[OUTPUT].clone().unwrap_or_else(|| panic!("{fluid} on {cast} made nothing"))
    };
    let head = cast_part("tconstruct:pick_head_cast", "tconstruct:molten_iron");
    let handle = cast_part("tconstruct:tool_handle_cast", "tconstruct:molten_copper");
    let binding = cast_part("tconstruct:tool_binding_cast", "tconstruct:molten_iron");
    let world = world();
    let tools: Vec<(Identifier, &crate::tool_definitions::Data)> = world.tools.iter().filter(|(_, data)| !data.tool_parts().is_empty()).map(|(id, data)| (id.clone(), data)).collect();
    let item_max = |_: &Identifier| 1;
    let ctx = Context { materials: &world.materials, modifiers: &world.registry, item_tags_loaded: true, item_tags: None, tools: &tools, item_max: &item_max };
    let part = |stack: &ItemSlot| Some(Input::Part { item: stack.item.clone(), material: crate::tool_materials::part_material(&stack.custom_data) });
    let Outcome::Crafted(built) = lookup(&ctx, None, &[part(&head), part(&handle), part(&binding), None]).unwrap() else { panic!("the cast parts did not build a tool") };
    assert_eq!(built.result.item.to_string(), "tconstruct:pickaxe");
    let strings = built.result.custom_model_data.as_ref().map(|data| data.strings.clone()).unwrap_or_default();
    assert_eq!(strings, ["iron", "copper", "iron"], "the per-material sprites show");
}

/// The world a faucet sits in, answered the way the host answers neighbour queries: a melter holding fluid behind it,
/// a casting table below it reached through its own fluid rule, so the table's capacity comes from the cast's recipe.
struct FaucetWorld<'a> {
    live: Live<'a>,
    melter: Option<FluidStack>,
    table: State,
    items: Vec<Option<ItemSlot>>,
}

const FAUCET_POS: [i32; 3] = [0, 64, 0];
const MELTER_POS: [i32; 3] = [-1, 64, 0];
const TABLE_POS: [i32; 3] = [0, 63, 0];

impl pluto_lib::NeighbourOps for FaucetWorld<'_> {
    fn item_slots(&mut self, _: pluto_lib::BlockPos, _: pluto_lib::Direction) -> Option<Vec<pluto_lib::NeighbourSlot>> { None }
    fn extract_item(&mut self, _: pluto_lib::BlockPos, _: pluto_lib::Direction, _: usize, _: u32) -> Option<ItemSlot> { None }
    fn insert_item(&mut self, _: pluto_lib::BlockPos, _: pluto_lib::Direction, stack: ItemSlot) -> Option<ItemSlot> { Some(stack) }
    fn fluid_tanks(&mut self, pos: pluto_lib::BlockPos, _: pluto_lib::Direction) -> Option<Vec<pluto_lib::NeighbourTank>> {
        match pos {
            MELTER_POS => Some(vec![pluto_lib::NeighbourTank { fluid: self.melter.clone(), capacity: 4_000 }]),
            TABLE_POS => Some(vec![pluto_lib::NeighbourTank { fluid: self.table.fluid.clone(), capacity: crate::casting::HOST_CAPACITY }]),
            _ => None,
        }
    }
    fn drain_fluid(&mut self, pos: pluto_lib::BlockPos, _: pluto_lib::Direction, amount: u32) -> Option<FluidStack> {
        if pos != MELTER_POS { return None; }
        let held = self.melter.as_mut()?;
        let taken = amount.min(held.amount);
        if taken == 0 { return None; }
        let out = FluidStack { fluid: held.fluid.clone(), amount: taken, tag: held.tag.clone() };
        held.amount -= taken;
        if held.amount == 0 { self.melter = None; }
        Some(out)
    }
    fn fill_fluid(&mut self, pos: pluto_lib::BlockPos, _: pluto_lib::Direction, stack: &FluidStack, execute: bool) -> u32 {
        if pos != TABLE_POS { return 0; }
        if execute { return self.table.fill(&self.live, &mut self.items, stack, false); }
        let (mut table, mut items) = (self.table.clone(), self.items.clone());
        table.fill(&self.live, &mut items, stack, false)
    }
    fn block_state(&mut self, pos: pluto_lib::BlockPos) -> Option<String> {
        (pos == FAUCET_POS).then(|| "tconstruct:seared_faucet[facing=east]".to_owned())
    }
}

#[test]
fn a_faucet_click_pours_the_melters_iron_into_the_table_and_the_cast_sets_the_capacity() {
    use crate::faucet::{Around, State as Faucet};
    let game = game();
    let mut world = FaucetWorld {
        live: game.live(Station::Table),
        melter: Some(FluidStack { fluid: id("tconstruct:molten_iron"), amount: 90, tag: None }),
        table: State::default(),
        items: vec![Some(ItemSlot::new(id("tconstruct:ingot_cast"), 1)), None],
    };
    pluto_lib::with_neighbours(&mut world, || {
        let facing = Around::facing_of(FAUCET_POS);
        assert_eq!(facing, pluto_lib::Direction::East, "the faucet reads its own facing");
        let mut faucet = Faucet::default();
        let mut around = Around::of(FAUCET_POS, facing);
        faucet.activate(&mut around);
        assert!(faucet.is_pouring(), "a click starts the pour");
        for _ in 0..20 {
            if !faucet.is_pouring() { break; }
            faucet.tick(&mut around);
        }
        assert!(!faucet.is_pouring(), "the pour ends when the melter is dry");
    });
    assert_eq!(world.melter, None, "the melter gave up its 90");
    assert_eq!(world.table.fluid, Some(FluidStack { fluid: id("tconstruct:molten_iron"), amount: 90, tag: None }));
    assert_eq!(world.table.capacity, 90, "the capacity is the ingot recipe's amount, chosen by the cast in the table");
    // the table finishes the loop: cool it, and the ingot lands in the output while the cast stays
    let cooling = world.table.cooling.max(0);
    assert!(cooling > 0, "a full table starts cooling");
    for _ in 0..cooling { world.table.tick(&world.live, &mut world.items); }
    assert_eq!(world.items[OUTPUT].as_ref().map(|stack| stack.item.to_string()), Some("minecraft:iron_ingot".to_owned()));
    assert_eq!(world.items[INPUT].as_ref().map(|stack| stack.item.to_string()), Some("tconstruct:ingot_cast".to_owned()));
}

/// The first material a casting fluid casts that has stats of the given type.
fn usable_material(game: &Game, stat: &Identifier) -> crate::tool_materials::MaterialVariant {
    game.data.recipes.material_casting.casting.iter()
        .find_map(|recipe| recipe.output.material_id().filter(|material| game.materials.get_stats(material, stat).is_some()).map(|_| recipe.output.clone()))
        .unwrap_or_else(|| panic!("no castable material has {stat} stats"))
}

/// The first material (in ID order) that has stats of the given type; it need not be castable.
fn any_material(game: &Game, stat: &Identifier) -> crate::tool_materials::MaterialVariant {
    let material = game.materials.materials.keys().find(|material| game.materials.get_stats(material, stat).is_some())
        .unwrap_or_else(|| panic!("no material has {stat} stats"));
    crate::tool_materials::MaterialVariant::parse(&material.to_string()).unwrap()
}

/// A tool built the way the station builds it, for the casting tests that start from a finished tool.
fn tool_stack_of(game: &Game, item: &Identifier, materials: &[crate::tool_materials::MaterialVariant]) -> ItemSlot {
    let definition = &game.tools[item];
    let env = crate::tool_stack::Environment { definition: Some(definition), has_materials: true, materials: &game.materials, modifiers: &game.registry,
        item_tags_loaded: true, item: Some(item), item_tags: Some(&game.items) };
    let mut stack = ItemSlot::new(item.clone(), 1);
    stack.custom_data = crate::tool_build::create_tool(&env, &crate::tool_materials::MaterialList::from_variants(materials.to_vec()), false).unwrap();
    crate::tool_slot::sync_visuals(&mut stack, &env).unwrap();
    stack
}

fn materials_of(stack: &ItemSlot) -> Vec<String> {
    crate::tool_stack::materials(&stack.custom_data, true).as_slice().iter().map(ToString::to_string).collect()
}

#[test]
fn tool_casting_builds_tools_from_the_cast_and_fluid_materials_and_swaps_onto_the_finished_tool() {
    let game = game();
    let in_fluid = |fluid: &Identifier, tag: &Identifier| game.in_fluid_tag(fluid, tag);
    let basin = game.live(Station::Basin);
    // a slime skull: the extra material takes the head's slot, the fluid's material follows; the head is used up
    let skull = game.data.recipes.casting.iter().find(|recipe| recipe.id.to_string() == "tconstruct:fluids/tools/armor/slime_skull/blaze").unwrap();
    let CastingKind::Tool { result, item_cost, .. } = &skull.kind else { panic!("{:?}", skull.kind) };
    let fluid_stat = game.tools[result].material_stat_types().unwrap()[1].0.clone();
    let fluid_recipe = game.data.recipes.material_casting.casting.iter()
        .find(|recipe| recipe.output.material_id().is_some_and(|material| game.materials.get_stats(&material, &fluid_stat).is_some())).unwrap();
    let fluid = game.fluid_of(fluid_recipe);
    let (state, items, taken) = pour_and_cool(&basin, &ItemSlot::new(id("tconstruct:blaze_head"), 1), &fluid);
    assert_eq!(taken as i32, MaterialCasting::fluid_amount(fluid_recipe, *item_cost, &fluid, &in_fluid), "the skull takes the fluid amount times its item cost");
    let helmet = items[OUTPUT].as_ref().expect("a slime helmet");
    assert_eq!(helmet.item.to_string(), "tconstruct:slime_helmet");
    assert_eq!(materials_of(helmet), ["tconstruct:blaze", &fluid_recipe.output.to_string()], "the extra material first, then the fluid's");
    assert!(items[INPUT].is_none() && state.fluid.is_none(), "the head is used up and the basin is reset");
    // a plate shield on the table: the cast part's material first, the fluid's second (first_material)
    let table = game.live(Station::Table);
    let mut core = ItemSlot::new(id("tconstruct:shield_core"), 1);
    crate::tool_materials::set_part_material(&mut core, &crate::tool_materials::MaterialVariant::parse("tconstruct:iron").unwrap());
    let (_, items, taken) = pour_and_cool(&table, &core, &id("tconstruct:molten_copper"));
    assert_eq!(taken, 270, "three ingots of copper");
    let shield = items[OUTPUT].as_ref().expect("a plate shield");
    assert_eq!(shield.item.to_string(), "tconstruct:plate_shield");
    assert_eq!(materials_of(shield), ["tconstruct:iron", "tconstruct:copper"]);
    // slimelytra: the elytra is used up into wings of the fluid's material, and pouring onto the wings swaps that material.
    // The slime materials' trait is the overslime code modifier, which the test world leaves to the tests that need it.
    let mut world = world();
    for modifier in crate::overslime::code_modifiers().unwrap() { world.registry.register_static(modifier).unwrap(); }
    let tools: BTreeMap<Identifier, crate::tool_definitions::Data> = world.tools.into_iter().collect();
    let basin = Live { data: &game.data, item_tags: Some(&game.items), station: Station::Basin, materials: Some(&game.materials), tool_defs: Some(&tools), modifiers: Some(&world.registry) };
    let wings_id = id("tconstruct:slime_wings");
    let wings_stat = tools[&wings_id].material_stat_types().unwrap().last().unwrap().0.clone();
    let first = usable_material(game, &wings_stat);
    let first_recipe = game.data.recipes.material_casting.casting.iter().find(|recipe| recipe.output == first).unwrap();
    let first_fluid = game.fluid_of(first_recipe);
    let (_, items, taken) = pour_and_cool(&basin, &ItemSlot::new(id("minecraft:elytra"), 1), &first_fluid);
    assert!(taken > 0, "the elytra casts into wings");
    let wings = items[OUTPUT].clone().expect("slime wings");
    let built = materials_of(&wings);
    assert_eq!(built.last().map(String::as_str), Some(first.to_string().as_str()), "the fluid's material is the wings' last material");
    let second = game.data.recipes.material_casting.casting.iter().map(|recipe| &recipe.output)
        .find(|output| **output != first && output.material_id().is_some_and(|material| game.materials.get_stats(&material, &wings_stat).is_some())).unwrap().clone();
    let second_recipe = game.data.recipes.material_casting.casting.iter().find(|recipe| recipe.output == second).unwrap();
    let second_fluid = game.fluid_of(second_recipe);
    let (_, items, taken) = pour_and_cool(&basin, &wings, &second_fluid);
    assert_eq!(taken as i32, MaterialCasting::fluid_amount(second_recipe, 8, &second_fluid, &in_fluid), "swapping onto the wings costs the tool recipe's item cost");
    let swapped = items[OUTPUT].as_ref().expect("the wings again");
    let mut expected = built.clone();
    *expected.last_mut().unwrap() = second.to_string();
    assert_eq!(materials_of(swapped), expected, "the wings take the new material in the fluid's slot");
}

#[test]
fn part_swapping_replaces_one_material_of_the_tool_on_the_station() {
    let game = game();
    let in_fluid = |fluid: &Identifier, tag: &Identifier| game.in_fluid_tag(fluid, tag);
    let table = game.live(Station::Table);
    let boots = id("tconstruct:plate_boots");
    let stats = game.tools[&boots].material_stat_types().unwrap();
    let iron = crate::tool_materials::MaterialVariant::parse("tconstruct:iron").unwrap();
    assert!(game.materials.get_stats(&iron.material_id().unwrap(), &stats[0].0).is_some(), "iron serves the boots' first stat");
    let second = any_material(game, &stats[1].0);
    let tool = tool_stack_of(game, &boots, &[iron.clone(), second.clone()]);
    // the same material again is not a swap (and no composite coats iron in itself), so the table takes nothing
    let (_, _, taken) = pour_and_cool(&table, &tool, &id("tconstruct:molten_iron"));
    assert_eq!(taken, 0, "a material cannot be swapped for itself");
    // copper swaps the boots' first material; the second is kept, and the boots come out whole
    let copper = game.data.recipes.material_casting.casting.iter().find(|recipe| recipe.output.to_string() == "tconstruct:copper").unwrap();
    let copper_fluid = game.fluid_of(copper);
    let (_, items, taken) = pour_and_cool(&table, &tool, &copper_fluid);
    assert_eq!(taken as i32, MaterialCasting::fluid_amount(copper, 2, &copper_fluid, &in_fluid), "the swapping recipe's item cost times the fluid amount");
    let swapped = items[OUTPUT].as_ref().expect("the boots");
    assert_eq!((&swapped.item, swapped.count), (&boots, 1));
    assert_eq!(materials_of(swapped), ["tconstruct:copper", &second.to_string()]);
    assert!(items[INPUT].is_none(), "the tool on the station is used up into the result");
}

#[test]
fn tipped_clearing_washes_the_tip_off_and_a_plain_pour_does_not_tip() {
    let game = game();
    let table = game.live(Station::Table);
    let arrow_item = id("tconstruct:arrow");
    let stats = game.tools[&arrow_item].material_stat_types().unwrap();
    let materials: Vec<_> = stats.iter().map(|(stat, _)| any_material(game, stat)).collect();
    let mut arrow = tool_stack_of(game, &arrow_item, &materials);
    // a tipped arrow carries the modifier in its list and the potion in its persistent data
    let tipped = id("tconstruct:tipped");
    let list = crate::modifier_entries::List::default().with_modifier(tipped.clone(), 1).unwrap();
    arrow.custom_data.insert(crate::tool_stack::TAG_MODIFIERS.into(), list.to_nbt());
    let mut persistent = pluto_lib::NbtCompound::new();
    persistent.insert(tipped.to_string(), pluto_lib::Nbt::String("minecraft:strong_swiftness".to_owned()));
    arrow.custom_data.insert(crate::tool_stack::TAG_PERSISTENT.into(), pluto_lib::Nbt::Compound(persistent));
    // tipping cannot match a plain pour: which potion was poured is the fluid's tag, and this stack carries none
    let (_, _, taken) = pour_and_cool(&table, &arrow, &id("tconstruct:potion"));
    assert_eq!(taken, 0, "tipping needs the poured fluid's tag to name a potion");
    // water washes the tip off; the tool, its modifier and its materials are otherwise untouched
    let (_, items, taken) = pour_and_cool(&table, &arrow, &id("minecraft:water"));
    assert_eq!(taken, 50, "the ammo clearing recipe takes 50 of water");
    let washed = items[OUTPUT].as_ref().expect("the washed arrow");
    assert_eq!(washed.item, arrow_item);
    assert_eq!(crate::casting::Live::persistent_string(washed, &tipped), "", "the tip is gone");
    assert_eq!(crate::modifier_entries::List::from_nbt(washed.custom_data.get(crate::tool_stack::TAG_MODIFIERS)).level(&tipped), 1, "the modifier itself stays");
    assert_eq!(materials_of(washed), materials.iter().map(ToString::to_string).collect::<Vec<_>>(), "the materials are untouched");
}

#[test]
fn tipping_names_the_poured_potion_on_the_tool_and_potion_casting_hands_the_result_the_fluids_whole_tag() {
    let game = game();
    let table = game.live(Station::Table);
    let tipped = id("tconstruct:tipped");
    let arrow_item = id("tconstruct:arrow");
    let stats = game.tools[&arrow_item].material_stat_types().unwrap();
    let materials: Vec<_> = stats.iter().map(|(stat, _)| any_material(game, stat)).collect();
    let mut arrow = tool_stack_of(game, &arrow_item, &materials);
    let list = crate::modifier_entries::List::default().with_modifier(tipped.clone(), 1).unwrap();
    arrow.custom_data.insert(crate::tool_stack::TAG_MODIFIERS.into(), list.to_nbt());
    let potion_fluid = |amount: u32, potion: &str| {
        let mut tag = pluto_lib::NbtCompound::new();
        tag.insert("Potion".to_owned(), pluto_lib::Nbt::String(potion.to_owned()));
        FluidStack { fluid: id("tconstruct:potion"), amount, tag: Some(tag) }
    };
    // 50 of a potion fluid whose tag names a potion: the arrow takes that potion as its tip
    let (items, taken) = pour_stack(&table, &arrow, &potion_fluid(50, "minecraft:poison"));
    assert_eq!(taken, 50, "the ammo tipping recipe takes 50 of the potion fluid");
    let tipped_arrow = items[OUTPUT].as_ref().expect("the tipped arrow");
    assert_eq!(tipped_arrow.item, arrow_item);
    assert_eq!(crate::casting::Live::persistent_string(tipped_arrow, &tipped), "minecraft:poison", "the tip is the poured potion");
    assert_eq!(crate::modifier_entries::List::from_nbt(tipped_arrow.custom_data.get(crate::tool_stack::TAG_MODIFIERS)).level(&tipped), 1, "the modifier itself stays");
    // the same potion again does not match, a tag without a potion name does not match, and a tool without the modifier takes nothing
    let (_, taken) = pour_stack(&table, tipped_arrow, &potion_fluid(50, "minecraft:poison"));
    assert_eq!(taken, 0, "a tool already tipped with the poured potion takes nothing");
    let mut no_name = pluto_lib::NbtCompound::new();
    no_name.insert("CustomPotionColor".to_owned(), pluto_lib::Nbt::Int(1));
    let (_, taken) = pour_stack(&table, &arrow, &FluidStack { fluid: id("tconstruct:potion"), amount: 50, tag: Some(no_name) });
    assert_eq!(taken, 0, "a tag with no Potion string names no tip");
    let plain = tool_stack_of(game, &arrow_item, &materials);
    let (_, taken) = pour_stack(&table, &plain, &potion_fluid(50, "minecraft:poison"));
    assert_eq!(taken, 0, "tipping needs the modifier on the tool");
    // potion casting: a glass bottle and 250 of the fluid make the result item, carrying the fluid's whole tag
    let bottle = ItemSlot::new(id("minecraft:glass_bottle"), 1);
    let mut whole = pluto_lib::NbtCompound::new();
    whole.insert("Potion".to_owned(), pluto_lib::Nbt::String("minecraft:healing".to_owned()));
    whole.insert("CustomPotionColor".to_owned(), pluto_lib::Nbt::Int(12345));
    let (items, taken) = pour_stack(&table, &bottle, &FluidStack { fluid: id("tconstruct:potion"), amount: 250, tag: Some(whole.clone()) });
    assert_eq!(taken, 250, "the bottle recipe takes 250 of the potion fluid");
    let made = items[OUTPUT].as_ref().expect("the potion");
    assert_eq!(made.item, id("minecraft:potion"));
    assert_eq!(made.custom_data, whole, "the result carries the fluid's whole tag, as PotionCastingRecipe.assemble sets it");
    assert!(items[INPUT].is_none(), "the bottle is used up");
    // and with no tag at all the same recipe still matches (matches never inspects the tag) and makes a plain potion
    let (items, taken) = pour_stack(&table, &bottle, &FluidStack { fluid: id("tconstruct:potion"), amount: 250, tag: None });
    assert_eq!(taken, 250);
    assert!(items[OUTPUT].as_ref().expect("the plain potion").custom_data.is_empty());
}

#[test]
fn slimesuit_pieces_build_by_basin_casting() {
    // T1: the slimesuit pieces whose cast is a tool part (the helmet and the wings are covered above): the
    // part's material takes the first slot, the poured slime material the second. The chestplate's recipe
    // named the original's pre-rename item id, which no port item has; it now names the port's item.
    let game = game();
    let in_fluid = |fluid: &Identifier, tag: &Identifier| game.in_fluid_tag(fluid, tag);
    let mut world = world();
    for modifier in crate::overslime::code_modifiers().unwrap() { world.registry.register_static(modifier).unwrap(); }
    let tools: BTreeMap<Identifier, crate::tool_definitions::Data> = world.tools.into_iter().collect();
    let basin = Live { data: &game.data, item_tags: Some(&game.items), station: Station::Basin, materials: Some(&game.materials), tool_defs: Some(&tools), modifiers: Some(&world.registry) };
    let earthslime = game.data.recipes.material_casting.casting.iter().find(|recipe| recipe.output.to_string() == "tconstruct:earthslime").unwrap();
    let fluid = game.fluid_of(earthslime);
    for (recipe_path, part, part_material, result, durability) in [
        ("tconstruct:fluids/tools/building/slimecage", "tconstruct:ribcage", "tconstruct:bone", "tconstruct:slime_chestplate", 80),
        ("tconstruct:fluids/tools/building/slimeshell", "tconstruct:shell", "tconstruct:dragon_scale", "tconstruct:slime_leggings", 75),
        ("tconstruct:fluids/tools/building/slime_boots", "tconstruct:laces", "tconstruct:cheese", "tconstruct:slime_boots", 65),
    ] {
        let recipe = game.data.recipes.casting.iter().find(|recipe| recipe.id.to_string() == recipe_path).unwrap();
        let CastingKind::Tool { item_cost, .. } = &recipe.kind else { panic!("{:?}", recipe.kind) };
        let mut cast = ItemSlot::new(id(part), 1);
        crate::tool_materials::set_part_material(&mut cast, &crate::tool_materials::MaterialVariant::parse(part_material).unwrap());
        let (_, items, taken) = pour_and_cool(&basin, &cast, &fluid);
        assert_eq!(taken as i32, MaterialCasting::fluid_amount(earthslime, *item_cost, &fluid, &in_fluid), "{result} takes its item cost of the fluid");
        let built = items[OUTPUT].as_ref().expect("a built slimesuit piece");
        assert_eq!(built.item.to_string(), result);
        assert_eq!(materials_of(built), [part_material, "tconstruct:earthslime"], "{result} materials");
        assert_eq!(crate::tool_stack::durability(&built.custom_data), durability, "{result} durability");
    }
}
