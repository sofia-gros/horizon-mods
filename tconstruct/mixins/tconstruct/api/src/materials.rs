//! MaterialManager definition loading, ported from SlimeKnights (MIT; see LICENSE).
//! Conditions are decoded completely before any evaluation, as with MaterialJson's GSON adapter.
use pluto_lib::{Identifier, ResourceManagerHelper, ResourceType};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, RwLock};

pub(crate) const FOLDER: &str = "tinkering/materials/definition";
pub(crate) const TAG_FOLDER: &str = "tinkering/tags/materials";

/// Original MaterialsLoadedEvent: notify only after the complete registry is published.
/// Integrated consumers use the server registry; remote clients use the received registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MaterialSide { Server, RemoteClient }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MaterialsLoaded { pub side: MaterialSide, pub generation: u64 }
pub(crate) static MATERIALS_LOADED: pluto_lib::Event<MaterialsLoaded, ()> = pluto_lib::Event::void();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Rarity { Common, Uncommon, Rare, Epic }
impl Rarity {
    pub fn ordinal(self) -> i32 { self as i32 }
    fn for_tier(tier: i32) -> Self {
        match tier { 3 => Self::Uncommon, 4 => Self::Rare, 5.. => Self::Epic, _ => Self::Common }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Material {
    pub id: Identifier,
    pub tier: i32,
    pub sort_order: i32,
    pub rarity: Rarity,
    pub craftable: bool,
    pub hidden: bool,
}

/// The common condition registry supplies parsing and the captured reload context.
/// Do not replace unavailable registry/config data with fabricated false answers.
pub(crate) trait Conditions {
    type Parsed;
    fn decode(&self, json: &Value) -> Result<Self::Parsed, String>;
    fn test(&self, condition: &Self::Parsed) -> Result<bool, String>;
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Definitions {
    pub materials: BTreeMap<Identifier, Material>,
    pub redirects: BTreeMap<Identifier, Identifier>,
    pub visible: Vec<Identifier>,
    pub tags: BTreeMap<Identifier, Vec<Identifier>>,
    pub reverse_tags: BTreeMap<Identifier, BTreeSet<Identifier>>,
    pub traits: BTreeMap<Identifier, crate::material_traits::MaterialTraits>,
    pub stats: crate::material_stats::MaterialStats,
    /// Server data capture completed, not a claim about client synchronization.
    pub server_loaded: bool,
    pub generation: u64,
    pub snapshot: Option<Arc<[u8]>>,
    pub problems: Vec<String>,
}
impl Definitions {
    pub fn resolve_redirect<'a>(&'a self, id: &'a Identifier) -> &'a Identifier {
        self.redirects.get(id).unwrap_or(id)
    }

    /// Stats/traits do not implicitly resolve aliases in MaterialRegistryImpl.
    pub fn get_stats(&self, material: &Identifier, stat: &Identifier) -> Option<&crate::material_stats::Stats> {
        self.stats.get(material)?.get(stat)
    }
    pub fn stats_or_default(&self, material: &Identifier, stat: &Identifier) -> Option<std::borrow::Cow<'_, crate::material_stats::Stats>> {
        self.get_stats(material, stat).map(std::borrow::Cow::Borrowed)
            .or_else(|| crate::material_stats::default_stats(stat).map(std::borrow::Cow::Owned))
    }
    pub fn get_traits(&self, material: &Identifier, stat: &Identifier) -> &[Option<crate::material_traits::ModifierEntry>] {
        self.traits.get(material).map(|traits| traits.get(stat)).unwrap_or(&[])
    }
    /// Generation-owned lookup avoids the Java FIRST_MATERIALS cache becoming stale.
    pub fn first_with_stat(&self, stat: &Identifier) -> Option<&Material> {
        self.visible.iter().find(|id| self.get_stats(id, stat).is_some()).and_then(|id| self.materials.get(id))
    }

    /// Complete effective batch: a reload never merges with the previous generation.
    pub fn load<C: Conditions>(files: &[(Identifier, Value)], conditions: &C) -> Self {
        let mut result = Self::default();
        for (id, json) in files {
            if !json.is_object() { continue; }
            match Definition::decode(json, conditions).and_then(|definition| definition.select(id, conditions)) {
                Ok(Selection::Material(material)) => { result.materials.insert(id.clone(), material); }
                Ok(Selection::Redirect(target)) => { result.redirects.insert(id.clone(), target); }
                Ok(Selection::Skipped) => {}
                Err(error) => result.problems.push(format!("Could not deserialize material {id}: {error}")),
            }
        }
        result.redirects.retain(|from, to| {
            if result.materials.contains_key(to) { true } else {
                result.problems.push(format!("Invalid material redirect {from}: material {to} does not exist"));
                false
            }
        });
        let mut visible: Vec<_> = result.materials.values().filter(|material| !material.hidden).collect();
        // ResourceLocation compares path before namespace, not its serialized spelling.
        visible.sort_by(|a, b| a.tier.cmp(&b.tier).then(a.sort_order.cmp(&b.sort_order))
            .then(a.id.path().cmp(b.id.path())).then(a.id.namespace().cmp(b.id.namespace())));
        result.visible = visible.into_iter().map(|material| material.id.clone()).collect();
        result
    }
}

#[derive(Clone)]
struct TagEntry { id: Identifier, tag: bool, required: bool, remove: bool }
impl TagEntry {
    fn decode(value: &Value, remove: bool) -> Result<Self, String> {
        let (name, required) = if let Some(name) = value.as_str() { (name, true) } else {
            let object = value.as_object().ok_or("tag entry must be an ID or object")?;
            let name = object.get("id").and_then(Value::as_str).ok_or("tag entry needs an ID")?;
            let required = object.get("required").and_then(codec_boolean).unwrap_or(true);
            (name, required)
        };
        let tag = name.starts_with('#');
        let id = Identifier::parse(if tag { &name[1..] } else { name }).map_err(|e| e.to_string())?;
        Ok(Self { id, tag, required, remove })
    }
}
// DFU 6.0.8 JsonOps BOOL accepts booleans and numeric byteValue != 0;
// OptionalFieldCodec silently substitutes defaults for malformed optional fields.
fn codec_boolean(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(value) => Some(*value),
        Value::Number(value) => Some(number_int(&value.to_string()) as i8 != 0),
        _ => None,
    }
}
fn decode_tag_file(value: &Value) -> Result<(bool, Vec<TagEntry>), String> {
    let object = value.as_object().ok_or("tag must be an object")?;
    let replace = object.get("replace").and_then(codec_boolean).unwrap_or(false);
    let values = object.get("values").and_then(Value::as_array).ok_or("tag needs values array")?;
    let mut entries = values.iter().map(|v| TagEntry::decode(v, false)).collect::<Result<Vec<_>, _>>()?;
    if let Some(remove) = object.get("remove").and_then(Value::as_array) {
        // A partially invalid optional list defaults as a whole, not per entry.
        if let Ok(remove) = remove.iter().map(|v| TagEntry::decode(v, true)).collect::<Result<Vec<_>, _>>() {
            entries.extend(remove);
        }
    }
    Ok((replace, entries))
}

impl Definitions {
    /// TagLoader uses concrete materials (including hidden ones), NOT redirects.
    /// Preserve pack order and insertion order, including remove/re-add behavior.
    pub fn load_tags(&mut self, stacks: &[pluto_lib::JsonResourceStack]) {
        self.tags = load_resource_tags(stacks, |id| self.materials.contains_key(id).then(|| id.clone()), &mut self.problems, "material");
        self.reverse_tags.clear();
        for (tag, members) in &self.tags {
            for member in members { self.reverse_tags.entry(member.clone()).or_default().insert(tag.clone()); }
        }
    }
}

/// Shared inherited TagLoader algorithm. The resolver supplies concrete identity:
/// materials exclude redirects; modifiers resolve aliases to their target object ID.
pub(crate) fn load_resource_tags(
    stacks: &[pluto_lib::JsonResourceStack], resolve: impl Fn(&Identifier) -> Option<Identifier>,
    problems: &mut Vec<String>, kind: &str,
) -> BTreeMap<Identifier, Vec<Identifier>> {
        let mut tags: BTreeMap<Identifier, Vec<Identifier>> = BTreeMap::new();
        let mut files: BTreeMap<Identifier, Vec<TagEntry>> = BTreeMap::new();
        for stack in stacks {
            let entries = files.entry(stack.id.clone()).or_default();
            for (layer, value) in stack.layers.iter().enumerate() {
                match value.as_ref().map_err(Clone::clone).and_then(decode_tag_file) {
                    Ok((replace, new)) => { if replace { entries.clear(); } entries.extend(new); }
                    Err(error) => problems.push(format!("Could not read {kind} tag {} layer {layer}: {error}", stack.id)),
                }
            }
        }
        // DependencySorter installs required edges before optional ones and drops
        // cycle-closing edges. Java's independent HashMap order is unspecified;
        // use ID order as our deterministic tie-break without weakening required refs.
        let mut edges: BTreeMap<Identifier, BTreeSet<Identifier>> = BTreeMap::new();
        fn reaches(edges: &BTreeMap<Identifier, BTreeSet<Identifier>>, from: &Identifier, to: &Identifier) -> bool {
            from == to || edges.get(from).is_some_and(|deps| deps.iter().any(|dep| reaches(edges, dep, to)))
        }
        for required in [true, false] {
            for (id, entries) in &files {
                for entry in entries.iter().filter(|e| e.tag && e.required == required) {
                    if !reaches(&edges, &entry.id, id) { edges.entry(id.clone()).or_default().insert(entry.id.clone()); }
                }
            }
        }
        fn visit(id: &Identifier, edges: &BTreeMap<Identifier, BTreeSet<Identifier>>, seen: &mut BTreeSet<Identifier>, order: &mut Vec<Identifier>) {
            if seen.insert(id.clone()) {
                if let Some(deps) = edges.get(id) { for dep in deps { visit(dep, edges, seen, order); } }
                order.push(id.clone());
            }
        }
        let mut order = Vec::new();
        let mut seen = BTreeSet::new();
        for id in files.keys() { visit(id, &edges, &mut seen, &mut order); }
        for id in order {
            let Some(entries) = files.get(&id) else { continue; };
            let mut members = Vec::new();
            let mut missing = Vec::new();
            for entry in entries {
                let values = if entry.tag { tags.get(&entry.id).cloned() }
                    else { resolve(&entry.id).map(|id| vec![id]) };
                match values {
                    Some(values) => for value in values {
                        if entry.remove { members.retain(|id| id != &value); }
                        else if !members.contains(&value) { members.push(value); }
                    },
                    None if entry.required && !entry.remove => missing.push(entry.id.to_string()),
                    None => {},
                }
            }
            if missing.is_empty() { tags.insert(id, members); }
            else { problems.push(format!("Could not load {kind} tag {id}: missing {}", missing.join(", "))); }
        }
        tags
}

/// Registration boundary for common setup. Each callback captures its own condition
/// context and publishes definitions, tags, stats and traits together. All folders
/// must be captured before server_loaded is set; client sync/publication is separate.
pub(crate) fn register<C: Conditions + 'static>(
    context: impl Fn() -> Result<C, String> + Send + Sync + 'static,
) -> Result<Arc<RwLock<Definitions>>, String> {
    let state = Arc::new(RwLock::new(Definitions::default()));
    let output = state.clone();
    // Declare the folder for capture; all folders are prepared before callbacks.
    // Loading it from the definition callback avoids cross-listener generation races.
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "material_tags").map_err(|e| e.to_string())?, TAG_FOLDER, |_| Ok(()),
    )?;
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "material_traits").map_err(|e| e.to_string())?, crate::material_traits::FOLDER, |_| Ok(()),
    )?;
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "material_stats").map_err(|e| e.to_string())?, crate::material_stats::FOLDER, |_| Ok(()),
    )?;
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "material_definitions").map_err(|e| e.to_string())?, FOLDER,
        move |files| {
            let prepared = context().and_then(|context| {
                let tags = ResourceManagerHelper::reload_json_stacks(TAG_FOLDER).ok_or("material tag folder was not captured")?;
                let traits = ResourceManagerHelper::reload_json_stacks(crate::material_traits::FOLDER).ok_or("material traits folder was not captured")?;
                let stats = ResourceManagerHelper::reload_json_stacks(crate::material_stats::FOLDER).ok_or("material stats folder was not captured")?;
                let mut loaded = Definitions::load(files, &context);
                loaded.load_tags(&tags);
                loaded.traits = crate::material_traits::load(&traits, &mut loaded.problems);
                loaded.stats = crate::material_stats::load(&stats, &mut loaded.problems);
                loaded.server_loaded = true;
                loaded.snapshot = Some(crate::material_snapshot::encode(&loaded, &stats, &traits)?.into());
                Ok(loaded)
            });
            let (mut replacement, failure) = match prepared {
                Ok(loaded) => (loaded, None),
                Err(error) => (Definitions { problems: vec![error.clone()], ..Definitions::default() }, Some(error)),
            };
            for problem in &replacement.problems { eprintln!("Tinkers: {problem}"); }
            // Do not retain a previous world's materials when this context is unavailable.
            if replacement.snapshot.is_none() {
                replacement.snapshot = Some(crate::material_snapshot::encode(&replacement, &[], &[])?.into());
            }
            let mut previous = output.write().map_err(|_| "material definition registry poisoned")?;
            replacement.generation = previous.generation.wrapping_add(1);
            let completed = replacement.server_loaded.then_some(MaterialsLoaded {
                side: MaterialSide::Server, generation: replacement.generation,
            });
            *previous = replacement;
            drop(previous);
            // Listeners may read the published registry; never invoke under its write lock.
            if let Some(completed) = completed { MATERIALS_LOADED.invoke(&completed); }
            failure.map_or(Ok(()), Err)
        },
    )?;
    Ok(state)
}

struct Definition<C> {
    condition: Option<C>,
    redirects: Vec<(Identifier, Option<C>)>,
    craftable: Option<bool>,
    hidden: bool,
    tier: i32,
    sort_order: i32,
    rarity: Rarity,
}
enum Selection { Material(Material), Redirect(Identifier), Skipped }
impl<P> Definition<P> {
    fn decode<C: Conditions<Parsed = P>>(json: &Value, context: &C) -> Result<Self, String> {
        let condition = optional(json, "condition").map(|value| context.decode(value)).transpose()?;
        let craftable = boolean(json, "craftable")?;
        let hidden = boolean(json, "hidden")?.unwrap_or(false);
        let tier = integer(json, "tier")?.unwrap_or(0);
        let sort_order = integer(json, "sortOrder")?.unwrap_or(100);
        // GSON's enum adapter returns null for unknown strings; Java then computes rarity.
        let rarity = match optional(json, "rarity") {
            None => None,
            Some(Value::String(name)) => match name.as_str() {
                "COMMON" => Some(Rarity::Common), "UNCOMMON" => Some(Rarity::Uncommon),
                "RARE" => Some(Rarity::Rare), "EPIC" => Some(Rarity::Epic), _ => None,
            },
            Some(_) => return Err("rarity must be a string".into()),
        }.unwrap_or_else(|| Rarity::for_tier(tier));
        let mut redirects = Vec::new();
        if let Some(value) = optional(json, "redirect") {
            for redirect in value.as_array().ok_or("redirect must be an array")? {
                let id = redirect.get("id").and_then(Value::as_str).ok_or("redirect needs an id")?;
                let id = Identifier::parse(id).map_err(|e| e.to_string())?;
                let condition = optional(redirect, "condition").map(|value| context.decode(value)).transpose()?;
                redirects.push((id, condition));
            }
        }
        Ok(Self { condition, redirects, craftable, hidden, tier, sort_order, rarity })
    }
    fn select<C: Conditions<Parsed = P>>(self, id: &Identifier, context: &C) -> Result<Selection, String> {
        for (target, condition) in self.redirects {
            if condition.as_ref().map(|c| context.test(c)).transpose()?.unwrap_or(true) {
                return Ok(Selection::Redirect(target));
            }
        }
        if !self.condition.as_ref().map(|c| context.test(c)).transpose()?.unwrap_or(true) {
            return Ok(Selection::Skipped);
        }
        let craftable = self.craftable.ok_or("material needs crafting information (craftable)")?;
        Ok(Selection::Material(Material { id: id.clone(), craftable, hidden: self.hidden,
            tier: self.tier, sort_order: self.sort_order, rarity: self.rarity }))
    }
}
fn optional<'a>(json: &'a Value, key: &str) -> Option<&'a Value> { json.get(key).filter(|v| !v.is_null()) }
fn boolean(json: &Value, key: &str) -> Result<Option<bool>, String> {
    optional(json, key).map(|value| match value {
        Value::Bool(value) => Ok(*value),
        Value::String(value) => Ok(value.eq_ignore_ascii_case("true")),
        _ => Err(format!("{key} must be a boolean")),
    }).transpose()
}
fn integer(json: &Value, key: &str) -> Result<Option<i32>, String> {
    optional(json, key).map(|value| {
        // fromJson(JsonObject) uses JsonTreeReader: strings parse as Integer;
        // numeric primitives use LazilyParsedNumber/BigDecimal.intValue (truncate
        // toward zero and retain the low 32 bits, rather than saturating).
        match value {
            Value::String(text) => text.parse::<i32>().map_err(|_| format!("invalid {key}")),
            Value::Number(number) => Ok(number_int(&number.to_string())),
            _ => Err(format!("{key} must be an integer")),
        }
    }).transpose()
}

// Input is already a valid JSON number, serialized by serde_json.
pub(super) fn number_int(text: &str) -> i32 {
    let negative = text.starts_with('-');
    let text = text.strip_prefix('-').unwrap_or(text);
    let (mantissa, exponent) = text.split_once(['e', 'E'])
        .map(|(m, e)| (m, e.parse::<i32>().expect("JSON number exponent"))).unwrap_or((text, 0));
    let point = mantissa.find('.').unwrap_or(mantissa.len()) as i32;
    let places = point + exponent;
    if places <= 0 { return 0; }
    let mut result = 0i32;
    let mut consumed = 0;
    for digit in mantissa.bytes().filter(|b| *b != b'.').take(places as usize) {
        result = result.wrapping_mul(10).wrapping_add(i32::from(digit - b'0'));
        consumed += 1;
    }
    let zeros = places - consumed;
    if zeros >= 32 { return 0; }
    for _ in 0..zeros { result = result.wrapping_mul(10); }
    if negative { result.wrapping_neg() } else { result }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    struct Booleans;
    impl Conditions for Booleans {
        type Parsed = bool;
        fn decode(&self, value: &Value) -> Result<bool, String> { value.as_bool().ok_or("invalid condition".into()) }
        fn test(&self, value: &bool) -> Result<bool, String> { Ok(*value) }
    }
    fn batch(files: Vec<(&str, Value)>) -> Definitions {
        Definitions::load(&files.into_iter().map(|(id, json)| (Identifier::parse(id).unwrap(), json)).collect::<Vec<_>>(), &Booleans)
    }
    #[test]
    fn redirect_precedes_material_condition_but_entire_document_must_decode() {
        let loaded = batch(vec![
            ("tconstruct:wood", json!({"craftable":true})),
            ("tconstruct:alias", json!({"condition":false,"redirect":[{"id":"tconstruct:wood"}]})),
            ("tconstruct:invalid", json!({"redirect":[{"id":"tconstruct:wood"}],"tier":{}})),
            ("tconstruct:bad_condition", json!({"condition":{},"redirect":[{"id":"tconstruct:wood"}]})),
            ("tconstruct:chain", json!({"redirect":[{"id":"tconstruct:alias"}]})),
        ]);
        let alias = Identifier::parse("tconstruct:alias").unwrap();
        assert_eq!(loaded.resolve_redirect(&alias).to_string(), "tconstruct:wood");
        assert_eq!(loaded.redirects.len(), 1);
        assert_eq!(loaded.problems.len(), 3);
    }
    #[test]
    fn defaults_required_fields_and_original_identifier_sort_order() {
        let loaded = batch(vec![
            ("a:z", json!({"craftable":true})), ("z:a", json!({"craftable":false})),
            ("tconstruct:missing", json!({})), ("tconstruct:skipped", json!({"condition":false})),
            ("tconstruct:hidden", json!({"craftable":true,"hidden":true})),
            ("tconstruct:rare", json!({"craftable":true,"tier":4,"rarity":"unknown"})),
        ]);
        assert_eq!(loaded.visible.iter().map(ToString::to_string).collect::<Vec<_>>(), ["z:a", "a:z", "tconstruct:rare"]);
        let rare = &loaded.materials[&Identifier::parse("tconstruct:rare").unwrap()];
        assert_eq!((rare.sort_order, rare.rarity), (100, Rarity::Rare));
        assert_eq!(loaded.problems.len(), 1);
        assert!(Definitions::load(&[], &Booleans).materials.is_empty());
    }
    #[test]
    fn numeric_tree_fields_truncate_and_wrap_but_string_fields_parse_strictly() {
        assert_eq!(integer(&json!({"tier":4294967297u64}), "tier").unwrap(), Some(1));
        assert_eq!(integer(&json!({"tier":-2.9}), "tier").unwrap(), Some(-2));
        assert_eq!(integer(&json!({"tier":"12"}), "tier").unwrap(), Some(12));
        assert!(integer(&json!({"tier":"2.9"}), "tier").is_err());
        assert_eq!(number_int("1e32"), 0);
        assert_eq!(number_int("123.4e-1"), 12);
        assert_eq!(number_int("-0.1"), 0);
    }
    #[test]
    fn tag_optional_fields_follow_dfu_defaults_and_numeric_boolean_coercion() {
        let (replace, entries) = decode_tag_file(&json!({"replace":"true","values":[
            {"id":"test:a","required":"false"}, {"id":"test:b","required":256},
            {"id":"test:c","required":-1}],"remove":["test:a",{}]})).unwrap();
        assert!(!replace);
        assert_eq!(entries.len(), 3, "invalid optional remove list defaults entirely");
        assert!(entries[0].required);
        assert!(!entries[1].required);
        assert!(entries[2].required);
        assert!(decode_tag_file(&json!({"replace":1,"values":[]})).unwrap().0);
        assert!(decode_tag_file(&json!({"values":[{}]})).is_err());
    }
    #[test]
    fn layered_tags_replace_remove_readd_and_reject_missing_required_references() {
        let mut loaded = batch(vec![
            ("test:a", json!({"craftable":true})),
            ("test:b", json!({"craftable":true,"hidden":true})),
            ("test:alias", json!({"redirect":[{"id":"test:a"}]})),
        ]);
        let stack = |id: &str, layers: Vec<Result<Value, String>>| pluto_lib::JsonResourceStack { id: Identifier::parse(id).unwrap(), layers };
        loaded.load_tags(&[
            stack("test:base", vec![Ok(json!({"values":["test:a","test:b","test:a"]})),
                Ok(json!({"values":[],"remove":["test:a","test:absent"]})),
                Ok(json!({"values":["test:a"]}))]),
            stack("test:replaced", vec![Ok(json!({"values":["test:absent"]})),
                Ok(json!({"replace":true,"values":["#test:base"],"remove":["test:b"]}))]),
            stack("test:bad_layer", vec![Ok(json!({"values":["test:a"]})),
                Ok(json!({"replace":true,"values":[{}]})), Err("bad JSON".into())]),
            stack("test:optional", vec![Ok(json!({"values":[{"id":"test:absent","required":false}]}))]),
            stack("test:alias_is_not_material", vec![Ok(json!({"values":["test:alias"]}))]),
            stack("test:cycle_a", vec![Ok(json!({"values":["#test:cycle_b"]}))]),
            stack("test:cycle_b", vec![Ok(json!({"values":["#test:cycle_a"]}))]),
            stack("test:mixed_a", vec![Ok(json!({"values":["#test:mixed_b"]}))]),
            stack("test:mixed_b", vec![Ok(json!({"values":["test:b",{"id":"#test:mixed_a","required":false}]}))]),
        ]);
        let members = |id: &str| loaded.tags[&Identifier::parse(id).unwrap()].iter().map(ToString::to_string).collect::<Vec<_>>();
        assert_eq!(members("test:base"), ["test:b", "test:a"]);
        assert_eq!(members("test:replaced"), ["test:a"]);
        assert_eq!(members("test:bad_layer"), ["test:a"], "bad replacing layer must not clear valid lower layers");
        assert!(members("test:optional").is_empty());
        assert_eq!(members("test:mixed_a"), ["test:b"], "required dependency wins over optional back edge");
        for absent in ["test:alias_is_not_material", "test:cycle_a", "test:cycle_b"] {
            assert!(!loaded.tags.contains_key(&Identifier::parse(absent).unwrap()));
        }
        assert_eq!(loaded.problems.len(), 5);
        loaded.load_tags(&[]);
        assert!(loaded.tags.is_empty() && loaded.reverse_tags.is_empty());
    }
    #[test]
    fn false_redirect_falls_back_and_invalid_later_redirect_still_rejects() {
        let loaded = batch(vec![
            ("tconstruct:fallback", json!({"craftable":true,"redirect":[{"id":"tconstruct:absent","condition":false}]})),
            ("tconstruct:bad", json!({"redirect":[{"id":"tconstruct:fallback"},{"id":"bad id"}]})),
        ]);
        assert!(loaded.materials.contains_key(&Identifier::parse("tconstruct:fallback").unwrap()));
        assert!(loaded.redirects.is_empty());
        assert_eq!(loaded.problems.len(), 1);
    }
}
