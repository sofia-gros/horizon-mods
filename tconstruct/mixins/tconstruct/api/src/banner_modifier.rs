//! Banner modifier recipe (BannerModifierRecipe): a banner tool and one banner
//! on the tinker station take the banner's patterns, the original's
//! `BannerModifierRecipe` + `BannerModule.copyPatterns` (SlimeKnights, MIT; see
//! LICENSE). The banner item's dye becomes the base pattern unless a stack
//! matching the recipe's `clear_input` is also on the table, which clears the
//! base instead; a clear with no patterns at all is the recipe's one failure.
//! The patterns are stored in the tool's persistent data under
//! `tconstruct:banner_patterns` (a list of `{pattern, dye, color}` compounds,
//! the hashname string, the DyeColor id and the ARGB diffuse color) with their
//! hash under `tconstruct:banner_cache` — exactly the form the `tconstruct:banner`
//! display module (BannerModule) reads back for the name colour and tooltip —
//! and the tool gains the `tconstruct:banner` modifier if it lacks it.
//! The recipe file (`tools/modifiers/slotless/banner.json`) names the clear
//! input; its mere presence enables the recipe.
use crate::{
    material_recipes::Ingredient,
    materials::Conditions,
    modifier_entries::List,
    tool_slot,
    tool_stack::{rebuild_stats, try_validate, Environment, TAG_MODIFIERS, TAG_PERSISTENT, TAG_UPGRADES},
};
use pluto_lib::{Identifier, ItemSlot, Nbt, NbtCompound, ResourceManagerHelper, ResourceType, ServerLifecycleEvents};
use serde_json::Value;
use std::sync::{Arc, RwLock};

/// `TinkerTags.Items.BANNER` (the tools a banner applies to).
const TOOL_TAG: &str = "tconstruct:modifiable/banner";
/// The vanilla item tag of the sixteen banner items.
const BANNER_TAG: &str = "minecraft:banners";
/// `TinkerModifiers.banner`.
const BANNER: &str = "tconstruct:banner";
/// BannerModule's pattern key: the modifier id with the `_patterns` suffix.
const PATTERNS_KEY: &str = "tconstruct:banner_patterns";
/// BannerModule's cache key: the modifier id with the `_cache` suffix.
const CACHE_KEY: &str = "tconstruct:banner_cache";
/// The failure when a clear would leave the tool with no patterns at all.
const NO_PATTERNS: &str = "recipe.tconstruct.banner.clear.no_patterns";

/// A vanilla dye by its DyeColor id: the diffuse RGB `DyeColor` reports (the
/// post-1.12 values dyeing.rs also uses). Out-of-range ids wrap, as
/// `DyeColor.byId` wraps through its by-id map.
fn dye_rgb(id: i32) -> i32 {
    match id.rem_euclid(16) {
        0 => 0xF9FFFE,
        1 => 0xF9801D,
        2 => 0xC74EBD,
        3 => 0x3AB3DA,
        4 => 0xFED83D,
        5 => 0x80C71F,
        6 => 0xF38BAA,
        7 => 0x474F52,
        8 => 0x9D9D97,
        9 => 0x169C9C,
        10 => 0x8932B8,
        11 => 0x3C44AA,
        12 => 0x835432,
        13 => 0x5E7C16,
        14 => 0xB02E26,
        _ => 0x1D1D21,
    }
}

/// The (DyeColor id, diffuse RGB) of a vanilla banner item, from its colour
/// name (`minecraft:light_blue_banner` and kin). A banner item outside the
/// sixteen vanilla colours has no dye to take here.
fn dye_of(item: &Identifier) -> Option<(i32, i32)> {
    if item.namespace() != "minecraft" {
        return None;
    }
    let id = match item.path().strip_suffix("_banner")? {
        "white" => 0,
        "orange" => 1,
        "magenta" => 2,
        "light_blue" => 3,
        "yellow" => 4,
        "lime" => 5,
        "pink" => 6,
        "gray" => 7,
        "light_gray" => 8,
        "cyan" => 9,
        "purple" => 10,
        "blue" => 11,
        "brown" => 12,
        "green" => 13,
        "red" => 14,
        "black" => 15,
        _ => return None,
    };
    Some((id, dye_rgb(id)))
}

/// `Util.getColor`: the dye's diffuse colour as an ARGB int (alpha 255).
fn argb(rgb: i32) -> i32 {
    (0xFF00_0000u32 | (rgb as u32 & 0x00FF_FFFF)) as i32
}

/// Java's `String.hashCode`, over UTF-16 code units.
fn java_hash(text: &str) -> i32 {
    let mut hash = 0i32;
    for unit in text.encode_utf16() {
        hash = hash.wrapping_mul(31).wrapping_add(i32::from(unit));
    }
    hash
}

/// One hash step of BannerModule.copyPatterns: `31 * (31 * hash + color) + pattern.hashCode()`.
fn fold(hash: i32, color: i32, pattern: &str) -> i32 {
    hash.wrapping_mul(31).wrapping_add(color).wrapping_mul(31).wrapping_add(java_hash(pattern))
}

/// One stored pattern entry, in BannerModule's written form.
fn pattern_entry(pattern: &str, dye: i32, color: i32) -> Nbt {
    let mut tag = NbtCompound::new();
    tag.insert("pattern".into(), Nbt::String(pattern.to_owned()));
    tag.insert("dye".into(), Nbt::Int(dye));
    tag.insert("color".into(), Nbt::Int(color));
    Nbt::Compound(tag)
}

/// BannerModule.copyPatterns: the stored pattern list and its cache hash for a
/// base dye (None clears the base) and the banner's own patterns as
/// (hashname, DyeColor id) pairs. The base entry is the base pattern `b`.
fn copy_patterns(dye: Option<(i32, i32)>, patterns: &[(String, i32)]) -> (Vec<Nbt>, i32) {
    let mut stored = Vec::new();
    let mut hash = 0i32;
    if let Some((dye_id, rgb)) = dye {
        let color = argb(rgb);
        hash = color;
        stored.push(pattern_entry("b", dye_id, color));
        hash = fold(hash, color, "b");
    }
    for (pattern, dye_id) in patterns {
        let color = argb(dye_rgb(*dye_id));
        stored.push(pattern_entry(pattern, *dye_id, color));
        hash = fold(hash, color, pattern);
    }
    (stored, hash)
}

/// The patterns a banner stack carries, in the stack form the game writes:
/// the `Patterns` list of its `BlockEntityTag`, each entry's `Pattern`
/// hashname and `Color` DyeColor id. No block entity data means no patterns.
fn banner_patterns(stack: &ItemSlot) -> Vec<(String, i32)> {
    let Some(Nbt::Compound(data)) = stack.custom_data.get("BlockEntityTag") else { return Vec::new() };
    let Some(Nbt::List(list)) = data.get("Patterns") else { return Vec::new() };
    list.iter()
        .map(|entry| match entry {
            Nbt::Compound(tag) => (
                match tag.get("Pattern") {
                    Some(Nbt::String(text)) => text.clone(),
                    _ => String::new(),
                },
                crate::modifier_entries::nbt_int(tag.get("Color")),
            ),
            _ => (String::new(), 0),
        })
        .collect()
}

/// Whether a stack is a banner item (the original tests `instanceof BannerItem`):
/// a member of the vanilla banners tag, or one of the sixteen by name.
fn is_banner(stack: &ItemSlot, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> bool {
    let tag = Identifier::parse(BANNER_TAG).expect("valid tag id");
    in_tag(&stack.item, &tag) || dye_of(&stack.item).is_some()
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

/// One `tconstruct:banner_modifier` recipe: its clear input (absent in the
/// file means the original's empty ingredient, which matches nothing).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Recipe {
    pub clear_input: Option<Ingredient>,
}

impl Recipe {
    /// matches + getValidatedResult in one evaluation. None is no match; the
    /// returned flag says the banner modifier was newly gained (its rebuild
    /// is the caller's, needing the environment).
    fn craft(
        &self,
        tool: &ItemSlot,
        stacks: &[Option<ItemSlot>],
        in_tag: &dyn Fn(&Identifier, &Identifier) -> bool,
    ) -> Option<Result<(ItemSlot, Vec<u32>, bool), String>> {
        let mut banner: Option<(usize, &ItemSlot)> = None;
        let mut clear: Option<usize> = None;
        for (index, stack) in stacks.iter().enumerate() {
            let Some(stack) = stack else { continue };
            // The original's else-if chain: a banner counts as the banner
            // first; only a stack that is not (or no longer) the banner can
            // be the clear input; anything else does not match.
            if is_banner(stack, in_tag) && banner.is_none() {
                banner = Some((index, stack));
            } else if clear.is_none()
                && self.clear_input.as_ref().is_some_and(|ingredient| ingredient.matches(&stack.item, in_tag))
            {
                clear = Some(index);
            } else {
                return None;
            }
        }
        let (banner_index, banner_stack) = banner?;
        // A banner item whose colour is not one of the sixteen has no dye to
        // take; the recipe simply does not take it.
        let dye = dye_of(&banner_stack.item)?;
        let patterns = banner_patterns(banner_stack);
        let dye = if clear.is_some() { None } else { Some(dye) };
        if dye.is_none() && patterns.is_empty() {
            return Some(Err(NO_PATTERNS.into()));
        }
        let mut result = tool.clone();
        result.count = 1;
        let (stored, hash) = copy_patterns(dye, &patterns);
        let mut persistent = match result.custom_data.get(TAG_PERSISTENT) {
            Some(Nbt::Compound(data)) => data.clone(),
            _ => NbtCompound::new(),
        };
        persistent.insert(PATTERNS_KEY.into(), Nbt::List(stored));
        persistent.insert(CACHE_KEY.into(), Nbt::Int(hash));
        result.custom_data.insert(TAG_PERSISTENT.into(), Nbt::Compound(persistent));
        let mut consumed = vec![0u32; stacks.len()];
        consumed[banner_index] = 1;
        if let Some(index) = clear {
            consumed[index] = 1;
        }
        let banner_id = Identifier::parse(BANNER).expect("valid modifier id");
        match gain(&mut result, &banner_id) {
            Ok(gained) => Some(Ok((result, consumed, gained))),
            Err(error) => Some(Err(error)),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Recipes {
    pub recipes: Vec<Recipe>,
    pub problems: Vec<String>,
    pub loaded: bool,
}

impl Recipes {
    pub fn load<C: Conditions>(files: &[(Identifier, Value)], conditions: &C) -> Self {
        let mut result = Self { loaded: true, ..Self::default() };
        let mut sorted: Vec<&(Identifier, Value)> = files.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for (id, value) in sorted {
            let outcome = (|| -> Result<Option<Recipe>, String> {
                let fields = value.as_object().ok_or("recipe must be an object")?;
                if fields.get("type").and_then(Value::as_str) != Some("tconstruct:banner_modifier") {
                    return Ok(None);
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
                            return Ok(None);
                        }
                    }
                }
                let clear_input = match fields.get("clear_input") {
                    Some(value) => Some(Ingredient::decode(value)?),
                    None => None,
                };
                Ok(Some(Recipe { clear_input }))
            })();
            match outcome {
                Ok(Some(recipe)) => result.recipes.push(recipe),
                Ok(None) => {}
                Err(error) => result.problems.push(format!("{id}: {error}")),
            }
        }
        result
    }

    /// The station's evaluation: no loaded recipe, or a tool outside the
    /// banner tag, is no match.
    pub fn attempt(
        &self,
        tool: &ItemSlot,
        env: &Environment,
        stacks: &[Option<ItemSlot>],
        in_tag: &dyn Fn(&Identifier, &Identifier) -> bool,
    ) -> Option<Result<(ItemSlot, Vec<u32>), String>> {
        if self.recipes.is_empty() {
            return None;
        }
        let tag = Identifier::parse(TOOL_TAG).ok()?;
        if !in_tag(&tool.item, &tag) {
            return None;
        }
        for recipe in &self.recipes {
            if let Some(outcome) = recipe.craft(tool, stacks, in_tag) {
                return Some(match outcome {
                    Ok((mut result, consumed, gained)) => {
                        if gained {
                            if let Err(error) = finish(&mut result, env) {
                                return Some(Err(error));
                            }
                        }
                        Ok((result, consumed))
                    }
                    Err(message) => Err(message),
                });
            }
        }
        None
    }
}

/// Reload listener, shaped like the other recipe listeners: it scans the
/// modifier recipe folder for the banner recipe type.
pub(crate) fn register<C: Conditions + 'static>(
    context: impl Fn() -> Result<C, String> + Send + Sync + 'static,
) -> Result<Arc<RwLock<Recipes>>, String> {
    let state = Arc::new(RwLock::new(Recipes::default()));
    let output = state.clone();
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "banner_recipes").unwrap(),
        crate::modifier_recipes::FOLDER,
        move |files| {
            let context = context();
            let mut state = output.write().map_err(|_| "banner recipes poisoned")?;
            match context {
                Ok(context) => {
                    *state = Recipes::load(files, &context);
                    for problem in &state.problems {
                        eprintln!("Tinkers: {problem}");
                    }
                    Ok(())
                }
                Err(error) => {
                    *state = Recipes { problems: vec![error.clone()], ..Recipes::default() };
                    Err(error)
                }
            }
        },
    )?;
    let stopped = state.clone();
    ServerLifecycleEvents::SERVER_STOPPED.register(move |_| {
        *stopped.write().expect("banner recipes poisoned") = Recipes::default()
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

    /// The station tag lookup of these tests: shields take banners, the
    /// sixteen vanilla items are banners.
    fn in_tag(item: &Identifier, tag: &Identifier) -> bool {
        match tag.to_string().as_str() {
            TOOL_TAG => matches!(item.to_string().as_str(), "tconstruct:travelers_shield" | "tconstruct:plate_shield"),
            BANNER_TAG => dye_of(item).is_some(),
            _ => false,
        }
    }

    fn shipped() -> Recipes {
        let files = vec![
            (item("tconstruct:tools/modifiers/slotless/banner"), json!({"type": "tconstruct:banner_modifier", "clear_input": {"item": "tconstruct:sky_slime_bottle"}})),
            (item("tconstruct:tools/modifiers/slotless/trim"), json!({"type": "tconstruct:armor_trim_modifier"})),
        ];
        let recipes = Recipes::load(&files, &Flags);
        assert!(recipes.problems.is_empty(), "{:?}", recipes.problems);
        recipes
    }

    fn banner(item_id: &str, patterns: &[(&str, i32)]) -> ItemSlot {
        let mut stack = ItemSlot::new(item(item_id), 1);
        if !patterns.is_empty() {
            let list: Vec<Nbt> = patterns
                .iter()
                .map(|(pattern, color)| {
                    let mut tag = NbtCompound::new();
                    tag.insert("Pattern".into(), Nbt::String(pattern.to_string()));
                    tag.insert("Color".into(), Nbt::Int(*color));
                    Nbt::Compound(tag)
                })
                .collect();
            let mut data = NbtCompound::new();
            data.insert("Patterns".into(), Nbt::List(list));
            stack.custom_data.insert("BlockEntityTag".into(), Nbt::Compound(data));
        }
        stack
    }

    fn persistent(result: &ItemSlot) -> NbtCompound {
        match result.custom_data.get(TAG_PERSISTENT) {
            Some(Nbt::Compound(data)) => data.clone(),
            _ => panic!("no persistent data"),
        }
    }

    #[test]
    fn the_shipped_recipe_decodes_its_clear_input() {
        let recipes = shipped();
        assert_eq!(recipes.recipes.len(), 1);
        let clear = recipes.recipes[0].clear_input.as_ref().expect("clear input");
        assert!(clear.matches(&item("tconstruct:sky_slime_bottle"), &in_tag));
        assert!(!clear.matches(&item("minecraft:white_banner"), &in_tag));
    }

    #[test]
    fn a_banner_applies_its_patterns_and_dye_as_the_display_module_reads_them() {
        let recipes = shipped();
        let tool = ItemSlot::new(item("tconstruct:travelers_shield"), 1);
        let banner = banner("minecraft:blue_banner", &[("cre", 11), ("sku", 15)]);
        let (result, consumed, gained) = recipes.recipes[0]
            .craft(&tool, &[Some(banner)], &in_tag)
            .expect("a match")
            .expect("a craft");
        assert!(gained, "the tool gains the banner modifier");
        assert_eq!(consumed, vec![1]);
        // The form BannerModule reads: a list of {pattern, dye, color} under
        // `<modifier>_patterns`, its hash under `<modifier>_cache`; the first
        // entry is the base pattern in the banner's own dye (blue, id 11).
        let data = persistent(&result);
        let Some(Nbt::List(stored)) = data.get(PATTERNS_KEY) else { panic!("no stored patterns") };
        let entries: Vec<(String, i32, i32)> = stored
            .iter()
            .map(|entry| match entry {
                Nbt::Compound(tag) => (
                    match tag.get("pattern") {
                        Some(Nbt::String(text)) => text.clone(),
                        _ => panic!("pattern must be a string"),
                    },
                    crate::modifier_entries::nbt_int(tag.get("dye")),
                    crate::modifier_entries::nbt_int(tag.get("color")),
                ),
                _ => panic!("entries are compounds"),
            })
            .collect();
        assert_eq!(
            entries,
            vec![
                ("b".to_string(), 11, argb(0x3C44AA)),
                ("cre".to_string(), 11, argb(0x3C44AA)),
                ("sku".to_string(), 15, argb(0x1D1D21)),
            ]
        );
        // The cache hash, folded by hand from BannerModule's formula
        // (`31 * (31 * hash + color) + pattern.hashCode()`, wrapping as Java
        // ints wrap; the steps run in i64 so the test itself cannot overflow).
        let step = |hash: i32, color: i32, pattern_hash: i32| {
            (31 * (31 * i64::from(hash) + i64::from(color)) + i64::from(pattern_hash)) as i32
        };
        let mut hash = argb(0x3C44AA); // the hash starts at the base colour
        hash = step(hash, argb(0x3C44AA), 98); // "b".hashCode()
        hash = step(hash, argb(0x3C44AA), 98_774); // "cre".hashCode()
        hash = step(hash, argb(0x1D1D21), 113_949); // "sku".hashCode()
        assert_eq!(hash, 852_680_106);
        assert_eq!(data.get(CACHE_KEY), Some(&Nbt::Int(hash)));
        // The modifier gain the display module's binding stands for.
        assert_eq!(List::from_nbt(result.custom_data.get(TAG_UPGRADES)).level(&item(BANNER)), 1);
    }

    #[test]
    fn a_clear_input_drops_the_base_and_a_patternless_clear_fails() {
        let recipes = shipped();
        let tool = ItemSlot::new(item("tconstruct:plate_shield"), 1);
        let bottle = ItemSlot::new(item("tconstruct:sky_slime_bottle"), 1);
        // Patterns but no base dye once cleared.
        let patterned = banner("minecraft:red_banner", &[("hh", 14)]);
        let (result, consumed, _) = recipes.recipes[0]
            .craft(&tool, &[Some(patterned), Some(bottle.clone())], &in_tag)
            .expect("a match")
            .expect("a craft");
        assert_eq!(consumed, vec![1, 1]);
        let data = persistent(&result);
        let Some(Nbt::List(stored)) = data.get(PATTERNS_KEY) else { panic!("no stored patterns") };
        assert_eq!(stored.len(), 1, "only the banner's own pattern remains");
        // No patterns and a clear: the recipe's failure, not a silent craft.
        let plain = banner("minecraft:red_banner", &[]);
        let failure = recipes.recipes[0]
            .craft(&tool, &[Some(plain), Some(bottle)], &in_tag)
            .expect("a match")
            .expect_err("a failure");
        assert_eq!(failure, NO_PATTERNS);
        // No patterns without a clear still applies the base dye.
        let plain = banner("minecraft:red_banner", &[]);
        assert!(recipes.recipes[0].craft(&tool, &[Some(plain)], &in_tag).expect("a match").is_ok());
    }

    #[test]
    fn wrong_inputs_do_not_match() {
        let recipes = shipped();
        let recipe = &recipes.recipes[0];
        let shield = ItemSlot::new(item("tconstruct:travelers_shield"), 1);
        let banner = banner("minecraft:white_banner", &[("b", 0)]);
        // A tool outside the banner tag.
        let pickaxe = ItemSlot::new(item("tconstruct:pickaxe"), 1);
        assert!(recipes.attempt(&pickaxe, &dummy_env(), &[Some(banner.clone())], &in_tag).is_none());
        // A stray item beside the banner.
        let dirt = ItemSlot::new(item("minecraft:dirt"), 1);
        assert!(recipe.craft(&shield, &[Some(banner.clone()), Some(dirt)], &in_tag).is_none());
        // Two banners: the second is neither the banner nor the clear input.
        assert!(recipe.craft(&shield, &[Some(banner.clone()), Some(banner)], &in_tag).is_none());
        // No banner at all.
        assert!(recipe.craft(&shield, &[None, None], &in_tag).is_none());
    }

    fn dummy_env() -> Environment<'static> {
        // Only the tag gate runs before this environment would be touched, and
        // the gate rejects the pickaxe first; the pieces are never read.
        Environment {
            definition: None,
            has_materials: false,
            materials: Box::leak(Box::new(crate::materials::Definitions::default())),
            modifiers: Box::leak(Box::new(crate::modifiers::Registry::default())),
            item_tags_loaded: false,
            item: None,
            item_tags: None,
        }
    }
}
