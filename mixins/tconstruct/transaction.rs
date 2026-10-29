use horizon_items::items::ItemStack;

/// MaterialBlockEntity uses Java object identity when deciding whether to
/// write the unknown material. A parsed "tconstruct:unknown" is equal to, but
/// is not the same object as, the singleton used for an absent or invalid tag.
/// This source model is not yet connected to a registered storage block.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum SourceBlockMaterial {
    UnknownSingleton,
    Parsed(String),
}
#[allow(dead_code)]
impl SourceBlockMaterial {
    fn id(&self) -> &str {
        match self { Self::UnknownSingleton => "tconstruct:unknown", Self::Parsed(id) => id }
    }
    /// The original load ignores absent/non-string tags; invalid strings reset
    /// the field to the UNKNOWN singleton.
    pub(super) fn load(&mut self, string_tag: Option<&str>) {
        if let Some(raw) = string_tag {
            *self = super::tconstruct_source_render_info::material_id(raw)
                .map_or(Self::UnknownSingleton, Self::Parsed);
        }
    }
    /// setMaterial always assigns, even when the IDs compare equal; its return
    /// indicates whether the source would mark the entity dirty and rerender.
    pub(super) fn set(&mut self, material: Self) -> bool {
        let changed = self.id() != material.id();
        *self = material;
        changed
    }
    pub(super) fn load_tag(&mut self, tag: &horizon_world::nbt::Tag) {
        use horizon_world::nbt::Tag;
        let material = match tag.get("Material") { Some(Tag::String(value)) => Some(value.to_string_lossy()), _ => None };
        self.load(material.as_deref());
    }
    /// saveSynced writes only Material when the source reference is not the
    /// singleton. It does not clear unrelated fields or a pre-existing key.
    pub(super) fn write_tag(&self, tag: &mut horizon_world::nbt::Compound) {
        if let Some(material) = self.save_string() {
            tag.insert("Material".into(), horizon_world::nbt::Tag::String(material.into()));
        }
    }
    pub(super) fn save_string(&self) -> Option<&str> {
        match self { Self::UnknownSingleton => None, Self::Parsed(id) => Some(id) }
    }
    /// MaterialItem.getMaterialId returns the singleton for an empty or
    /// malformed stack tag; a valid string constructs a distinct object.
    pub(super) fn placed_material(string_tag: Option<&str>) -> Self {
        string_tag.filter(|s| !s.is_empty())
            .and_then(super::tconstruct_source_render_info::material_id)
            .map_or(Self::UnknownSingleton, Self::Parsed)
    }
}

/// The current preview still has only three fixed choices, but selection is
/// stored as the original Pattern identity rather than its transient index.
const PREVIEW_PATTERNS: [&str; 3] = ["tconstruct:pick_head", "tconstruct:tool_handle", "tconstruct:tool_binding"];
#[allow(dead_code)]
pub(super) fn preview_pattern(index: i16) -> Option<&'static str> {
    usize::try_from(index).ok().and_then(|i| PREVIEW_PATTERNS.get(i).copied())
}
#[allow(dead_code)]
pub(super) fn pattern_index(selected: &str, patterns: &[&str]) -> Option<usize> {
    patterns.iter().position(|pattern| *pattern == selected)
}
#[allow(dead_code)]
pub(super) fn preview_pattern_index(selected: &str) -> Option<i16> {
    pattern_index(selected, &PREVIEW_PATTERNS).map(|i| i as i16)
}

/// `IMaterialValue.getItemsUsed` and `getRemainder` from the pinned original.
/// Recipe loaders require strictly positive cost, needed and value.
fn material_cost(cost: i32, needed: i32, value: i32) -> Option<(i32, i32)> {
    if cost <= 0 || needed <= 0 || value <= 0 { return None; }
    // Java's int multiplication/subtraction wrap, and division truncates to zero.
    let total = cost.wrapping_mul(needed);
    let used = total / value + i32::from(total % value != 0);
    let mut remainder = value.wrapping_sub(total) % value;
    if remainder < 0 { remainder += value; }
    Some((used, remainder))
}

/// `PartRecipe.assemble` changes the part count when a material's value
/// exceeds the part cost and the material recipe has no leftover item.
fn material_output_count(output_count: i32, value: i32, cost: i32, has_leftover: bool) -> Option<i32> {
    if output_count <= 0 || value <= 0 || cost <= 0 { return None; }
    Some(if !has_leftover && value > cost { output_count.wrapping_mul(value) / cost } else { output_count })
}

/// Numeric portion of `FloatToolStat`: updates accumulate additive values,
/// percent is accumulated separately, then the built value is multiplied.
/// This is not yet wired into the prototype's fixed tool statistics.
#[allow(dead_code)]
struct SourceFloatStat {
    base: f32,
    add: f32,
    percent: f32,
    multiply: f32,
}
#[allow(dead_code)]
impl SourceFloatStat {
    fn new(base: f32) -> Self { Self { base, add: 0.0, percent: 0.0, multiply: 1.0 } }
    fn update(&mut self, value: f32) { self.add += value; self.base = 0.0; }
    fn add(&mut self, value: f64) { self.add = (self.add as f64 + value) as f32; }
    // Java's `float += double` and `float *= double` evaluate in double and
    // narrow after each operation. Its final build expression is float-only.
    fn percent(&mut self, value: f64) { self.percent = (self.percent as f64 + value) as f32; }
    fn multiply(&mut self, value: f64) { self.multiply = (self.multiply as f64 * value) as f32; }
    fn build(&self) -> f32 { (self.base + self.add) * (1.0 + self.percent) * self.multiply }
}

/// Source head/handle additions for a pickaxe before tiers, modifiers, clamps,
/// persistence or item components. These remaining stages are not yet ported.
#[allow(dead_code)]
struct SourceHeadStats { durability: i32, mining_speed: f32, attack_damage: f32 }
#[allow(dead_code)]
struct SourceHandleStats { durability: f32, mining_speed: f32, attack_speed: f32, attack_damage: f32 }
#[allow(dead_code)]
fn source_pickaxe_base(head: &SourceHeadStats, handle: &SourceHandleStats) -> [f32; 4] {
    let mut durability = SourceFloatStat::new(1.0);
    let mut mining = SourceFloatStat::new(1.0);
    let mut attack = SourceFloatStat::new(0.0);
    let mut speed = SourceFloatStat::new(1.0);
    durability.update(head.durability as f32);
    mining.update(head.mining_speed);
    attack.update(head.attack_damage);
    durability.percent(handle.durability as f64);
    mining.percent(handle.mining_speed as f64);
    attack.percent(handle.attack_damage as f64);
    speed.percent(handle.attack_speed as f64);
    // `tool_definitions/pickaxe.json`: base stats update after part stats.
    attack.update(0.5);
    speed.update(1.2);
    [durability.build(), mining.build(), attack.build(), speed.build()]
}

/// Ingredient forms used by pinned original material recipes. This parser and
/// matcher are not yet connected to world recipes, tags or item components.
#[allow(dead_code)]
#[derive(Debug)]
enum SourceIngredient {
    Item(String), Tag(String), Any(Vec<Self>), Difference(Box<Self>, Box<Self>),
    Instrument { item: String, exact: Option<String>, ignore: Option<String> },
}
#[allow(dead_code)]
impl SourceIngredient {
    fn parse(v: &serde_json::Value) -> Option<Self> {
        if let Some(list) = v.as_array() {
            if list.is_empty() { return None; }
            return Some(Self::Any(list.iter().map(Self::parse).collect::<Option<_>>()?));
        }
        let o = v.as_object()?;
        let get = |key: &str| o.get(key).and_then(serde_json::Value::as_str).map(str::to_owned);
        match o.get("type").and_then(serde_json::Value::as_str) {
            None => match (get("item"), get("tag")) {
                (Some(item), None) => Some(Self::Item(item)),
                (None, Some(tag)) => Some(Self::Tag(tag)),
                _ => None,
            },
            Some("forge:difference") => Some(Self::Difference(Box::new(Self::parse(o.get("base")?)?), Box::new(Self::parse(o.get("subtracted")?)?))),
            Some("tconstruct:instrument") => {
                let item = get("item")?;
                let exact = get("instrument");
                let ignore = get("ignore");
                if exact.is_none() && ignore.is_none() { return None; }
                Some(Self::Instrument { item, exact, ignore })
            }
            _ => None,
        }
    }

    /// `instrument` is a normalized valid ID, or None if absent or malformed.
    /// Tags and known instruments must come from the current world's registries.
    fn matches(&self, item: &str, instrument: Option<&str>, item_tags: &std::collections::HashMap<String, std::collections::HashSet<String>>, known_instruments: &std::collections::HashSet<String>, instrument_tags: &std::collections::HashMap<String, std::collections::HashSet<String>>) -> bool {
        match self {
            Self::Item(id) => item == id,
            Self::Tag(tag) => item_tags.get(tag).is_some_and(|set| set.contains(item)),
            Self::Any(list) => list.iter().any(|v| v.matches(item, instrument, item_tags, known_instruments, instrument_tags)),
            Self::Difference(base, subtracted) => base.matches(item, instrument, item_tags, known_instruments, instrument_tags) && !subtracted.matches(item, instrument, item_tags, known_instruments, instrument_tags),
            Self::Instrument { item: expected, exact, ignore } => {
                if item != expected { return false; }
                match (instrument, exact, ignore) {
                    (Some(id), Some(expected), _) => id == expected,
                    (Some(id), None, Some(tag)) => known_instruments.contains(id) && !instrument_tags.get(tag).is_some_and(|set| set.contains(id)),
                    (None, None, Some(_)) => true,
                    _ => false,
                }
            }
        }
    }
}

/// Material-recipe load conditions present in the pinned source catalog.
/// Evaluation takes explicit registry and config snapshots; this remains unconnected to
/// world datapacks and recipe reloads.
#[allow(dead_code)]
#[derive(Debug)]
enum SourceCondition {
    TagFilled { registry: String, tag: String },
    TagCombination { registry: String, required: Vec<String>, ignore: Option<String> },
    Or(Vec<Self>),
    And(Vec<Self>),
    Config(String),
}
#[allow(dead_code)]
impl SourceCondition {
    fn parse(v: &serde_json::Value) -> Option<Self> {
        let o = v.as_object()?;
        let text = |name: &str| o.get(name).and_then(serde_json::Value::as_str).map(str::to_owned);
        let registry = text("registry").unwrap_or_else(|| "minecraft:item".into());
        match o.get("type")?.as_str()? {
            "mantle:tag_filled" => Some(Self::TagFilled { registry, tag: text("tag")? }),
            "mantle:tag_combination_filled" => {
                let value = o.get("match")?;
                let required: Vec<String> = if let Some(id) = value.as_str() { vec![id.into()] }
                    else { value.as_array()?.iter().map(|v| v.as_str().map(str::to_owned)).collect::<Option<_>>()? };
                if required.is_empty() { return None; }
                Some(Self::TagCombination { registry, required, ignore: text("ignore") })
            }
            "tconstruct:config" => {
                let property = text("prop")?.to_lowercase();
                const KNOWN: &[&str] = &["spawn_with_book", "gravel_to_flint", "cheaper_netherite_alloy", "wither_bone_drop", "wither_bone_conversion", "slime_recipe_fix", "glass_recipe_fix", "allow_ingotless_alloys", "force_integration_materials", "slimy_loot_chests", "sync_knockback_resistance"];
                if !KNOWN.contains(&property.as_str()) { return None; }
                Some(Self::Config(if property == "slime_recipe_fix" { "glass_recipe_fix".into() } else { property }))
            }
            "forge:or" | "forge:and" => {
                let values = o.get("values")?.as_array()?;
                if values.is_empty() { return None; }
                let parsed = values.iter().map(Self::parse).collect::<Option<_>>()?;
                Some(if o.get("type")?.as_str()? == "forge:or" { Self::Or(parsed) } else { Self::And(parsed) })
            }
            _ => None,
        }
    }
    fn matches(&self, registries: &std::collections::HashMap<String, std::collections::HashMap<String, std::collections::HashSet<String>>>, config: &std::collections::HashMap<String, bool>) -> Option<bool> {
        match self {
            Self::TagFilled { registry, tag } => Some(registries.get(registry)?.get(tag).is_some_and(|values| !values.is_empty())),
            Self::TagCombination { registry, required, ignore } => {
                let tags = registries.get(registry)?;
                let Some(first) = required.first().and_then(|tag| tags.get(tag)) else { return Some(false); };
                Some(first.iter().any(|entry| !ignore.as_ref().and_then(|tag| tags.get(tag)).is_some_and(|values| values.contains(entry))
                    && required[1..].iter().all(|tag| tags.get(tag).is_some_and(|values| values.contains(entry)))))
            }
            Self::Or(values) => {
                let mut unknown = false;
                for value in values {
                    match value.matches(registries, config) {
                        Some(true) => return Some(true),
                        Some(false) => {},
                        None => unknown = true,
                    }
                }
                if unknown { None } else { Some(false) }
            }
            Self::And(values) => {
                let mut unknown = false;
                for value in values {
                    match value.matches(registries, config) {
                        Some(false) => return Some(false),
                        Some(true) => {},
                        None => unknown = true,
                    }
                }
                if unknown { None } else { Some(true) }
            }
            Self::Config(property) if property == "wither_bone_conversion" => Some(false),
            Self::Config(property) => config.get(property).copied(),
        }
    }
}

/// Source ItemOutput forms in the pinned material recipes. Tag selection and
/// NBT decoding are deliberately deferred until the world registry is present.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub(super) enum SourceOutputKind { Item(String), Tag(String) }
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub(super) struct SourceItemOutput { pub kind: SourceOutputKind, pub count: i32, pub nbt: Option<serde_json::Value> }
#[allow(dead_code)]
impl SourceItemOutput {
    fn parse(value: &serde_json::Value) -> Option<Self> {
        if let Some(id) = value.as_str() {
            if id == "minecraft:air" { return None; }
            return Some(Self { kind: SourceOutputKind::Item(id.into()), count: 1, nbt: None });
        }
        let obj = value.as_object()?;
        let count = match obj.get("count") { Some(value) => i32::try_from(value.as_i64()?).ok()?, None => 1 };
        if count <= 0 { return None; }
        let kind = if let Some(tag) = obj.get("tag").and_then(serde_json::Value::as_str) {
            SourceOutputKind::Tag(tag.into())
        } else {
            let id = obj.get("item")?.as_str()?;
            if id == "minecraft:air" { return None; }
            SourceOutputKind::Item(id.into())
        };
        Some(Self { kind, count, nbt: obj.get("nbt").cloned() })
    }
    /// Item tag preference from Mantle 1.11.115. Outer None means the
    /// namespace preference snapshot is unavailable; Some(None) is an empty tag.
    pub(super) fn resolve_id(&self, tags: &std::collections::HashMap<String, std::collections::HashSet<String>>, preferences: Option<&[String]>) -> Option<Option<String>> {
        match &self.kind {
            SourceOutputKind::Item(id) => Some(Some(id.clone())),
            SourceOutputKind::Tag(tag) => {
                let Some(values) = tags.get(tag) else { return Some(None); };
                if values.len() <= 1 { return Some(values.iter().next().cloned()); }
                let preferences = preferences?;
                let key = |id: &String| {
                    let (namespace, path) = id.split_once(':').unwrap_or(("minecraft", id));
                    let rank = preferences.iter().position(|value| value == namespace).unwrap_or(preferences.len());
                    (rank, namespace.to_owned(), path.to_owned())
                };
                Some(values.iter().min_by_key(|id| key(id)).cloned())
            }
        }
    }
    /// `IMaterialValue.getLeftover`: Java int multiply then truncating divide.
    pub(super) fn count_for_remainder(&self, remainder: i32, needed: i32) -> Option<i32> {
        if remainder <= 0 || needed <= 0 { return None; }
        let count = self.count.wrapping_mul(remainder) / needed;
        (count > 0).then_some(count)
    }
}

/// Pinned `MaterialRecipe` fields used for lookup and cost calculation.
/// Leftover output stays unresolved until Mantle's tag preference and NBT
/// rules are connected to the current world. Not used by gameplay yet.
#[allow(dead_code)]
struct SourceMaterialRecipe {
    ingredient: SourceIngredient,
    material: String,
    value: i32,
    needed: i32,
    leftover: Option<SourceItemOutput>,
    conditions: Vec<SourceCondition>,
}
#[allow(dead_code)]
impl SourceMaterialRecipe {
    fn parse(v: &serde_json::Value) -> Option<Self> {
        if v.get("type")?.as_str()? != "tconstruct:material" { return None; }
        let positive = |name: &str| -> Option<i32> {
            let n = match v.get(name) { Some(value) => value.as_i64()?, None => 1 };
            i32::try_from(n).ok().filter(|n| *n > 0)
        };
        let value = positive("value")?;
        let conditions = match v.get("conditions") {
            Some(list) => list.as_array()?.iter().map(SourceCondition::parse).collect::<Option<_>>()?,
            None => Vec::new(),
        };
        // The original loader validates the optional output before the
        // constructor discards it for value 1. Unsupported explicit empty
        // output forms fail closed rather than silently changing the recipe.
        let leftover = match v.get("leftover") {
            None | Some(serde_json::Value::Null) => None,
            Some(output) => Some(SourceItemOutput::parse(output)?),
        };
        Some(Self {
            ingredient: SourceIngredient::parse(v.get("ingredient")?)?,
            material: v.get("material")?.as_str()?.to_owned(),
            value,
            needed: positive("needed")?,
            leftover: if value > 1 { leftover } else { None },
            conditions,
        })
    }
    fn enabled(&self, registries: &std::collections::HashMap<String, std::collections::HashMap<String, std::collections::HashSet<String>>>, config: &std::collections::HashMap<String, bool>) -> Option<bool> {
        let mut unknown = false;
        for condition in &self.conditions {
            match condition.matches(registries, config) {
                Some(false) => return Some(false),
                Some(true) => {},
                None => unknown = true,
            }
        }
        if unknown { None } else { Some(true) }
    }
}

/// The source material-definition loader checks redirects before its own
/// condition, then requires a craftable value. This parser is not yet wired
/// into the world's material registry or the Part Builder.
#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq)]
pub(super) enum SourceMaterialDefinition {
    Material { craftable: bool, hidden: bool, tier: i32, sort_order: i32, rarity: String },
    Redirect(String),
    Disabled,
}

#[allow(dead_code)]
pub(super) fn source_material_definition(
    value: &serde_json::Value,
    registries: &std::collections::HashMap<String, std::collections::HashMap<String, std::collections::HashSet<String>>>,
    config: &std::collections::HashMap<String, bool>,
    complete: bool,
) -> Result<SourceMaterialDefinition, SourceLookupError> {
    if !complete { return Err(SourceLookupError::IncompleteContext); }
    let object = value.as_object().ok_or(SourceLookupError::UnsupportedRecipe)?;
    let condition = |value: &serde_json::Value| -> Result<Option<bool>, SourceLookupError> {
        if value.is_null() { return Ok(Some(true)); }
        let parsed = SourceCondition::parse(value).ok_or(SourceLookupError::UnsupportedRecipe)?;
        Ok(parsed.matches(registries, config))
    };
    if let Some(redirects) = object.get("redirect").filter(|v| !v.is_null()) {
        for redirect in redirects.as_array().ok_or(SourceLookupError::UnsupportedRecipe)? {
            let redirect = redirect.as_object().ok_or(SourceLookupError::UnsupportedRecipe)?;
            let target = redirect.get("id").and_then(serde_json::Value::as_str).ok_or(SourceLookupError::UnsupportedRecipe)?;
            match redirect.get("condition").map_or(Ok(Some(true)), &condition)? {
                Some(true) => return Ok(SourceMaterialDefinition::Redirect(target.to_owned())),
                Some(false) => {},
                None => return Err(SourceLookupError::UnknownCondition),
            }
        }
    }
    match object.get("condition").map_or(Ok(Some(true)), &condition)? {
        Some(false) => return Ok(SourceMaterialDefinition::Disabled),
        Some(true) => {},
        None => return Err(SourceLookupError::UnknownCondition),
    }
    let craftable = object.get("craftable").and_then(serde_json::Value::as_bool).ok_or(SourceLookupError::UnsupportedRecipe)?;
    let hidden = match object.get("hidden") { None | Some(serde_json::Value::Null) => false, Some(v) => v.as_bool().ok_or(SourceLookupError::UnsupportedRecipe)? };
    let integer = |key: &str, default| -> Result<i32, SourceLookupError> {
        match object.get(key) { None | Some(serde_json::Value::Null) => Ok(default), Some(v) => v.as_i64().and_then(|n| i32::try_from(n).ok()).ok_or(SourceLookupError::UnsupportedRecipe) }
    };
    let tier = integer("tier", 0)?;
    let sort_order = integer("sortOrder", 100)?;
    let rarity = match object.get("rarity") {
        None | Some(serde_json::Value::Null) => if tier >= 5 { "epic" } else if tier == 4 { "rare" } else if tier == 3 { "uncommon" } else { "common" },
        Some(_) => return Err(SourceLookupError::UnsupportedRecipe),
    };
    Ok(SourceMaterialDefinition::Material { craftable, hidden, tier, sort_order, rarity: rarity.into() })
}

/// Assemble definitions from one world's ordered source-pack snapshot. Until
/// all Forge conditions and source JSON error handling are ported, unsupported
/// input fails the whole snapshot instead of fabricating a partial registry.
#[allow(dead_code)]
pub(super) fn source_material_registry(
    files: &std::collections::BTreeMap<String, Vec<Vec<u8>>>,
    registries: &std::collections::HashMap<String, std::collections::HashMap<String, std::collections::HashSet<String>>>,
    config: &std::collections::HashMap<String, bool>,
    complete: bool,
) -> Result<(std::collections::HashMap<String, SourceMaterialDefinition>, std::collections::HashMap<String, String>), SourceLookupError> {
    if !complete { return Err(SourceLookupError::IncompleteContext); }
    let mut materials = std::collections::HashMap::new();
    let mut redirects = std::collections::HashMap::new();
    for (key, layers) in files {
        let Some((namespace, path)) = key.split_once(':') else { return Err(SourceLookupError::UnsupportedRecipe); };
        let Some(path) = path.strip_prefix("tinkering/materials/definition/") else { continue; };
        let bytes = layers.last().ok_or(SourceLookupError::UnsupportedRecipe)?;
        let value = serde_json::from_slice(bytes).map_err(|_| SourceLookupError::UnsupportedRecipe)?;
        let id = format!("{namespace}:{path}");
        match source_material_definition(&value, registries, config, true)? {
            definition @ SourceMaterialDefinition::Material { .. } => { materials.insert(id, definition); },
            SourceMaterialDefinition::Redirect(target) => { redirects.insert(id, target); },
            SourceMaterialDefinition::Disabled => {},
        }
    }
    redirects.retain(|_, target| materials.contains_key(target));
    Ok((materials, redirects))
}

/// Source MaterialStatsManager merges stat objects by stat ID and field,
/// while a null stat removes that entry. Invalid layers fail closed until the
/// original loader's error/partial-update behavior is ported in full.
#[allow(dead_code)]
pub(super) fn source_merge_material_stats(files: &[Vec<u8>]) -> Result<std::collections::BTreeMap<String, serde_json::Map<String, serde_json::Value>>, SourceLookupError> {
    let mut stats: std::collections::BTreeMap<String, serde_json::Map<String, serde_json::Value>> = std::collections::BTreeMap::new();
    for file in files {
        let value: serde_json::Value = serde_json::from_slice(file).map_err(|_| SourceLookupError::UnsupportedRecipe)?;
        let object = value.as_object().ok_or(SourceLookupError::UnsupportedRecipe)?;
        let Some(raw) = object.get("stats").filter(|v| !v.is_null()) else { continue; };
        for (id, value) in raw.as_object().ok_or(SourceLookupError::UnsupportedRecipe)? {
            if value.is_null() { stats.remove(id); continue; }
            let fields = value.as_object().ok_or(SourceLookupError::UnsupportedRecipe)?;
            stats.entry(id.clone()).or_default().extend(fields.clone());
        }
    }
    Ok(stats)
}

/// Preserve the source trait loader's layer operations. Modifier entries and
/// per-stat fallback rules are not yet deserialized or connected to gameplay.
#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub(super) struct SourceRawTraits {
    pub default: Option<Vec<serde_json::Value>>,
    pub per_stat: std::collections::BTreeMap<String, Vec<serde_json::Value>>,
}
#[allow(dead_code)]
pub(super) fn source_merge_material_traits(files: &[Vec<u8>]) -> Result<SourceRawTraits, SourceLookupError> {
    let mut result = SourceRawTraits { default: None, per_stat: std::collections::BTreeMap::new() };
    for file in files {
        let value: serde_json::Value = serde_json::from_slice(file).map_err(|_| SourceLookupError::UnsupportedRecipe)?;
        let object = value.as_object().ok_or(SourceLookupError::UnsupportedRecipe)?;
        if let Some(default) = object.get("default").filter(|v| !v.is_null()) {
            result.default = Some(default.as_array().ok_or(SourceLookupError::UnsupportedRecipe)?.clone());
        }
        if let Some(per_stat) = object.get("perStat").filter(|v| !v.is_null()) {
            for (id, entries) in per_stat.as_object().ok_or(SourceLookupError::UnsupportedRecipe)? {
                if entries.is_null() { result.per_stat.remove(id); }
                else { result.per_stat.insert(id.clone(), entries.as_array().ok_or(SourceLookupError::UnsupportedRecipe)?.clone()); }
            }
        }
    }
    Ok(result)
}

/// Source material lookup over a complete, ordered recipe snapshot. This is
/// deliberately not used by gameplay until world registries, config, recipe
/// provenance and all ingredient forms can be supplied without approximation.
#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq)]
pub(super) enum SourceLookupError { IncompleteContext, UnsupportedOrder, UnsupportedRecipe, UnknownCondition }

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub(super) struct SourceMaterialMatch {
    pub id: String,
    pub material: String,
    pub value: i32,
    pub needed: i32,
    pub leftover: Option<SourceItemOutput>,
}

#[allow(dead_code)]
pub(super) fn source_material_lookup(
    recipes: &std::collections::BTreeMap<String, Vec<u8>>,
    item: &str,
    instrument: Option<&str>,
    item_tags: &std::collections::HashMap<String, std::collections::HashSet<String>>,
    instruments: &std::collections::HashSet<String>,
    instrument_tags: &std::collections::HashMap<String, std::collections::HashSet<String>>,
    registries: &std::collections::HashMap<String, std::collections::HashMap<String, std::collections::HashSet<String>>>,
    config: &std::collections::HashMap<String, bool>,
    complete: bool,
) -> Result<Option<SourceMaterialMatch>, SourceLookupError> {
    if !complete { return Err(SourceLookupError::IncompleteContext); }
    let parsed: std::collections::BTreeMap<String, serde_json::Value> = recipes.iter()
        .filter_map(|(id, bytes)| serde_json::from_slice(bytes).ok().map(|value| (id.clone(), value)))
        .collect();
    let ids = parsed.keys().cloned().collect::<Vec<_>>();
    let order = super::tconstruct_recipe_order::java_recipe_order(&ids).map_err(|_| SourceLookupError::UnsupportedOrder)?;
    for id in order {
        if id.split_once(':').is_some_and(|(_, path)| path.starts_with('_')) { continue; }
        let value = &parsed[&id];
        if value.get("type").and_then(serde_json::Value::as_str) != Some("tconstruct:material") { continue; }
        let recipe = SourceMaterialRecipe::parse(value).ok_or(SourceLookupError::UnsupportedRecipe)?;
        if !recipe.ingredient.matches(item, instrument, item_tags, instruments, instrument_tags) { continue; }
        match recipe.enabled(registries, config) {
            Some(true) => return Ok(Some(SourceMaterialMatch { id, material: recipe.material, value: recipe.value, needed: recipe.needed, leftover: recipe.leftover })),
            Some(false) => continue,
            None => return Err(SourceLookupError::UnknownCondition),
        }
    }
    Ok(None)
}

/// Read the original ToolPartsHook from the highest-priority definition.
/// The caller must have fully decoded the definition and validated every
/// module, hook and item against the actual world registries first. This
/// helper does not substitute a list of guessed module implementations.
#[allow(dead_code)]
pub(super) fn source_tool_parts_hook(
    layers: &[Vec<u8>], definition_validated: bool,
    module_defaults: &std::collections::HashMap<String, bool>,
    registered_parts: &std::collections::HashSet<String>,
) -> Result<Vec<String>, SourceLookupError> {
    if !definition_validated { return Err(SourceLookupError::IncompleteContext); }
    let bytes = layers.last().ok_or(SourceLookupError::IncompleteContext)?;
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| SourceLookupError::UnsupportedRecipe)?;
    let modules = match value.get("modules") {
        None => return Ok(Vec::new()),
        Some(value) => value.as_array().ok_or(SourceLookupError::UnsupportedRecipe)?,
    };
    let mut result = None;
    for module in modules {
        let object = module.as_object().ok_or(SourceLookupError::UnsupportedRecipe)?;
        let typ = object.get("type").and_then(serde_json::Value::as_str).ok_or(SourceLookupError::UnsupportedRecipe)?;
        let default = *module_defaults.get(typ).ok_or(SourceLookupError::IncompleteContext)?;
        let hooks = match object.get("hooks") {
            None => Vec::new(),
            Some(value) => value.as_array().ok_or(SourceLookupError::UnsupportedRecipe)?.iter()
                .map(|hook| hook.as_str().ok_or(SourceLookupError::UnsupportedRecipe)).collect::<Result<Vec<_>,_>>()?,
        };
        let supplies_parts = if hooks.is_empty() { default } else { hooks.contains(&"tconstruct:tool_parts") };
        if !supplies_parts { continue; }
        if !matches!(typ, "tconstruct:part_stats" | "tconstruct:tool_parts") { return Err(SourceLookupError::IncompleteContext); }
        let parts = object.get("parts").and_then(serde_json::Value::as_array).ok_or(SourceLookupError::UnsupportedRecipe)?;
        if parts.is_empty() { return Err(SourceLookupError::UnsupportedRecipe); }
        let mut resolved = Vec::new();
        for part in parts {
            let name = part.as_str().or_else(|| part.get("item").and_then(serde_json::Value::as_str)).ok_or(SourceLookupError::UnsupportedRecipe)?;
            let id = super::tconstruct_source_render_info::material_id(name).ok_or(SourceLookupError::UnsupportedRecipe)?;
            if id.contains('#') { return Err(SourceLookupError::UnsupportedRecipe); }
            if !registered_parts.contains(&id) { return Err(SourceLookupError::IncompleteContext); }
            resolved.push(id);
        }
        if let Some((previous_type, previous_parts)) = &result {
            // PartsModule is a Java record: LinkedHashMultimap collapses
            // identical modules, whereas PartStatsModule uses identity.
            if typ == "tconstruct:tool_parts" && previous_type == typ && previous_parts == &resolved { continue; }
            return Err(SourceLookupError::UnsupportedRecipe);
        }
        result = Some((typ.to_owned(), resolved));
    }
    Ok(result.map(|(_, parts)| parts).unwrap_or_default())
}

/// Original PartBuilderRecycle, which can expose several patterns from one
/// input. Random leftover choice and world item/ingredient validation are
/// separate dependencies and are not connected to gameplay.
#[allow(dead_code)]
#[derive(Debug)]
pub(super) struct SourceRecyclingRecipe {
    pub id: String,
    tool: SourceIngredient,
    pattern_item: SourceIngredient,
    pub results: std::collections::BTreeMap<String, SourceItemOutput>,
    conditions: Vec<SourceRecyclingCondition>,
}

#[allow(dead_code)]
#[derive(Debug)]
enum SourceRecyclingCondition { ModLoaded(String), Other(SourceCondition) }

#[allow(dead_code)]
impl SourceRecyclingRecipe {
    pub(super) fn parse(id: String, value: &serde_json::Value) -> Option<Self> {
        if value.get("type")?.as_str()? != "tconstruct:part_builder_recycling" { return None; }
        let tool = SourceIngredient::parse(value.get("tool")?)?;
        let pattern_item = SourceIngredient::parse(value.get("pattern")?)?;
        let mut results = std::collections::BTreeMap::new();
        for (pattern, output) in value.get("results")?.as_object()? {
            let pattern = super::tconstruct_source_render_info::material_id(pattern)?;
            if pattern.contains('#') || results.insert(pattern, SourceItemOutput::parse(output)?).is_some() { return None; }
        }
        if results.is_empty() { return None; }
        let conditions = match value.get("conditions") {
            None => Vec::new(),
            Some(conditions) => conditions.as_array()?.iter().map(|condition| {
                if condition.get("type").and_then(serde_json::Value::as_str) == Some("forge:mod_loaded") {
                    Some(SourceRecyclingCondition::ModLoaded(condition.get("modid")?.as_str()?.to_owned()))
                } else { Some(SourceRecyclingCondition::Other(SourceCondition::parse(condition)?)) }
            }).collect::<Option<Vec<_>>>()?,
        };
        Some(Self { id, tool, pattern_item, results, conditions })
    }

    pub(super) fn enabled(
        &self,
        registries: &std::collections::HashMap<String, std::collections::HashMap<String, std::collections::HashSet<String>>>,
        config: &std::collections::HashMap<String, bool>,
        loaded_mods: Option<&std::collections::HashSet<String>>,
    ) -> Option<bool> {
        let mut unknown = false;
        for condition in &self.conditions {
            let state = match condition {
                SourceRecyclingCondition::ModLoaded(id) => loaded_mods.map(|mods| mods.contains(id)),
                SourceRecyclingCondition::Other(condition) => condition.matches(registries, config),
            };
            match state { Some(false) => return Some(false), Some(true) => {}, None => unknown = true }
        }
        if unknown { None } else { Some(true) }
    }

    pub(super) fn partial_match(
        &self, pattern_item: &str, tool_item: &str,
        tags: &std::collections::HashMap<String, std::collections::HashSet<String>>,
        instruments: &std::collections::HashSet<String>,
        instrument_tags: &std::collections::HashMap<String, std::collections::HashSet<String>>,
    ) -> bool {
        self.pattern_item.matches(pattern_item, None, tags, instruments, instrument_tags)
            && self.tool.matches(tool_item, None, tags, instruments, instrument_tags)
    }

    pub(super) fn maximum_amount(&self, max_damage: i32, damage: i32) -> Result<i32, SourceLookupError> {
        if max_damage < 0 || damage < 0 { return Err(SourceLookupError::UnsupportedRecipe); }
        let maximum = self.results.values().fold(0i32, |sum, result| sum.wrapping_add(result.count));
        if max_damage == 0 { Ok(maximum) }
        else { Ok(maximum.wrapping_mul(max_damage.wrapping_sub(damage)) / max_damage) }
    }

    /// Preserve the original Java conditional expression, including its
    /// precedence: the else branch checks enchantment independently.
    pub(super) fn matches(&self, partial: bool, max_damage: i32, damage: i32, modifiable: bool, upgrades: bool, enchanted: bool) -> Result<bool, SourceLookupError> {
        Ok(if partial && self.maximum_amount(max_damage, damage)? > 0 && modifiable { !upgrades } else { !enchanted })
    }

    pub(super) fn output(&self, pattern: &str, max_damage: i32, damage: i32) -> Result<Option<SourceItemOutput>, SourceLookupError> {
        let Some(output) = self.results.get(pattern) else { return Ok(None); };
        let count = self.maximum_amount(max_damage, damage)?.min(output.count);
        if count <= 0 { return Ok(None); }
        let mut result = output.clone(); result.count = count;
        Ok(Some(result))
    }
}

/// Source tool-recycling recipe metadata. Material-bearing part resolution is
/// intentionally supplied by the caller from the actual tool definition.
#[allow(dead_code)]
#[derive(Debug)]
pub(super) struct SourceToolRecyclingRecipe {
    pub id: String,
    tool: SourceIngredient,
    pattern_item: SourceIngredient,
    pub amount_needed: i32,
    pub parts: Vec<String>,
    conditions: Vec<SourceRecyclingCondition>,
}

#[allow(dead_code)]
impl SourceToolRecyclingRecipe {
    pub(super) fn parse(id: String, value: &serde_json::Value) -> Option<Self> {
        if value.get("type")?.as_str()? != "tconstruct:part_builder_tool_recycling" { return None; }
        let default_tool = serde_json::json!({"tag":"tconstruct:modifiable/multipart"});
        let tool_value = value.get("tools").unwrap_or(&default_tool);
        let tool = SourceIngredient::parse(tool_value.get("ingredient").unwrap_or(tool_value))?;
        let amount_needed = match tool_value.get("amount_needed") {
            None => 1,
            Some(amount) => i32::try_from(amount.as_i64()?).ok()?,
        };
        if amount_needed < 1 { return None; }
        let parts = match value.get("parts") {
            None => Vec::new(),
            Some(parts) => parts.as_array()?.iter().map(|part| {
                let id = super::tconstruct_source_render_info::material_id(part.as_str()?)?;
                if id.contains('#') { None } else { Some(id) }
            }).collect::<Option<Vec<_>>>()?,
        };
        let conditions = match value.get("conditions") {
            None => Vec::new(),
            Some(conditions) => conditions.as_array()?.iter().map(|condition| {
                if condition.get("type").and_then(serde_json::Value::as_str) == Some("forge:mod_loaded") {
                    Some(SourceRecyclingCondition::ModLoaded(condition.get("modid")?.as_str()?.to_owned()))
                } else { Some(SourceRecyclingCondition::Other(SourceCondition::parse(condition)?)) }
            }).collect::<Option<Vec<_>>>()?,
        };
        Some(Self { id, tool, pattern_item: SourceIngredient::parse(value.get("pattern")?)?, amount_needed, parts, conditions })
    }

    pub(super) fn partial_match(
        &self, pattern_item: &str, tool_item: &str, count: i32,
        tags: &std::collections::HashMap<String, std::collections::HashSet<String>>,
        instruments: &std::collections::HashSet<String>,
        instrument_tags: &std::collections::HashMap<String, std::collections::HashSet<String>>,
    ) -> bool {
        count >= self.amount_needed
            && self.pattern_item.matches(pattern_item, None, tags, instruments, instrument_tags)
            && self.tool.matches(tool_item, None, tags, instruments, instrument_tags)
    }

    pub(super) fn enabled(
        &self,
        registries: &std::collections::HashMap<String, std::collections::HashMap<String, std::collections::HashSet<String>>>,
        config: &std::collections::HashMap<String, bool>,
        loaded_mods: Option<&std::collections::HashSet<String>>,
    ) -> Option<bool> {
        let mut unknown = false;
        for condition in &self.conditions {
            let state = match condition {
                SourceRecyclingCondition::ModLoaded(id) => loaded_mods.map(|mods| mods.contains(id)),
                SourceRecyclingCondition::Other(condition) => condition.matches(registries, config),
            };
            match state { Some(false) => return Some(false), Some(true) => {}, None => unknown = true }
        }
        if unknown { None } else { Some(true) }
    }
}

/// Enumerate the material-supported patterns for the original tool recycler.
/// `materials` must already be decoded as original MaterialVariantIds and
/// `supported` must come from the validated world material-stat registry.
#[allow(dead_code)]
pub(super) fn source_tool_recycling_patterns(
    recipe: &SourceToolRecyclingRecipe, tool_parts: Option<&[String]>, materials: &[String],
    supported: &std::collections::HashMap<String, std::collections::HashSet<String>>,
    complete: bool,
) -> Result<Vec<String>, SourceLookupError> {
    if !complete { return Err(SourceLookupError::IncompleteContext); }
    let parts = if recipe.parts.is_empty() { tool_parts.ok_or(SourceLookupError::IncompleteContext)? } else { &recipe.parts };
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for (index, part) in parts.iter().enumerate() {
        if part == "minecraft:air" { continue; }
        let material = materials.get(index).map(String::as_str).unwrap_or("tconstruct:unknown");
        let normalized = super::tconstruct_source_render_info::material_id(material).ok_or(SourceLookupError::IncompleteContext)?;
        let base = normalized.split_once('#').map_or(normalized.as_str(), |(base, _)| base);
        let allowed = supported.get(base).ok_or(SourceLookupError::IncompleteContext)?;
        if allowed.contains(part) && seen.insert(part.clone()) { result.push(part.clone()); }
    }
    Ok(result)
}

/// Parsed source PartRecipe and ItemPartRecipe. Item registry validation,
/// material stat support and result resolution are supplied by the caller.
#[allow(dead_code)]
#[derive(Debug)]
pub(super) enum SourcePartKind {
    MaterialItem { item: String, count: i32, allow_uncraftable: bool },
    Item { material: String, output: SourceItemOutput },
}
#[allow(dead_code)]
#[derive(Debug)]
pub(super) struct SourcePartRecipe {
    pub id: String,
    pub pattern: String,
    pattern_item: SourceIngredient,
    pub cost: i32,
    pub kind: SourcePartKind,
    conditions: Vec<SourceCondition>,
}
#[allow(dead_code)]
impl SourcePartRecipe {
    pub(super) fn parse(id: String, value: &serde_json::Value) -> Option<Self> {
        let object = value.as_object()?;
        let typ = object.get("type")?.as_str()?;
        let pattern = object.get("pattern")?.as_str()?.to_owned();
        let pattern_item = match object.get("pattern_item") {
            Some(value) => SourceIngredient::parse(value)?,
            None => SourceIngredient::Tag("tconstruct:patterns/default".into()),
        };
        let cost = match object.get("cost") { Some(value) => i32::try_from(value.as_i64()?).ok()?, None if typ == "tconstruct:item_part_builder" => 0, None => return None };
        let conditions = match object.get("conditions") {
            Some(value) => value.as_array()?.iter().map(SourceCondition::parse).collect::<Option<_>>()?,
            None => Vec::new(),
        };
        let kind = match typ {
            "tconstruct:part_builder" => {
                if cost < 1 { return None; }
                let output = object.get("result")?.as_object()?;
                let item = output.get("item")?.as_str()?.to_owned();
                let count = match output.get("count") { Some(value) => i32::try_from(value.as_i64()?).ok()?, None => 1 };
                if count < 1 { return None; }
                let allow_uncraftable = match object.get("allow_uncraftable") { Some(value) => value.as_bool()?, None => false };
                SourcePartKind::MaterialItem { item, count, allow_uncraftable }
            },
            "tconstruct:item_part_builder" => {
                if cost < 0 { return None; }
                let material = match object.get("material") { Some(value) => value.as_str()?.to_owned(), None => "tconstruct:unknown".into() };
                if cost == 0 && material != "tconstruct:unknown" { return None; }
                SourcePartKind::Item { material, output: SourceItemOutput::parse(object.get("result")?)? }
            },
            _ => return None,
        };
        Some(Self { id, pattern, pattern_item, cost, kind, conditions })
    }
}

#[allow(dead_code)]
pub(super) enum SourcePartInput<'a> {
    Empty,
    Invalid,
    Material { input_item: &'a str, material: &'a str, craftable: bool, supported_outputs: &'a std::collections::HashSet<String> },
}

/// Map the pinned ToolPartItem and RepairKitItem registrations to already
/// validated material stat IDs. Fake ingot and block items are not covered.
/// The caller must deserialize the registry first; merely seeing a
/// raw JSON stat key is not enough to establish that the original accepts it.
/// `has_repairable_stat` must likewise come from the registered stat types.
#[allow(dead_code)]
pub(super) fn source_supported_original_parts(
    valid_stats: &std::collections::HashSet<String>,
    has_repairable_stat: bool,
) -> std::collections::HashSet<String> {
    let mut result = std::collections::HashSet::new();
    for (stat, parts) in [
        ("head", &["pick_head", "hammer_head", "small_axe_head", "broad_axe_head", "small_blade", "broad_blade", "adze_head", "large_plate"][..]),
        ("limb", &["bow_limb"][..]), ("grip", &["bow_grip"][..]),
        ("bowstring", &["bowstring"][..]), ("arrow_head", &["arrow_head"][..]),
        ("arrow_shaft", &["arrow_shaft"][..]), ("fletching", &["fletching"][..]),
        ("binding", &["tool_binding", "tough_binding"][..]),
        ("handle", &["tool_handle", "tough_handle"][..]),
        ("plating_helmet", &["helmet_plating"][..]),
        ("plating_chestplate", &["chestplate_plating"][..]),
        ("plating_leggings", &["leggings_plating"][..]),
        ("plating_boots", &["boots_plating"][..]),
        ("maille", &["maille"][..]), ("shield_core", &["shield_core"][..]),
        ("ribcage", &["ribcage"][..]), ("shell", &["shell"][..]),
        ("laces", &["laces"][..]),
    ] {
        if valid_stats.contains(&format!("tconstruct:{stat}")) {
            result.extend(parts.iter().map(|part| format!("tconstruct:{part}")));
        }
    }
    if has_repairable_stat || valid_stats.contains("tconstruct:repair_kit") {
        result.insert("tconstruct:repair_kit".into());
    }
    result
}

/// FakeIngotItem.hasItem examines the visible material-recipe cache for the
/// base MaterialId (not its variants), dividing Java float values. The caller
/// must resolve the material tag and provide precisely that filtered cache.
#[allow(dead_code)]
pub(super) fn source_fake_material_allowed(
    in_material_tag: bool,
    visible_base_recipes: &[(i32, i32)],
    repair_amount: f32,
) -> bool {
    in_material_tag && !visible_base_recipes.iter().any(|&(value, needed)| {
        value as f32 / needed as f32 == repair_amount
    })
}

/// Values in `MaterialRecipeCache.getRecipes(baseMaterial)`: only recipes
/// for the exact base variant whose resolved material is visible are relevant.
/// Registry visibility and Forge condition contexts must be complete.
#[allow(dead_code)]
pub(super) fn source_visible_base_recipe_values(
    material: &str,
    recipes: &std::collections::BTreeMap<String, Vec<u8>>,
    visible_materials: &std::collections::HashSet<String>,
    known_items: &std::collections::HashSet<String>,
    registries: &std::collections::HashMap<String, std::collections::HashMap<String, std::collections::HashSet<String>>>,
    config: &std::collections::HashMap<String, bool>,
    complete: bool,
) -> Result<Vec<(i32, i32)>, SourceLookupError> {
    if !complete { return Err(SourceLookupError::IncompleteContext); }
    let material = super::tconstruct_source_render_info::material_id(material).ok_or(SourceLookupError::UnsupportedRecipe)?;
    if material.contains('#') { return Err(SourceLookupError::UnsupportedRecipe); }
    if !visible_materials.contains(&material) { return Ok(Vec::new()); }
    fn valid_ingredient(ingredient: &SourceIngredient, known_items: &std::collections::HashSet<String>) -> bool {
        let id = |id: &str| super::tconstruct_source_render_info::material_id(id).filter(|id| !id.contains('#'));
        match ingredient {
            SourceIngredient::Item(item) => id(item).is_some_and(|item| known_items.contains(&item)),
            SourceIngredient::Tag(tag) => id(tag).is_some(),
            SourceIngredient::Any(values) => values.iter().all(|value| valid_ingredient(value, known_items)),
            SourceIngredient::Difference(base, subtracted) => valid_ingredient(base, known_items) && valid_ingredient(subtracted, known_items),
            // Instrument registry validation is not yet implemented here.
            SourceIngredient::Instrument { .. } => false,
        }
    }
    let mut result = Vec::new();
    for (id, bytes) in recipes {
        if id.split_once(':').is_some_and(|(_, path)| path.starts_with('_')) { continue; }
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else { continue; };
        if value.get("type").and_then(serde_json::Value::as_str) != Some("tconstruct:material") { continue; }
        let Some(identity) = value.get("material").and_then(serde_json::Value::as_str)
            .and_then(super::tconstruct_source_render_info::material_id) else { continue; };
        if identity != material { continue; }
        // Forge checks conditions before attempting to deserialize a recipe.
        if let Some(conditions) = value.get("conditions") {
            let conditions = conditions.as_array().ok_or(SourceLookupError::UnsupportedRecipe)?;
            let mut enabled = true;
            for condition in conditions {
                match SourceCondition::parse(condition).ok_or(SourceLookupError::UnsupportedRecipe)?.matches(registries, config) {
                    Some(false) => { enabled = false; break; },
                    Some(true) => {},
                    None => return Err(SourceLookupError::UnknownCondition),
                }
            }
            if !enabled { continue; }
        }
        // Validate the output before relying on this recipe's constructor.
        // NBT and explicit-empty output forms remain unsupported here.
        if let Some(output) = value.get("leftover").filter(|output| !output.is_null()) {
            let output = SourceItemOutput::parse(output).ok_or(SourceLookupError::UnsupportedRecipe)?;
            if output.nbt.is_some() { return Err(SourceLookupError::UnsupportedRecipe); }
            let valid = |id: &str| super::tconstruct_source_render_info::material_id(id).filter(|id| !id.contains('#'));
            match &output.kind {
                SourceOutputKind::Item(item) if !valid(item).is_some_and(|item| known_items.contains(&item)) => return Err(SourceLookupError::IncompleteContext),
                SourceOutputKind::Tag(tag) if valid(tag).is_none() => return Err(SourceLookupError::UnsupportedRecipe),
                _ => {},
            }
        }
        let recipe = SourceMaterialRecipe::parse(&value).ok_or(SourceLookupError::UnsupportedRecipe)?;
        if !valid_ingredient(&recipe.ingredient, known_items) { return Err(SourceLookupError::IncompleteContext); }
        result.push((recipe.value, recipe.needed));
    }
    Ok(result)
}

/// `TinkerToolParts` registers fake ingot with repair amount 1 and fake
/// storage block with repair amount 9. The caller supplies the world's
/// resolved material tags and the already-filtered, visible base recipes.
#[allow(dead_code)]
pub(super) fn source_supported_fake_items(
    material: &str,
    material_tags: &std::collections::HashMap<String, std::collections::HashSet<String>>,
    visible_base_recipes: &[(i32, i32)],
    complete: bool,
) -> Result<std::collections::HashSet<String>, SourceLookupError> {
    if !complete { return Err(SourceLookupError::IncompleteContext); }
    let material = super::tconstruct_source_render_info::material_id(material).ok_or(SourceLookupError::UnsupportedRecipe)?;
    if material.contains('#') || visible_base_recipes.iter().any(|(value, needed)| *value <= 0 || *needed <= 0) {
        return Err(SourceLookupError::UnsupportedRecipe);
    }
    let mut supported = std::collections::HashSet::new();
    for (tag, item, amount) in [
        ("tconstruct:compatibility_metals", "tconstruct:fake_ingot", 1.0),
        ("tconstruct:compatibility_metals/blocks", "tconstruct:fake_storage_block", 9.0),
    ] {
        let in_tag = material_tags.get(tag).is_some_and(|values| values.contains(&material));
        if source_fake_material_allowed(in_tag, visible_base_recipes, amount) {
            supported.insert(item.to_owned());
        }
    }
    Ok(supported)
}

/// MaterialCastingRecipe registers each result's smallest item_cost during
/// recipe loading. Caller must provide a complete registry of IMaterialItems;
/// unsupported source predicate forms fail closed instead of being ignored.
#[allow(dead_code)]
pub(super) fn source_material_casting_costs(
    recipes: &std::collections::BTreeMap<String, Vec<u8>>,
    material_items: &std::collections::HashSet<String>,
    registries: &std::collections::HashMap<String, std::collections::HashMap<String, std::collections::HashSet<String>>>,
    config: &std::collections::HashMap<String, bool>,
    complete: bool,
) -> Result<std::collections::HashMap<String, i32>, SourceLookupError> {
    if !complete { return Err(SourceLookupError::IncompleteContext); }
    let mut costs = std::collections::HashMap::new();
    for (id, bytes) in recipes {
        if id.split_once(':').is_some_and(|(_, path)| path.starts_with('_')) { continue; }
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else { continue; };
        if !matches!(value.get("type").and_then(serde_json::Value::as_str), Some("tconstruct:table_casting_material" | "tconstruct:basin_casting_material")) { continue; }
        let conditions = match value.get("conditions") {
            None => Vec::new(),
            Some(v) => v.as_array().ok_or(SourceLookupError::UnsupportedRecipe)?.iter()
                .map(SourceCondition::parse).collect::<Option<Vec<_>>>().ok_or(SourceLookupError::UnsupportedRecipe)?,
        };
        let mut enabled = true;
        for condition in conditions {
            match condition.matches(registries, config) {
                Some(false) => { enabled = false; break; },
                Some(true) => {},
                None => return Err(SourceLookupError::UnknownCondition),
            }
        }
        if !enabled { continue; }
        if value.get("allowed_materials").is_some_and(|v| !v.is_null()) { return Err(SourceLookupError::UnsupportedRecipe); }
        if let Some(cast) = value.get("cast").filter(|v| !v.is_null()) {
            let object = cast.as_object().ok_or(SourceLookupError::UnsupportedRecipe)?;
            let tag = object.get("tag").and_then(serde_json::Value::as_str).ok_or(SourceLookupError::UnsupportedRecipe)?;
            let (namespace, path) = tag.split_once(':').unwrap_or(("minecraft", tag));
            if object.len() != 1 || namespace.is_empty() || path.is_empty()
                || !namespace.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_.-".contains(&c))
                || !path.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_./-".contains(&c)) {
                return Err(SourceLookupError::UnsupportedRecipe);
            }
        }
        if value.get("group").is_some_and(|v| !v.is_null() && !v.is_string()) { return Err(SourceLookupError::UnsupportedRecipe); }
        for name in ["cast_consumed", "switch_slots"] {
            if value.get(name).is_some_and(|v| !v.is_null() && !v.is_boolean()) { return Err(SourceLookupError::UnsupportedRecipe); }
        }
        let item = value.get("result").and_then(serde_json::Value::as_str).ok_or(SourceLookupError::UnsupportedRecipe)?;
        if !material_items.contains(item) { return Err(SourceLookupError::IncompleteContext); }
        let cost = value.get("item_cost").and_then(serde_json::Value::as_i64)
            .and_then(|n| i32::try_from(n).ok()).filter(|n| *n > 0).ok_or(SourceLookupError::UnsupportedRecipe)?;
        costs.entry(item.to_owned()).and_modify(|previous: &mut i32| *previous = (*previous).min(cost)).or_insert(cost);
    }
    Ok(costs)
}
/// `PartBuilderContainerWrapper.getMaterial` diverts `tconstruct:parts` tagged items away
/// from ordinary material recipes. The caller must resolve tags, the item
/// interface registry and casting costs for the same world/reload snapshot.
#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq)]
pub(super) enum SourceReusedPart {
    NotToolPart,
    NoMaterial,
    Material { material: String, value: i32, needed: i32 },
}

#[allow(dead_code)]
pub(super) fn source_reused_part_material(
    in_tool_parts_tag: bool,
    is_material_item: bool,
    material_tag: Option<&str>,
    casting_cost: Option<i32>,
    complete: bool,
) -> Result<SourceReusedPart, SourceLookupError> {
    if !complete { return Err(SourceLookupError::IncompleteContext); }
    if !in_tool_parts_tag { return Ok(SourceReusedPart::NotToolPart); }
    if !is_material_item { return Ok(SourceReusedPart::NoMaterial); }
    let Some(material) = material_tag.and_then(super::tconstruct_source_render_info::material_id) else { return Ok(SourceReusedPart::NoMaterial); };
    let base = material.split_once('#').map_or(material.as_str(), |(base, _)| base);
    let Some(value) = casting_cost.filter(|cost| *cost > 0) else { return Ok(SourceReusedPart::NoMaterial); };
    if base == "tconstruct:unknown" { return Ok(SourceReusedPart::NoMaterial); }
    Ok(SourceReusedPart::Material { material, value, needed: 1 })
}

fn source_variant_matches(expected: &str, actual: &str) -> bool {
    let (base, variant) = expected.split_once('#').unwrap_or((expected, ""));
    let (actual_base, actual_variant) = actual.split_once('#').unwrap_or((actual, ""));
    base == actual_base && (variant.is_empty() || variant == actual_variant || variant == "default" && actual_variant.is_empty())
}
fn source_id_order(id: &str) -> (&str, &str) {
    let (namespace, path) = id.split_once(':').unwrap_or(("minecraft", id));
    (path, namespace)
}

/// Ordinary part recipe candidates only; the complete source candidate set
/// also includes recycling and is assembled by `source_all_part_choices`.
/// World tag/config/stat contexts must be complete before use.
#[allow(dead_code)]
pub(super) fn source_part_choices(
    recipes: &std::collections::BTreeMap<String, Vec<u8>>,
    pattern_item: &str,
    input: &SourcePartInput<'_>,
    item_tags: &std::collections::HashMap<String, std::collections::HashSet<String>>,
    instruments: &std::collections::HashSet<String>,
    instrument_tags: &std::collections::HashMap<String, std::collections::HashSet<String>>,
    registries: &std::collections::HashMap<String, std::collections::HashMap<String, std::collections::HashSet<String>>>,
    config: &std::collections::HashMap<String, bool>,
    complete: bool,
) -> Result<Vec<SourcePartRecipe>, SourceLookupError> {
    if !complete { return Err(SourceLookupError::IncompleteContext); }
    if pattern_item.is_empty() { return Ok(Vec::new()); }
    let mut candidates = Vec::new();
    for (id, bytes) in recipes {
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else { continue; };
        if id.split_once(':').is_some_and(|(_, path)| path.starts_with('_')) { continue; }
        if !matches!(value.get("type").and_then(serde_json::Value::as_str), Some("tconstruct:part_builder" | "tconstruct:item_part_builder")) { continue; }
        let recipe = SourcePartRecipe::parse(id.clone(), &value).ok_or(SourceLookupError::UnsupportedRecipe)?;
        if !recipe.pattern_item.matches(pattern_item, None, item_tags, instruments, instrument_tags) { continue; }
        let matches = match (&recipe.kind, input) {
            (_, SourcePartInput::Empty) => true,
            (_, SourcePartInput::Invalid) => false,
            (SourcePartKind::MaterialItem { item, allow_uncraftable, .. }, SourcePartInput::Material { input_item, craftable, supported_outputs, .. }) => item != input_item && (*allow_uncraftable || *craftable) && supported_outputs.contains(item),
            (SourcePartKind::Item { material, .. }, SourcePartInput::Material { material: actual, .. }) => material != "tconstruct:unknown" && source_variant_matches(material, actual),
        };
        if !matches { continue; }
        let mut enabled = true;
        for condition in &recipe.conditions {
            match condition.matches(registries, config) {
                Some(false) => { enabled = false; break; },
                Some(true) => {},
                None => return Err(SourceLookupError::UnknownCondition),
            }
        }
        if enabled { candidates.push(recipe); }
    }
    candidates.sort_by(|a, b| source_id_order(&a.id).cmp(&source_id_order(&b.id)));
    let mut patterns = std::collections::HashSet::new();
    candidates.retain(|recipe| patterns.insert(recipe.pattern.clone()));
    candidates.sort_by(|a, b| a.cost.cmp(&b.cost).then_with(|| source_id_order(&a.pattern).cmp(&source_id_order(&b.pattern))));
    Ok(candidates)
}

/// Source candidate metadata, before preview or inventory mutation.
#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq)]
pub(super) struct SourceAllPartChoice { pub id: String, pub pattern: String, pub cost: i32 }

/// Merge ordinary and both recycling recipe types using the original
/// recipe-ID deduplication and cost/pattern ordering. Tool recycling requires
/// the caller to provide patterns resolved from that actual tool and its
/// material stats; absent context is an error, never an empty approximation.
#[allow(dead_code)]
pub(super) fn source_all_part_choices(
    recipes: &std::collections::BTreeMap<String, Vec<u8>>,
    pattern_item: &str, input: &SourcePartInput<'_>, tool_item: &str, tool_count: i32,
    item_tags: &std::collections::HashMap<String, std::collections::HashSet<String>>,
    instruments: &std::collections::HashSet<String>,
    instrument_tags: &std::collections::HashMap<String, std::collections::HashSet<String>>,
    registries: &std::collections::HashMap<String, std::collections::HashMap<String, std::collections::HashSet<String>>>,
    config: &std::collections::HashMap<String, bool>,
    loaded_mods: Option<&std::collections::HashSet<String>>,
    tool_patterns: Option<&std::collections::HashMap<String, Vec<String>>>, complete: bool,
) -> Result<Vec<SourceAllPartChoice>, SourceLookupError> {
    let base = source_part_choices(recipes, pattern_item, input, item_tags, instruments, instrument_tags, registries, config, complete)?;
    if pattern_item.is_empty() { return Ok(Vec::new()); }
    let mut choices: Vec<_> = base.into_iter().map(|r| SourceAllPartChoice { id: r.id, pattern: r.pattern, cost: r.cost }).collect();
    for (id, bytes) in recipes {
        if id.split_once(':').is_some_and(|(_, path)| path.starts_with('_')) { continue; }
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else { continue; };
        match value.get("type").and_then(serde_json::Value::as_str) {
            Some("tconstruct:part_builder_recycling") => {
                let recipe = SourceRecyclingRecipe::parse(id.clone(), &value).ok_or(SourceLookupError::UnsupportedRecipe)?;
                if tool_count < 1 || !recipe.partial_match(pattern_item, tool_item, item_tags, instruments, instrument_tags) { continue; }
                match recipe.enabled(registries, config, loaded_mods) {
                    Some(false) => continue, None => return Err(SourceLookupError::UnknownCondition), Some(true) => {},
                }
                for pattern in recipe.results.keys() { choices.push(SourceAllPartChoice { id: id.clone(), pattern: pattern.clone(), cost: 0 }); }
            },
            Some("tconstruct:part_builder_tool_recycling") => {
                let recipe = SourceToolRecyclingRecipe::parse(id.clone(), &value).ok_or(SourceLookupError::UnsupportedRecipe)?;
                if !recipe.partial_match(pattern_item, tool_item, tool_count, item_tags, instruments, instrument_tags) { continue; }
                match recipe.enabled(registries, config, loaded_mods) {
                    Some(false) => continue, None => return Err(SourceLookupError::UnknownCondition), Some(true) => {},
                }
                let patterns = tool_patterns.and_then(|all| all.get(id)).ok_or(SourceLookupError::IncompleteContext)?;
                for pattern in patterns { choices.push(SourceAllPartChoice { id: id.clone(), pattern: pattern.clone(), cost: 0 }); }
            },
            _ => {},
        }
    }
    choices.sort_by(|a,b| source_id_order(&a.id).cmp(&source_id_order(&b.id)));
    let mut patterns = std::collections::HashSet::new();
    choices.retain(|c| patterns.insert(c.pattern.clone()));
    choices.sort_by(|a,b| a.cost.cmp(&b.cost).then_with(|| source_id_order(&a.pattern).cmp(&source_id_order(&b.pattern))));
    Ok(choices)
}

/// A single source Part Builder craft plan. ItemOutput tag resolution, item
/// crafting-container returns, inventory capacity, events and atomic commit
/// are deliberately deferred; this is not used by player gameplay. Callers
/// must revalidate the pattern and conditions before any eventual commit.
#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub(super) enum SourcePartOutput {
    MaterialItem { item: String, material: String, count: i32 },
    Item { output: SourceItemOutput, count: i32 },
}
#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub(super) struct SourcePartPlan {
    pub output: SourcePartOutput,
    pub materials_used: i32,
    pub pattern_used: i32,
    pub leftover: Option<(SourceItemOutput, i32)>,
}

/// Resolve a planned source result and leftover against the live item/tag
/// registry. An absent preference or an unsupported NBT form is not guessed.
/// This does not mutate slots or deliver leftovers to the player.
#[allow(dead_code)]
pub(super) fn source_resolve_part_plan(
    plan: &SourcePartPlan,
    tags: &std::collections::HashMap<String, std::collections::HashSet<String>>,
    preferences: Option<&[String]>,
    registered: &std::collections::HashSet<String>,
) -> Result<Option<(ItemStack, Option<ItemStack>)>, SourceLookupError> {
    let resolve = |output: &SourceItemOutput, count: i32| -> Result<Option<ItemStack>, SourceLookupError> {
        if output.nbt.is_some() { return Err(SourceLookupError::UnsupportedRecipe); }
        if let SourceOutputKind::Tag(tag) = &output.kind {
            if !tags.contains_key(tag) { return Err(SourceLookupError::IncompleteContext); }
        }
        let Some(id) = output.resolve_id(tags, preferences).ok_or(SourceLookupError::IncompleteContext)? else { return Ok(None); };
        if !registered.contains(&id) { return Err(SourceLookupError::IncompleteContext); }
        let count = u32::try_from(count).ok().filter(|n| *n > 0).ok_or(SourceLookupError::UnsupportedRecipe)?;
        Ok(Some(ItemStack::new(&id, count)))
    };
    let output = match &plan.output {
        SourcePartOutput::MaterialItem { item, material, count } => {
            if !registered.contains(item) { return Err(SourceLookupError::IncompleteContext); }
            let count = u32::try_from(*count).ok().filter(|n| *n > 0).ok_or(SourceLookupError::UnsupportedRecipe)?;
            let material = super::tconstruct_source_render_info::material_id(material).ok_or(SourceLookupError::UnsupportedRecipe)?;
            let mut stack = ItemStack::new(item, count);
            stack.data_mut().set_other("minecraft:custom_data", Some(horizon_world::nbt::Tag::compound([
                ("Material", horizon_world::nbt::Tag::String(material.as_str().into())),
            ])));
            stack
        },
        SourcePartOutput::Item { output, count } => {
            let Some(stack) = resolve(output, *count)? else { return Ok(None); };
            stack
        },
    };
    let leftover = match &plan.leftover {
        Some((item, count)) => resolve(item, *count)?,
        None => None,
    };
    Ok(Some((output, leftover)))
}

/// Apply a previously revalidated plan to the authoritative pattern/material
/// slots and cursor. Return the source-ordered stacks to give to the player:
/// recipe leftover first, then crafting containers from material and pattern.
/// The caller must obtain each input's real crafting container and deliver or
/// drop the returned stacks using the player's inventory rules.
#[allow(dead_code)]
pub(super) fn source_take_part_plan(
    slots: &mut [Option<ItemStack>], carried: &mut Option<ItemStack>,
    plan: &SourcePartPlan, output: ItemStack, leftover: Option<ItemStack>,
    material_container: Option<ItemStack>, pattern_container: Option<ItemStack>,
    output_max: u32,
) -> Option<Vec<ItemStack>> {
    if slots.len() < 2 || output.count == 0 || output.count > output_max { return None; }
    let material_used = u32::try_from(plan.materials_used).ok()?;
    let pattern_used = u32::try_from(plan.pattern_used).ok()?;
    for (slot, used) in [(1, material_used), (0, pattern_used)] {
        if used != 0 && slots[slot].as_ref().is_none_or(|s| s.count < used) { return None; }
    }
    if let Some(existing) = carried.as_ref() {
        if !existing.same_item_same_components(&output)
            || existing.count.checked_add(output.count).is_none_or(|n| n > output_max) { return None; }
    }
    let mut give = Vec::new();
    if let Some(leftover) = leftover { give.push(leftover); }
    for (slot, used, container) in [(1, material_used, material_container), (0, pattern_used, pattern_container)] {
        if used == 0 { continue; }
        let stack = slots[slot].as_mut().expect("inputs were validated");
        let container = container.map(|mut value| { value.count = value.count.wrapping_mul(used); value });
        if stack.count <= used { slots[slot] = container; }
        else {
            stack.count -= used;
            if let Some(container) = container { give.push(container); }
        }
    }
    match carried.as_mut() { Some(stack) => stack.count += output.count, None => *carried = Some(output) }
    Some(give)
}
#[allow(dead_code)]
pub(super) fn source_part_plan(
    recipe: &SourcePartRecipe,
    input: &SourcePartInput<'_>,
    material: Option<&SourceMaterialMatch>,
    available: i32,
    reusable_pattern: bool,
) -> Result<Option<SourcePartPlan>, SourceLookupError> {
    let material = material.map(|source| SourceMaterialValue {
        material: &source.material, value: source.value, needed: source.needed, leftover: source.leftover.as_ref(),
    });
    source_part_plan_value(recipe, input, material, available, reusable_pattern)
}

/// Shared source IMaterialValue view. It supports both material recipes and
/// MaterialValue built from an existing tool part without inventing a recipe ID.
#[allow(dead_code)]
pub(super) struct SourceMaterialValue<'a> {
    pub material: &'a str,
    pub value: i32,
    pub needed: i32,
    pub leftover: Option<&'a SourceItemOutput>,
}

#[allow(dead_code)]
pub(super) fn source_part_plan_value(
    recipe: &SourcePartRecipe,
    input: &SourcePartInput<'_>,
    material: Option<SourceMaterialValue<'_>>,
    available: i32,
    reusable_pattern: bool,
) -> Result<Option<SourcePartPlan>, SourceLookupError> {
    let pattern_used = i32::from(!reusable_pattern);
    match (&recipe.kind, input, material) {
        (SourcePartKind::Item { material: required, output }, SourcePartInput::Empty, None) if required == "tconstruct:unknown" => {
            Ok(Some(SourcePartPlan { output: SourcePartOutput::Item { output: output.clone(), count: output.count }, materials_used: 0, pattern_used, leftover: None }))
        },
        (kind, SourcePartInput::Material { input_item, material: identity, craftable, supported_outputs }, Some(source)) => {
            if source.material != *identity { return Err(SourceLookupError::IncompleteContext); }
            match kind {
                SourcePartKind::MaterialItem { item, allow_uncraftable, .. } if item == input_item || (!allow_uncraftable && !craftable) || !supported_outputs.contains(item) => return Ok(None),
                SourcePartKind::Item { material: required, .. } if required == "tconstruct:unknown" || !source_variant_matches(required, identity) => return Ok(None),
                _ => {},
            }
            let (used, remainder) = material_cost(recipe.cost, source.needed, source.value).ok_or(SourceLookupError::UnsupportedRecipe)?;
            if available < used { return Ok(None); }
            let leftover = source.leftover.and_then(|output| output.count_for_remainder(remainder, source.needed).map(|count| (output.clone(), count)));
            let count = match kind { SourcePartKind::MaterialItem { count, .. } => *count, SourcePartKind::Item { output, .. } => output.count };
            let count = material_output_count(count, source.value, recipe.cost, source.leftover.is_some()).ok_or(SourceLookupError::UnsupportedRecipe)?;
            if count <= 0 { return Err(SourceLookupError::UnsupportedRecipe); }
            let output = match kind {
                SourcePartKind::MaterialItem { item, .. } => SourcePartOutput::MaterialItem { item: item.clone(), material: source.material.to_owned(), count },
                SourcePartKind::Item { output, .. } => SourcePartOutput::Item { output: output.clone(), count },
            };
            Ok(Some(SourcePartPlan { output, materials_used: used, pattern_used, leftover }))
        },
        _ => Ok(None),
    }
}

/// `TinkerStationRepairRecipe.repairFromSlot`: Java performs the division and
/// multiplication as floats, rounds the item count up, then truncates the
/// repaired durability. The caller supplies the material/modifier-derived rate.
fn repair_from_slot(needed: i32, stack: i32, durability_per_item: f32) -> (i32, i32) {
    if needed <= 0 || stack <= 0 || !(durability_per_item > 0.0) { return (0, 0); }
    let applied = stack.min(((needed as f32) / durability_per_item).ceil() as i32);
    (applied, (applied as f32 * durability_per_item) as i32)
}

/// The original `IMaterialItem` stores the chosen material under `Material`.
/// Horizon 26.3 keeps that legacy item tag inside `minecraft:custom_data`.
fn part_with_material(id: &str, material: &str) -> ItemStack {
    let mut part = ItemStack::new(id, 1);
    part.data_mut().set_other("minecraft:custom_data", Some(horizon_world::nbt::Tag::compound([
        ("Material", horizon_world::nbt::Tag::String(material.into())),
    ])));
    let model = match (id, material) {
        ("tconstruct:pick_head", "tconstruct:flint") => "tconstruct:flint_pick_head",
        ("tconstruct:pick_head", "tconstruct:wood") => "tconstruct:wood_pick_head",
        ("tconstruct:tool_handle", "tconstruct:flint") => "tconstruct:flint_handle",
        ("tconstruct:tool_handle", "tconstruct:wood") => "tconstruct:wood_handle",
        ("tconstruct:tool_binding", "tconstruct:flint") => "tconstruct:flint_binding",
        ("tconstruct:tool_binding", "tconstruct:wood") => "tconstruct:wood_binding",
        _ => return part,
    };
    part.data_mut().set_other("minecraft:item_model", Some(horizon_world::nbt::Tag::String(model.into())));
    part
}

/// Legacy prototype IDs encode a single material. If a Material tag is
/// present, reject a conflicting value rather than silently assembling it.
fn part_matches_material(stack: &ItemStack, expected: &str) -> bool {
    use horizon_world::nbt::Tag;
    match stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|v| v.get("Material")) {
        None => matches!(&*stack.item, "tconstruct:flint_pick_head" | "tconstruct:wood_handle" | "tconstruct:wood_binding"),
        Some(Tag::String(value)) => value.to_string_lossy() == expected,
        Some(_) => false,
    }
}

/// The fixed prototype has a flint head. Original `MaterialStatsModule`
/// checks the tool's material list before accepting a repair material.
fn fixed_pickaxe_has_flint_head(stack: &ItemStack) -> bool {
    use horizon_world::nbt::Tag;
    if &*stack.item == "tconstruct:flint_pickaxe" { return true; }
    match stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|d| d.get("tic_materials")) {
        Some(Tag::List(materials)) => matches!(materials.first(), Some(Tag::String(id)) if id.to_string_lossy() == "tconstruct:flint"),
        _ => false,
    }
}

fn fixed_pickaxe_with_materials() -> ItemStack {
    use horizon_world::nbt::Tag;
    let mut tool = ItemStack::new("tconstruct:pickaxe", 1);
    let materials = ["tconstruct:flint", "tconstruct:wood", "tconstruct:wood"]
        .into_iter().map(|id| Tag::String(id.into())).collect();
    tool.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_materials", Tag::List(materials))])));
    tool.data_mut().set_other("minecraft:item_model", Some(Tag::String("tconstruct:flint_pickaxe".into())));
    tool.data_mut().set_other("minecraft:item_name", Some(Tag::String("Flint Pickaxe".into())));
    tool
}

/// `ToolStack.setDamage` for the currently supported fixed-durability tool.
/// Preserve all unrelated custom data while writing the source broken flag.
pub(super) fn fixed_set_damage(stack: &mut ItemStack, damage: u32) {
    use horizon_world::nbt::Tag;
    let mut custom = stack.data().and_then(|data| data.other("minecraft:custom_data"))
        .cloned().unwrap_or_else(|| Tag::compound(Vec::<(&str, Tag)>::new()));
    if let Tag::Compound(ref mut entries) = custom {
        entries.insert("tic_broken".into(), Tag::Byte(i8::from(damage >= 85)));
        stack.data_mut().set_other("minecraft:custom_data", Some(custom));
    }
    stack.damage = damage.min(85);
}

/// No damage is applied to an already broken or source-unbreakable tool.
pub(super) fn fixed_damage(stack: &mut ItemStack, amount: u32) {
    use horizon_world::nbt::Tag;
    let flag = |name: &str| matches!(stack.data().and_then(|data| data.other("minecraft:custom_data")).and_then(|tag| tag.get(name)), Some(Tag::Byte(value)) if *value != 0);
    if amount == 0 || stack.damage >= 85 || flag("tic_broken") || flag("Unbreakable") { return; }
    fixed_set_damage(stack, stack.damage.saturating_add(amount));
}

/// Apply a single result pickup to the authoritative slots and cursor.
/// Inputs are checked before any mutation; tool data is carried by cloning it.
pub(super) fn take(stacks: &mut [Option<ItemStack>], carried: &mut Option<ItemStack>, station: bool, action: i16) -> bool {
    if stacks.len() < 27 { return false; }
    let is = |slot: usize, item: &str, count: u32| stacks.get(slot).and_then(Option::as_ref).is_some_and(|s| &*s.item == item && s.count >= count);
    let (output, consume): (ItemStack, Vec<(usize, u32)>) = if !station {
        let (cost, result) = match action {
            24 => (2, "tconstruct:pick_head"),
            25 => (1, "tconstruct:tool_handle"),
            26 => (1, "tconstruct:tool_binding"),
            _ => return false,
        };
        let (material, material_id) = match stacks[1].as_ref().map(|s| &*s.item) {
            Some("minecraft:flint") => ("minecraft:flint", "tconstruct:flint"),
            Some("minecraft:oak_planks") => ("minecraft:oak_planks", "tconstruct:wood"),
            _ => return false,
        };
        let Some((amount, remainder)) = material_cost(cost, 1, 1) else { return false; };
        let Some(count) = material_output_count(1, 1, cost, false) else { return false; };
        // Only unit-value materials and one-item results are currently wired in.
        if count != 1 { return false; }
        // The current two inputs have unit value and no remainder. Other
        // material recipes and their remainder items are not yet supported.
        let Ok(amount) = u32::try_from(amount) else { return false; };
        if amount == 0 || remainder != 0 || !is(0, "tconstruct:pattern", 1) || !is(1, material, amount) { return false; }
        (part_with_material(result, material_id), vec![(0, 1), (1, amount)])
    } else if action == 24 {
        if !(is(0, "tconstruct:pick_head", 1) || is(0, "tconstruct:flint_pick_head", 1)) || !(is(1, "tconstruct:tool_handle", 1) || is(1, "tconstruct:wood_handle", 1)) || !(is(2, "tconstruct:tool_binding", 1) || is(2, "tconstruct:wood_binding", 1)) { return false; }
        for (slot, material) in [(0, "tconstruct:flint"), (1, "tconstruct:wood"), (2, "tconstruct:wood")] {
            if !part_matches_material(stacks[slot].as_ref().unwrap(), material) { return false; }
        }
        (fixed_pickaxe_with_materials(), vec![(0, 1), (1, 1), (2, 1)])
    } else if action == 25 {
        if !(is(0, "tconstruct:pickaxe", 1) || is(0, "tconstruct:flint_pickaxe", 1)) { return false; }
        let mut tool = stacks[0].as_ref().unwrap().clone();
        if tool.count != 1 || tool.damage == 0 || !fixed_pickaxe_has_flint_head(&tool) { return false; }
        if (1..=2).any(|i| stacks[i].as_ref().is_some_and(|s| &*s.item != "minecraft:flint" || s.count == 0)) { return false; }
        let Ok(mut needed) = i32::try_from(tool.damage) else { return false; };
        let mut consume = vec![(0, 1)];
        for slot in 1..=2 {
            if let Some(stack) = &stacks[slot] {
                let Ok(available) = i32::try_from(stack.count) else { return false; };
                // Fixed prototype rate; material stats and modifier hooks still need porting.
                let (count, repaired) = repair_from_slot(needed, available, 56.0);
                if count > 0 { consume.push((slot, count as u32)); needed = (needed - repaired).max(0); }
            }
        }
        if consume.len() == 1 { return false; }
        fixed_set_damage(&mut tool, needed as u32);
        (tool, consume)
    } else { return false; };
    if let Some(existing) = carried.as_ref() {
        let max = if &*output.item == "tconstruct:flint_pickaxe" || &*output.item == "tconstruct:pickaxe" { 1 } else { 64 };
        let mut compare = existing.clone();
        compare.count = output.count;
        if compare != output || existing.count >= max { return false; }
    }
    for (slot, count) in consume {
        let stack = stacks[slot].as_mut().unwrap();
        stack.count -= count;
        if stack.count == 0 { stacks[slot] = None; }
    }
    match carried.as_mut() {
        Some(stack) => stack.count += output.count,
        None => *carried = Some(output),
    }
    true
}

#[cfg(test)]
mod tests {
    #[test]
    fn material_block_preserves_original_unknown_identity_and_load_rules() {
        use super::SourceBlockMaterial as M;
        let mut m = M::UnknownSingleton;
        assert_eq!(m.save_string(), None);
        m.load(None);
        assert_eq!(m.save_string(), None);
        m.load(Some("tconstruct:unknown"));
        assert_eq!(m.save_string(), Some("tconstruct:unknown"));
        assert!(!m.set(M::UnknownSingleton));
        assert_eq!(m.save_string(), None);
        assert!(!m.set(M::Parsed("tconstruct:unknown".into())));
        assert_eq!(m.save_string(), Some("tconstruct:unknown"));
        m.load(Some("tconstruct:wood#warped"));
        assert_eq!(m.save_string(), Some("tconstruct:wood#warped"));
        m.load(None);
        assert_eq!(m.save_string(), Some("tconstruct:wood#warped"));
        m.load(Some("bad namespace:wood"));
        assert_eq!(m.save_string(), None);
        assert_eq!(M::placed_material(Some("")), M::UnknownSingleton);
        assert_eq!(M::placed_material(Some("wood#oak")), M::Parsed("minecraft:wood#oak".into()));
        use horizon_world::nbt::{Compound, Tag};
        let tag = Tag::compound([("Material", Tag::String("tconstruct:bronze".into()))]);
        m.load_tag(&tag);
        let mut output = Compound::new();
        output.insert("Other".into(), Tag::Int(42));
        m.write_tag(&mut output);
        assert_eq!(output.get(&"Material".into()), Some(&Tag::String("tconstruct:bronze".into())));
        assert_eq!(output.get(&"Other".into()), Some(&Tag::Int(42)));
        m.load_tag(&Tag::compound([("Material", Tag::Int(1))]));
        assert_eq!(m.save_string(), Some("tconstruct:bronze"));
        m.load_tag(&Tag::compound([("Material", Tag::String("INVALID".into()))]));
        m.write_tag(&mut output);
        assert_eq!(output.get(&"Material".into()), Some(&Tag::String("tconstruct:bronze".into())));
    }

    #[test]
    fn selection_identity_survives_reordering_and_missing_choices() {
        use super::*;
        let selected = preview_pattern(1).unwrap();
        assert_eq!(selected, "tconstruct:tool_handle");
        assert_eq!(pattern_index(selected, &["tconstruct:pick_head", selected, "tconstruct:tool_binding"]), Some(1));
        assert_eq!(pattern_index(selected, &[selected, "tconstruct:tool_binding", "tconstruct:pick_head"]), Some(0));
        assert_eq!(pattern_index(selected, &["tconstruct:pick_head"]), None);
        assert_eq!(preview_pattern_index(selected), Some(1));
        assert_eq!(preview_pattern(-1), None);
        assert_eq!(preview_pattern(3), None);
        let mut many = vec!["test:other"; 32768];
        many.push(selected);
        assert_eq!(pattern_index(selected, &many), Some(32768));
        assert_eq!(pattern_index(selected, &many).map(|i| i as i16), Some(i16::MIN));
    }

    use super::*;
    #[test]
    fn original_material_cost_rounds_up_and_tracks_remainder() {
        assert_eq!(material_cost(2, 1, 1), Some((2, 0)));
        assert_eq!(material_cost(1, 2, 3), Some((1, 1)));
        assert_eq!(material_cost(2, 1, 3), Some((1, 1)));
        assert_eq!(material_cost(2, 3, 2), Some((3, 0)));
        assert_eq!(material_cost(i32::MAX, 2, 3), Some((1, 2)));
        assert_eq!(material_cost(1, 1, 0), None);
    }
    #[test]
    fn original_part_count_depends_on_material_value_and_leftover() {
        assert_eq!(material_output_count(1, 1, 2, false), Some(1));
        assert_eq!(material_output_count(1, 3, 1, false), Some(3));
        assert_eq!(material_output_count(2, 5, 2, false), Some(5));
        assert_eq!(material_output_count(1, 3, 2, true), Some(1));
        assert_eq!(material_output_count(1, 3, 0, false), None);
    }
    #[test]
    fn original_material_ingredients_cover_tags_difference_and_instruments() {
        use std::collections::{HashMap, HashSet};
        let mut tags = HashMap::new();
        tags.insert("minecraft:planks".into(), HashSet::from(["minecraft:oak_planks".into(), "minecraft:warped_planks".into()]));
        tags.insert("tconstruct:wood_variants/planks".into(), HashSet::from(["minecraft:warped_planks".into()]));
        let difference = SourceIngredient::parse(&serde_json::json!({"type":"forge:difference", "base":{"tag":"minecraft:planks"}, "subtracted":{"tag":"tconstruct:wood_variants/planks"}})).unwrap();
        let known = HashSet::from(["minecraft:ponder_goat_horn".into(), "minecraft:sing_goat_horn".into()]);
        let mut instruments = HashMap::new();
        instruments.insert("tconstruct:variant_horns".into(), HashSet::from(["minecraft:sing_goat_horn".into()]));
        assert!(difference.matches("minecraft:oak_planks", None, &tags, &known, &instruments));
        assert!(!difference.matches("minecraft:warped_planks", None, &tags, &known, &instruments));
        let exact = SourceIngredient::parse(&serde_json::json!({"type":"tconstruct:instrument", "item":"minecraft:goat_horn", "instrument":"minecraft:sing_goat_horn"})).unwrap();
        assert!(exact.matches("minecraft:goat_horn", Some("minecraft:sing_goat_horn"), &tags, &known, &instruments));
        assert!(!exact.matches("minecraft:goat_horn", None, &tags, &known, &instruments));
        let fallback = SourceIngredient::parse(&serde_json::json!({"type":"tconstruct:instrument", "item":"minecraft:goat_horn", "ignore":"tconstruct:variant_horns"})).unwrap();
        assert!(fallback.matches("minecraft:goat_horn", None, &tags, &known, &instruments));
        assert!(fallback.matches("minecraft:goat_horn", Some("minecraft:ponder_goat_horn"), &tags, &known, &instruments));
        assert!(!fallback.matches("minecraft:goat_horn", Some("minecraft:sing_goat_horn"), &tags, &known, &instruments));
        assert!(!fallback.matches("minecraft:goat_horn", Some("example:unknown"), &tags, &known, &instruments));
        let alternatives = SourceIngredient::parse(&serde_json::json!([{"tag":"forge:storage_blocks/copper"}, {"item":"minecraft:waxed_copper_block"}])).unwrap();
        assert!(alternatives.matches("minecraft:waxed_copper_block", None, &tags, &known, &instruments));
        assert!(SourceIngredient::parse(&serde_json::json!({"type":"example:unknown"})).is_none());
    }
    #[test]
    fn source_tag_conditions_use_intersection_exclusion_and_or() {
        use std::collections::{HashMap, HashSet};
        let mut items = HashMap::new();
        items.insert("minecraft:planks".into(), HashSet::from(["minecraft:oak_planks".into(), "minecraft:warped_planks".into()]));
        items.insert("tconstruct:wood_variants/planks".into(), HashSet::from(["minecraft:warped_planks".into()]));
        items.insert("test:only_warped".into(), HashSet::from(["minecraft:warped_planks".into()]));
        let mut registries = HashMap::new();
        registries.insert("minecraft:item".into(), items);
        let condition = SourceCondition::parse(&serde_json::json!({"type":"mantle:tag_combination_filled", "match":"minecraft:planks", "ignore":"tconstruct:wood_variants/planks"})).unwrap();
        let mut config = HashMap::new();
        assert_eq!(condition.matches(&registries, &config), Some(true));
        let intersection = SourceCondition::parse(&serde_json::json!({"type":"mantle:tag_combination_filled", "match":["minecraft:planks", "test:only_warped"], "ignore":"tconstruct:wood_variants/planks"})).unwrap();
        assert_eq!(intersection.matches(&registries, &config), Some(false));
        let or = SourceCondition::parse(&serde_json::json!({"type":"forge:or", "values":[{"type":"mantle:tag_filled","tag":"test:missing"}, {"type":"mantle:tag_filled","tag":"minecraft:planks"}]})).unwrap();
        assert_eq!(or.matches(&registries, &config), Some(true));
        let other = SourceCondition::parse(&serde_json::json!({"type":"mantle:tag_filled", "registry":"minecraft:instrument", "tag":"minecraft:planks"})).unwrap();
        assert_eq!(other.matches(&registries, &config), None);
        registries.insert("minecraft:instrument".into(), Default::default());
        assert_eq!(other.matches(&registries, &config), Some(false));
        let configured = SourceCondition::parse(&serde_json::json!({"type":"tconstruct:config", "prop":"FORCE_INTEGRATION_MATERIALS"})).unwrap();
        assert_eq!(configured.matches(&registries, &config), None);
        let config_or_tag = SourceCondition::parse(&serde_json::json!({"type":"forge:or", "values":[{"type":"tconstruct:config","prop":"force_integration_materials"}, {"type":"mantle:tag_filled","tag":"test:missing"}]})).unwrap();
        assert_eq!(config_or_tag.matches(&registries, &config), None);
        config.insert("force_integration_materials".into(), false);
        assert_eq!(config_or_tag.matches(&registries, &config), Some(false));
        config.insert("force_integration_materials".into(), true);
        assert_eq!(configured.matches(&registries, &config), Some(true));
        config.insert("glass_recipe_fix".into(), true);
        let alias = SourceCondition::parse(&serde_json::json!({"type":"tconstruct:config", "prop":"slime_recipe_fix"})).unwrap();
        assert_eq!(alias.matches(&registries, &config), Some(true));
        config.insert("wither_bone_conversion".into(), true);
        let removed = SourceCondition::parse(&serde_json::json!({"type":"tconstruct:config", "prop":"wither_bone_conversion"})).unwrap();
        assert_eq!(removed.matches(&registries, &config), Some(false));
        assert!(SourceCondition::parse(&serde_json::json!({"type":"tconstruct:config", "prop":"unknown"})).is_none());
        assert!(SourceCondition::parse(&serde_json::json!({"type":"forge:or", "values":[]})).is_none());
        let and = SourceCondition::parse(&serde_json::json!({"type":"forge:and", "values":[{"type":"tconstruct:config","prop":"force_integration_materials"}, {"type":"mantle:tag_filled","tag":"test:missing"}]})).unwrap();
        config.remove("force_integration_materials");
        assert_eq!(and.matches(&registries, &config), Some(false));
        let and = SourceCondition::parse(&serde_json::json!({"type":"forge:and", "values":[{"type":"tconstruct:config","prop":"force_integration_materials"}, {"type":"mantle:tag_filled","tag":"minecraft:planks"}]})).unwrap();
        assert_eq!(and.matches(&registries, &config), None);
        config.insert("force_integration_materials".into(), true);
        assert_eq!(and.matches(&registries, &config), Some(true));

    }
    #[test]
    fn parses_all_pinned_material_recipe_ingredients() {
        use std::io::Read;
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors().map(|dir| dir.join("docs/upstream-recipes.json.gz"))
            .find(|path| path.is_file()).expect("pinned reference catalog");
        let file = std::fs::File::open(path).expect("read pinned reference catalog");
        let mut json = String::new();
        flate2::read::GzDecoder::new(file).read_to_string(&mut json).unwrap();
        let catalog: serde_json::Value = serde_json::from_str(&json).unwrap();
        let mut count = 0;
        let mut condition_count = 0;
        let mut leftover_count = 0;
        for recipe in catalog["groups"]["materials"].as_array().unwrap() {
            let data = &recipe["data"];
            if data["type"] == "tconstruct:material" {
                let parsed = SourceMaterialRecipe::parse(data).unwrap_or_else(|| panic!("{}", recipe["source"]));
                assert!(parsed.value > 0 && parsed.needed > 0 && !parsed.material.is_empty());
                if data.get("leftover").is_some() && parsed.value > 1 {
                    assert!(parsed.leftover.is_some(), "{}", recipe["source"]);
                    leftover_count += 1;
                }
                for condition in data["conditions"].as_array().into_iter().flatten() {
                    assert!(SourceCondition::parse(condition).is_some(), "{}", recipe["source"]);
                    condition_count += 1;
                }
                count += 1;
            }
        }
        assert_eq!(count, 217);
        assert_eq!(condition_count, 44);
        assert_eq!(leftover_count, 47);
    }
    #[test]
    fn source_material_recipe_defaults_and_deferred_leftovers() {
        let single = SourceMaterialRecipe::parse(&serde_json::json!({"type":"tconstruct:material", "ingredient":{"item":"minecraft:flint"}, "material":"tconstruct:flint", "leftover":"minecraft:stick"})).unwrap();
        assert_eq!((single.value, single.needed), (1, 1));
        assert!(single.leftover.is_none());
        let block = SourceMaterialRecipe::parse(&serde_json::json!({"type":"tconstruct:material", "ingredient":{"tag":"forge:storage_blocks/copper"}, "material":"tconstruct:copper", "value":9, "leftover":{"tag":"forge:ingots/copper", "count":1}})).unwrap();
        assert_eq!(material_cost(2, block.needed, block.value), Some((1, 7)));
        let output = block.leftover.unwrap();
        assert!(matches!(&output.kind, SourceOutputKind::Tag(tag) if tag == "forge:ingots/copper"));
        assert_eq!(output.count_for_remainder(7, 1), Some(7));
        assert_eq!(output.count_for_remainder(1, 2), None);
        let mut tags = std::collections::HashMap::new();
        tags.insert("forge:ingots/copper".into(), std::collections::HashSet::from(["alpha:z".into(), "beta:a".into(), "minecraft:copper_ingot".into()]));
        assert_eq!(output.resolve_id(&tags, None), None);
        assert_eq!(output.resolve_id(&tags, Some(&["minecraft".into()])), Some(Some("minecraft:copper_ingot".into())));
        assert_eq!(output.resolve_id(&tags, Some(&[])), Some(Some("alpha:z".into())));
        assert_eq!(output.resolve_id(&std::collections::HashMap::new(), Some(&[])), Some(None));
        assert!(SourceMaterialRecipe::parse(&serde_json::json!({"type":"tconstruct:material", "ingredient":{"item":"minecraft:flint"}, "material":"tconstruct:flint", "needed":0})).is_none());
    }
    #[test]
    fn original_head_and_handle_durability_uses_float_then_integer() {
        for (head, handle, expected) in [(85.0, 0.0, 85), (85.0, -0.15_f32, 72), (60.0, 0.0, 60), (60.0, -0.15_f32, 51)] {
            let mut stat = SourceFloatStat::new(1.0);
            stat.update(head);
            stat.percent(handle as f64);
            assert_eq!(stat.build() as i32, expected);
        }
        let mut stat = SourceFloatStat::new(1.0);
        stat.update(85.0);
        stat.percent(-0.15_f32 as f64);
        stat.add(5.0);
        stat.multiply(1.5);
        assert_eq!(stat.build(), 114.75);
    }
    #[test]
    fn original_pickaxe_part_stats_preserve_float_precision_before_modifiers() {
        let flint = SourceHeadStats { durability: 85, mining_speed: 3.5, attack_damage: 1.25 };
        let wood_handle = SourceHandleStats { durability: 0.0, mining_speed: 0.0, attack_speed: 0.0, attack_damage: 0.0 };
        assert_eq!(source_pickaxe_base(&flint, &wood_handle), [85.0, 3.5, 1.75, 1.2]);
        let flint_handle = SourceHandleStats { durability: -0.15, mining_speed: 0.0, attack_speed: 0.0, attack_damage: 0.1 };
        let stats = source_pickaxe_base(&flint, &flint_handle);
        assert_eq!(stats[0], 72.25);
        assert_eq!(stats[2].to_bits(), 0x3ff66667);
    }
    #[test]
    fn original_repair_rounds_count_before_truncating_total() {
        let per_item = (85.0_f32 / 3.0) * 2.0;
        assert_eq!(repair_from_slot(85, 2, per_item), (2, 113));
        assert_eq!(repair_from_slot(85, 1, per_item), (1, 56));
        assert_eq!(repair_from_slot(29, 1, per_item), (1, 56));
        assert_eq!(repair_from_slot(85, 2, 0.0), (0, 0));
    }
    #[test]
    fn part_take_is_atomic_and_consumes_default_pattern() {
        let mut slots = vec![None; 27];
        slots[0] = Some(ItemStack::new("tconstruct:pattern", 1));
        slots[1] = Some(ItemStack::new("minecraft:flint", 2));
        let mut cursor = Some(ItemStack::new("minecraft:stick", 1));
        let before = slots.clone();
        assert!(!take(&mut slots, &mut cursor, false, 24));
        assert_eq!(slots, before);
        cursor = None;
        assert!(take(&mut slots, &mut cursor, false, 24));
        assert_eq!(cursor.as_ref().unwrap().item.as_ref(), "tconstruct:pick_head");
        assert_eq!(cursor.as_ref().unwrap().data().and_then(|d| d.other("minecraft:custom_data")), Some(&horizon_world::nbt::Tag::compound([("Material", horizon_world::nbt::Tag::String("tconstruct:flint".into()))])));
        assert!(slots[1].is_none());
        assert!(slots[0].is_none());
        cursor = None;
        assert!(!take(&mut slots, &mut cursor, false, 24));
        assert!(cursor.is_none());
    }
    #[test]
    fn both_supported_materials_make_each_fixed_part_with_source_cost() {
        for (item, id) in [("minecraft:flint", "tconstruct:flint"), ("minecraft:oak_planks", "tconstruct:wood")] {
            for (action, output, cost) in [(24, "tconstruct:pick_head", 2), (25, "tconstruct:tool_handle", 1), (26, "tconstruct:tool_binding", 1)] {
                let mut slots = vec![None; 27];
                slots[0] = Some(ItemStack::new("tconstruct:pattern", 1));
                slots[1] = Some(ItemStack::new(item, cost));
                let mut cursor = None;
                assert!(take(&mut slots, &mut cursor, false, action));
                let part = cursor.unwrap();
                assert_eq!(&*part.item, output);
                assert!(part_matches_material(&part, id));
                assert!(slots[0].is_none() && slots[1].is_none());
            }
        }
    }
    #[test]
    fn preview_does_not_consume_and_rechecks_material() {
        let mut slots = vec![None; 27];
        slots[0] = Some(ItemStack::new("tconstruct:pattern", 1));
        slots[1] = Some(ItemStack::new("minecraft:flint", 2));
        let before = slots.clone();
        assert_eq!(part_preview(&slots, 0).unwrap().item.as_ref(), "tconstruct:pick_head");
        assert_eq!(slots, before);
        assert_eq!(part_preview(&slots, 1).unwrap().item.as_ref(), "tconstruct:tool_handle");
        slots[1] = Some(ItemStack::new("minecraft:flint", 1));
        assert!(part_preview(&slots, 0).is_none());
    }
    #[test]
    fn assembly_retains_materials_and_rejects_conflicting_identity() {
        assert!(!part_matches_material(&ItemStack::new("tconstruct:pick_head", 1), "tconstruct:flint"));
        assert!(part_matches_material(&ItemStack::new("tconstruct:flint_pick_head", 1), "tconstruct:flint"));
        let mut slots = vec![None; 27];
        slots[0] = Some(part_with_material("tconstruct:flint_pick_head", "tconstruct:flint"));
        slots[1] = Some(part_with_material("tconstruct:wood_handle", "tconstruct:wood"));
        slots[2] = Some(part_with_material("tconstruct:wood_binding", "tconstruct:flint"));
        let before = slots.clone();
        let mut cursor = None;
        assert!(!take(&mut slots, &mut cursor, true, 24));
        assert_eq!(slots, before);
        slots[2] = Some(part_with_material("tconstruct:wood_binding", "tconstruct:wood"));
        assert!(take(&mut slots, &mut cursor, true, 24));
        let data = cursor.unwrap();
        let list = data.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|v| v.get("tic_materials"));
        assert_eq!(list, Some(&horizon_world::nbt::Tag::List(["tconstruct:flint", "tconstruct:wood", "tconstruct:wood"].into_iter().map(|id| horizon_world::nbt::Tag::String(id.into())).collect())));
    }
    #[test]
    fn repair_rejects_missing_or_wrong_head_material_without_mutating() {
        let mut slots = vec![None; 27];
        let mut tool = ItemStack::new("tconstruct:pickaxe", 1);
        tool.damage = 50;
        slots[0] = Some(tool);
        slots[1] = Some(ItemStack::new("minecraft:flint", 1));
        let before = slots.clone();
        let mut cursor = None;
        assert!(!take(&mut slots, &mut cursor, true, 25));
        assert_eq!(slots, before);
        let mut wrong = fixed_pickaxe_with_materials();
        wrong.damage = 50;
        wrong.data_mut().set_other("minecraft:custom_data", Some(horizon_world::nbt::Tag::compound([
            ("tic_materials", horizon_world::nbt::Tag::List(vec![horizon_world::nbt::Tag::String("tconstruct:wood".into())]))
        ])));
        slots[0] = Some(wrong);
        let before = slots.clone();
        assert!(!take(&mut slots, &mut cursor, true, 25));
        assert_eq!(slots, before);
        assert!(cursor.is_none());
    }
    #[test]
    fn repair_preserves_name_and_unknown_components() {
        let mut slots = vec![None; 27];
        let mut tool = ItemStack::new("tconstruct:flint_pickaxe", 1);
        tool.damage = 85;
        tool.data_mut().custom_name = Some("Named tool".into());
        tool.data_mut().other.push(("example:custom".into(), horizon_world::nbt::Tag::Int(17)));
        slots[0] = Some(tool.clone());
        slots[1] = Some(ItemStack::new("minecraft:flint", 1));
        let mut cursor = None;
        assert!(take(&mut slots, &mut cursor, true, 25));
        let repaired = cursor.as_ref().unwrap();
        assert_eq!(repaired.damage, 29);
        assert_eq!(repaired.data().and_then(|d| d.other("example:custom")), tool.data().and_then(|d| d.other("example:custom")));
        assert_eq!(repaired.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|v| v.get("tic_broken")), Some(&horizon_world::nbt::Tag::Byte(0)));
        assert!(slots[0].is_none() && slots[1].is_none());
        assert!(!take(&mut slots, &mut None, true, 25));
    }
    #[test]
    fn fixed_damage_breaks_without_removing_tool_and_preserves_custom_data() {
        let mut tool = fixed_pickaxe_with_materials();
        tool.damage = 84;
        fixed_damage(&mut tool, 2);
        assert_eq!(tool.damage, 85);
        let data = tool.data().and_then(|d| d.other("minecraft:custom_data")).unwrap();
        assert_eq!(data.get("tic_broken"), Some(&horizon_world::nbt::Tag::Byte(1)));
        assert!(data.get("tic_materials").is_some());
        let before = tool.clone();
        fixed_damage(&mut tool, 2);
        assert_eq!(tool, before);
        fixed_set_damage(&mut tool, 29);
        assert_eq!(tool.damage, 29);
        assert_eq!(tool.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|v| v.get("tic_broken")), Some(&horizon_world::nbt::Tag::Byte(0)));
    }
    #[test]
    fn parses_pinned_material_definitions_without_guessing() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors().map(|dir| dir.join("content/reference/tconstruct-materials.zip"))
            .find(|path| path.is_file()).expect("pinned material data");
        let mut archive = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
        let mut config = std::collections::HashMap::new();
        for key in ["spawn_with_book", "gravel_to_flint", "cheaper_netherite_alloy", "wither_bone_drop", "wither_bone_conversion", "slime_recipe_fix", "glass_recipe_fix", "allow_ingotless_alloys", "force_integration_materials", "slimy_loot_chests", "sync_knockback_resistance"] {
            config.insert(key.to_owned(), true);
        }
        let registries = ["minecraft:item", "minecraft:fluid"].into_iter()
            .map(|id| (id.to_owned(), Default::default())).collect();
        let mut count = 0;
        for index in 0..archive.len() {
            let mut file = archive.by_index(index).unwrap();
            if !file.name().contains("/materials/definition/") { continue; }
            let name = file.name().to_owned();
            let value: serde_json::Value = serde_json::from_reader(&mut file).unwrap();
            source_material_definition(&value, &registries, &config, true).unwrap_or_else(|e| panic!("{name}: {e:?}"));
            count += 1;
        }
        assert_eq!(count, 104);
    }

    #[test]
    fn original_material_traits_replace_lists_and_reset_per_stat() {
        let files = [br#"{"default":[{"name":"test:a","level":1}],"perStat":{"test:head":[{"name":"test:b","level":1}]}}"#.to_vec(),
                     br#"{"default":null,"perStat":{"test:head":null,"test:handle":[{"name":"test:c","level":2}]}}"#.to_vec()];
        let traits = source_merge_material_traits(&files).unwrap();
        assert_eq!(traits.default.unwrap()[0]["name"], "test:a");
        assert!(!traits.per_stat.contains_key("test:head"));
        assert_eq!(traits.per_stat["test:handle"][0]["level"], 2);
    }

    #[test]
    fn original_material_stats_merge_fields_and_remove_types() {
        let files = [br#"{"stats":{"tconstruct:head":{"durability":60,"mining_speed":2.0},"tconstruct:binding":{}}}"#.to_vec(),
                     br#"{"stats":{"tconstruct:head":{"durability":80},"tconstruct:binding":null,"tconstruct:handle":{}}}"#.to_vec()];
        let stats = source_merge_material_stats(&files).unwrap();
        assert_eq!(stats["tconstruct:head"]["durability"], 80);
        assert_eq!(stats["tconstruct:head"]["mining_speed"], 2.0);
        assert!(!stats.contains_key("tconstruct:binding"));
        assert!(stats.contains_key("tconstruct:handle"));
        assert_eq!(source_merge_material_stats(&[br#"{"stats":{"tconstruct:head":7}}"#.to_vec()]), Err(SourceLookupError::UnsupportedRecipe));
    }

    #[test]
    fn original_material_definition_defaults_conditions_and_redirects() {
        let registries = Default::default();
        let mut config = std::collections::HashMap::new();
        let wood = serde_json::json!({"craftable":true,"hidden":false,"sortOrder":0,"tier":0});
        assert_eq!(source_material_definition(&wood, &registries, &config, true), Ok(SourceMaterialDefinition::Material { craftable:true, hidden:false, tier:0, sort_order:0, rarity:"common".into() }));
        let redirect = serde_json::json!({"redirect":[{"id":"tconstruct:rose_gold"}]});
        assert_eq!(source_material_definition(&redirect, &registries, &config, true), Ok(SourceMaterialDefinition::Redirect("tconstruct:rose_gold".into())));
        let conditional = serde_json::json!({"condition":{"type":"tconstruct:config","prop":"force_integration_materials"},"craftable":false,"tier":5});
        assert_eq!(source_material_definition(&conditional, &registries, &config, true), Err(SourceLookupError::UnknownCondition));
        config.insert("force_integration_materials".into(), false);
        assert_eq!(source_material_definition(&conditional, &registries, &config, true), Ok(SourceMaterialDefinition::Disabled));
        config.insert("force_integration_materials".into(), true);
        assert_eq!(source_material_definition(&conditional, &registries, &config, true), Ok(SourceMaterialDefinition::Material { craftable:false, hidden:false, tier:5, sort_order:100, rarity:"epic".into() }));
    }

    #[test]
    fn source_world_material_registry_changes_with_creosote_condition() {
        use std::io::Read;
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors().map(|dir| dir.join("content/reference/tconstruct-materials.zip"))
            .find(|path| path.is_file()).expect("pinned material data");
        let mut archive = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
        let mut files = std::collections::BTreeMap::new();
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();
            let name = file.name().to_owned();
            if let Some(path) = name.strip_prefix("data/tconstruct/tinkering/materials/definition/").and_then(|s| s.strip_suffix(".json")) {
                let mut data = Vec::new(); file.read_to_end(&mut data).unwrap();
                files.insert(format!("tconstruct:tinkering/materials/definition/{path}"), vec![data]);
            }
        }
        let mut registries = [("minecraft:item".to_owned(), Default::default()), ("minecraft:fluid".to_owned(), Default::default())].into_iter().collect::<std::collections::HashMap<String,std::collections::HashMap<String,std::collections::HashSet<String>>>>();
        let config = ["spawn_with_book", "gravel_to_flint", "cheaper_netherite_alloy", "wither_bone_drop", "slimy_loot_chests", "sync_knockback_resistance", "glass_recipe_fix", "allow_ingotless_alloys", "force_integration_materials"].into_iter().map(|key| (key.to_owned(), false)).collect();
        let (base, _) = source_material_registry(&files, &registries, &config, true).unwrap();
        assert!(base.contains_key("tconstruct:wood"));
        assert!(!base.contains_key("tconstruct:treated_wood"));
        registries.get_mut("minecraft:fluid").unwrap().insert("forge:creosote".into(), ["test:creosote".to_owned()].into_iter().collect());
        let (with_creosote, _) = source_material_registry(&files, &registries, &config, true).unwrap();
        assert!(with_creosote.contains_key("tconstruct:treated_wood"));
    }

    #[test]
    fn source_world_material_registry_uses_last_layer_and_validates_redirects() {
        let mut files = std::collections::BTreeMap::new();
        files.insert("test:tinkering/materials/definition/a".into(), vec![
            br#"{"craftable":false}"#.to_vec(), br#"{"craftable":true}"#.to_vec(),
        ]);
        files.insert("test:tinkering/materials/definition/alias".into(), vec![
            br#"{"redirect":[{"id":"test:a"}]}"#.to_vec(),
        ]);
        files.insert("test:tinkering/materials/definition/bad_alias".into(), vec![
            br#"{"redirect":[{"id":"test:missing"}]}"#.to_vec(),
        ]);
        let (materials, redirects) = source_material_registry(&files, &Default::default(), &Default::default(), true).unwrap();
        assert!(matches!(materials.get("test:a"), Some(SourceMaterialDefinition::Material { craftable:true, .. })));
        assert_eq!(redirects.get("test:alias").map(String::as_str), Some("test:a"));
        assert!(!redirects.contains_key("test:bad_alias"));
        assert_eq!(source_material_registry(&files, &Default::default(), &Default::default(), false), Err(SourceLookupError::IncompleteContext));
    }

    #[test]
    fn source_fake_material_checks_tag_and_visible_base_recipe_ratio() {
        assert!(!source_fake_material_allowed(false, &[], 1.0));
        assert!(source_fake_material_allowed(true, &[], 1.0));
        assert!(!source_fake_material_allowed(true, &[(2, 2)], 1.0));
        assert!(source_fake_material_allowed(true, &[(2, 2)], 9.0));
        assert!(!source_fake_material_allowed(true, &[(18, 2)], 9.0));
    }

    #[test]
    fn source_casting_costs_use_loaded_recipe_results_and_minimum() {
        use std::io::Read;
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).ancestors()
            .map(|p| p.join("content/reference/tconstruct-recipes.zip"))
            .find(|p| p.is_file()).expect("pinned recipe archive");
        let mut zip = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
        let mut recipes = std::collections::BTreeMap::new();
        let mut items = std::collections::HashSet::new();
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i).unwrap();
            let mut bytes = Vec::new(); entry.read_to_end(&mut bytes).unwrap();
            let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else { continue; };
            if !matches!(value.get("type").and_then(serde_json::Value::as_str), Some("tconstruct:table_casting_material" | "tconstruct:basin_casting_material")) { continue; }
            items.insert(value["result"].as_str().unwrap().to_owned());
            recipes.insert(format!("test:recipe_{i}"), bytes);
        }
        assert_eq!(recipes.len(), 43);
        let costs = source_material_casting_costs(&recipes, &items, &Default::default(), &Default::default(), true).unwrap();
        assert_eq!(costs.get("tconstruct:pick_head"), Some(&2));
        assert_eq!(costs.get("tconstruct:tool_handle"), Some(&1));
        recipes.insert("test:override".into(), br#"{"type":"tconstruct:table_casting_material","result":"tconstruct:pick_head","item_cost":1}"#.to_vec());
        let costs = source_material_casting_costs(&recipes, &items, &Default::default(), &Default::default(), true).unwrap();
        assert_eq!(costs.get("tconstruct:pick_head"), Some(&1));
        assert_eq!(source_material_casting_costs(&recipes, &Default::default(), &Default::default(), &Default::default(), true), Err(SourceLookupError::IncompleteContext));
        recipes.insert("test:conditional".into(), br#"{"type":"tconstruct:table_casting_material","result":"test:unregistered","item_cost":1,"conditions":[{"type":"tconstruct:config","prop":"force_integration_materials"}]}"#.to_vec());
        assert_eq!(source_material_casting_costs(&recipes, &items, &Default::default(), &Default::default(), true), Err(SourceLookupError::UnknownCondition));
        let config = [("force_integration_materials".to_owned(), false)].into_iter().collect();
        assert!(source_material_casting_costs(&recipes, &items, &Default::default(), &config, true).is_ok());
    }

    #[test]
    fn source_part_support_uses_validated_stat_types_and_repairability() {
        let stats = ["tconstruct:head", "tconstruct:handle", "tconstruct:binding"]
            .into_iter().map(str::to_owned).collect();
        let parts = source_supported_original_parts(&stats, true);
        assert!(parts.contains("tconstruct:pick_head"));
        assert!(parts.contains("tconstruct:tool_handle"));
        assert!(parts.contains("tconstruct:tool_binding"));
        assert!(parts.contains("tconstruct:repair_kit"));
        assert!(!parts.contains("tconstruct:bow_limb"));
        assert!(!parts.contains("tconstruct:helmet_plating"));
        let only_repair_kit = ["tconstruct:repair_kit".to_owned()].into_iter().collect();
        assert_eq!(source_supported_original_parts(&only_repair_kit, false),
            ["tconstruct:repair_kit".to_owned()].into_iter().collect());
        assert!(source_supported_original_parts(&Default::default(), false).is_empty());
    }

    #[test]
    fn source_tool_parts_hook_uses_overrides_and_validated_modules() {
        use std::io::Read;
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).ancestors()
            .map(|p| p.join("content/reference/tconstruct-tool-definitions.zip"))
            .find(|p| p.is_file()).expect("pinned tool definitions archive");
        let mut archive = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
        let mut definitions = std::collections::HashMap::new();
        let mut defaults = std::collections::HashMap::new();
        let mut parts = std::collections::HashSet::new();
        for index in 0..archive.len() {
            let mut file = archive.by_index(index).unwrap();
            let name = file.name().to_owned(); let mut bytes = Vec::new(); file.read_to_end(&mut bytes).unwrap();
            let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            for module in value["modules"].as_array().unwrap() {
                let typ = module["type"].as_str().unwrap();
                let supplies = matches!(typ, "tconstruct:part_stats" | "tconstruct:tool_parts");
                defaults.insert(typ.to_owned(), supplies);
                if supplies { for part in module["parts"].as_array().unwrap() {
                    parts.insert(part.as_str().or_else(|| part["item"].as_str()).unwrap().to_owned());
                }}
            }
            definitions.insert(name, bytes);
        }
        assert_eq!(definitions.len(), 45);
        let get = |name: &str| vec![definitions[&format!("data/tconstruct/tinkering/tool_definitions/{name}.json")].clone()];
        assert_eq!(source_tool_parts_hook(&get("pickaxe"), true, &defaults, &parts).unwrap(), vec!["tconstruct:pick_head", "tconstruct:tool_handle", "tconstruct:tool_binding"]);
        assert_eq!(source_tool_parts_hook(&get("broad_axe"), true, &defaults, &parts).unwrap(), vec!["tconstruct:broad_axe_head", "tconstruct:tough_handle", "tconstruct:pick_head", "tconstruct:tough_binding"]);
        assert_eq!(source_tool_parts_hook(&get("plate_shield"), true, &defaults, &parts).unwrap(), vec!["tconstruct:shield_core"]);
        assert_eq!(source_tool_parts_hook(&get("pickaxe"), false, &defaults, &parts), Err(SourceLookupError::IncompleteContext));
        let mut missing = parts.clone(); missing.remove("tconstruct:pick_head");
        assert_eq!(source_tool_parts_hook(&get("pickaxe"), true, &defaults, &missing), Err(SourceLookupError::IncompleteContext));
        let disabled = serde_json::to_vec(&serde_json::json!({"modules":[{"type":"tconstruct:tool_parts", "parts":["tconstruct:pick_head"], "hooks":["tconstruct:tool_stats"]}]})).unwrap();
        assert!(source_tool_parts_hook(&[get("pickaxe")[0].clone(), disabled], true, &defaults, &parts).unwrap().is_empty());
        let duplicate = serde_json::to_vec(&serde_json::json!({"modules":[{"type":"tconstruct:tool_parts", "parts":["tconstruct:pick_head"]},{"type":"tconstruct:tool_parts", "parts":["tconstruct:tool_handle"]}]})).unwrap();
        assert_eq!(source_tool_parts_hook(&[duplicate], true, &defaults, &parts), Err(SourceLookupError::UnsupportedRecipe));
        let identical = serde_json::to_vec(&serde_json::json!({"modules":[{"type":"tconstruct:tool_parts", "parts":["tconstruct:pick_head"]},{"type":"tconstruct:tool_parts", "parts":["tconstruct:pick_head"]}]})).unwrap();
        assert_eq!(source_tool_parts_hook(&[identical], true, &defaults, &parts).unwrap(), vec!["tconstruct:pick_head"]);
    }

    #[test]
    fn pinned_vanilla_recycling_preserves_patterns_damage_and_conditions() {
        use std::io::Read;
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).ancestors()
            .map(|p| p.join("content/reference/tconstruct-recipes.zip"))
            .find(|p| p.is_file()).expect("pinned recipes archive");
        let mut archive = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
        let mut recipes = std::collections::BTreeMap::new();
        let mut tool_recipes = std::collections::BTreeMap::new();
        let mut raw_recipes = std::collections::BTreeMap::new();
        for index in 0..archive.len() {
            let mut file = archive.by_index(index).unwrap();
            let name = file.name().to_owned();
            let Some(path) = name.strip_prefix("data/tconstruct/recipes/").and_then(|p| p.strip_suffix(".json")) else { continue; };
            let mut bytes = Vec::new(); file.read_to_end(&mut bytes).unwrap();
            let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else { continue; };
            raw_recipes.insert(format!("tconstruct:{path}"), bytes);
            if value.get("type").and_then(serde_json::Value::as_str) == Some("tconstruct:part_builder_tool_recycling") {
                let id = format!("tconstruct:{path}");
                tool_recipes.insert(id.clone(), SourceToolRecyclingRecipe::parse(id, &value).expect(&name));
            }
            if value.get("type").and_then(serde_json::Value::as_str) == Some("tconstruct:part_builder_recycling") {
                let id = format!("tconstruct:{path}");
                recipes.insert(id.clone(), SourceRecyclingRecipe::parse(id, &value).expect(&name));
            }
        }
        let actual_choices = source_all_part_choices(&raw_recipes, "tconstruct:venom_bottle", &SourcePartInput::Invalid, "minecraft:flint_and_steel", 1, &Default::default(), &Default::default(), &Default::default(), &Default::default(), &Default::default(), Some(&Default::default()), None, true).unwrap();
        assert_eq!(actual_choices.iter().map(|choice| choice.pattern.as_str()).collect::<Vec<_>>(), vec!["tconstruct:ingot", "tconstruct:shard"]);
        assert_eq!(recipes.len(), 37);
        assert_eq!(tool_recipes.len(), 15);
        let material_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).ancestors()
            .map(|p| p.join("content/reference/tconstruct-materials.zip"))
            .find(|p| p.is_file()).expect("pinned material archive");
        let mut material_archive = zip::ZipArchive::new(std::fs::File::open(material_path).unwrap()).unwrap();
        let tier_names = ["wood", "stone", "iron", "gold", "diamond", "netherite"].into_iter().map(|id| format!("minecraft:{id}")).collect();
        let tiers = super::super::tconstruct_source_stat_types::TierRegistry { names: &tier_names, complete: false };
        let mut supported: std::collections::HashMap<String, std::collections::HashSet<String>> = Default::default();
        for id in ["flint", "wood"] {
            let mut file = material_archive.by_name(&format!("data/tconstruct/tinkering/materials/stats/{id}.json")).unwrap();
            let mut bytes = Vec::new(); file.read_to_end(&mut bytes).unwrap();
            let merged = source_merge_material_stats(&[bytes]).unwrap();
            let validated = super::super::tconstruct_source_stat_types::validate_builtin_stats(&merged, Some(&tiers)).unwrap();
            let stat_ids = validated.keys().cloned().collect();
            supported.insert(format!("tconstruct:{id}"), source_supported_original_parts(&stat_ids, validated.values().any(|repairable| *repairable)));
        }
        supported.insert("tconstruct:unknown".into(), Default::default());
        let travelers = tool_recipes.values().find(|r| r.parts == ["tconstruct:repair_kit", "tconstruct:repair_kit"]).unwrap();
        assert_eq!(source_tool_recycling_patterns(travelers, None, &["tconstruct:flint".into(), "tconstruct:wood#warped".into()], &supported, true).unwrap(), vec!["tconstruct:repair_kit"]);
        assert_eq!(source_tool_recycling_patterns(travelers, None, &[], &supported, true).unwrap(), Vec::<String>::new());
        assert_eq!(source_tool_recycling_patterns(travelers, None, &[], &Default::default(), true), Err(SourceLookupError::IncompleteContext));
        let dagger = tool_recipes.values().find(|recipe| recipe.amount_needed == 2).unwrap();
        assert!(!dagger.partial_match("tconstruct:venom_bottle", "tconstruct:dagger", 1, &Default::default(), &Default::default(), &Default::default()));
        assert!(dagger.partial_match("tconstruct:venom_bottle", "tconstruct:dagger", 2, &Default::default(), &Default::default(), &Default::default()));
        let tool_parts = ["tconstruct:pick_head".to_owned(), "tconstruct:tool_handle".to_owned(), "tconstruct:tool_binding".to_owned()];
        let materials = ["tconstruct:flint".to_owned(), "tconstruct:wood#warped".to_owned(), "tconstruct:wood".to_owned()];
        assert_eq!(source_tool_recycling_patterns(dagger, Some(&tool_parts), &materials, &supported, true).unwrap(), tool_parts);
        assert_eq!(source_tool_recycling_patterns(dagger, None, &materials, &supported, true), Err(SourceLookupError::IncompleteContext));
        // Controlled end-to-end source-data context: original definition,
        // original material stat loaders and original general recycling recipe.
        let tool_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).ancestors()
            .map(|p| p.join("content/reference/tconstruct-tool-definitions.zip"))
            .find(|p| p.is_file()).expect("pinned tool archive");
        let mut tool_archive = zip::ZipArchive::new(std::fs::File::open(tool_path).unwrap()).unwrap();
        let mut file = tool_archive.by_name("data/tconstruct/tinkering/tool_definitions/pickaxe.json").unwrap();
        let mut definition = Vec::new(); file.read_to_end(&mut definition).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&definition).unwrap();
        let defaults = json["modules"].as_array().unwrap().iter().map(|module| {
            let id = module["type"].as_str().unwrap();
            (id.to_owned(), matches!(id, "tconstruct:part_stats" | "tconstruct:tool_parts"))
        }).collect();
        let registered = tool_parts.iter().cloned().collect();
        let hooks = source_tool_parts_hook(&[definition], true, &defaults, &registered).unwrap();
        let general = &tool_recipes["tconstruct:tools/recycling/general"];
        let patterns = source_tool_recycling_patterns(general, Some(&hooks), &materials, &supported, true).unwrap();
        let pattern_context = [(general.id.clone(), patterns)].into_iter().collect();
        let tags = [("tconstruct:modifiable/multipart".to_owned(), ["tconstruct:pickaxe".to_owned()].into_iter().collect())].into_iter().collect();
        let choices = source_all_part_choices(&raw_recipes, "tconstruct:venom_bottle", &SourcePartInput::Invalid, "tconstruct:pickaxe", 1, &tags, &Default::default(), &Default::default(), &Default::default(), &Default::default(), Some(&Default::default()), Some(&pattern_context), true).unwrap();
        assert_eq!(choices.iter().map(|choice| choice.pattern.as_str()).collect::<Vec<_>>(), vec!["tconstruct:pick_head", "tconstruct:tool_binding", "tconstruct:tool_handle"]);
        let recipe = &recipes["tconstruct:tables/recycling/flint_and_steel"];
        let tags = Default::default(); let instruments = Default::default(); let instrument_tags = Default::default();
        assert!(recipe.partial_match("tconstruct:venom_bottle", "minecraft:flint_and_steel", &tags, &instruments, &instrument_tags));
        assert!(!recipe.partial_match("tconstruct:pattern", "minecraft:flint_and_steel", &tags, &instruments, &instrument_tags));
        assert_eq!(recipe.maximum_amount(64, 0), Ok(2));
        assert_eq!(recipe.maximum_amount(64, 1), Ok(1));
        assert_eq!(recipe.maximum_amount(64, 32), Ok(1));
        assert_eq!(recipe.maximum_amount(64, 33), Ok(0));
        assert_eq!(recipe.maximum_amount(0, 0), Ok(2));
        assert_eq!(recipe.output("tconstruct:shard", 64, 0).unwrap().unwrap().kind, SourceOutputKind::Item("minecraft:flint".into()));
        assert_eq!(recipe.output("tconstruct:shard", 64, 33).unwrap(), None);
        assert_eq!(recipe.matches(true, 64, 0, false, false, false), Ok(true));
        assert_eq!(recipe.matches(true, 64, 0, false, false, true), Ok(false));
        assert_eq!(recipe.matches(true, 64, 0, true, true, false), Ok(false));
        assert_eq!(recipe.matches(false, 64, 0, false, false, false), Ok(true));
        let conditional = &recipes["tconstruct:tables/recycling/twilightforest/arctic_boots"];
        assert_eq!(conditional.enabled(&Default::default(), &Default::default(), None), None);
        assert_eq!(conditional.enabled(&Default::default(), &Default::default(), Some(&Default::default())), Some(false));
        let mods = ["twilightforest".to_owned()].into_iter().collect();
        assert_eq!(conditional.enabled(&Default::default(), &Default::default(), Some(&mods)), Some(true));
    }

    #[test]
    fn all_part_choices_include_recycling_and_require_tool_context() {
        let entry = |value: serde_json::Value| serde_json::to_vec(&value).unwrap();
        let recipes = [
            ("test:recycle".to_owned(), entry(serde_json::json!({
                "type":"tconstruct:part_builder_recycling", "pattern":{"item":"tconstruct:venom_bottle"},
                "tool":{"item":"minecraft:flint_and_steel"},
                "results":{"tconstruct:shard":{"item":"minecraft:flint"},"tconstruct:ingot":{"item":"minecraft:iron_ingot"}}
            }))),
            ("test:tool".to_owned(), entry(serde_json::json!({
                "type":"tconstruct:part_builder_tool_recycling", "pattern":{"item":"tconstruct:venom_bottle"},
                "tools":{"item":"tconstruct:dagger","amount_needed":2}
            }))),
        ].into_iter().collect();
        let tags = Default::default(); let instruments = Default::default(); let instrument_tags = Default::default();
        let registries = Default::default(); let config = Default::default(); let mods = Default::default();
        let get = |tool: &str, count, patterns| source_all_part_choices(&recipes, "tconstruct:venom_bottle", &SourcePartInput::Invalid, tool, count, &tags, &instruments, &instrument_tags, &registries, &config, Some(&mods), patterns, true);
        let choices = get("minecraft:flint_and_steel", 1, None).unwrap();
        assert_eq!(choices.iter().map(|x| x.pattern.as_str()).collect::<Vec<_>>(), vec!["tconstruct:ingot", "tconstruct:shard"]);
        assert!(choices.iter().all(|choice| choice.cost == 0));
        assert!(get("tconstruct:dagger", 1, None).unwrap().is_empty());
        assert_eq!(get("tconstruct:dagger", 2, None).unwrap_err(), SourceLookupError::IncompleteContext);
        let patterns = [("test:tool".to_owned(), vec!["tconstruct:repair_kit".to_owned()])].into_iter().collect();
        assert_eq!(get("tconstruct:dagger", 2, Some(&patterns)).unwrap()[0].pattern, "tconstruct:repair_kit");
    }

    #[test]
    fn fake_item_cache_uses_only_visible_enabled_base_variant_recipes() {
        let entry = |material: &str, value: i32, needed: i32| serde_json::to_vec(&serde_json::json!({
            "type":"tconstruct:material", "ingredient":{"item":"minecraft:flint"},
            "material":material, "value":value, "needed":needed
        })).unwrap();
        let recipes = [
            ("test:base".to_owned(), entry("test:bronze", 2, 2)),
            ("test:variant".to_owned(), entry("test:bronze#dark", 9, 1)),
            ("test:other".to_owned(), entry("test:copper", 9, 1)),
            ("test:disabled".to_owned(), serde_json::to_vec(&serde_json::json!({
                "type":"tconstruct:material", "material":"test:bronze", "ingredient":42,
                "conditions":[{"type":"tconstruct:config","prop":"spawn_with_book"}]
            })).unwrap()),
        ].into_iter().collect();
        let visible = ["test:bronze".to_owned()].into_iter().collect();
        let known = ["minecraft:flint".to_owned()].into_iter().collect();
        let registries = Default::default(); let config = [("spawn_with_book".to_owned(), false)].into_iter().collect();
        assert_eq!(source_visible_base_recipe_values("test:bronze", &recipes, &visible, &known, &registries, &config, true), Ok(vec![(2,2)]));
        assert_eq!(source_visible_base_recipe_values("test:other", &recipes, &visible, &known, &registries, &config, true), Ok(Vec::new()));
        assert_eq!(source_visible_base_recipe_values("test:bronze#dark", &recipes, &visible, &known, &registries, &config, true), Err(SourceLookupError::UnsupportedRecipe));
        assert_eq!(source_visible_base_recipe_values("test:bronze", &recipes, &visible, &known, &registries, &config, false), Err(SourceLookupError::IncompleteContext));
        assert_eq!(source_visible_base_recipe_values("test:bronze", &recipes, &visible, &Default::default(), &registries, &config, true), Err(SourceLookupError::IncompleteContext));
        let mut malformed = recipes.clone();
        malformed.insert("test:bad_leftover".into(), serde_json::to_vec(&serde_json::json!({
            "type":"tconstruct:material", "ingredient":{"item":"minecraft:flint"},
            "material":"test:bronze", "value":1, "leftover":42
        })).unwrap());
        assert_eq!(source_visible_base_recipe_values("test:bronze", &malformed, &visible, &known, &registries, &config, true), Err(SourceLookupError::UnsupportedRecipe));
    }

    #[test]
    fn original_bronze_recipes_enable_and_disable_fake_forms() {
        use std::io::Read;
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).ancestors()
            .map(|p| p.join("content/reference/tconstruct-recipes.zip"))
            .find(|p| p.is_file()).expect("pinned recipes archive");
        let mut archive = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
        let mut recipes = std::collections::BTreeMap::new();
        for index in 0..archive.len() {
            let mut file = archive.by_index(index).unwrap();
            let name = file.name().to_owned();
            if let Some(path) = name.strip_prefix("data/tconstruct/recipes/").and_then(|p| p.strip_suffix(".json")) {
                let mut bytes = Vec::new(); file.read_to_end(&mut bytes).unwrap();
                recipes.insert(format!("tconstruct:{path}"), bytes);
            }
        }
        let visible = ["tconstruct:bronze".to_owned()].into_iter().collect();
        let material_tags = [
            ("tconstruct:compatibility_metals".to_owned(), ["tconstruct:bronze".to_owned()].into_iter().collect()),
            ("tconstruct:compatibility_metals/blocks".to_owned(), ["tconstruct:bronze".to_owned()].into_iter().collect()),
        ].into_iter().collect();
        let mut item_tags = std::collections::HashMap::new();
        let config = Default::default();
        let known = std::collections::HashSet::new();
        for (ingots, blocks, expected) in [(false,false,2), (true,false,1), (true,true,0)] {
            if ingots { item_tags.insert("forge:ingots/bronze".to_owned(), ["test:ingot".to_owned()].into_iter().collect()); }
            if blocks { item_tags.insert("forge:storage_blocks/bronze".to_owned(), ["test:block".to_owned()].into_iter().collect()); }
            let registries = [("minecraft:item".to_owned(), item_tags.clone())].into_iter().collect();
            let values = source_visible_base_recipe_values("tconstruct:bronze", &recipes, &visible, &known, &registries, &config, true).unwrap();
            assert_eq!(source_supported_fake_items("tconstruct:bronze", &material_tags, &values, true).unwrap().len(), expected);
        }
    }

    #[test]
    fn fake_ingot_and_storage_block_use_distinct_source_tags_and_values() {
        let tags = [
            ("tconstruct:compatibility_metals".to_owned(), ["test:bronze".to_owned()].into_iter().collect()),
            ("tconstruct:compatibility_metals/blocks".to_owned(), ["test:bronze".to_owned()].into_iter().collect()),
        ].into_iter().collect();
        let both = ["tconstruct:fake_ingot".to_owned(), "tconstruct:fake_storage_block".to_owned()].into_iter().collect();
        assert_eq!(source_supported_fake_items("test:bronze", &tags, &[], true), Ok(both));
        let block_only = ["tconstruct:fake_storage_block".to_owned()].into_iter().collect();
        assert_eq!(source_supported_fake_items("test:bronze", &tags, &[(2, 2)], true), Ok(block_only));
        assert!(source_supported_fake_items("test:bronze", &tags, &[(1, 1), (18, 2)], true).unwrap().is_empty());
        assert!(source_supported_fake_items("test:other", &tags, &[], true).unwrap().is_empty());
        assert_eq!(source_supported_fake_items("test:bronze#variant", &tags, &[], true), Err(SourceLookupError::UnsupportedRecipe));
        assert_eq!(source_supported_fake_items("test:bronze", &tags, &[], false), Err(SourceLookupError::IncompleteContext));
    }

    #[test]
    fn reused_parts_use_casting_cost_and_bypass_normal_material_recipes() {
        assert_eq!(source_reused_part_material(false, true, Some("tconstruct:wood"), Some(2), true), Ok(SourceReusedPart::NotToolPart));
        assert_eq!(source_reused_part_material(true, false, Some("tconstruct:wood"), Some(2), true), Ok(SourceReusedPart::NoMaterial));
        assert_eq!(source_reused_part_material(true, true, Some("tconstruct:unknown#variant"), Some(2), true), Ok(SourceReusedPart::NoMaterial));
        assert_eq!(source_reused_part_material(true, true, Some("tconstruct:wood#Bad"), Some(2), true), Ok(SourceReusedPart::NoMaterial));
        assert_eq!(source_reused_part_material(true, true, Some("tconstruct:wood#warped"), Some(0), true), Ok(SourceReusedPart::NoMaterial));
        assert_eq!(source_reused_part_material(true, true, Some("tconstruct:wood#warped"), Some(2), true), Ok(SourceReusedPart::Material { material: "tconstruct:wood#warped".into(), value: 2, needed: 1 }));
        assert_eq!(source_reused_part_material(true, true, Some("tconstruct:wood"), Some(2), false), Err(SourceLookupError::IncompleteContext));
    }

    #[test]
    fn reused_part_value_yields_source_count_and_material_variant() {
        let recipe = SourcePartRecipe::parse("test:handle".into(), &serde_json::json!({
            "type":"tconstruct:part_builder", "pattern":"tconstruct:tool_handle", "cost":1,
            "result":{"item":"tconstruct:tool_handle"}
        })).unwrap();
        let reuse = source_reused_part_material(true, true, Some("tconstruct:wood#warped"), Some(2), true).unwrap();
        let SourceReusedPart::Material { material, value, needed } = reuse else { panic!("missing reusable material"); };
        let supported = ["tconstruct:tool_handle".to_owned()].into_iter().collect();
        let input = SourcePartInput::Material { input_item:"tconstruct:pick_head", material:&material, craftable:true, supported_outputs:&supported };
        let source = SourceMaterialValue { material:&material, value, needed, leftover:None };
        let plan = source_part_plan_value(&recipe, &input, Some(source), 1, false).unwrap().unwrap();
        assert_eq!(plan.materials_used, 1);
        assert_eq!(plan.pattern_used, 1);
        assert_eq!(plan.leftover, None);
        assert_eq!(plan.output, SourcePartOutput::MaterialItem { item:"tconstruct:tool_handle".into(), material:"tconstruct:wood#warped".into(), count:2 });
        let same_item = SourcePartInput::Material { input_item:"tconstruct:tool_handle", material:&material, craftable:true, supported_outputs:&supported };
        let source = SourceMaterialValue { material:&material, value, needed, leftover:None };
        assert_eq!(source_part_plan_value(&recipe, &same_item, Some(source), 1, false).unwrap(), None);
        let source = SourceMaterialValue { material:&material, value, needed, leftover:None };
        assert_eq!(source_part_plan_value(&recipe, &input, Some(source), 0, false).unwrap(), None);
    }

    #[test]
    fn source_part_choices_follow_pinned_patterns_and_partial_matches() {
        use std::io::Read;
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors().map(|dir| dir.join("docs/upstream-recipes.json.gz"))
            .find(|path| path.is_file()).expect("pinned reference catalog");
        let mut json = String::new();
        flate2::read::GzDecoder::new(std::fs::File::open(path).unwrap()).read_to_string(&mut json).unwrap();
        let catalog: serde_json::Value = serde_json::from_str(&json).unwrap();
        let mut recipes = std::collections::BTreeMap::new();
        for entry in catalog["groups"]["part_builder"].as_array().unwrap() {
            let path = entry["source"].as_str().unwrap().split_once("/recipes/").unwrap().1.strip_suffix(".json").unwrap();
            recipes.insert(format!("tconstruct:{path}"), serde_json::to_vec(&entry["data"]).unwrap());
        }
        assert_eq!(recipes.len(), 28);
        let mut tags = std::collections::HashMap::new();
        tags.insert("tconstruct:patterns/default".to_owned(), ["tconstruct:pattern".to_owned()].into_iter().collect());
        let set = Default::default(); let regs = Default::default(); let config = Default::default();
        let choices = source_part_choices(&recipes, "tconstruct:pattern", &SourcePartInput::Empty, &tags, &set, &Default::default(), &regs, &config, true).unwrap();
        assert_eq!(choices.len(), 28);
        assert!(choices.windows(2).all(|pair| (pair[0].cost, source_id_order(&pair[0].pattern)) <= (pair[1].cost, source_id_order(&pair[1].pattern))));
        assert_eq!(choices[0].pattern, "tconstruct:arrow_head");
        let supported = ["tconstruct:pick_head", "tconstruct:tool_handle", "tconstruct:tool_binding"].into_iter().map(str::to_owned).collect();
        let input = SourcePartInput::Material { input_item:"minecraft:oak_planks", material:"tconstruct:wood", craftable:true, supported_outputs:&supported };
        let choices = source_part_choices(&recipes, "tconstruct:pattern", &input, &tags, &set, &Default::default(), &regs, &config, true).unwrap();
        assert_eq!(choices.iter().map(|choice| choice.pattern.as_str()).collect::<Vec<_>>(), ["tconstruct:tool_binding", "tconstruct:tool_handle", "tconstruct:pick_head"]);
        assert_eq!(choices[2].cost, 2); // partial match does not require enough input yet
        assert!(source_part_choices(&recipes, "", &input, &tags, &set, &Default::default(), &regs, &config, true).unwrap().is_empty());
        assert_eq!(source_part_choices(&recipes, "tconstruct:pattern", &input, &tags, &set, &Default::default(), &regs, &config, false).unwrap_err(), SourceLookupError::IncompleteContext);
        assert!(source_variant_matches("tconstruct:wood", "tconstruct:wood#warped"));
        assert!(source_variant_matches("tconstruct:wood#default", "tconstruct:wood"));
        assert!(!source_variant_matches("tconstruct:wood#warped", "tconstruct:wood"));
    }

    #[test]
    fn source_part_choices_deduplicate_by_original_resource_order() {
        let mut recipes = std::collections::BTreeMap::new();
        recipes.insert("z:a".into(), br#"{"type":"tconstruct:part_builder","pattern":"test:part","cost":3,"result":{"item":"test:first"}}"#.to_vec());
        recipes.insert("a:z".into(), br#"{"type":"tconstruct:part_builder","pattern":"test:part","cost":1,"result":{"item":"test:second"}}"#.to_vec());
        let mut tags = std::collections::HashMap::new();
        tags.insert("tconstruct:patterns/default".to_owned(), ["tconstruct:pattern".to_owned()].into_iter().collect());
        let choices = source_part_choices(&recipes, "tconstruct:pattern", &SourcePartInput::Empty, &tags, &Default::default(), &Default::default(), &Default::default(), &Default::default(), true).unwrap();
        assert_eq!(choices.len(), 1);
        assert_eq!((choices[0].id.as_str(), choices[0].cost), ("z:a", 3));
    }

    #[test]
    fn source_part_choices_handle_uncraftable_materials_and_item_recipe_variants() {
        let mut recipes = std::collections::BTreeMap::new();
        recipes.insert("tconstruct:normal".into(), br#"{"type":"tconstruct:part_builder","pattern":"tconstruct:normal","cost":2,"result":{"item":"tconstruct:pick_head"}}"#.to_vec());
        recipes.insert("tconstruct:allowed".into(), br#"{"type":"tconstruct:part_builder","pattern":"tconstruct:allowed","cost":1,"allow_uncraftable":true,"result":{"item":"tconstruct:arrow_head"}}"#.to_vec());
        recipes.insert("tconstruct:fixed".into(), br#"{"type":"tconstruct:item_part_builder","material":"tconstruct:rock","pattern":"tconstruct:fixed","cost":2,"result":"tconstruct:helmet_plating_dummy"}"#.to_vec());
        let mut tags = std::collections::HashMap::new();
        tags.insert("tconstruct:patterns/default".to_owned(), ["tconstruct:pattern".to_owned()].into_iter().collect());
        let supported = ["tconstruct:pick_head", "tconstruct:arrow_head"].into_iter().map(str::to_owned).collect();
        let input = SourcePartInput::Material { input_item:"minecraft:stone", material:"tconstruct:rock#granite", craftable:false, supported_outputs:&supported };
        let choices = source_part_choices(&recipes, "tconstruct:pattern", &input, &tags, &Default::default(), &Default::default(), &Default::default(), &Default::default(), true).unwrap();
        assert_eq!(choices.iter().map(|choice| choice.pattern.as_str()).collect::<Vec<_>>(), ["tconstruct:allowed", "tconstruct:fixed"]);
        let input = SourcePartInput::Material { input_item:"tconstruct:arrow_head", material:"tconstruct:rock#granite", craftable:false, supported_outputs:&supported };
        let choices = source_part_choices(&recipes, "tconstruct:pattern", &input, &tags, &Default::default(), &Default::default(), &Default::default(), &Default::default(), true).unwrap();
        assert_eq!(choices.iter().map(|choice| choice.pattern.as_str()).collect::<Vec<_>>(), ["tconstruct:fixed"]);
    }

    #[test]
    fn source_part_plan_calculates_cost_remainders_and_reusable_patterns() {
        let recipe = SourcePartRecipe::parse("tconstruct:pick_head".into(), &serde_json::json!({"type":"tconstruct:part_builder","pattern":"tconstruct:pick_head","cost":2,"result":{"item":"tconstruct:pick_head"}})).unwrap();
        let supported = ["tconstruct:pick_head".to_owned()].into_iter().collect();
        let wood = SourcePartInput::Material { input_item:"minecraft:oak_log", material:"tconstruct:wood", craftable:true, supported_outputs:&supported };
        let source = SourceMaterialMatch { id:"test:oak_log".into(), material:"tconstruct:wood".into(), value:4, needed:1, leftover:SourceItemOutput::parse(&serde_json::json!("minecraft:oak_planks")) };
        let plan = source_part_plan(&recipe, &wood, Some(&source), 1, false).unwrap().unwrap();
        assert_eq!((plan.materials_used, plan.pattern_used), (1, 1));
        assert_eq!(plan.leftover.as_ref().map(|(_, count)| *count), Some(2));
        assert_eq!(plan.output, SourcePartOutput::MaterialItem { item:"tconstruct:pick_head".into(), material:"tconstruct:wood".into(), count:1 });
        let registry = ["tconstruct:pick_head".to_owned(), "minecraft:oak_planks".to_owned()].into_iter().collect();
        let (output, leftover) = source_resolve_part_plan(&plan, &Default::default(), None, &registry).unwrap().unwrap();
        assert_eq!((&*output.item, output.count), ("tconstruct:pick_head", 1));
        assert_eq!(output.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|tag| tag.get("Material")), Some(&horizon_world::nbt::Tag::String("tconstruct:wood".into())));
        assert_eq!(leftover.as_ref().map(|s| (s.item.to_string(), s.count)), Some(("minecraft:oak_planks".into(), 2)));
        assert_eq!(source_resolve_part_plan(&plan, &Default::default(), None, &Default::default()), Err(SourceLookupError::IncompleteContext));
        let original = vec![Some(ItemStack::new("tconstruct:pattern", 1)), Some(ItemStack::new("minecraft:oak_log", 2))];
        let mut slots = original.clone();
        let mut full_cursor = Some(ItemStack::new("minecraft:stone", 64));
        assert!(source_take_part_plan(&mut slots, &mut full_cursor, &plan, output.clone(), leftover.clone(), None, None, 64).is_none());
        assert_eq!(slots, original);
        assert_eq!(full_cursor.unwrap().count, 64);
        let mut cursor = None;
        let gave = source_take_part_plan(&mut slots, &mut cursor, &plan, output, leftover, None, None, 64).unwrap();
        assert_eq!(slots[0], None);
        assert_eq!(slots[1].as_ref().unwrap().count, 1);
        assert_eq!(cursor.as_ref().unwrap().item.as_ref(), "tconstruct:pick_head");
        assert_eq!((&*gave[0].item, gave[0].count), ("minecraft:oak_planks", 2));
        let after = slots.clone();
        let (second, leftover) = source_resolve_part_plan(&plan, &Default::default(), None, &registry).unwrap().unwrap();
        assert!(source_take_part_plan(&mut slots, &mut cursor, &plan, second, leftover, None, None, 64).is_none());
        assert_eq!(slots, after);
        let (output, leftover) = source_resolve_part_plan(&plan, &Default::default(), None, &registry).unwrap().unwrap();
        let mut slots = original;
        let mut cursor = None;
        let gave = source_take_part_plan(&mut slots, &mut cursor, &plan, output, leftover,
            Some(ItemStack::new("minecraft:bucket", 1)), Some(ItemStack::new("minecraft:glass_bottle", 1)), 64).unwrap();
        assert_eq!(slots[0].as_ref().map(|s| &*s.item), Some("minecraft:glass_bottle"));
        assert_eq!(gave.iter().map(|s| &*s.item).collect::<Vec<_>>(), ["minecraft:oak_planks", "minecraft:bucket"]);
        assert_eq!(source_part_plan(&recipe, &wood, Some(&source), 0, false).unwrap(), None);
        assert_eq!(source_part_plan(&recipe, &wood, Some(&source), 1, true).unwrap().unwrap().pattern_used, 0);
        let no_leftover = SourceMaterialMatch { leftover:None, ..source };
        let plan = source_part_plan(&recipe, &wood, Some(&no_leftover), 1, false).unwrap().unwrap();
        assert_eq!(plan.output, SourcePartOutput::MaterialItem { item:"tconstruct:pick_head".into(), material:"tconstruct:wood".into(), count:2 });
    }

    #[test]
    fn source_part_plan_handles_no_material_and_item_material_variants() {
        let free = SourcePartRecipe::parse("test:free".into(), &serde_json::json!({"type":"tconstruct:item_part_builder","pattern":"test:free","result":"minecraft:stick"})).unwrap();
        let plan = source_part_plan(&free, &SourcePartInput::Empty, None, 0, false).unwrap().unwrap();
        assert_eq!((plan.materials_used, plan.pattern_used), (0, 1));
        let recipe = SourcePartRecipe::parse("test:rock".into(), &serde_json::json!({"type":"tconstruct:item_part_builder","pattern":"test:rock","material":"tconstruct:rock","cost":2,"result":"minecraft:stone"})).unwrap();
        let supported = Default::default();
        let input = SourcePartInput::Material { input_item:"minecraft:stone", material:"tconstruct:rock#granite", craftable:false, supported_outputs:&supported };
        let source = SourceMaterialMatch { id:"test:stone".into(), material:"tconstruct:rock#granite".into(), value:1, needed:1, leftover:None };
        assert!(source_part_plan(&recipe, &input, Some(&source), 1, false).unwrap().is_none());
        assert_eq!(source_part_plan(&recipe, &input, Some(&source), 2, false).unwrap().unwrap().materials_used, 2);
    }

    #[test]
    fn source_material_lookup_requires_context_and_matches_original_flint() {
        let mut recipes = std::collections::BTreeMap::new();
        recipes.insert("tconstruct:tools/materials/flint/flint".into(), br#"{"type":"tconstruct:material","ingredient":{"item":"minecraft:flint"},"material":"tconstruct:flint","needed":1,"value":1}"#.to_vec());
        recipes.insert("tconstruct:_metadata".into(), br#"{"type":"tconstruct:material"}"#.to_vec());
        let tags = Default::default();
        let set = Default::default();
        let registries = Default::default();
        let config = Default::default();
        assert_eq!(source_material_lookup(&recipes, "minecraft:flint", None, &tags, &set, &tags, &registries, &config, false), Err(SourceLookupError::IncompleteContext));
        assert_eq!(source_material_lookup(&recipes, "minecraft:flint", None, &tags, &set, &tags, &registries, &config, true).unwrap(), Some(SourceMaterialMatch { id: "tconstruct:tools/materials/flint/flint".into(), material: "tconstruct:flint".into(), value: 1, needed: 1, leftover: None }));
        assert_eq!(source_material_lookup(&recipes, "minecraft:stick", None, &tags, &set, &tags, &registries, &config, true).unwrap(), None);
    }

    #[test]
    fn source_material_lookup_does_not_guess_unknown_conditions() {
        let ids = vec!["test:first".to_owned(), "test:second".to_owned()];
        let order = super::super::tconstruct_recipe_order::java_recipe_order(&ids).unwrap();
        let mut recipes = std::collections::BTreeMap::new();
        recipes.insert(order[0].clone(), br#"{"type":"tconstruct:material","ingredient":{"item":"minecraft:flint"},"material":"test:conditional","conditions":[{"type":"tconstruct:config","prop":"gravel_to_flint"}]}"#.to_vec());
        recipes.insert(order[1].clone(), br#"{"type":"tconstruct:material","ingredient":{"item":"minecraft:flint"},"material":"test:fallback"}"#.to_vec());
        let tags = Default::default();
        let set = Default::default();
        let registries = Default::default();
        let mut config = std::collections::HashMap::new();
        assert_eq!(source_material_lookup(&recipes, "minecraft:flint", None, &tags, &set, &tags, &registries, &config, true), Err(SourceLookupError::UnknownCondition));
        config.insert("gravel_to_flint".into(), false);
        let found = source_material_lookup(&recipes, "minecraft:flint", None, &tags, &set, &tags, &registries, &config, true).unwrap().unwrap();
        assert_eq!((found.id, found.material), (order[1].clone(), "test:fallback".into()));
        config.insert("gravel_to_flint".into(), true);
        let found = source_material_lookup(&recipes, "minecraft:flint", None, &tags, &set, &tags, &registries, &config, true).unwrap().unwrap();
        assert_eq!((found.id, found.material), (order[0].clone(), "test:conditional".into()));
    }

    #[test]
    fn source_material_lookup_reads_pinned_material_recipes() {
        use std::io::Read;
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors().map(|dir| dir.join("docs/upstream-recipes.json.gz"))
            .find(|path| path.is_file()).expect("pinned reference catalog");
        let mut json = String::new();
        flate2::read::GzDecoder::new(std::fs::File::open(path).unwrap()).read_to_string(&mut json).unwrap();
        let catalog: serde_json::Value = serde_json::from_str(&json).unwrap();
        let mut recipes = std::collections::BTreeMap::new();
        for entry in catalog["groups"]["materials"].as_array().unwrap() {
            if entry["data"]["type"] != "tconstruct:material" { continue; }
            let source = entry["source"].as_str().unwrap();
            let path = source.split_once("/recipes/").unwrap().1.strip_suffix(".json").unwrap();
            recipes.insert(format!("tconstruct:{path}"), serde_json::to_vec(&entry["data"]).unwrap());
        }
        assert_eq!(recipes.len(), 217);
        let mut tags = std::collections::HashMap::new();
        tags.insert("minecraft:oak_logs".to_owned(), ["minecraft:oak_log".to_owned()].into_iter().collect());
        tags.insert("minecraft:planks".to_owned(), ["minecraft:oak_planks".to_owned(), "minecraft:warped_planks".to_owned()].into_iter().collect());
        tags.insert("tconstruct:wood_variants/planks".to_owned(), ["minecraft:warped_planks".to_owned()].into_iter().collect());
        let set = Default::default();
        let regs = [("minecraft:item".to_owned(), tags.clone())].into_iter().collect();
        let config = Default::default();
        let flint = source_material_lookup(&recipes, "minecraft:flint", None, &tags, &set, &Default::default(), &regs, &config, true).unwrap().unwrap();
        assert_eq!((flint.material.as_str(), flint.value, flint.needed), ("tconstruct:flint", 1, 1));
        let oak = source_material_lookup(&recipes, "minecraft:oak_log", None, &tags, &set, &Default::default(), &regs, &config, true).unwrap().unwrap();
        assert_eq!((oak.material.as_str(), oak.value, oak.needed), ("tconstruct:wood", 4, 1));
        assert_eq!(oak.leftover.unwrap().count_for_remainder(3, oak.needed), Some(3));
        let oak_planks = source_material_lookup(&recipes, "minecraft:oak_planks", None, &tags, &set, &Default::default(), &regs, &config, true).unwrap().unwrap();
        assert_eq!(oak_planks.material, "tconstruct:wood");
        let warped = source_material_lookup(&recipes, "minecraft:warped_planks", None, &tags, &set, &Default::default(), &regs, &config, true).unwrap().unwrap();
        assert_eq!(warped.material, "tconstruct:wood#warped");
    }

    #[test]
    fn source_material_lookup_preserves_fractional_cost_and_leftover() {
        let mut recipes = std::collections::BTreeMap::new();
        recipes.insert("test:log".into(), br#"{"type":"tconstruct:material","ingredient":{"tag":"minecraft:oak_logs"},"material":"tconstruct:wood","needed":1,"value":4,"leftover":"minecraft:oak_planks"}"#.to_vec());
        let mut tags = std::collections::HashMap::new();
        tags.insert("minecraft:oak_logs".to_owned(), ["minecraft:oak_log".to_owned()].into_iter().collect());
        let set = Default::default();
        let regs = Default::default();
        let config = Default::default();
        let found = source_material_lookup(&recipes, "minecraft:oak_log", None, &tags, &set, &Default::default(), &regs, &config, true).unwrap().unwrap();
        assert_eq!(material_cost(1, found.needed, found.value), Some((1, 3)));
        assert_eq!(found.leftover.unwrap().count_for_remainder(3, found.needed), Some(3));
    }

    #[test]
    fn repair_rechecks_split_inputs_and_never_consumes_excess() {
        let mut slots = vec![None; 27];
        let mut tool = ItemStack::new("tconstruct:flint_pickaxe", 1);
        tool.damage = 85;
        slots[0] = Some(tool);
        slots[1] = Some(ItemStack::new("minecraft:flint", 1));
        slots[2] = Some(ItemStack::new("minecraft:flint", 3));
        let mut cursor = None;
        assert!(take(&mut slots, &mut cursor, true, 25));
        assert_eq!(cursor.unwrap().damage, 0);
        assert!(slots[1].is_none());
        assert_eq!(slots[2].as_ref().unwrap().count, 2);
    }

}

/// The selected recipe has no result unless the current inputs still match.
pub(super) fn part_preview(stacks: &[Option<ItemStack>], selected: i16) -> Option<ItemStack> {
    let mut copy = stacks.to_vec();
    let mut result = None;
    take(&mut copy, &mut result, false, selected + 24).then_some(result).flatten()
}
