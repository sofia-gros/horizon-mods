//! Armor trim recipe (ArmorTrimRecipe): a trimmable tool, a trim template and
//! a trim material on the tinker station take the trim, the original's
//! `tools/recipe/ArmorTrimRecipe.java` + `TrimModule` (SlimeKnights, MIT; see
//! LICENSE). The material and pattern the inputs resolve to are stored in the
//! tool's persistent data as id strings under `tconstruct:trim_material` and
//! `tconstruct:trim_pattern` — exactly the keys the `tconstruct:trim` display
//! module (TrimModule) reads for the name and forgets on removal (see
//! modifier_modules' `on_removed`) — and the tool gains the `tconstruct:trim`
//! modifier if it lacks it. A tool in the `trim/no_pattern` tag needs no
//! pattern and stores only the material.
//!
//! The original resolves inputs through the vanilla trim registries
//! (`TrimMaterials.getFromIngredient`, `TrimPatterns.getFromTemplate`), whose
//! entries name their item in the data (`ingredient`, `template_item`). The
//! host's own trim data carries neither field — it resolves templates and
//! materials by its own conventions — so this port resolves the same way the
//! host does: the shipped trim material data (which does carry `ingredient`)
//! is loaded from the `trim_material` folder, patterns register from the
//! `trim_pattern` folder, and the vanilla entries are the built-in tables
//! below, mirroring dyeing's built-in dye table. A template item is one named
//! `<pattern>_armor_trim_smithing_template`, as every shipped template is;
//! the original gates the inputs on the vanilla `trim_templates` /
//! `trim_materials` tags, of which the host ships only the materials tag.
use crate::{
    materials::Conditions,
    modifier_entries::List,
    tool_slot,
    tool_stack::{rebuild_stats, try_validate, Environment, TAG_MODIFIERS, TAG_PERSISTENT, TAG_UPGRADES},
};
use pluto_lib::{Identifier, ItemSlot, Nbt, NbtCompound, ResourceManagerHelper, ResourceType, ServerLifecycleEvents};
use serde_json::Value;
use std::sync::{Arc, RwLock};

/// `TinkerTags.Items.TRIM` (the tools a trim applies to).
const TOOL_TAG: &str = "tconstruct:modifiable/armor/trim";
/// `TinkerTags.Items.TRIM_NO_PATTERN`: tools that take a material only.
const NO_PATTERN_TAG: &str = "tconstruct:modifiable/armor/trim/no_pattern";
/// The vanilla item tag of trim material items (the mod extends it with its
/// own ingots and crystals in its content).
const MATERIALS_TAG: &str = "minecraft:trim_materials";
/// `TinkerModifiers.trim`.
const TRIM: &str = "tconstruct:trim";
/// TrimModule's material key: the modifier id with the `_material` suffix.
const MATERIAL_KEY: &str = "tconstruct:trim_material";
/// TrimModule's pattern key: the modifier id with the `_pattern` suffix.
const PATTERN_KEY: &str = "tconstruct:trim_pattern";
/// The suffix every trim template item's name carries.
const TEMPLATE_SUFFIX: &str = "_armor_trim_smithing_template";

/// The vanilla trim materials by their ingredient item (the 1.21 set the host
/// ships), as material ids in the `minecraft` namespace.
fn vanilla_material(item: &Identifier) -> Option<Identifier> {
    if item.namespace() != "minecraft" {
        return None;
    }
    let material = match item.path() {
        "quartz" => "quartz",
        "iron_ingot" => "iron",
        "netherite_ingot" => "netherite",
        "redstone" => "redstone",
        "copper_ingot" => "copper",
        "gold_ingot" => "gold",
        "emerald" => "emerald",
        "diamond" => "diamond",
        "lapis_lazuli" => "lapis",
        "amethyst_shard" => "amethyst",
        "resin_brick" => "resin",
        _ => return None,
    };
    Identifier::new("minecraft", material).ok()
}

/// The vanilla trim patterns (the eighteen the host ships).
fn vanilla_pattern(name: &str) -> bool {
    matches!(
        name,
        "sentry" | "vex" | "wild" | "coast" | "dune" | "wayfinder" | "raiser" | "shaper" | "host" | "ward"
            | "silence" | "tide" | "snout" | "rib" | "eye" | "spire" | "flow" | "bolt"
    )
}

/// Adds the modifier at level 1 when the tool lacks it in either list, the
/// dyeing family's gain step. Returns whether it was added.
fn gain(result: &mut ItemSlot, modifier: &Identifier) -> Result<bool, String> {
    let level = List::from_nbt(result.custom_data.get(TAG_UPGRADES)).level(modifier)
        + List::from_nbt(result.custom_data.get(TAG_MODIFIERS)).level(modifier);
    if level > 0 {
        return Ok(false);
    }
    let upgrades = List::from_nbt(result.custom_data.get(TAG_UPGRADES)).with_modifier(modifier.clone(), 1)?;
    result.custom_data.insert(TAG_UPGRADES.into(), upgrades.to_nbt());
    Ok(true)
}

/// The rebuild a fresh modifier gain runs, as every station recipe runs it.
fn finish(result: &mut ItemSlot, env: &Environment) -> Result<(), String> {
    if !rebuild_stats(&mut result.custom_data, env)? {
        return Err("the tool could not be rebuilt".into());
    }
    if let Some(error) = try_validate(&result.custom_data, env) {
        return Err(error);
    }
    tool_slot::sync_visuals(result, env)?;
    Ok(())
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Recipes {
    /// A `tconstruct:armor_trim_modifier` file was loaded.
    pub present: bool,
    pub problems: Vec<String>,
    pub loaded: bool,
    /// Trim materials from data: ingredient item -> material id, from the
    /// `trim_material` folder's `ingredient` fields.
    pub materials: Vec<(Identifier, Identifier)>,
    pub material_problems: Vec<String>,
    /// Trim pattern ids registered from the `trim_pattern` folder.
    pub patterns: Vec<Identifier>,
}

impl Recipes {
    pub fn load<C: Conditions>(files: &[(Identifier, Value)], conditions: &C) -> Self {
        let mut result = Self { loaded: true, ..Self::default() };
        let mut sorted: Vec<&(Identifier, Value)> = files.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for (id, value) in sorted {
            let outcome = (|| -> Result<bool, String> {
                let fields = value.as_object().ok_or("recipe must be an object")?;
                if fields.get("type").and_then(Value::as_str) != Some("tconstruct:armor_trim_modifier") {
                    return Ok(false);
                }
                if let Some(list) = fields.get("conditions") {
                    let parsed = list
                        .as_array()
                        .ok_or("conditions must be an array")?
                        .iter()
                        .map(|c| conditions.decode(c))
                        .collect::<Result<Vec<_>, _>>()?;
                    for condition in &parsed {
                        if !conditions.test(condition)? {
                            return Ok(false);
                        }
                    }
                }
                Ok(true)
            })();
            match outcome {
                Ok(true) => result.present = true,
                Ok(false) => {}
                Err(error) => result.problems.push(format!("{id}: {error}")),
            }
        }
        result
    }

    /// The trim material registry's data half: each file naming an
    /// `ingredient` item registers that item for the file's own id. Files
    /// without one (the host's own newer form) name their item by the host's
    /// conventions instead and are not this loader's to read.
    pub fn load_materials(files: &[(Identifier, Value)]) -> (Vec<(Identifier, Identifier)>, Vec<String>) {
        let mut materials = Vec::new();
        let mut problems = Vec::new();
        let mut sorted: Vec<&(Identifier, Value)> = files.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for (id, value) in sorted {
            let Some(ingredient) = value.as_object().and_then(|fields| fields.get("ingredient")).and_then(Value::as_str)
            else {
                continue;
            };
            match Identifier::parse(ingredient) {
                Ok(item) => materials.push((item, (*id).clone())),
                Err(error) => problems.push(format!("{id}: {error}")),
            }
        }
        (materials, problems)
    }

    /// The trim pattern registry's data half: every file's id is a pattern.
    pub fn load_patterns(files: &[(Identifier, Value)]) -> Vec<Identifier> {
        let mut patterns: Vec<Identifier> = files.iter().map(|(id, _)| id.clone()).collect();
        patterns.sort();
        patterns.dedup();
        patterns
    }

    /// TrimMaterials.getFromIngredient: the material a stack's item names,
    /// data entries first (a pack overrides), then the vanilla table.
    pub fn material_of(&self, item: &Identifier) -> Option<Identifier> {
        self.materials
            .iter()
            .find(|(ingredient, _)| ingredient == item)
            .map(|(_, material)| material.clone())
            .or_else(|| vanilla_material(item))
    }

    /// TrimPatterns.getFromTemplate: the pattern a template item names,
    /// `<pattern>_armor_trim_smithing_template` in the template's namespace.
    pub fn pattern_of(&self, item: &Identifier) -> Option<Identifier> {
        let name = item.path().strip_suffix(TEMPLATE_SUFFIX)?;
        let candidate = Identifier::new(item.namespace(), name).ok()?;
        if (candidate.namespace() == "minecraft" && vanilla_pattern(name)) || self.patterns.contains(&candidate) {
            Some(candidate)
        } else {
            None
        }
    }

    /// Whether a stack can be the recipe's material input: in the vanilla
    /// trim materials tag, or an item a trim material names as its ingredient.
    fn is_material(&self, stack: &ItemSlot, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> bool {
        let tag = Identifier::parse(MATERIALS_TAG).expect("valid tag id");
        in_tag(&stack.item, &tag) || self.material_of(&stack.item).is_some()
    }

    /// matches + getValidatedResult in one evaluation, when the recipe is
    /// present at all. None is no match; the returned flag says the trim
    /// modifier was newly gained (its rebuild is the caller's).
    fn craft(
        &self,
        tool: &ItemSlot,
        stacks: &[Option<ItemSlot>],
        in_tag: &dyn Fn(&Identifier, &Identifier) -> bool,
    ) -> Option<Result<(ItemSlot, Vec<u32>, bool), String>> {
        let mut template: Option<(usize, &ItemSlot)> = None;
        let mut material: Option<(usize, &ItemSlot)> = None;
        for (index, stack) in stacks.iter().enumerate() {
            let Some(stack) = stack else { continue };
            // ArmorTrimRecipe.findInputs: a template first, then a material;
            // a duplicate of either, or anything else, does not match.
            if stack.item.path().ends_with(TEMPLATE_SUFFIX) {
                if template.is_some() {
                    return None;
                }
                template = Some((index, stack));
            } else if self.is_material(stack, in_tag) {
                if material.is_some() {
                    return None;
                }
                material = Some((index, stack));
            } else {
                return None;
            }
        }
        let (template_index, template_stack) = template?;
        let (material_index, material_stack) = material?;
        let Some(material_id) = self.material_of(&material_stack.item) else {
            return Some(Err(format!(
                "recipe.tconstruct.modifier.armor_trim.invalid_material {}",
                material_stack.item
            )));
        };
        let no_pattern = Identifier::parse(NO_PATTERN_TAG).expect("valid tag id");
        let pattern_id = if in_tag(&tool.item, &no_pattern) {
            None
        } else {
            let Some(pattern) = self.pattern_of(&template_stack.item) else {
                return Some(Err(format!(
                    "recipe.tconstruct.modifier.armor_trim.invalid_pattern {}",
                    template_stack.item
                )));
            };
            Some(pattern)
        };
        let mut result = tool.clone();
        result.count = 1;
        let mut persistent = match result.custom_data.get(TAG_PERSISTENT) {
            Some(Nbt::Compound(data)) => data.clone(),
            _ => NbtCompound::new(),
        };
        persistent.insert(MATERIAL_KEY.into(), Nbt::String(material_id.to_string()));
        if let Some(pattern) = &pattern_id {
            persistent.insert(PATTERN_KEY.into(), Nbt::String(pattern.to_string()));
        }
        result.custom_data.insert(TAG_PERSISTENT.into(), Nbt::Compound(persistent));
        let mut consumed = vec![0u32; stacks.len()];
        consumed[template_index] = 1;
        consumed[material_index] = 1;
        let trim_id = Identifier::parse(TRIM).expect("valid modifier id");
        match gain(&mut result, &trim_id) {
            Ok(gained) => Some(Ok((result, consumed, gained))),
            Err(error) => Some(Err(error)),
        }
    }

    /// The station's evaluation: recipe absent, or a tool outside the trim
    /// tag, is no match.
    pub fn attempt(
        &self,
        tool: &ItemSlot,
        env: &Environment,
        stacks: &[Option<ItemSlot>],
        in_tag: &dyn Fn(&Identifier, &Identifier) -> bool,
    ) -> Option<Result<(ItemSlot, Vec<u32>), String>> {
        if !self.present {
            return None;
        }
        let tag = Identifier::parse(TOOL_TAG).ok()?;
        if !in_tag(&tool.item, &tag) {
            return None;
        }
        match self.craft(tool, stacks, in_tag)? {
            Ok((mut result, consumed, gained)) => {
                if gained {
                    if let Err(error) = finish(&mut result, env) {
                        return Some(Err(error));
                    }
                }
                Some(Ok((result, consumed)))
            }
            Err(message) => Some(Err(message)),
        }
    }
}

/// Reload listeners, shaped like the other recipe listeners: the recipe file
/// in the modifier recipe folder, plus the two trim registry folders the
/// recipe resolves through.
pub(crate) fn register<C: Conditions + 'static>(
    context: impl Fn() -> Result<C, String> + Send + Sync + 'static,
) -> Result<Arc<RwLock<Recipes>>, String> {
    let state = Arc::new(RwLock::new(Recipes::default()));
    let output = state.clone();
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "armor_trim_recipes").unwrap(),
        crate::modifier_recipes::FOLDER,
        move |files| {
            let context = context();
            let mut state = output.write().map_err(|_| "armor trim recipes poisoned")?;
            match context {
                Ok(context) => {
                    let loaded = Recipes::load(files, &context);
                    for problem in &loaded.problems {
                        eprintln!("Tinkers: {problem}");
                    }
                    state.present = loaded.present;
                    state.problems = loaded.problems;
                    state.loaded = true;
                    Ok(())
                }
                Err(error) => {
                    state.problems = vec![error.clone()];
                    Err(error)
                }
            }
        },
    )?;
    let output = state.clone();
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "trim_materials").unwrap(),
        "trim_material",
        move |files| {
            let mut state = output.write().map_err(|_| "armor trim recipes poisoned")?;
            let (materials, problems) = Recipes::load_materials(files);
            for problem in &problems {
                eprintln!("Tinkers: {problem}");
            }
            state.materials = materials;
            state.material_problems = problems;
            Ok(())
        },
    )?;
    let output = state.clone();
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "trim_patterns").unwrap(),
        "trim_pattern",
        move |files| {
            let mut state = output.write().map_err(|_| "armor trim recipes poisoned")?;
            state.patterns = Recipes::load_patterns(files);
            Ok(())
        },
    )?;
    let stopped = state.clone();
    ServerLifecycleEvents::SERVER_STOPPED.register(move |_| {
        *stopped.write().expect("armor trim recipes poisoned") = Recipes::default()
    });
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct Flags;
    impl Conditions for Flags {
        type Parsed = bool;
        fn decode(&self, value: &Value) -> Result<bool, String> {
            value.as_bool().ok_or("condition must be boolean".into())
        }
        fn test(&self, value: &bool) -> Result<bool, String> {
            Ok(*value)
        }
    }

    fn item(text: &str) -> Identifier {
        Identifier::parse(text).unwrap()
    }

    /// The station tag lookup of these tests: the travelers chestplate takes
    /// a trim and needs a pattern; the tag of material items holds the fake
    /// ingot no material names (for the invalid-material failure).
    fn in_tag(item: &Identifier, tag: &Identifier) -> bool {
        match tag.to_string().as_str() {
            TOOL_TAG => item.to_string() == "tconstruct:travelers_chestplate",
            NO_PATTERN_TAG => false,
            MATERIALS_TAG => item.to_string() == "test:fake_ingot",
            _ => false,
        }
    }

    fn shipped() -> Recipes {
        let files = vec![
            (item("tconstruct:tools/modifiers/slotless/trim"), json!({"type": "tconstruct:armor_trim_modifier"})),
            (item("tconstruct:tools/modifiers/slotless/banner"), json!({"type": "tconstruct:banner_modifier", "clear_input": {"item": "tconstruct:sky_slime_bottle"}})),
        ];
        let mut recipes = Recipes::load(&files, &Flags);
        assert!(recipes.problems.is_empty(), "{:?}", recipes.problems);
        assert!(recipes.present);
        let (materials, problems) = Recipes::load_materials(&[(
            item("tconstruct:cobalt"),
            json!({"asset_name": "cobalt", "description": {"color": "#4DB9E2", "text": "Cobalt"}, "ingredient": "tconstruct:cobalt_ingot", "item_model_index": 0.8}),
        )]);
        assert!(problems.is_empty());
        recipes.materials = materials;
        recipes
    }

    fn stack(item_id: &str) -> ItemSlot {
        ItemSlot::new(item(item_id), 1)
    }

    fn persistent(result: &ItemSlot) -> NbtCompound {
        match result.custom_data.get(TAG_PERSISTENT) {
            Some(Nbt::Compound(data)) => data.clone(),
            _ => panic!("no persistent data"),
        }
    }

    #[test]
    fn a_template_and_material_write_the_trim_the_display_module_reads() {
        let recipes = shipped();
        let tool = stack("tconstruct:travelers_chestplate");
        let template = stack("minecraft:coast_armor_trim_smithing_template");
        let material = stack("minecraft:diamond");
        let (result, consumed, gained) = recipes
            .craft(&tool, &[Some(template), Some(material)], &in_tag)
            .expect("a match")
            .expect("a craft");
        assert!(gained, "the tool gains the trim modifier");
        assert_eq!(consumed, vec![1, 1]);
        // The form TrimModule reads (and its on_removed forgets): the two id
        // strings under `<modifier>_material` and `<modifier>_pattern`.
        let data = persistent(&result);
        assert_eq!(data.get(MATERIAL_KEY), Some(&Nbt::String("minecraft:diamond".to_string())));
        assert_eq!(data.get(PATTERN_KEY), Some(&Nbt::String("minecraft:coast".to_string())));
        assert_eq!(List::from_nbt(result.custom_data.get(TAG_UPGRADES)).level(&item(TRIM)), 1);
        // A data trim material resolves through its ingredient item.
        let cobalt = stack("tconstruct:cobalt_ingot");
        let template = stack("minecraft:sentry_armor_trim_smithing_template");
        let (result, _, _) = recipes
            .craft(&tool, &[Some(cobalt), Some(template)], &in_tag)
            .expect("a match")
            .expect("a craft");
        let data = persistent(&result);
        assert_eq!(data.get(MATERIAL_KEY), Some(&Nbt::String("tconstruct:cobalt".to_string())));
        assert_eq!(data.get(PATTERN_KEY), Some(&Nbt::String("minecraft:sentry".to_string())));
    }

    #[test]
    fn unresolvable_inputs_fail_and_wrong_inputs_do_not_match() {
        let recipes = shipped();
        let tool = stack("tconstruct:travelers_chestplate");
        // A template naming no registered pattern: the recipe's failure.
        let bogus = stack("minecraft:bogus_armor_trim_smithing_template");
        let diamond = stack("minecraft:diamond");
        let failure = recipes
            .craft(&tool, &[Some(bogus), Some(diamond.clone())], &in_tag)
            .expect("a match")
            .expect_err("a failure");
        assert!(failure.starts_with("recipe.tconstruct.modifier.armor_trim.invalid_pattern"), "{failure}");
        // A tag member no trim material names: the other failure.
        let template = stack("minecraft:coast_armor_trim_smithing_template");
        let fake = stack("test:fake_ingot");
        let failure = recipes
            .craft(&tool, &[Some(template.clone()), Some(fake)], &in_tag)
            .expect("a match")
            .expect_err("a failure");
        assert!(failure.starts_with("recipe.tconstruct.modifier.armor_trim.invalid_material"), "{failure}");
        // Duplicates and strays do not match at all.
        assert!(recipes.craft(&tool, &[Some(template.clone()), Some(template.clone())], &in_tag).is_none());
        assert!(recipes.craft(&tool, &[Some(diamond.clone()), Some(diamond)], &in_tag).is_none());
        assert!(recipes.craft(&tool, &[Some(template), Some(stack("minecraft:dirt"))], &in_tag).is_none());
        // One input missing does not match.
        let template = stack("minecraft:coast_armor_trim_smithing_template");
        assert!(recipes.craft(&tool, &[Some(template), None], &in_tag).is_none());
    }

    #[test]
    fn host_form_trim_material_files_register_no_ingredient() {
        // The host's own trim material files carry no `ingredient`; they are
        // skipped, not errors, and the vanilla table still resolves.
        let (materials, problems) = Recipes::load_materials(&[(
            item("minecraft:iron"),
            json!({"description": {"color": "#D8D8D8", "text": "Iron"}, "palette_id": "minecraft:iron"}),
        )]);
        assert!(materials.is_empty());
        assert!(problems.is_empty());
        let recipes = Recipes::default();
        assert_eq!(recipes.material_of(&item("minecraft:iron_ingot")), Some(item("minecraft:iron")));
        assert_eq!(recipes.material_of(&item("minecraft:dirt")), None);
    }
}
