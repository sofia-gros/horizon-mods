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

pub(crate) struct Game { pub data: Data, pub items: RegistryLookup, pub materials: Definitions }

pub(crate) fn game() -> &'static Game {
    static GAME: std::sync::OnceLock<Game> = std::sync::OnceLock::new();
    GAME.get_or_init(|| {
        let items = item_registry();
        let context = crate::conditions::Context { items: Some(items.clone()), config: crate::config::parse("").unwrap() };
        let recipes = crate::fluid_recipes::Recipes::load(&recursive_files("fluids", "fluids"), &context);
        let tags = crate::fluid_ingredient::FluidTags::load(&fluid_tag_files());
        let data = Data { recipes, tags, recipes_loaded: true, tags_loaded: true };
        Game { data, items, materials: world().materials }
    })
}

impl Game {
    fn live(&self, station: Station) -> Live<'_> { Live { data: &self.data, item_tags: Some(&self.items), station, materials: Some(&self.materials) } }
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
    let taken = state.fill(live, &mut items, &FluidStack { fluid: fluid.clone(), amount: 100_000 }, false);
    let cooling = state.cooling.max(0);
    for _ in 0..cooling { state.tick(live, &mut items); }
    (state, items, taken)
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
            // the fake ingot and fake storage block parts show a material as an ingot or block; they are not declared yet
            if !declared.contains(&result.to_string()) { skipped.insert(result.to_string()); continue; }
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
    assert_eq!(skipped.into_iter().collect::<Vec<_>>(), ["tconstruct:fake_ingot", "tconstruct:fake_storage_block"], "only the undeclared fake parts are skipped");
    // every tool part and the repair kit has a cast that makes it, of every kind: gold (multi use), sand and red sand
    for (part, _) in crate::creative_parts::PARTS {
        assert!(parts_seen.contains(&format!("tconstruct:{part}")) || ["bowstring", "arrow_head", "arrow_shaft", "fletching", "shield_core", "ribcage", "shell", "laces"].contains(part),
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
    let mut coated = 0;
    for station in [Station::Table, Station::Basin] {
        let live = game.live(station);
        for recipe in game.data.recipes.casting.iter().filter(|recipe| recipe.station == station) {
            let CastingKind::Composite { result, item_cost, stat_conflict } = &recipe.kind else { continue };
            if !declared.contains(&result.to_string()) { continue; }
            let part = crate::tool_parts::material_items().get(result).cloned().unwrap_or_else(|| panic!("{result} is not a material item"));
            for fluid_recipe in &game.data.recipes.material_casting.composite {
                let Some(input) = &fluid_recipe.input else { continue };
                let fluid = game.fluid_of(fluid_recipe);
                let mut cast = ItemSlot::new(result.clone(), 1);
                crate::tool_materials::set_part_material(&mut cast, input);
                let (_, items, taken) = pour_and_cool(&live, &cast, &fluid);
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
