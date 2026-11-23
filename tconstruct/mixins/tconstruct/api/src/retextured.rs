//! Retexturing, from Mantle's RetexturedHelper, RetexturedRecipe, RetexturedBlockEntity and
//! RetexturedLootFunction (the Mantle side of SlimeKnights' libraries, MIT; see LICENSE): a
//! `mantle:crafting_shaped_retextured` recipe makes its result from whichever material filled
//! the recipe's texture slot, and the result stack names that material in its data under
//! `texture`; a block placed from the stack keeps the texture in its block entity, and breaking
//! it drops the item with the texture restored by the `mantle:fill_retextured_block` loot
//! function, which reads the block entity the block had.
//!
//! The data path is complete; the picture is not. Neither the host's block mesher nor its item
//! models ever read a block entity's data (or a stack's `texture`) to choose a texture: the
//! original chooses through Mantle's model data (`RetexturedHelper.BLOCK_PROPERTY` feeding
//! `RetexturedModel`, whose model JSON names the `retextured` texture slots to swap), and
//! PlutoLib has no model-data surface to offer. Until one exists, a retextured block renders
//! with its model's fixed textures while its texture record travels exactly as the original's
//! does: recipe to stack, stack to block entity on placement, block entity to the drop.

use std::collections::BTreeMap;
use std::sync::{OnceLock, RwLock};

use pluto_lib::recipe_api_v1::{AssemblyDecision, AssemblyRegistration, AssemblyRequest, RecipeFacts, RecipeForm, StackView};
use pluto_lib::{BlockEntityPlace, Identifier, LootFunctionContext, LootTableEvents, Nbt, NbtCompound, ResourceManagerHelper, ResourceType, ServerLifecycleEvents};
use serde_json::Value;

/// The game's recipe folder: the converted retextured recipes ship beside every other plain
/// recipe, told apart by their `texture` field.
const FOLDER: &str = "recipe";

/// The key the texture item's id is kept under, on a stack's data and in a block entity's data
/// alike (Mantle: `RetexturedHelper.TAG_TEXTURE`).
pub(crate) const TEXTURE_KEY: &str = "texture";

/// The loot function kind that restores the texture on a broken retextured block's drop.
pub(crate) const FILL_FUNCTION: &str = "mantle:fill_retextured_block";

/* The texture record */

/// The texture a stack's or block entity's data names, if it names one.
pub(crate) fn texture_of(data: &NbtCompound) -> Option<String> {
    match data.get(TEXTURE_KEY) {
        Some(Nbt::String(item)) => Some(item.clone()),
        _ => None,
    }
}

/// `RetexturedBlock.updateTextureBlock` by way of PlutoLib's placement hook: a block placed
/// from a stack that names a texture keeps the texture in its own data. Registered as the
/// `on_place` of every retexturable block's block entity type.
pub(crate) fn keep_texture(placed: &mut BlockEntityPlace<'_>) {
    if let Some(texture) = texture_of(&placed.stack.custom_data) {
        placed.data.insert(TEXTURE_KEY.to_owned(), Nbt::String(texture));
    }
}

/* The loot function */

/// `mantle:fill_retextured_block` (Mantle's RetexturedLootFunction): the drop of a retextured
/// block takes the texture its block entity kept.
fn fill_retextured(context: &LootFunctionContext) {
    if context.kind != FILL_FUNCTION {
        return;
    }
    let Some(data) = &context.block_entity else { return };
    if let Some(texture) = texture_of(data) {
        context.stack.borrow_mut().custom_data.insert(TEXTURE_KEY.to_owned(), Nbt::String(texture));
    }
}

/* The retextured recipe kind */

/// One key entry of a converted retextured recipe: the plain forms the conversion leaves (an
/// item id, a `#tag`, or the list a tag expanded to).
#[derive(Clone, Debug)]
enum KeyIngredient {
    Item(String),
    Tag(Identifier),
    Any(Vec<String>),
}

fn decode_key_ingredient(value: &Value) -> Result<KeyIngredient, String> {
    match value {
        Value::String(text) => match text.strip_prefix('#') {
            Some(tag) => Identifier::parse(tag).map(KeyIngredient::Tag).map_err(|_| format!("invalid tag id {tag}")),
            None => Ok(KeyIngredient::Item(text.clone())),
        },
        Value::Array(members) => Ok(KeyIngredient::Any(
            members
                .iter()
                .map(|member| member.as_str().map(str::to_owned).ok_or("key list members must be item ids"))
                .collect::<Result<_, _>>()?,
        )),
        Value::Object(fields) => {
            if let Some(item) = fields.get("item").and_then(Value::as_str) {
                return Ok(KeyIngredient::Item(item.to_owned()));
            }
            if let Some(tag) = fields.get("tag").and_then(Value::as_str) {
                return Identifier::parse(tag).map(KeyIngredient::Tag).map_err(|_| format!("invalid tag id {tag}"));
            }
            Err("retextured key holds an unconverted ingredient".into())
        }
        _ => Err("retextured key ingredient must be a string, list or object".into()),
    }
}

fn key_test(ingredient: &KeyIngredient, stack: &StackView<'_>) -> bool {
    match ingredient {
        KeyIngredient::Item(item) => stack.id == item,
        KeyIngredient::Tag(tag) => Identifier::parse(stack.id).is_ok_and(|item| {
            match crate::common::item_snapshot() {
                // Tag membership comes from the captured snapshot; with none captured it is
                // undecidable here, and a request only reaches this provider for a grid the
                // game's own matcher already accepted against the shipped recipe (whose tag
                // the game resolved), so defer to that match instead of refusing the cell —
                // refusing it would also swallow the match_all disagreement verdict, which
                // is a property of the texture stacks themselves, not of the tag.
                None => true,
                Some(items) => items.get_tag(tag).is_some_and(|members| members.contains(&item)),
            }
        }),
        KeyIngredient::Any(items) => items.iter().any(|item| stack.id == item),
    }
}

/// A `mantle:crafting_shaped_retextured` recipe in its converted form: a plain shaped recipe
/// whose `texture` names the key symbol whose stack textures the result, and whose `match_all`
/// says every texture-slot stack must be the same item.
#[derive(Clone, Debug)]
pub(crate) struct RetexturedRecipe {
    pub pattern: Vec<String>,
    pub key: BTreeMap<char, KeyIngredient>,
    pub texture: char,
    pub match_all: bool,
    pub result: Identifier,
}

pub(crate) fn decode_recipe(value: &Value) -> Result<RetexturedRecipe, String> {
    let fields = value.as_object().ok_or("recipe must be an object")?;
    let texture = fields
        .get("texture")
        .and_then(Value::as_str)
        .and_then(|text| text.chars().next())
        .ok_or("recipe needs a texture symbol")?;
    let pattern: Vec<String> = fields
        .get("pattern")
        .and_then(Value::as_array)
        .ok_or("recipe needs a pattern")?
        .iter()
        .map(|row| row.as_str().map(str::to_owned).ok_or("pattern rows must be strings"))
        .collect::<Result<_, _>>()?;
    let key = fields
        .get("key")
        .and_then(Value::as_object)
        .ok_or("recipe needs a key")?
        .iter()
        .map(|(symbol, ingredient)| {
            let symbol = symbol.chars().next().ok_or("key symbol must not be empty")?;
            Ok((symbol, decode_key_ingredient(ingredient)?))
        })
        .collect::<Result<BTreeMap<_, _>, String>>()?;
    if !key.contains_key(&texture) {
        return Err("the texture symbol is not in the key".into());
    }
    let match_all = fields.get("match_all").and_then(Value::as_bool).unwrap_or(false);
    let result = fields
        .get("result")
        .and_then(|result| result.get("id"))
        .and_then(Value::as_str)
        .ok_or("recipe needs a result id")?;
    let result = Identifier::parse(result).map_err(|_| format!("invalid result id {result}"))?;
    Ok(RetexturedRecipe { pattern, key, texture, match_all, result })
}

/// This consumer's retextured recipes by result item, from the current reload.
fn registry() -> &'static RwLock<BTreeMap<Identifier, Vec<RetexturedRecipe>>> {
    static REGISTRY: OnceLock<RwLock<BTreeMap<Identifier, Vec<RetexturedRecipe>>>> = OnceLock::new();
    REGISTRY.get_or_init(|| RwLock::new(BTreeMap::new()))
}

pub(crate) fn load_recipes(files: &[(Identifier, Value)]) -> BTreeMap<Identifier, Vec<RetexturedRecipe>> {
    let mut recipes: BTreeMap<Identifier, Vec<RetexturedRecipe>> = BTreeMap::new();
    for (_, value) in files {
        // Only the retextured conversions carry a string `texture` field.
        if !matches!(value.get("texture"), Some(Value::String(_))) {
            continue;
        }
        match decode_recipe(value) {
            Ok(recipe) => {
                recipes.entry(recipe.result.clone()).or_default().push(recipe);
            }
            Err(error) => eprintln!("Tinkers: retextured recipe: {error}"),
        }
    }
    recipes
}

/// The grid against the recipe, in the pattern's orientation or mirrored (the game's shaped
/// match accepts both): the texture the result takes when the grid matches, `Err` when it
/// matches but the texture-slot stacks disagree under `match_all`, and `None` when the grid is
/// not this recipe's.
fn grid_texture(recipe: &RetexturedRecipe, request: &AssemblyRequest<'_>) -> Option<Result<String, ()>> {
    let (width, height) = (request.grid.width, request.grid.height);
    if recipe.pattern.len() != height
        || recipe.pattern.iter().any(|row| row.chars().count() != width)
        || request.grid.cells.len() != width * height
    {
        return None;
    }
    for mirrored in [false, true] {
        let mut texture: Option<String> = None;
        let mut disagrees = false;
        let mut matches = true;
        'cells: for (y, row) in recipe.pattern.iter().enumerate() {
            for (x, symbol) in row.chars().enumerate() {
                let cell = request.grid.cells[y * width + if mirrored { width - 1 - x } else { x }];
                let held = match (symbol, cell) {
                    (' ', None) => true,
                    (' ', Some(_)) => false,
                    (_, None) => false,
                    (_, Some(stack)) => {
                        let held = recipe.key.get(&symbol).is_some_and(|ingredient| key_test(ingredient, &stack));
                        if held && symbol == recipe.texture {
                            match &texture {
                                None => texture = Some(stack.id.to_owned()),
                                Some(first) if first != stack.id => disagrees = true,
                                _ => {}
                            }
                        }
                        held
                    }
                };
                if !held {
                    matches = false;
                    break 'cells;
                }
            }
        }
        if matches {
            if disagrees && recipe.match_all {
                return Some(Err(()));
            }
            return texture.map(Ok);
        }
    }
    None
}

fn assembly_interest(request: &RecipeFacts<'_>) -> bool {
    matches!(request.form, RecipeForm::Shaped { .. })
        && request
            .result_item
            .is_some_and(|item| Identifier::parse(item).is_ok_and(|id| registry().read().is_ok_and(|recipes| recipes.contains_key(&id))))
}

fn assembly_callback(request: &AssemblyRequest<'_>) -> AssemblyDecision {
    let RecipeForm::Shaped { .. } = request.recipe.form else { return AssemblyDecision::Pass };
    let Some(result_item) = request.recipe.result_item else { return AssemblyDecision::Pass };
    let Ok(result) = Identifier::parse(result_item) else { return AssemblyDecision::Pass };
    let candidates = registry()
        .read()
        .ok()
        .and_then(|recipes| recipes.get(&result).cloned());
    let Some(candidates) = candidates else { return AssemblyDecision::Pass };
    for recipe in &candidates {
        match grid_texture(recipe, request) {
            // RetexturedRecipe.assemble: the result carries the texture it was made from.
            Some(Ok(texture)) => {
                let mut custom_data = NbtCompound::new();
                custom_data.insert(TEXTURE_KEY.to_owned(), Nbt::String(texture));
                return AssemblyDecision::Assemble { custom_data };
            }
            // The grid is this recipe's but its texture stacks disagree under match_all:
            // RetexturedRecipe.matches refuses it, so the native result must not stand.
            Some(Err(())) => return AssemblyDecision::Deny,
            None => {}
        }
    }
    AssemblyDecision::Pass
}

/* The Deco tab's variants (creative.rs builds the tab from these) */

/// The metal blocks an anvil takes its texture from (the anvil recipes' `m` key).
pub(crate) const ANVIL_TEXTURES: &[&str] = &[
    "tconstruct:slimesteel_block", "tconstruct:amethyst_bronze_block", "tconstruct:rose_gold_block",
    "tconstruct:pig_iron_block", "tconstruct:cinderslime_block", "tconstruct:queens_slime_block",
    "tconstruct:manyullyn_block", "tconstruct:hepatizon_block", "tconstruct:knightmetal_block",
    "tconstruct:knightslime_block", "minecraft:netherite_block",
];

/// `minecraft:logs`, the crafting station's texture tag.
pub(crate) const LOGS: &[&str] = &[
    "minecraft:oak_log", "minecraft:oak_wood", "minecraft:stripped_oak_log", "minecraft:stripped_oak_wood",
    "minecraft:spruce_log", "minecraft:spruce_wood", "minecraft:stripped_spruce_log", "minecraft:stripped_spruce_wood",
    "minecraft:birch_log", "minecraft:birch_wood", "minecraft:stripped_birch_log", "minecraft:stripped_birch_wood",
    "minecraft:jungle_log", "minecraft:jungle_wood", "minecraft:stripped_jungle_log", "minecraft:stripped_jungle_wood",
    "minecraft:acacia_log", "minecraft:acacia_wood", "minecraft:stripped_acacia_log", "minecraft:stripped_acacia_wood",
    "minecraft:dark_oak_log", "minecraft:dark_oak_wood", "minecraft:stripped_dark_oak_log", "minecraft:stripped_dark_oak_wood",
    "minecraft:mangrove_log", "minecraft:mangrove_wood", "minecraft:stripped_mangrove_log", "minecraft:stripped_mangrove_wood",
    "minecraft:cherry_log", "minecraft:cherry_wood", "minecraft:stripped_cherry_log", "minecraft:stripped_cherry_wood",
    "minecraft:crimson_stem", "minecraft:crimson_hyphae", "minecraft:warped_stem", "minecraft:warped_hyphae",
];

/// `minecraft:planks`, the part builder's and tinker station's texture tag.
pub(crate) const PLANKS: &[&str] = &[
    "minecraft:oak_planks", "minecraft:spruce_planks", "minecraft:birch_planks", "minecraft:jungle_planks",
    "minecraft:acacia_planks", "minecraft:dark_oak_planks", "minecraft:mangrove_planks", "minecraft:cherry_planks",
    "minecraft:bamboo_planks", "minecraft:crimson_planks", "minecraft:warped_planks",
];

/// `tconstruct:workstation_rock` as this port ships it, without the other mods' optional
/// entries: the modifier worktable's texture tag.
pub(crate) const WORKSTATION_ROCK: &[&str] = &[
    "minecraft:cobblestone", "minecraft:mossy_cobblestone", "minecraft:stone", "minecraft:blackstone",
    "minecraft:granite", "minecraft:diorite", "minecraft:andesite", "minecraft:cobbled_deepslate",
    "minecraft:deepslate", "minecraft:basalt", "minecraft:tuff", "minecraft:dripstone_block", "minecraft:calcite",
];

/// `tconstruct:seared_blocks` as this port ships it (its `seared_bricks` subtag expanded).
pub(crate) const SEARED_BLOCKS: &[&str] = &[
    "tconstruct:seared_stone", "tconstruct:seared_cracked_bricks", "tconstruct:seared_cobble",
    "tconstruct:seared_paver", "tconstruct:seared_bricks", "tconstruct:seared_fancy_bricks",
    "tconstruct:seared_triangle_bricks",
];

/// `tconstruct:scorched_blocks` as this port ships it.
pub(crate) const SCORCHED_BLOCKS: &[&str] = &[
    "tconstruct:scorched_stone", "tconstruct:polished_scorched_stone", "tconstruct:scorched_bricks",
    "tconstruct:scorched_road", "tconstruct:chiseled_scorched_bricks",
];

/// The Deco tab's retextured variants in the original's order (TinkerTables.addTableVariants,
/// then TinkerSmeltery.addTableVariants): each block once per item of the tag its texture comes
/// from, the anvils first with their metal blocks. The tab's stacks carry the texture record,
/// so a variant taken from the tab places with its texture kept.
pub(crate) const DECO_VARIANTS: &[(&str, &[&str])] = &[
    ("tinkers_anvil", ANVIL_TEXTURES),
    ("scorched_anvil", ANVIL_TEXTURES),
    ("crafting_station", LOGS),
    ("part_builder", PLANKS),
    ("tinker_station", PLANKS),
    ("modifier_worktable", WORKSTATION_ROCK),
    ("smeltery_controller", SEARED_BLOCKS),
    ("seared_drain", SEARED_BLOCKS),
    ("seared_duct", SEARED_BLOCKS),
    ("seared_chute", SEARED_BLOCKS),
    ("foundry_controller", SCORCHED_BLOCKS),
    ("scorched_drain", SCORCHED_BLOCKS),
    ("scorched_duct", SCORCHED_BLOCKS),
    ("scorched_chute", SCORCHED_BLOCKS),
];

/// Installs the assembly provider, the loot function and the recipe reload listener.
pub(crate) fn register() {
    let _ = pluto_lib::recipe_api_v1::register_assembly(AssemblyRegistration {
        id: "tconstruct:retextured",
        interest: assembly_interest,
        callback: assembly_callback,
    });
    LootTableEvents::FUNCTION.register(fill_retextured);
    let output = registry();
    ResourceManagerHelper::get(ResourceType::ServerData)
        .register_json_reload_listener(
            Identifier::new("tconstruct", "retextured").unwrap(),
            FOLDER,
            move |files| {
                let mut state = output.write().map_err(|_| "retextured recipes poisoned")?;
                *state = load_recipes(files);
                Ok(())
            },
        )
        .expect("Tinkers retextured listener registration failed");
    ServerLifecycleEvents::SERVER_STOPPED.register(move |_| {
        *registry().write().expect("retextured recipes poisoned") = BTreeMap::new();
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::recipe_api_v1::GridFacts;
    use serde_json::json;

    fn view(id: &str) -> StackView<'_> {
        StackView { id, count: 1, damage: 0, custom_data: None }
    }

    #[test]
    fn the_recipe_decodes_and_the_grid_gives_its_texture() {
        let recipe = decode_recipe(&json!({
            "type": "minecraft:crafting_shaped",
            "key": { "#": ["tconstruct:seared_stone", "tconstruct:seared_bricks"], "C": ["minecraft:copper_ingot"] },
            "pattern": ["C#C"], "texture": "#", "match_all": false,
            "result": { "id": "tconstruct:seared_drain" },
        }))
        .unwrap();
        assert_eq!(recipe.texture, '#');
        let copper = view("minecraft:copper_ingot");
        let brick = view("tconstruct:seared_bricks");
        let cells = [Some(copper), Some(brick), Some(copper)];
        let facts = RecipeFacts { form: RecipeForm::Shaped { width: 3, height: 1 }, result_item: Some("tconstruct:seared_drain"), result_count: 1 };
        let request = AssemblyRequest { recipe: &facts, grid: GridFacts { width: 3, height: 1, left: 0, top: 0, cells: &cells }, result: None };
        assert_eq!(grid_texture(&recipe, &request), Some(Ok("tconstruct:seared_bricks".to_owned())));
        // a stack outside the key is not this recipe's grid
        let stone = view("minecraft:stone");
        let other = [Some(copper), Some(stone), Some(copper)];
        let request = AssemblyRequest { recipe: &facts, grid: GridFacts { width: 3, height: 1, left: 0, top: 0, cells: &other }, result: None };
        assert_eq!(grid_texture(&recipe, &request), None);
    }

    #[test]
    fn match_all_refuses_mixed_texture_stacks() {
        let recipe = decode_recipe(&json!({
            "type": "minecraft:crafting_shaped",
            "key": { "p": "tconstruct:pattern", "w": "#minecraft:planks" },
            "pattern": ["pp", "ww"], "texture": "w", "match_all": true,
            "result": { "id": "tconstruct:part_builder" },
        }))
        .unwrap();
        let pattern = view("tconstruct:pattern");
        let oak = view("minecraft:oak_planks");
        let birch = view("minecraft:birch_planks");
        let facts = RecipeFacts { form: RecipeForm::Shaped { width: 2, height: 2 }, result_item: Some("tconstruct:part_builder"), result_count: 1 };
        let mixed = [Some(pattern), Some(pattern), Some(oak), Some(birch)];
        let request = AssemblyRequest { recipe: &facts, grid: GridFacts { width: 2, height: 2, left: 0, top: 0, cells: &mixed }, result: None };
        assert_eq!(grid_texture(&recipe, &request), Some(Err(())), "two woods under match_all do not match");
        let same = [Some(pattern), Some(pattern), Some(oak), Some(oak)];
        let request = AssemblyRequest { recipe: &facts, grid: GridFacts { width: 2, height: 2, left: 0, top: 0, cells: &same }, result: None };
        assert_eq!(grid_texture(&recipe, &request), Some(Ok("minecraft:oak_planks".to_owned())));
    }

    #[test]
    fn the_loot_function_restores_the_texture_the_block_entity_kept() {
        use pluto_lib::ItemSlot;
        use std::cell::RefCell;
        let mut kept = NbtCompound::new();
        kept.insert(TEXTURE_KEY.to_owned(), Nbt::String("tconstruct:seared_bricks".to_owned()));
        let context = LootFunctionContext {
            kind: FILL_FUNCTION.to_owned(),
            data: json!({}),
            origin: Some([0.5; 3]),
            dimension: Some("minecraft:overworld".to_owned()),
            block_entity: Some(kept),
            stack: RefCell::new(ItemSlot::new(Identifier::parse("tconstruct:seared_drain").unwrap(), 1)),
        };
        fill_retextured(&context);
        assert_eq!(texture_of(&context.stack.borrow().custom_data), Some("tconstruct:seared_bricks".to_owned()));
    }
}
