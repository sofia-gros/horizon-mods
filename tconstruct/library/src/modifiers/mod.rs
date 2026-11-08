//! `slimeknights.tconstruct.library.modifiers`: modifiers as data (`ComposableModifier`), their ids and entries, and the manager that loads
//! them from `data/<namespace>/tinkering/modifiers/*.json` with redirects, conditions, tags and the enchantment mapping.
//!
//! A modifier is a list of modules. The module types the original registers are listed in `module_types`; a type whose behaviour is
//! ported has a loader in the `ModuleRegistry`, the others load as raw data and are counted by `Modifier::uncarried_modules`, so a
//! modifier that loads is not mistaken for one that works.
pub mod module_types;

use std::any::Any;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::sync::Arc;

use serde_json::{Map, Value};

use crate::condition::{self, ConditionContext};
use crate::resource::ResourceId;

/// `Modifier.DEFAULT_PRIORITY`.
pub const DEFAULT_PRIORITY: i32 = 100;

/// The id of a modifier (`tconstruct:sharpness`).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModifierId(ResourceId);

impl ModifierId {
    /// `ModifierId.EMPTY`.
    pub fn empty() -> Self {
        Self(ResourceId::try_new("tconstruct", "empty").expect("valid id"))
    }

    pub fn try_parse(text: &str) -> Option<Self> {
        ResourceId::try_parse(text).map(Self)
    }

    pub fn from_resource(id: ResourceId) -> Self {
        Self(id)
    }

    pub fn resource(&self) -> &ResourceId {
        &self.0
    }
}

impl fmt::Display for ModifierId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// `ModifierEntry`: a modifier at a level (1 or more; 0 only for the empty entry).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModifierEntry {
    pub modifier: ModifierId,
    pub level: i32,
}

impl ModifierEntry {
    /// `ModifierEntry.EMPTY`.
    pub fn empty() -> Self {
        Self { modifier: ModifierId::empty(), level: 0 }
    }

    /// `ModifierEntry.LOADABLE`: `{"name": "tconstruct:unburdened", "level": 1}`; a bare string is level 1.
    pub fn from_json(value: &Value) -> Result<Self, String> {
        match value {
            Value::String(name) => Ok(Self { modifier: ModifierId::try_parse(name).ok_or_else(|| format!("bad modifier id '{name}'"))?, level: 1 }),
            Value::Object(object) => {
                let name = object.get("name").or_else(|| object.get("id")).and_then(Value::as_str).ok_or("a modifier entry needs a 'name'")?;
                let level = object.get("level").and_then(Value::as_i64).unwrap_or(1) as i32;
                if level < 1 {
                    return Err(format!("modifier level of {name} must be at least 1"));
                }
                Ok(Self { modifier: ModifierId::try_parse(name).ok_or_else(|| format!("bad modifier id '{name}'"))?, level })
            }
            _ => Err("a modifier entry must be an object".to_owned()),
        }
    }
}

/// One module of a modifier (`ModifierModule`).
pub trait ModifierModule: Any + Send + Sync {
    /// The type id as written in the data (`tconstruct:stat_boost`).
    fn kind(&self) -> &str;
    /// `getPriority`: the priority the module prefers for its modifier, if it has an opinion.
    fn priority(&self) -> Option<i32> {
        None
    }
    /// False for a module that is kept as data only and does nothing yet.
    fn carried(&self) -> bool {
        true
    }
    fn as_any(&self) -> &dyn Any;
}

/// A module of a registered type whose behaviour is not ported: its JSON is kept as written.
#[derive(Clone, Debug, PartialEq)]
pub struct RawModule {
    pub kind: String,
    pub json: Map<String, Value>,
}

impl ModifierModule for RawModule {
    fn kind(&self) -> &str {
        &self.kind
    }
    fn carried(&self) -> bool {
        false
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Reads a module of one type from the JSON object that names it.
pub type ModuleLoader = Arc<dyn Fn(&Map<String, Value>) -> Result<Arc<dyn ModifierModule>, String> + Send + Sync>;

/// `ModifierModule.LOADER`: the module types by id.
#[derive(Default, Clone)]
pub struct ModuleRegistry {
    loaders: HashMap<String, ModuleLoader>,
}

impl ModuleRegistry {
    /// Every type the original registers, each as raw data until its behaviour is ported and registered over it.
    pub fn with_original_types() -> Self {
        let mut registry = Self::default();
        for name in module_types::ORIGINAL_MODULE_TYPES {
            let kind = format!("tconstruct:{name}");
            let kept = kind.clone();
            registry.loaders.insert(kind, Arc::new(move |json| Ok(Arc::new(RawModule { kind: kept.clone(), json: json.clone() }) as Arc<dyn ModifierModule>)));
        }
        registry
    }

    /// Registers (or replaces) the loader of a type.
    pub fn register(&mut self, kind: &str, loader: ModuleLoader) {
        self.loaders.insert(kind.to_owned(), loader);
    }

    pub fn knows(&self, kind: &str) -> bool {
        self.loaders.contains_key(kind)
    }

    pub fn load(&self, json: &Value) -> Result<Arc<dyn ModifierModule>, String> {
        let object = json.as_object().ok_or("a module must be an object")?;
        let kind = object.get("type").and_then(Value::as_str).ok_or("a module needs a 'type'")?;
        let loader = self.loaders.get(kind).ok_or_else(|| format!("unknown module type '{kind}'"))?;
        loader(object)
    }
}

/// A modifier: its id, how its level shows, its priority and its modules.
pub struct Modifier {
    pub id: ModifierId,
    /// The `level_display` object as written (the drawing is the game's).
    pub level_display: Option<Value>,
    /// The `tooltip_display` / `show_in_tooltips` value as written.
    pub tooltip_display: Option<Value>,
    pub priority: i32,
    pub modules: Vec<Arc<dyn ModifierModule>>,
    pub translation_key: Option<String>,
    /// True for a modifier the original defines as a Java class; its behaviour is not carried until that class is ported.
    pub in_code: bool,
}

impl Modifier {
    /// The modules that are kept as data only.
    pub fn uncarried_modules(&self) -> usize {
        self.modules.iter().filter(|m| !m.carried()).count() + usize::from(self.in_code)
    }

    /// `computePriority`: the first module's preferred priority, else the default.
    fn compute_priority(modules: &[Arc<dyn ModifierModule>]) -> i32 {
        modules.iter().find_map(|m| m.priority()).unwrap_or(DEFAULT_PRIORITY)
    }
}

/// `ModifierManager.FOLDER`.
pub const FOLDER: &str = "tinkering/modifiers";
/// `ModifierManager.TAG_FOLDER`.
pub const TAG_FOLDER: &str = "tinkering/tags/modifiers";

/// The loaded modifiers, redirects, tags and enchantment mappings.
#[derive(Default)]
pub struct ModifierManager {
    modifiers: HashMap<ModifierId, Arc<Modifier>>,
    redirects: HashMap<ModifierId, ModifierId>,
    tags: BTreeMap<ResourceId, Vec<ModifierId>>,
    enchantments: BTreeMap<String, ModifierId>,
}

impl ModifierManager {
    /// `apply`: `files` are the modifier definitions as (id, JSON); `tag_files` the tag files; `enchantment_map` the
    /// `enchantments_to_modifiers.json` objects in load order. Returns the manager and what went wrong, which the original logs.
    pub fn load(files: &[(ResourceId, Value)], tag_files: &[(ResourceId, Value)], enchantment_map: &[Value], modules: &ModuleRegistry, context: &dyn ConditionContext) -> (Self, Vec<String>) {
        let mut problems = Vec::new();
        let mut manager = Self::default();
        // the modifiers that are Java classes in the original exist by id; the data may replace one, as the original allows
        for name in module_types::STATIC_MODIFIERS {
            let id = ModifierId::try_parse(&format!("tconstruct:{name}")).expect("valid id");
            manager.modifiers.insert(id.clone(), Arc::new(Modifier { id, level_display: None, tooltip_display: None, priority: DEFAULT_PRIORITY, modules: Vec::new(), translation_key: None, in_code: true }));
        }
        let empty = ModifierId::empty();
        manager.modifiers.insert(empty.clone(), Arc::new(Modifier { id: empty, level_display: None, tooltip_display: None, priority: DEFAULT_PRIORITY, modules: Vec::new(), translation_key: None, in_code: false }));
        let mut redirects = HashMap::new();
        for (key, json) in files {
            match load_modifier(key, json, &mut redirects, modules, context) {
                Ok(Some(modifier)) => {
                    manager.modifiers.insert(modifier.id.clone(), Arc::new(modifier));
                }
                Ok(None) => {}
                Err(error) => problems.push(format!("Failed to load modifier {key}: {error}")),
            }
        }
        // redirects are resolved against the loaded modifiers only, so they cannot depend on one another (no double redirects)
        let mut resolved = HashMap::new();
        for (from, to) in &redirects {
            match manager.modifiers.get(to) {
                Some(target) => {
                    resolved.insert(from.clone(), target.clone());
                }
                None => problems.push(format!("Invalid modifier redirect {from} as modifier {to} does not exist")),
            }
        }
        manager.modifiers.extend(resolved);
        manager.redirects = redirects.into_iter().filter(|(_, to)| manager.modifiers.contains_key(to)).collect();
        let (tags, tag_problems) = crate::tags::load(tag_files, &|id| manager.modifiers.contains_key(&ModifierId::from_resource(id.clone())));
        problems.extend(tag_problems);
        manager.tags = tags.into_iter().map(|(tag, members)| (tag, members.into_iter().map(ModifierId::from_resource).collect())).collect();
        for object in enchantment_map {
            let Some(object) = object.as_object() else { continue };
            for (key, value) in object {
                let Some(text) = value.as_str() else {
                    problems.push(format!("Invalid enchantment to modifier mapping for {key}: not a string"));
                    continue;
                };
                let (text, optional) = text.strip_suffix('?').map_or((text, false), |t| (t, true));
                let Some(id) = ModifierId::try_parse(text) else {
                    problems.push(format!("Invalid enchantment to modifier mapping for {key}: bad modifier id '{text}'"));
                    continue;
                };
                if !manager.modifiers.contains_key(&id) {
                    if !optional {
                        problems.push(format!("Unknown modifier {id} for enchantment {key}"));
                    }
                    continue;
                }
                manager.enchantments.insert(key.clone(), id);
            }
        }
        (manager, problems)
    }

    /// `get`: the modifier behind an id (a redirect leads to its target), if loaded.
    pub fn get(&self, id: &ModifierId) -> Option<&Arc<Modifier>> {
        self.modifiers.get(id)
    }

    pub fn contains(&self, id: &ModifierId) -> bool {
        self.modifiers.contains_key(id)
    }

    /// `getAllLocations`: the ids of the modifiers themselves, not of redirects.
    pub fn ids(&self) -> Vec<&ModifierId> {
        let mut ids: Vec<&ModifierId> = self.modifiers.iter().filter(|(id, modifier)| **id == modifier.id).map(|(id, _)| id).collect();
        ids.sort();
        ids
    }

    pub fn tag(&self, tag: &ResourceId) -> &[ModifierId] {
        self.tags.get(tag).map_or(&[], Vec::as_slice)
    }

    pub fn is_in_tag(&self, id: &ModifierId, tag: &ResourceId) -> bool {
        self.tag(tag).contains(id)
    }

    /// The modifier an enchantment (`minecraft:sharpness`) or enchantment tag (`#minecraft:foo`) maps to.
    pub fn for_enchantment(&self, key: &str) -> Option<&ModifierId> {
        self.enchantments.get(key)
    }

    /// How many modules across all modifiers are kept as data only.
    pub fn uncarried_modules(&self) -> usize {
        self.ids().iter().filter_map(|id| self.get(id)).map(|m| m.uncarried_modules()).sum()
    }
}

fn load_modifier(key: &ResourceId, json: &Value, redirects: &mut HashMap<ModifierId, ModifierId>, modules: &ModuleRegistry, context: &dyn ConditionContext) -> Result<Option<Modifier>, String> {
    let object = json.as_object().ok_or("a modifier must be an object")?;
    let id = ModifierId::from_resource(key.clone());
    if let Some(list) = object.get("redirects") {
        for redirect in list.as_array().ok_or("'redirects' must be a list")? {
            let holds = match redirect.get("condition") {
                Some(condition) => condition::test(condition, context)?,
                None => true,
            };
            if holds {
                let target = redirect.get("id").and_then(Value::as_str).and_then(ModifierId::try_parse).ok_or("a redirect needs an 'id'")?;
                redirects.insert(id, target);
                return Ok(None);
            }
        }
    }
    if let Some(condition) = object.get("condition") {
        if !condition::test(condition, context)? {
            return Ok(None);
        }
    }
    let mut loaded = Vec::new();
    if let Some(list) = object.get("modules") {
        for module in list.as_array().ok_or("'modules' must be a list")? {
            loaded.push(modules.load(module)?);
        }
    }
    let priority = match object.get("priority").and_then(Value::as_i64) {
        Some(p) if p as i32 != i32::MIN => p as i32,
        _ => Modifier::compute_priority(&loaded),
    };
    Ok(Some(Modifier {
        id,
        level_display: object.get("level_display").cloned(),
        tooltip_display: object.get("tooltip_display").or_else(|| object.get("show_in_tooltips")).cloned(),
        priority,
        modules: loaded,
        translation_key: object.get("translation_key").and_then(Value::as_str).map(str::to_owned),
        in_code: false,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::condition::EmptyContext;
    use serde_json::json;

    fn rid(text: &str) -> ResourceId {
        ResourceId::try_parse(text).unwrap()
    }

    fn mid(text: &str) -> ModifierId {
        ModifierId::try_parse(text).unwrap()
    }

    fn load(files: Vec<(&str, Value)>, tags: Vec<(&str, Value)>, enchantments: Vec<Value>) -> (ModifierManager, Vec<String>) {
        let files: Vec<_> = files.into_iter().map(|(n, v)| (rid(n), v)).collect();
        let tags: Vec<_> = tags.into_iter().map(|(n, v)| (rid(n), v)).collect();
        ModifierManager::load(&files, &tags, &enchantments, &ModuleRegistry::with_original_types(), &EmptyContext)
    }

    #[test]
    fn a_modifier_loads_its_modules_and_priority() {
        let (manager, problems) = load(
            vec![
                ("tconstruct:sharpness", json!({"level_display": {"type": "tconstruct:unique"}, "modules": [{"type": "tconstruct:stat_boost", "each_level": 0.75, "operation": "add", "stat": "tconstruct:attack_damage"}]})),
                ("tconstruct:plain", json!({"priority": 7})),
            ],
            vec![],
            vec![],
        );
        assert!(problems.is_empty(), "{problems:?}");
        let sharpness = manager.get(&mid("tconstruct:sharpness")).unwrap();
        assert_eq!((sharpness.priority, sharpness.modules.len(), sharpness.uncarried_modules()), (DEFAULT_PRIORITY, 1, 1));
        assert_eq!(manager.get(&mid("tconstruct:plain")).unwrap().priority, 7);
        assert_eq!(manager.uncarried_modules(), 1 + module_types::STATIC_MODIFIERS.len());
    }

    #[test]
    fn an_unknown_module_type_fails_the_modifier() {
        let (manager, problems) = load(vec![("tconstruct:odd", json!({"modules": [{"type": "nobody:knows"}]})), ("tconstruct:ok", json!({}))], vec![], vec![]);
        assert!(!manager.contains(&mid("tconstruct:odd")));
        assert!(manager.contains(&mid("tconstruct:ok")));
        assert_eq!(problems.len(), 1);
    }

    #[test]
    fn redirects_conditions_and_tags() {
        let (manager, problems) = load(
            vec![
                ("tconstruct:target", json!({})),
                ("tconstruct:old", json!({"redirects": [{"id": "tconstruct:target"}]})),
                ("tconstruct:gone", json!({"redirects": [{"id": "tconstruct:missing"}]})),
                ("tconstruct:gated", json!({"condition": {"type": "forge:mod_loaded", "modid": "other"}})),
            ],
            vec![("tconstruct:group", json!({"values": ["tconstruct:target", "tconstruct:old", {"id": "tconstruct:later", "required": false}]}))],
            vec![],
        );
        assert_eq!(manager.get(&mid("tconstruct:old")).unwrap().id, mid("tconstruct:target"), "a redirect leads to its target");
        assert!(!manager.contains(&mid("tconstruct:gone")) && !manager.contains(&mid("tconstruct:gated")));
        assert_eq!(manager.ids().len(), 1 + 1 + module_types::STATIC_MODIFIERS.len(), "the target, the empty modifier and the static ones; redirects are not listed");
        assert_eq!(manager.tag(&rid("tconstruct:group")).len(), 2, "the redirect id is a member too, as the original's tag loader finds it");
        assert_eq!(problems.len(), 1, "{problems:?}");
    }

    #[test]
    fn enchantments_map_to_modifiers_with_optional_marks() {
        let (manager, problems) = load(
            vec![("tconstruct:sharpness", json!({}))],
            vec![],
            vec![json!({"minecraft:sharpness": "tconstruct:sharpness", "minecraft:gone": "tconstruct:missing?", "minecraft:bad": "tconstruct:missing", "#minecraft:tag": "tconstruct:sharpness"})],
        );
        assert_eq!(manager.for_enchantment("minecraft:sharpness"), Some(&mid("tconstruct:sharpness")));
        assert_eq!(manager.for_enchantment("#minecraft:tag"), Some(&mid("tconstruct:sharpness")));
        assert!(manager.for_enchantment("minecraft:gone").is_none() && manager.for_enchantment("minecraft:bad").is_none());
        assert_eq!(problems.len(), 1, "only the non-optional unknown modifier is a problem: {problems:?}");
    }

    #[test]
    fn entries_read_both_shapes() {
        assert_eq!(ModifierEntry::from_json(&json!({"name": "a:b", "level": 3})).unwrap().level, 3);
        assert_eq!(ModifierEntry::from_json(&json!("a:b")).unwrap().level, 1);
        assert!(ModifierEntry::from_json(&json!({"name": "a:b", "level": 0})).is_err());
        assert_eq!(ModifierEntry::empty().level, 0);
    }
}
