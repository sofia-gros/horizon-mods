// Raw modifier resources for a future book renderer.
use std::collections::{BTreeMap, HashSet};

pub(super) type Layers = pluto_api::resources::Layers;
pub(super) type Raw = (Layers, Layers); // definitions, then modifier tags

/// Merge in increasing pack priority. A listed but unreadable file makes the
/// snapshot unavailable rather than returning a silently incomplete registry.
pub(super) fn merge_pack(
    snapshot: &mut Raw,
    files: impl IntoIterator<Item = String>,
    read: impl Fn(&str) -> Option<Vec<u8>>,
) -> Result<(), String> {
    pluto_api::resources::visit_json_pack(
        files,
        read,
        |resource| {
            if resource.path.starts_with("tinkering/modifiers/") {
                Some(false)
            } else if resource.path.starts_with("tinkering/tags/modifiers/") {
                Some(true)
            } else {
                None
            }
        },
        |is_tag, id, bytes| {
            let layers = if is_tag { &mut snapshot.1 } else { &mut snapshot.0 };
            layers.entry(id).or_default().push(bytes);
        },
    ).map_err(|error| format!("unreadable modifier resource {}", error.file))
}

/// Source dynamic definitions select the highest-priority file; retaining the
/// layers also lets a future consumer diagnose or re-evaluate conditions.
#[allow(dead_code)]
pub(super) fn highest_definition<'a>(snapshot: &'a Raw, id: &str) -> Option<&'a [u8]> {
    pluto_api::resources::highest(&snapshot.0, id)
}

/// Resolve one tag only with a complete authoritative modifier registry. A
/// file name alone is not proof of registration: definitions may be invalid,
/// conditional, redirects, or shadow a static modifier. Missing registry is
/// explicitly unknown; no caller currently supplies a complete registry.
#[allow(dead_code)]
pub(super) fn resolve_tag(snapshot: &Raw, tag: &str, complete_registry: Option<&HashSet<String>>) -> Result<Vec<String>, String> {
    let registry = complete_registry.ok_or("modifier registry unavailable")?;
    let mut tags = BTreeMap::new();
    for (id, layers) in &snapshot.1 {
        let Some((namespace, path)) = id.split_once(':') else { return Err(format!("invalid modifier tag {id}")); };
        let Some(path) = path.strip_prefix("tinkering/tags/modifiers/") else { continue; };
        tags.insert(format!("{namespace}:{path}"), layers.clone());
    }
    super::tconstruct_source_tags::resolve_tag(tag, &tags, registry)
}

/// Resolve modifier identities only with a complete authoritative ID-to-object
/// map from `compose_registry`. Without remove operations, projecting IDs to
/// canonical object IDs after expansion preserves linked-set order. Removals
/// are conservatively unsupported here: an alias and its target may denote the
/// same object, and ID-based removal must not be mistaken for object removal.
#[allow(dead_code)]
pub(super) fn resolve_canonical_tag(
    snapshot: &Raw,
    tag: &str,
    registry: Option<&BTreeMap<String, String>>,
) -> Result<Vec<String>, String> {
    let registry = registry.ok_or("modifier registry unavailable")?;
    if registry.get("tconstruct:empty").map(String::as_str) != Some("tconstruct:empty")
        || registry.values().any(|canonical| registry.get(canonical) != Some(canonical)) {
        return Err("invalid canonical modifier registry".into());
    }
    if snapshot.1.values().any(|layers|
        super::tconstruct_source_tags::merge_tag_files(layers).iter().any(|entry| entry.remove)) {
        return Err("canonical modifier tag removals unsupported".into());
    }
    // Source's missing-value sentinel is accepted only for the explicit empty
    // ID, not for an alias that returns the same empty modifier object.
    let usable: HashSet<String> = registry.iter().filter(|(id, canonical)|
        canonical.as_str() != "tconstruct:empty" || id.as_str() == "tconstruct:empty")
        .map(|(id, _)| id.clone()).collect();
    let ids = resolve_tag(snapshot, tag, Some(&usable))?;
    let mut values = Vec::new();
    for id in ids {
        let canonical = registry.get(&id).ok_or("modifier registry changed")?;
        if !values.contains(canonical) { values.push(canonical.clone()); }
    }
    Ok(values)
}

/// Project a source guide tag onto the modifiers actually implemented by this
/// port. This is deliberately not source registry resolution: absent port IDs
/// are omitted, while missing required nested tags and cycles still fail.
/// The supplied IDs must come from the gameplay consumer, never filenames.
pub(super) fn supported_guide_tag(snapshot: &Raw, tag: &str, supported: &HashSet<String>) -> Result<Vec<String>, String> {
    let mut tags = BTreeMap::new();
    for (id, layers) in &snapshot.1 {
        let (namespace, path) = id.split_once(':').ok_or("invalid modifier tag key")?;
        let Some(path) = path.strip_prefix("tinkering/tags/modifiers/") else { continue; };
        tags.insert(format!("{namespace}:{path}"), layers.clone());
    }
    super::tconstruct_source_tags::resolve_available_entries(tag, &tags, supported)
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct BonusGuideRecipe {
    pub recipe: String,
    pub modifier: String,
    pub inputs: Vec<String>,
    pub tools: Vec<String>,
}

/// Only the ordinary single-input, level-one, no-slot-cost bonus recipe shape
/// is displayed. The caller supplies the condition-filtered world snapshot,
/// real item registry and the exact Station ingredient matcher. Unsupported
/// requirements/serializers are omitted rather than advertised as craftable.
pub(super) fn bonus_guide_recipes(
    recipes: &BTreeMap<String, Vec<u8>>, modifiers: &[String],
    items: &[String], tools: &[String],
    matches: impl Fn(&serde_json::Value, &str) -> bool,
) -> Vec<BonusGuideRecipe> {
    let mut result = Vec::new();
    for modifier in modifiers {
        for (recipe, bytes) in recipes {
            let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else { continue; };
            let Some(object) = value.as_object() else { continue; };
            if object.keys().any(|key| !matches!(key.as_str(), "type" | "result" | "inputs" | "tools" | "level" | "allow_crystal"))
                || value.get("type").and_then(|v| v.as_str()) != Some("tconstruct:modifier")
                || value.get("result").and_then(|v| v.as_str()) != Some(modifier.as_str())
                || value.get("level").and_then(|v| v.as_i64()) != Some(1)
                || value.get("allow_crystal").is_some_and(|v| !v.is_boolean()) { continue; }
            let Some(inputs) = value.get("inputs").and_then(|v| v.as_array()).filter(|v| v.len() == 1) else { continue; };
            let Some(tool_filter) = value.get("tools") else { continue; };
            let mut inputs: Vec<_> = items.iter().filter(|id| matches(&inputs[0], id)).cloned().collect();
            let mut tools: Vec<_> = tools.iter().filter(|id| matches(tool_filter, id)).cloned().collect();
            inputs.sort(); inputs.dedup(); tools.sort(); tools.dedup();
            if !inputs.is_empty() && !tools.is_empty() {
                result.push(BonusGuideRecipe { recipe: recipe.clone(), modifier: modifier.clone(), inputs, tools });
            }
        }
    }
    result
}

/// Authoritative results of evaluating the highest-priority JSON resource.
/// Producing these results requires the source condition context and modifier
/// deserializer; this module intentionally does not guess them from file names.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ClassifiedDefinition {
    Loaded,
    Skipped,
    Redirect(String),
}

/// Classify one highest-priority definition without inferring serializer or
/// condition results. The evaluator and deserializer must be authoritative for
/// the current datapack context. Unsupported conditional redirects remain
/// unknown: upstream parses their condition from the enclosing redirect object.
#[allow(dead_code)]
pub(super) fn classify_definition(
    bytes: &[u8],
    condition: impl Fn(&serde_json::Value) -> Option<bool>,
    deserialize: impl Fn(&serde_json::Value) -> Option<bool>,
) -> Result<ClassifiedDefinition, String> {
    let json: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| "invalid modifier JSON")?;
    let object = json.as_object().ok_or("modifier must be an object")?;
    if let Some(redirects) = object.get("redirects") {
        let redirects = redirects.as_array().ok_or("invalid modifier redirects")?;
        // Upstream parses the full list before testing the first condition.
        let mut ids = Vec::new();
        for redirect in redirects {
            let entry = redirect.as_object().ok_or("invalid modifier redirect")?;
            let id = entry.get("id").and_then(serde_json::Value::as_str).ok_or("missing redirect id")?;
            let (namespace, path) = id.split_once(':').unwrap_or(("minecraft", id));
            if namespace.is_empty() || path.is_empty()
                || !namespace.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_.-".contains(&c))
                || !path.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_./-".contains(&c)) {
                return Err("invalid redirect id".into());
            }
            if entry.contains_key("condition") { return Err("conditional redirects unsupported".into()); }
            ids.push(format!("{namespace}:{path}"));
        }
        if let Some(id) = ids.into_iter().next() { return Ok(ClassifiedDefinition::Redirect(id)); }
    }
    if let Some(value) = object.get("condition") {
        if !value.is_object() { return Err("invalid modifier condition".into()); }
        if !condition(value).ok_or("modifier condition unavailable")? { return Ok(ClassifiedDefinition::Skipped); }
    }
    match deserialize(&json) {
        Some(true) => Ok(ClassifiedDefinition::Loaded),
        Some(false) => Ok(ClassifiedDefinition::Skipped),
        None => Err("modifier deserializer unavailable".into()),
    }
}

/// A deliberately narrow authoritative serializer slice. These exact two
/// module-free shapes use registered singleton level displays and standard
/// tooltip settings; all other forms require the source's full loader.
#[allow(dead_code)]
pub(super) fn known_moduleless_modifier(json: &serde_json::Value) -> Option<bool> {
    let mut body = json.clone();
    if body.get("condition") == Some(&serde_json::json!({"type":"forge:true"})) {
        body.as_object_mut()?.remove("condition");
    }
    if body == serde_json::json!({"level_display":"tconstruct:default"})
        || body == serde_json::json!({"level_display":"tconstruct:no_levels", "show_in_tooltips":"advanced"}) {
        Some(true)
    } else { None }
}

/// Only literal Forge true/false are known without a world or mod context.
/// More complex or malformed conditions remain unknown, not false.
#[allow(dead_code)]
pub(super) fn known_literal_condition(json: &serde_json::Value) -> Option<bool> {
    if *json == serde_json::json!({"type":"forge:true"}) { Some(true) }
    else if *json == serde_json::json!({"type":"forge:false"}) { Some(false) }
    else { None }
}

/// Attempt a complete classification of every highest-priority definition.
/// One unknown stops the entire result: a partial map must not look complete.
#[allow(dead_code)]
pub(super) fn classify_bounded_snapshot(snapshot: &Raw) -> Result<BTreeMap<String, ClassifiedDefinition>, String> {
    let mut results = BTreeMap::new();
    for (key, layers) in &snapshot.0 {
        let (namespace, path) = key.split_once(':').ok_or("invalid modifier resource key")?;
        let path = path.strip_prefix("tinkering/modifiers/").ok_or("invalid modifier resource path")?;
        let bytes = layers.last().ok_or("modifier resource has no layers")?;
        let id = format!("{namespace}:{path}");
        let value = classify_definition(bytes, known_literal_condition, known_moduleless_modifier)
            .map_err(|error| format!("{id}: {error}"))?;
        if results.insert(id, value).is_some() { return Err("duplicate modifier resource".into()); }
    }
    Ok(results)
}

/// Compose already-classified definitions with the complete static registry.
/// The returned map resolves an ID to the actual modifier's ID. Source checks
/// static entries first, then ordinary dynamics; redirects are resolved in a
/// separate pass against those entries, so redirect chains do not resolve.
/// Missing any authoritative classification makes the whole result unknown.
#[allow(dead_code)]
pub(super) fn compose_registry(
    snapshot: &Raw,
    static_ids: Option<&HashSet<String>>,
    classifications: &BTreeMap<String, ClassifiedDefinition>,
) -> Result<BTreeMap<String, String>, String> {
    let static_ids = static_ids.ok_or("static modifier registry unavailable")?;
    let mut definition_ids = HashSet::new();
    for key in snapshot.0.keys() {
        let (namespace, path) = key.split_once(':').ok_or("invalid modifier resource key")?;
        let path = path.strip_prefix("tinkering/modifiers/").ok_or("invalid modifier resource path")?;
        definition_ids.insert(format!("{namespace}:{path}"));
    }
    if definition_ids.len() != classifications.len() || !definition_ids.iter().all(|id| classifications.contains_key(id)) {
        return Err("modifier classification incomplete".into());
    }
    let mut resolved = BTreeMap::new();
    for id in static_ids { resolved.insert(id.clone(), id.clone()); }
    for (id, decision) in classifications {
        if *decision == ClassifiedDefinition::Loaded {
            resolved.entry(id.clone()).or_insert_with(|| id.clone());
        }
    }
    // Do not let redirects see each other, including when their lexical order
    // would otherwise accidentally permit chains.
    let targets = resolved.clone();
    for (id, decision) in classifications {
        if let ClassifiedDefinition::Redirect(target) = decision {
            if let Some(canonical) = targets.get(target) {
                resolved.entry(id.clone()).or_insert_with(|| canonical.clone());
            }
        }
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn supported_guide_projection_reloads_nested_tags_without_inventing_registry() {
        let supported = HashSet::from(["tconstruct:writable".into(), "tconstruct:draconic".into()]);
        let mut raw = Raw::default();
        let root = "tconstruct:tinkering/tags/modifiers/slotless/bonus";
        raw.1.insert(root.into(), vec![br##"{"values":["tconstruct:unported","#tconstruct:child","tconstruct:writable"]}"##.to_vec()]);
        raw.1.insert("tconstruct:tinkering/tags/modifiers/child".into(), vec![br#"{"values":["tconstruct:draconic"]}"#.to_vec()]);
        assert_eq!(supported_guide_tag(&raw, "tconstruct:slotless/bonus", &supported).unwrap(), ["tconstruct:draconic", "tconstruct:writable"]);
        raw.1.get_mut(root).unwrap().push(br#"{"replace":true,"values":["tconstruct:writable"]}"#.to_vec());
        assert_eq!(supported_guide_tag(&raw, "tconstruct:slotless/bonus", &supported).unwrap(), ["tconstruct:writable"]);
        raw.1.get_mut(root).unwrap().push(br#"{"values":[],"remove":["tconstruct:writable"]}"#.to_vec());
        assert!(supported_guide_tag(&raw, "tconstruct:slotless/bonus", &supported).unwrap().is_empty());
        raw.1.insert(root.into(), vec![br##"{"values":["#tconstruct:missing"]}"##.to_vec()]);
        assert!(supported_guide_tag(&raw, "tconstruct:slotless/bonus", &supported).is_err());
        raw.1.insert(root.into(), vec![br##"{"values":["#tconstruct:slotless/bonus"]}"##.to_vec()]);
        assert!(supported_guide_tag(&raw, "tconstruct:slotless/bonus", &supported).is_err());
        assert!(supported_guide_tag(&Raw::default(), "tconstruct:slotless/bonus", &supported).is_err());
    }

    #[test]
    fn bonus_guide_uses_current_recipes_items_and_eligible_tools() {
        let recipe = serde_json::json!({"type":"tconstruct:modifier","allow_crystal":true,
            "inputs":[{"item":"minecraft:writable_book"}],"level":1,"result":"tconstruct:writable",
            "tools":{"item":"tconstruct:pickaxe"}});
        let modifiers = vec!["tconstruct:writable".into()];
        let items = vec!["minecraft:writable_book".into(), "minecraft:paper".into()];
        let tools = vec!["tconstruct:pickaxe".into(), "tconstruct:sword".into()];
        let matcher = |value: &serde_json::Value, id: &str| value.get("item").and_then(|v| v.as_str()) == Some(id);
        let mut recipes = BTreeMap::from([("x:bonus".into(), serde_json::to_vec(&recipe).unwrap())]);
        let rows = bonus_guide_recipes(&recipes, &modifiers, &items, &tools, matcher);
        assert_eq!(rows, vec![BonusGuideRecipe { recipe:"x:bonus".into(), modifier:"tconstruct:writable".into(), inputs:vec!["minecraft:writable_book".into()], tools:vec!["tconstruct:pickaxe".into()] }]);
        let mut changed = recipe.clone(); changed["inputs"][0]["item"] = serde_json::json!("minecraft:paper");
        recipes.insert("x:bonus".into(), serde_json::to_vec(&changed).unwrap());
        assert_eq!(bonus_guide_recipes(&recipes, &modifiers, &items, &tools, matcher)[0].inputs, ["minecraft:paper"]);
        assert!(bonus_guide_recipes(&recipes, &[], &items, &tools, matcher).is_empty());
        assert!(bonus_guide_recipes(&recipes, &modifiers, &[], &tools, matcher).is_empty());
        assert!(bonus_guide_recipes(&recipes, &modifiers, &items, &[], matcher).is_empty());
        for (key, value) in [("conditions", serde_json::json!([])), ("slots", serde_json::json!({"upgrades":1})),
            ("requirements", serde_json::json!({})), ("level", serde_json::json!(2)),
            ("allow_crystal", serde_json::json!("yes")), ("inputs", serde_json::json!([])),
            ("type", serde_json::json!("tconstruct:incremental_modifier"))] {
            let mut unsupported = recipe.clone(); unsupported[key] = value;
            recipes.insert("x:bonus".into(), serde_json::to_vec(&unsupported).unwrap());
            assert!(bonus_guide_recipes(&recipes, &modifiers, &items, &tools, matcher).is_empty(), "{key}");
        }
        recipes.clear();
        assert!(bonus_guide_recipes(&recipes, &modifiers, &items, &tools, matcher).is_empty());
    }

    #[test]
    fn capture_is_scoped_ordered_and_fails_on_unreadable_files() {
        let mut raw = Raw::default();
        let paths = ["data/tconstruct/tinkering/modifiers/a.json", "data/tconstruct/tinkering/tags/modifiers/abilities/a.json", "data/tconstruct/tinkering/materials/definition/a.json", "data/tconstruct/tinkering/tags/modifiers/../escape.json"];
        merge_pack(&mut raw, paths.iter().map(|s| s.to_string()), |_| Some(b"base".to_vec())).unwrap();
        merge_pack(&mut raw, paths.iter().map(|s| s.to_string()), |_| Some(b"higher".to_vec())).unwrap();
        assert_eq!(raw.0.len(), 1); assert_eq!(raw.1.len(), 1);
        assert_eq!(raw.0["tconstruct:tinkering/modifiers/a"], [b"base".to_vec(), b"higher".to_vec()]);
        assert_eq!(highest_definition(&raw, "tconstruct:tinkering/modifiers/a"), Some(&b"higher"[..]));
        assert_eq!(highest_definition(&raw, "tconstruct:tinkering/modifiers/missing"), None);
        assert_eq!(raw.1["tconstruct:tinkering/tags/modifiers/abilities/a"], [b"base".to_vec(), b"higher".to_vec()]);
        assert!(merge_pack(&mut raw, [paths[0].to_owned()], |_| None).is_err());
    }
    #[test]
    fn requires_registry_and_preserves_tag_operations_and_order() {
        let mut raw = Raw::default();
        raw.1.insert("tconstruct:tinkering/tags/modifiers/root".into(), vec![br##"{"values":["tconstruct:a","#tconstruct:child",{"id":"tconstruct:missing","required":false}]}"##.to_vec(), br##"{"values":["tconstruct:c"],"remove":["tconstruct:a"]}"##.to_vec()]);
        raw.1.insert("tconstruct:tinkering/tags/modifiers/child".into(), vec![br#"{"values":["tconstruct:b"]}"#.to_vec()]);
        let ids = ["tconstruct:a", "tconstruct:b", "tconstruct:c"].into_iter().map(str::to_owned).collect();
        assert!(resolve_tag(&raw, "tconstruct:root", None).is_err());
        assert_eq!(resolve_tag(&raw, "tconstruct:root", Some(&ids)).unwrap(), ["tconstruct:b", "tconstruct:c"]);
        raw.1.get_mut("tconstruct:tinkering/tags/modifiers/root").unwrap().push(br#"{"replace":true,"values":["tconstruct:a"]}"#.to_vec());
        assert_eq!(resolve_tag(&raw, "tconstruct:root", Some(&ids)).unwrap(), ["tconstruct:a"]);
    }
    #[test]
    fn required_missing_and_cycles_are_errors_not_empty_membership() {
        let mut raw = Raw::default(); let ids = HashSet::new();
        raw.1.insert("x:tinkering/tags/modifiers/root".into(), vec![br#"{"values":["x:absent"]}"#.to_vec()]);
        assert!(resolve_tag(&raw, "x:root", Some(&ids)).is_err());
        raw.1.insert("x:tinkering/tags/modifiers/root".into(), vec![br##"{"values":["#x:loop"]}"##.to_vec()]);
        raw.1.insert("x:tinkering/tags/modifiers/loop".into(), vec![br##"{"values":[{"id":"#x:root","required":false}]}"##.to_vec()]);
        assert!(resolve_tag(&raw, "x:root", Some(&ids)).is_err());
    }
    #[test]
    fn optional_nested_missing_is_skipped_but_required_is_not() {
        let mut raw = Raw::default();
        let ids: HashSet<String> = ["x:a".to_owned()].into_iter().collect();
        let key = "x:tinkering/tags/modifiers/root";
        raw.1.insert(key.into(), vec![br##"{"values":["x:a",{"id":"#x:absent","required":false}],"remove":["#x:missing"]}"##.to_vec()]);
        assert_eq!(resolve_tag(&raw, "x:root", Some(&ids)).unwrap(), ["x:a"]);
        raw.1.insert(key.into(), vec![br##"{"values":["#x:absent"]}"##.to_vec()]);
        assert!(resolve_tag(&raw, "x:root", Some(&ids)).is_err());
    }

    #[test]
    fn fresh_reload_does_not_retain_removed_pack_resources() {
        let path = "data/x/tinkering/modifiers/one.json";
        let mut before = Raw::default();
        merge_pack(&mut before, [path.to_owned()], |_| Some(b"old".to_vec())).unwrap();
        let mut after = Raw::default();
        merge_pack(&mut after, Vec::<String>::new(), |_| None).unwrap();
        assert!(highest_definition(&before, "x:tinkering/modifiers/one").is_some());
        assert!(highest_definition(&after, "x:tinkering/modifiers/one").is_none());
    }

    #[test]
    fn registry_requires_complete_inputs_and_classifications() {
        let mut raw = Raw::default();
        raw.0.insert("x:tinkering/modifiers/a".into(), vec![b"{}".to_vec()]);
        let static_ids = HashSet::new();
        let mut decisions = BTreeMap::new();
        assert!(compose_registry(&raw, None, &decisions).is_err());
        assert!(compose_registry(&raw, Some(&static_ids), &decisions).is_err());
        decisions.insert("x:a".into(), ClassifiedDefinition::Skipped);
        assert!(compose_registry(&raw, Some(&static_ids), &decisions).unwrap().is_empty());
        decisions.insert("x:unlisted".into(), ClassifiedDefinition::Loaded);
        assert!(compose_registry(&raw, Some(&static_ids), &decisions).is_err());
    }
    #[test]
    fn registry_static_priority_and_single_pass_redirects() {
        let mut raw = Raw::default();
        for id in ["static", "dyn", "alias", "chain", "missing", "skip"] {
            raw.0.insert(format!("x:tinkering/modifiers/{id}"), vec![b"{}".to_vec()]);
        }
        let static_ids = ["x:static".to_owned()].into_iter().collect();
        let decisions = [
            ("static", ClassifiedDefinition::Redirect("x:dyn".into())),
            ("dyn", ClassifiedDefinition::Loaded),
            ("alias", ClassifiedDefinition::Redirect("x:dyn".into())),
            ("chain", ClassifiedDefinition::Redirect("x:alias".into())),
            ("missing", ClassifiedDefinition::Redirect("x:unknown".into())),
            ("skip", ClassifiedDefinition::Skipped),
        ].into_iter().map(|(id, value)| (format!("x:{id}"), value)).collect();
        let registry = compose_registry(&raw, Some(&static_ids), &decisions).unwrap();
        assert_eq!(registry.get("x:static").map(String::as_str), Some("x:static"));
        assert_eq!(registry.get("x:alias").map(String::as_str), Some("x:dyn"));
        assert!(!registry.contains_key("x:chain"));
        assert!(!registry.contains_key("x:missing"));
        assert!(!registry.contains_key("x:skip"));
    }

    #[test]
    fn classification_obeys_redirect_then_condition_then_deserializer() {
        let never = |_: &serde_json::Value| -> Option<bool> { panic!("unreachable callback") };
        assert_eq!(classify_definition(br#"{"redirects":[{"id":"x:target"}],"condition":{"type":"unknown"}}"#, never, never).unwrap(), ClassifiedDefinition::Redirect("x:target".into()));
        assert_eq!(classify_definition(br#"{"condition":{"type":"x:false"}}"#, |_| Some(false), never).unwrap(), ClassifiedDefinition::Skipped);
        assert_eq!(classify_definition(br#"{"condition":{"type":"x:true"}}"#, |_| Some(true), |_| Some(true)).unwrap(), ClassifiedDefinition::Loaded);
        assert_eq!(classify_definition(br#"{}"#, never, |_| Some(false)).unwrap(), ClassifiedDefinition::Skipped);
        assert!(classify_definition(br#"{"condition":{"type":"x:unknown"}}"#, |_| None, never).is_err());
        assert!(classify_definition(br#"{}"#, never, |_| None).is_err());
    }
    #[test]
    fn classification_rejects_unsupported_or_malformed_redirects() {
        let none = |_: &serde_json::Value| None;
        assert!(classify_definition(br#"{"redirects":[{"id":"x:a","condition":{"type":"forge:true"}}]}"#, none, none).is_err());
        assert!(classify_definition(br#"{"redirects":[{"id":"x:a"},{"id":"UPPER:bad"}]}"#, none, none).is_err());
        assert!(classify_definition(br#"{"redirects":[{"id":"x:a"},{}]}"#, none, none).is_err());
        assert_eq!(classify_definition(br#"{"redirects":[{"id":"stone"}]}"#, none, none).unwrap(), ClassifiedDefinition::Redirect("minecraft:stone".into()));
        assert!(classify_definition(br#"[]"#, none, none).is_err());
    }

    #[test]
    fn canonical_tag_resolves_aliases_and_empty_sentinel() {
        let mut raw = Raw::default();
        let key = "x:tinkering/tags/modifiers/root";
        raw.1.insert(key.into(), vec![br##"{"values":["x:alias","x:actual","#x:child",{"id":"x:empty_alias","required":false},"tconstruct:empty"]}"##.to_vec()]);
        raw.1.insert("x:tinkering/tags/modifiers/child".into(), vec![br#"{"values":["x:other","x:alias"]}"#.to_vec()]);
        let registry = [("tconstruct:empty", "tconstruct:empty"), ("x:empty_alias", "tconstruct:empty"),
            ("x:actual", "x:actual"), ("x:alias", "x:actual"), ("x:other", "x:other")]
            .into_iter().map(|(k,v)| (k.into(),v.into())).collect();
        assert_eq!(resolve_canonical_tag(&raw, "x:root", Some(&registry)).unwrap(), ["x:actual", "x:other", "tconstruct:empty"]);
        raw.1.insert(key.into(), vec![br#"{"values":["x:empty_alias"]}"#.to_vec()]);
        assert!(resolve_canonical_tag(&raw, "x:root", Some(&registry)).is_err());
    }
    #[test]
    fn canonical_tag_refuses_removal_incomplete_registry_and_bad_targets() {
        let mut raw = Raw::default();
        raw.1.insert("x:tinkering/tags/modifiers/root".into(), vec![br#"{"values":["x:a"],"remove":["x:alias"]}"#.to_vec()]);
        let registry = [("tconstruct:empty", "tconstruct:empty"), ("x:a", "x:a"), ("x:alias", "x:a")]
            .into_iter().map(|(k,v)| (k.into(),v.into())).collect();
        assert!(resolve_canonical_tag(&raw, "x:root", None).is_err());
        assert!(resolve_canonical_tag(&raw, "x:root", Some(&registry)).is_err());
        raw.1.insert("x:tinkering/tags/modifiers/root".into(), vec![br#"{"values":["x:alias"]}"#.to_vec()]);
        let bad = [("tconstruct:empty", "tconstruct:empty"), ("x:alias", "x:missing")]
            .into_iter().map(|(k,v)| (k.into(),v.into())).collect();
        assert!(resolve_canonical_tag(&raw, "x:root", Some(&bad)).is_err());
    }

    #[test]
    fn bounded_classifier_recognizes_only_proven_shapes_and_literals() {
        assert_eq!(known_moduleless_modifier(&serde_json::json!({"level_display":"tconstruct:default"})), Some(true));
        assert_eq!(known_moduleless_modifier(&serde_json::json!({"level_display":"tconstruct:no_levels","show_in_tooltips":"advanced"})), Some(true));
        assert_eq!(known_moduleless_modifier(&serde_json::json!({"level_display":"tconstruct:default","modules":[]})), None);
        assert_eq!(known_moduleless_modifier(&serde_json::json!({"level_display":"x:unknown"})), None);
        assert_eq!(known_literal_condition(&serde_json::json!({"type":"forge:true"})), Some(true));
        assert_eq!(known_literal_condition(&serde_json::json!({"type":"forge:false"})), Some(false));
        assert_eq!(known_literal_condition(&serde_json::json!({"type":"forge:mod_loaded","modid":"x"})), None);
        let conditioned = br#"{"condition":{"type":"forge:true"},"level_display":"tconstruct:default"}"#;
        assert_eq!(classify_definition(conditioned, known_literal_condition, known_moduleless_modifier).unwrap(), ClassifiedDefinition::Loaded);
        let unsupported = br#"{"condition":{"type":"forge:mod_loaded","modid":"x"},"level_display":"tconstruct:default"}"#;
        assert!(classify_definition(unsupported, known_literal_condition, known_moduleless_modifier).is_err());
    }
    #[test]
    fn bounded_snapshot_rejects_unknown_without_returning_partial_map() {
        let mut raw = Raw::default();
        raw.0.insert("x:tinkering/modifiers/a".into(), vec![br#"{"level_display":"tconstruct:default"}"#.to_vec()]);
        raw.0.insert("x:tinkering/modifiers/redirect".into(), vec![br#"{"redirects":[{"id":"x:a"}]}"#.to_vec()]);
        raw.0.insert("x:tinkering/modifiers/off".into(), vec![br#"{"condition":{"type":"forge:false"},"modules":[{"type":"unknown"}]}"#.to_vec()]);
        let result = classify_bounded_snapshot(&raw).unwrap();
        assert_eq!(result["x:off"], ClassifiedDefinition::Skipped);
        assert_eq!(result["x:a"], ClassifiedDefinition::Loaded);
        assert_eq!(result["x:redirect"], ClassifiedDefinition::Redirect("x:a".into()));
        raw.0.get_mut("x:tinkering/modifiers/a").unwrap().push(br#"{"modules":[{"type":"unknown"}]}"#.to_vec());
        assert!(classify_bounded_snapshot(&raw).unwrap_err().contains("x:a"));
    }

}

// Exact pinned MIT definition. Unknown or changed definitions fail closed.
pub(super) const PINNED_OVERWIELD: &[u8] = br#"{
  "level_display": "tconstruct:default",
  "modules": [
    {
      "type": "tconstruct:conditional_mining_speed",
      "formula": [
        "$overslime",
        "$level",
        "min",
        0.2,
        "*",
        1.0,
        "+",
        "$speed",
        "*"
      ],
      "percent": true,
      "require_effective": true,
      "tool": {
        "type": "mantle:inverted",
        "inverted_type": "mantle:tag",
        "tag": "tconstruct:modifiable/broad"
      },
      "variables": {
        "overslime": {
          "type": "tconstruct:mod_data",
          "key": "tconstruct:overslime",
          "source": "persistent"
        }
      }
    },
    {
      "type": "tconstruct:mining_capacity",
      "grant": {
        "each_level": -1
      },
      "owner": "tconstruct:overslime",
      "run_before": true,
      "tool": {
        "type": "mantle:and",
        "predicates": [
          {
            "type": "mantle:tag",
            "tag": "tconstruct:modifiable/harvest"
          },
          {
            "type": "mantle:inverted",
            "inverted_type": "mantle:tag",
            "tag": "tconstruct:modifiable/broad"
          }
        ]
      }
    },
    {
      "type": "tconstruct:conditional_mining_speed",
      "formula": [
        "$overslime",
        5.0,
        "$level",
        "*",
        "min",
        5.0,
        "/",
        0.2,
        "*",
        1.0,
        "+",
        "$speed",
        "*"
      ],
      "percent": true,
      "require_effective": true,
      "tool": {
        "type": "mantle:tag",
        "tag": "tconstruct:modifiable/broad"
      },
      "variables": {
        "overslime": {
          "type": "tconstruct:mod_data",
          "key": "tconstruct:overslime",
          "source": "persistent"
        }
      }
    },
    {
      "type": "tconstruct:mining_capacity",
      "grant": {
        "each_level": -5
      },
      "owner": "tconstruct:overslime",
      "run_before": true,
      "tool": {
        "type": "mantle:tag",
        "tag": "tconstruct:modifiable/broad"
      }
    },
    {
      "type": "tconstruct:attribute",
      "attribute": "minecraft:generic.attack_speed",
      "formula": [
        "$overslime",
        "$level",
        "min",
        0.15,
        "*"
      ],
      "operation": "multiply_total",
      "slots": [
        "mainhand"
      ],
      "variables": {
        "overslime": {
          "type": "tconstruct:mod_data",
          "key": "tconstruct:overslime",
          "source": "persistent"
        }
      }
    },
    {
      "type": "tconstruct:melee_capacity",
      "grant": {
        "each_level": -1
      },
      "owner": "tconstruct:overslime",
      "run_before": true
    },
    {
      "type": "tconstruct:conditional_stat",
      "formula": [
        "$overslime",
        "$level",
        "min",
        0.06,
        "*",
        "$multiplier",
        "*",
        "$value",
        "+"
      ],
      "percent": false,
      "stat": "tconstruct:velocity",
      "variables": {
        "overslime": {
          "type": "tconstruct:mod_data",
          "key": "tconstruct:overslime",
          "source": "persistent"
        }
      }
    },
    {
      "type": "tconstruct:conditional_stat",
      "formula": [
        "$overslime",
        "$level",
        "min",
        0.06,
        "*",
        "$multiplier",
        "*",
        "$value",
        "+"
      ],
      "percent": false,
      "stat": "tconstruct:draw_speed",
      "variables": {
        "overslime": {
          "type": "tconstruct:mod_data",
          "key": "tconstruct:overslime",
          "source": "persistent"
        }
      }
    },
    {
      "type": "tconstruct:launch_capacity",
      "grant": {
        "each_level": -1
      },
      "owner": "tconstruct:overslime",
      "tool": {
        "type": "mantle:tag",
        "tag": "tconstruct:modifiable/ranged"
      }
    }
  ],
  "priority": 75
}"#;

pub(super) fn known_overwield_attribute(source: Option<&[u8]>) -> bool {
    source == Some(PINNED_OVERWIELD)
}

// Exact pinned MIT Raging definition; changed formulas/order remain unknown.
pub(super) const PINNED_RAGING: &[u8] = br#"{
  "level_display": "tconstruct:default",
  "modules": [
    {
      "type": "tconstruct:conditional_melee_damage",
      "formula": [
        11.0,
        "$health",
        "-",
        10.0,
        "/",
        "percent_clamp",
        "$level",
        "*",
        2.5,
        "*",
        "$multiplier",
        "*",
        "$damage",
        "+"
      ],
      "percent": false,
      "variables": {
        "health": {
          "type": "tconstruct:entity",
          "entity_type": "tconstruct:health",
          "fallback": 0.0,
          "which": "attacker"
        }
      }
    },
    {
      "type": "tconstruct:conditional_stat",
      "formula": [
        11.0,
        "$health",
        "-",
        10.0,
        "/",
        "percent_clamp",
        "$level",
        "*",
        "$multiplier",
        "*",
        "$value",
        "+"
      ],
      "percent": false,
      "stat": "tconstruct:projectile_damage",
      "variables": {
        "health": {
          "type": "tconstruct:entity",
          "entity_type": "tconstruct:health",
          "fallback": 0.0
        }
      }
    }
  ]
}"#;

pub(super) fn known_raging_melee(source: Option<&[u8]>) -> bool {
    source == Some(PINNED_RAGING)
}

// Exact pinned MIT jagged definition; unknown modules/order fail closed.
pub(super) const PINNED_JAGGED: &[u8] = br#"{
  "level_display": "tconstruct:default",
  "modules": [
    {
      "type": "tconstruct:conditional_melee_damage",
      "formula": [
        "$lost",
        "$max",
        "/",
        "sqrt",
        "$level",
        "*",
        0.005,
        "*",
        1.0,
        "+",
        "$damage",
        "*"
      ],
      "percent": true,
      "variables": {
        "lost": "tconstruct:tool_lost_durability",
        "max": {
          "type": "tconstruct:stat_multiplier",
          "stat": "tconstruct:durability"
        }
      }
    },
    {
      "type": "tconstruct:conditional_mining_speed",
      "formula": [
        "$lost",
        "$max",
        "/",
        "sqrt",
        "$level",
        "*",
        -0.01,
        "*",
        1.0,
        "+",
        "$speed",
        "*"
      ],
      "percent": true,
      "require_effective": true,
      "variables": {
        "lost": "tconstruct:tool_lost_durability",
        "max": {
          "type": "tconstruct:stat_multiplier",
          "stat": "tconstruct:durability"
        }
      }
    }
  ]
}"#;

// Exact pinned MIT stonebound definition; unknown modules/order fail closed.
pub(super) const PINNED_STONEBOUND: &[u8] = br#"{
  "level_display": "tconstruct:default",
  "modules": [
    {
      "type": "tconstruct:conditional_mining_speed",
      "formula": [
        "$lost",
        "$max",
        "/",
        "sqrt",
        "$level",
        "*",
        0.01,
        "*",
        1.0,
        "+",
        "$speed",
        "*"
      ],
      "percent": true,
      "require_effective": true,
      "variables": {
        "lost": "tconstruct:tool_lost_durability",
        "max": {
          "type": "tconstruct:stat_multiplier",
          "stat": "tconstruct:durability"
        }
      }
    },
    {
      "type": "tconstruct:conditional_melee_damage",
      "formula": [
        "$lost",
        "$max",
        "/",
        "sqrt",
        "$level",
        "*",
        -0.005,
        "*",
        1.0,
        "+",
        "$damage",
        "*"
      ],
      "percent": true,
      "variables": {
        "lost": "tconstruct:tool_lost_durability",
        "max": {
          "type": "tconstruct:stat_multiplier",
          "stat": "tconstruct:durability"
        }
      }
    }
  ]
}"#;

/// Only supported stock melee wear formulas may use a resolved trait level.
pub(super) fn known_melee_wear(snapshot: Option<&Raw>, id: &str) -> bool {
    let (key, expected) = match id {
        "tconstruct:jagged" => ("tconstruct:tinkering/modifiers/jagged", PINNED_JAGGED),
        "tconstruct:stonebound" => ("tconstruct:tinkering/modifiers/stonebound", PINNED_STONEBOUND),
        _ => return false,
    };
    snapshot.and_then(|raw| highest_definition(raw, key)) == Some(expected)
}

#[cfg(test)] mod melee_wear_definition_tests {
    use super::*;
    #[test] fn individual_stock_definitions_require_exact_highest_layer() {
        for (id,bytes) in [("tconstruct:jagged",PINNED_JAGGED),("tconstruct:stonebound",PINNED_STONEBOUND)] {
            let key=format!("tconstruct:tinkering/modifiers/{}",id.strip_prefix("tconstruct:").unwrap());
            let mut raw=(BTreeMap::from([(key.clone(),vec![b"{}".to_vec(),bytes.to_vec()])]),BTreeMap::new());
            assert!(known_melee_wear(Some(&raw),id));
            raw.0.get_mut(&key).unwrap().push([bytes,b" "].concat());
            assert!(!known_melee_wear(Some(&raw),id));
            raw.0.clear();assert!(!known_melee_wear(Some(&raw),id));
            assert!(!known_melee_wear(None,id));
        }
        assert!(!known_melee_wear(None,"tconstruct:raging"));
    }
}
