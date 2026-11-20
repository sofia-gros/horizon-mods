//! ModifierManager and composable rebuild contributions, from SlimeKnights (MIT; see LICENSE).
//! Unsupported behavior rejects a definition rather than registering an inert replacement.
use crate::{materials::Conditions, modifier_entries::Entry};
use pluto_lib::{Identifier, Nbt, NbtCompound, ResourceManagerHelper, ResourceType, ServerLifecycleEvents};
use serde_json::Value;
use std::{collections::{BTreeMap, BTreeSet}, sync::{Arc, RwLock}};

pub(crate) const FOLDER: &str = "tinkering/modifiers";
/// Module types only code-defined modifiers use; the original ships no data for them.
const CODE_MODULES: &[&str] = &["tconstruct:creative_slots", "tconstruct:stat_override"];
pub(crate) const TAG_FOLDER: &str = "tinkering/tags/modifiers";
#[derive(Clone, Debug)]
pub(crate) struct Modifier {
    pub id: Identifier,
    pub priority: i32,
    pub level_display: Identifier,
    pub tooltips: BTreeSet<String>,
    pub translation_key: Option<String>,
    modules: Vec<crate::modifier_modules::Module>,
    /// Known original module types this definition declares that Tinkers has not ported. Their
    /// fields are not validated and they have no effect; callers can see exactly what is missing.
    pub unbound: Vec<Identifier>,
    empty_sentinel: bool,
}
fn id(value: &Value) -> Result<Identifier, String> {
    Identifier::parse(value.as_str().ok_or("expected identifier string")?).map_err(|e| e.to_string())
}
fn integer(value: &Value) -> Result<i32, String> {
    let Value::Number(value) = value else { return Err("expected numeric integer".into()); };
    Ok(crate::materials::number_int(&value.to_string()))
}
pub(crate) fn range(value: Option<&Value>) -> Result<(i32, i32), String> {
    let (min, max) = match value.filter(|v| !v.is_null()) {
        None => (1, 32767),
        Some(Value::Object(fields)) => (
            fields.get("min").map(integer).transpose()?.unwrap_or(1),
            fields.get("max").map(integer).transpose()?.unwrap_or(32767)),
        Some(value) => { let value = integer(value)?; (value, value) },
    };
    if min < 1 || max > 32767 || min > max { return Err("modifier level range must be within 1..32767".into()); }
    Ok((min, max))
}
fn tooltips(value: Option<&Value>) -> Result<BTreeSet<String>, String> {
    let values: Vec<&str> = match value.filter(|v| !v.is_null()) {
        None => vec!["tool", "tinker_station", "tool_part", "part_builder", "book"],
        Some(Value::String(name)) => match name.as_str() {
            "always" => vec!["tool", "tinker_station", "tool_part", "part_builder", "book"],
            "never" => vec![],
            "advanced" => vec!["tinker_station", "part_builder", "book"],
            "bonus_slot" => vec!["tinker_station", "tool_part", "part_builder", "book"],
            "parts_only" => vec!["tool_part", "part_builder", "book"],
            _ => return Err(format!("unsupported tooltip preset {name}")),
        },
        Some(Value::Array(values)) => values.iter().map(|value| value.as_str().ok_or("tooltip context must be a string"))
            .collect::<Result<_, _>>()?,
        _ => return Err("tooltip display must be a preset or context array".into()),
    };
    if values.iter().any(|value| !["tool", "tinker_station", "tool_part", "part_builder", "book"].contains(value)) {
        return Err("unknown tooltip context".into());
    }
    Ok(values.into_iter().map(str::to_owned).collect())
}
impl Modifier {
    pub fn decode(name: Identifier, value: &Value) -> Result<Self, String> {
        let fields = value.as_object().ok_or("modifier must be an object")?;
        let priority = fields.get("priority").filter(|v| !v.is_null()).map(integer).transpose()?.unwrap_or(i32::MIN);
        let display = match fields.get("level_display").filter(|v| !v.is_null()) {
            None => Identifier::new("tconstruct", "default").unwrap(),
            Some(Value::Object(fields)) => id(fields.get("type").ok_or("level display type missing")?)?,
            Some(value) => id(value)?,
        };
        // map_level and unique carry parameters that only the (unported) renderer reads.
        if display.namespace() != "tconstruct" || !["default", "single_level", "no_levels", "pluses", "map_level", "unique"].contains(&display.path()) {
            return Err(format!("unimplemented level display {display}"));
        }
        let deprecated_tooltips = match fields.get("tooltip_display") {
            Some(value) if !fields.contains_key("show_in_tooltips") => Some(Value::String(match value.as_str() {
                Some("always") => "always", Some("tinker_station") => "bonus_slot", Some("never") => "parts_only",
                _ => return Err("invalid deprecated tooltip display".into()),
            }.into())),
            _ => None,
        };
        let tooltips = tooltips(deprecated_tooltips.as_ref().or_else(|| fields.get("show_in_tooltips")))?;
        let translation_key = fields.get("translation_key").filter(|v| !v.is_null()).map(|value| {
            let text = match value { Value::String(s) => s.clone(), Value::Number(_) | Value::Bool(_) => value.to_string(),
                _ => return Err("translation key must be primitive".to_string()) };
            if text.encode_utf16().count() > 32767 { return Err("translation key too long".into()); }
            Ok(text)
        }).transpose()?;
        let mut modules: Vec<crate::modifier_modules::Module> = Vec::new();
        let mut unbound: Vec<Identifier> = Vec::new();
        if let Some(list) = fields.get("modules").filter(|v| !v.is_null()) {
            for module in list.as_array().ok_or("modifier modules must be an array")? {
                let fields = module.as_object().ok_or("modifier module must be an object")?;
                let kind = id(fields.get("type").ok_or("modifier module type missing")?)?;
                let empty = kind.to_string() == "tconstruct:empty";
                // A module type the original never ships is a typo or a foreign module: reject it.
                if !crate::modifier_module_types::KNOWN.contains(&kind.to_string().as_str()) && !CODE_MODULES.contains(&kind.to_string().as_str()) { return Err(format!("unknown modifier module {kind}")); }
                let hooks = match fields.get("hooks").filter(|v| !v.is_null()) {
                    Some(hooks) => hooks.as_array().ok_or("hooks must be an array")?.iter().map(id).collect::<Result<Vec<_>, _>>()?,
                    None => Vec::new(),
                };
                if empty {
                    if !hooks.is_empty() { return Err("unsupported modifier hook binding".into()); }
                    continue;
                }
                // Implemented kinds decode here; a known kind whose shape or hook binding this port
                // cannot honor is recorded as unbound instead of half-installed.
                match crate::modifier_modules::Module::decode(&kind.to_string(), fields, &hooks)? {
                    Some(module) => { if !modules.contains(&module) { modules.push(module); } }
                    None => { if !unbound.contains(&kind) { unbound.push(kind); } }
                }
            }
        }
        Ok(Self { id: name, priority: if priority == i32::MIN { 100 } else { priority },
            level_display: display, tooltips, translation_key, modules, unbound, empty_sentinel: false })
    }
    /// Whether any module of this modifier adds nested traits (it has a MODIFIER_TRAITS binding).
    /// Every module, in definition order.
    pub fn modules(&self) -> &[crate::modifier_modules::Module] { &self.modules }
    pub fn has_trait_modules(&self) -> bool { self.modules.iter().any(crate::modifier_modules::Module::is_trait) }
    /// ModifierTraitHook through TraitBuilder: the nested entries for `entry`, in module order.
    pub fn trait_entries(&self, entry: &Entry, facts: &crate::tool_predicate::Facts, first_encounter: bool) -> Vec<Entry> {
        self.modules.iter().flat_map(|module| module.trait_entry(entry, facts, first_encounter).into_iter().chain(module.swappable_traits(entry, facts))).collect()
    }
    /// ToolDamageModifierHook: the damage left after every damage-reducing module, in module order.
    pub fn reduce_damage(&self, entry: &Entry, facts: &crate::tool_predicate::Facts, nbt: &NbtCompound, cause: Option<&Identifier>,
        modifier_in_tag: &dyn Fn(&Identifier, &Identifier) -> bool, mut amount: i32, random: &mut dyn FnMut() -> f32) -> i32 {
        for module in &self.modules { amount = module.reduce_damage(entry, facts, nbt, cause, modifier_in_tag, amount, random); }
        amount
    }
    /// ValidateModifierHook for a recipe-added modifier: the first failing requirement's translation key.
    pub fn validate_upgrade(&self, entry: &Entry, facts: &crate::tool_predicate::Facts) -> Option<String> {
        self.modules.iter().find_map(|module| module.validate_upgrade(entry, facts)).map(str::to_owned)
    }
    /// RepairFactorModifierHook: the factor after every repair module of this modifier.
    pub fn repair_factor(&self, entry: &Entry, facts: &crate::tool_predicate::Facts, mut factor: f32) -> f32 {
        for module in &self.modules { factor = module.repair_factor(entry, facts, factor); }
        factor
    }
    /// The volatile-data hook with tool facts (item tags, modifier levels, persistent keys).
    pub fn apply_volatile_with(&self, entry: &Entry, facts: &crate::tool_predicate::Facts, data: &mut NbtCompound) {
        for module in &self.modules { module.apply_volatile(entry, facts, data); }
    }
    /// ModifierRemovalHook: clears persistent data this modifier's modules own after it left the tool.
    pub fn on_removed(&self, persistent: &mut NbtCompound) {
        for module in &self.modules { module.on_removed(&self.id, persistent); }
    }
    /// The same without a tool: tag predicates are false and stat support is not filtered.
    pub fn apply_volatile(&self, entry: &Entry, data: &mut NbtCompound) {
        self.apply_volatile_with(entry, &no_tool(), data);
    }
    /// The conditional mining speed modules, each with its sort priority (a percent module runs at 75).
    pub fn mining_modules(&self) -> impl Iterator<Item = (i32, &crate::modifier_modules::Module)> {
        self.modules.iter().filter_map(move |module| module.mining_percent().map(|percent| (if percent { 75 } else { self.priority }, module)))
    }
    /// What this modifier adds to the protection cap attribute.
    pub fn protection_cap(&self, entry: &Entry, facts: &crate::tool_predicate::Facts, nbt: &NbtCompound) -> f32 {
        self.modules.iter().map(|module| module.protection_cap(entry, facts, nbt)).sum()
    }
    /// The running protection after this modifier's protection modules.
    pub fn protection(&self, entry: &Entry, facts: &crate::tool_predicate::Facts, nbt: &NbtCompound, source: &crate::protection::DamageSource, value: f32) -> f32 {
        self.modules.iter().fold(value, |value, module| module.protection(entry, facts, nbt, source, value))
    }
    /// The damage the wearer takes after this modifier's adjust damage modules.
    pub fn adjust_damage(&self, entry: &Entry, facts: &crate::tool_predicate::Facts, nbt: &NbtCompound, source: &crate::protection::DamageSource, entities: &crate::entity_facts::Entities, amount: f32) -> f32 {
        self.modules.iter().fold(amount, |amount, module| module.adjust_damage(entry, facts, nbt, source, entities, amount))
    }
    /// The enchantments this modifier's constant enchantment modules give.
    pub fn enchantments(&self, entry: &Entry, facts: &crate::tool_predicate::Facts) -> Vec<(String, i32)> {
        self.modules.iter().filter_map(|module| module.enchantment(entry, facts)).collect()
    }
    /// The tool actions this modifier's transform modules perform on a right click.
    pub fn tool_actions(&self, entry: &Entry, facts: &crate::tool_predicate::Facts) -> Vec<(crate::tool_actions::ToolAction, String, bool)> {
        self.modules.iter().filter_map(|module| module.tool_action(entry, facts)).collect()
    }
    /// ToolActionModifierHook: whether any tool actions module of this modifier lets the tool perform `name`.
    pub fn can_perform_action(&self, entry: &Entry, facts: &crate::tool_predicate::Facts, name: &str) -> bool {
        self.modules.iter().any(|module| module.can_perform_action(entry, facts, name))
    }
    /// The attribute modifier entries this modifier's attribute modules give for the `wanted` slots.
    pub fn attribute_entries(&self, entry: &Entry, facts: &crate::tool_predicate::Facts, nbt: &NbtCompound, wanted: &[&str]) -> Vec<NbtCompound> {
        self.modules.iter().flat_map(|module| module.attribute_entries(entry, facts, nbt, wanted)).collect()
    }
    /// The conditional melee damage modules, each with its sort priority (a percent module runs at 75).
    pub fn melee_modules(&self) -> impl Iterator<Item = (i32, &crate::modifier_modules::Module)> {
        self.modules.iter().filter_map(move |module| module.melee_percent().map(|percent| (if percent { 75 } else { self.priority }, module)))
    }
    /// The conditional stat modules, each with its sort priority (a percent module runs at 75).
    pub fn conditional_stat_modules(&self) -> impl Iterator<Item = (i32, &crate::modifier_modules::Module)> {
        self.modules.iter().filter_map(move |module| module.conditional_stat_percent().map(|percent| (if percent { 75 } else { self.priority }, module)))
    }
    /// The conditional power modules, each with its sort priority (a percent module runs at 75).
    pub fn conditional_power_modules(&self) -> impl Iterator<Item = (i32, &crate::conditional_stat::ConditionalPower)> {
        self.modules.iter().filter_map(move |module| module.conditional_power().map(|power| (if power.percent() { 75 } else { self.priority }, power)))
    }
    /// The knockback modules, each with the modifier's priority.
    pub fn knockback_modules(&self) -> impl Iterator<Item = (i32, &crate::modifier_modules::Module)> {
        self.modules.iter().filter(|module| module.is_knockback()).map(move |module| (self.priority, module))
    }
    /// The block damage modules.
    pub fn block_damage_modules(&self) -> impl Iterator<Item = &crate::modifier_modules::Module> {
        self.modules.iter().filter(|module| module.is_block_damage())
    }
    /// The first capacity bar module (the bar a shield of this modifier drains).
    pub fn capacity_bar(&self) -> Option<&crate::capacity::Bar> { self.modules.iter().find_map(|module| module.capacity_bar()) }
    /// The melee capacity modules.
    pub fn melee_capacity_modules(&self) -> impl Iterator<Item = &crate::modifier_modules::Module> { self.modules.iter().filter(|module| module.is_melee_capacity()) }
    /// The mining capacity modules.
    pub fn mining_capacity_modules(&self) -> impl Iterator<Item = &crate::modifier_modules::Module> { self.modules.iter().filter(|module| module.is_mining_capacity()) }
    /// The launch capacity modules.
    pub fn launch_capacity_modules(&self) -> impl Iterator<Item = &crate::modifier_modules::Module> { self.modules.iter().filter(|module| module.is_launch_capacity()) }
    /// The lifesteal modules.
    pub fn lifesteal_modules(&self) -> impl Iterator<Item = &crate::modifier_modules::Module> { self.modules.iter().filter(|module| module.is_lifesteal()) }
    /// The loot to capacity modules.
    pub fn loot_capacity_modules(&self) -> impl Iterator<Item = &crate::modifier_modules::Module> { self.modules.iter().filter(|module| module.is_loot_capacity()) }
    /// The effective levels of this modifier's severing modules, one per module.
    pub fn severing_levels(&self, entry: &Entry) -> Vec<f32> { self.modules.iter().filter_map(|module| module.severing_level(entry)).collect() }
    /// The damage to capacity modules.
    pub fn damage_capacity_modules(&self) -> impl Iterator<Item = &crate::modifier_modules::Module> { self.modules.iter().filter(|module| module.is_damage_capacity()) }
    /// The durability shields.
    pub fn shields(&self) -> impl Iterator<Item = &crate::capacity::Shield> { self.modules.iter().filter_map(|module| module.shield()) }
    /// The effect immunity modules.
    pub fn effect_immunity_modules(&self) -> impl Iterator<Item = &crate::modifier_modules::Module> {
        self.modules.iter().filter(|module| module.is_effect_immunity())
    }
    /// The inventory modules, in module order.
    pub fn inventory_parts(&self, entry: &Entry, facts: &crate::tool_predicate::Facts) -> Vec<crate::tool_inventory::Part> {
        self.modules.iter().filter_map(|module| module.inventory_part(entry, facts)).collect()
    }
    /// The keys of the inventory menu modules.
    pub fn inventory_menu_keys(&self) -> Vec<crate::tool_inventory::MenuKey> {
        self.modules.iter().filter_map(|module| module.inventory_menu_key()).collect()
    }
    /// The max armor attribute modules.
    pub fn max_armor_attribute_modules(&self) -> impl Iterator<Item = &crate::modifier_modules::Module> {
        self.modules.iter().filter(|module| module.is_max_armor_attribute())
    }
    /// The armor level modules.
    pub fn armor_level_modules(&self) -> impl Iterator<Item = &crate::modifier_modules::Module> {
        self.modules.iter().filter(|module| module.is_armor_level())
    }
    /// The mob disguise modules.
    pub fn mob_disguise_modules(&self) -> impl Iterator<Item = &crate::modifier_modules::Module> {
        self.modules.iter().filter(|module| module.is_mob_disguise())
    }
    /// Whether this modifier carries the edible module.
    pub fn is_edible(&self) -> bool { self.modules.iter().any(|module| module.is_edible()) }
    /// What eating a tool does for this modifier's edible effect modules, in module order.
    pub fn edible_effects(&self, entry: &Entry, facts: &crate::tool_predicate::Facts) -> Vec<crate::edible::Effect> {
        self.modules.iter().filter_map(|module| module.edible_effect(entry, facts)).collect()
    }
    /// The thorns modules.
    pub fn thorns_modules(&self) -> impl Iterator<Item = &crate::modifier_modules::Module> {
        self.modules.iter().filter(|module| module.is_thorns())
    }
    /// The fiery and freezing attack modules.
    pub fn attack_action_modules(&self) -> impl Iterator<Item = &crate::modifier_modules::Module> {
        self.modules.iter().filter(|module| module.is_attack_action())
    }
    /// The fiery, freezing and knockback counter modules.
    pub fn counter_action_modules(&self) -> impl Iterator<Item = &crate::modifier_modules::Module> {
        self.modules.iter().filter(|module| module.is_counter_action())
    }
    /// The armor counter effect modules.
    pub fn counter_effect_modules(&self) -> impl Iterator<Item = &crate::modifier_modules::Module> {
        self.modules.iter().filter(|module| module.is_counter_effect())
    }
    /// The armor attack effect modules.
    pub fn armor_attack_effect_modules(&self) -> impl Iterator<Item = &crate::modifier_modules::Module> {
        self.modules.iter().filter(|module| module.is_armor_attack_effect())
    }
    /// The tool usage effect modules.
    pub fn usage_effect_modules(&self) -> impl Iterator<Item = &crate::modifier_modules::Module> {
        self.modules.iter().filter(|module| module.is_usage_effect())
    }
    /// The weapon effect modules.
    pub fn weapon_effect_modules(&self) -> impl Iterator<Item = &crate::modifier_modules::Module> {
        self.modules.iter().filter(|module| module.is_weapon_effect())
    }
    /// The tool-stats hook.
    pub fn apply_stats(&self, entry: &Entry, facts: &crate::tool_predicate::Facts, builder: &mut crate::tool_stats::Builder) {
        // Module priorities: a stat copy runs at 50 so it sees what the others added; the rest take the modifier's priority (stable order).
        let mut ordered: Vec<(i32, &crate::modifier_modules::Module)> = self.modules.iter().map(|module| (module.stat_priority().unwrap_or(self.priority), module)).collect();
        ordered.sort_by_key(|(priority, _)| std::cmp::Reverse(*priority));
        for (_, module) in ordered { module.apply_stats(entry, facts, builder); }
    }
}

/// Facts for a context with no tool stack.
fn no_tool() -> crate::tool_predicate::Facts<'static> {
    crate::tool_predicate::Facts { item: None, in_tag: &|_, _| false, level: &|_, _| 0, tag_level: &|_, _| 0, has_persistent_key: &|_| false, persistent_string: &|_| String::new(), persistent_numbers: &|_| Vec::new(), hook_traits: &|_| Vec::new(), has_hook: &|_| false }
}

#[derive(Clone, Debug)]
pub(crate) struct Registry {
    static_modifiers: BTreeMap<Identifier, Arc<Modifier>>,
    expected: BTreeSet<Identifier>,
    dynamic: BTreeMap<Identifier, Arc<Modifier>>,
    pub tags: BTreeMap<Identifier, Vec<Identifier>>,
    pub reverse_tags: BTreeMap<Identifier, BTreeSet<Identifier>>,
    pub definitions_loaded: bool,
    pub generation: u64,
    pub problems: Vec<String>,
}
impl Default for Registry {
    fn default() -> Self {
        let id = Identifier::new("tconstruct", "empty").unwrap();
        let mut empty = Modifier::decode(id.clone(), &serde_json::json!({"show_in_tooltips":"never"})).unwrap();
        empty.empty_sentinel = true;
        Self { static_modifiers: [(id, Arc::new(empty))].into_iter().collect(), expected: BTreeSet::new(),
            dynamic: BTreeMap::new(), tags: BTreeMap::new(), reverse_tags: BTreeMap::new(),
            definitions_loaded: false, generation: 0, problems: Vec::new() }
    }
}
impl Registry {
    pub fn register_static(&mut self, modifier: Modifier) -> Result<(), String> {
        if self.expected.contains(&modifier.id) || self.static_modifiers.contains_key(&modifier.id) {
            return Err(format!("duplicate static/expected modifier {}", modifier.id));
        }
        self.static_modifiers.insert(modifier.id.clone(), Arc::new(modifier));
        Ok(())
    }
    pub fn expect_dynamic(&mut self, id: Identifier) -> Result<(), String> {
        if self.static_modifiers.contains_key(&id) { return Err(format!("modifier {id} is static")); }
        self.expected.insert(id); Ok(())
    }
    pub fn get(&self, id: &Identifier) -> Option<&Arc<Modifier>> {
        self.static_modifiers.get(id).or_else(|| self.dynamic.get(id))
    }
    /// Every modifier identifier this registry holds, static and loaded, sorted: the stable order menus index modifiers by.
    pub fn all_ids(&self) -> Vec<Identifier> {
        let mut ids: Vec<Identifier> = self.static_modifiers.keys().chain(self.dynamic.keys()).cloned().collect();
        ids.sort();
        ids.dedup();
        ids
    }
    /// Identifiers of every loaded dynamic modifier (including redirects), for audits.
    pub fn dynamic_ids(&self) -> impl Iterator<Item = &Identifier> { self.dynamic.keys() }
    pub fn contains_static(&self, id: &Identifier) -> bool { self.static_modifiers.contains_key(id) || self.expected.contains(id) }
    pub fn clear_dynamic(&mut self) {
        self.dynamic.clear(); self.tags.clear(); self.reverse_tags.clear();
        self.definitions_loaded = false; self.problems.clear();
        self.generation = self.generation.wrapping_add(1);
    }
    pub fn reload<C: Conditions>(&mut self, files: &[(Identifier, Value)], conditions: &C) {
        self.clear_dynamic();
        let mut redirects = BTreeMap::new();
        for (name, value) in files {
            let selected = (|| -> Result<Option<Identifier>, String> {
                let fields = value.as_object().ok_or("modifier must be an object")?;
                if let Some(values) = fields.get("redirects") {
                    // JsonHelper.parseList decodes the entire redirect list before testing any condition.
                    let redirects = values.as_array().ok_or("redirects must be an array")?.iter().map(|value| {
                        let fields = value.as_object().ok_or("redirect must be an object")?;
                        let target = id(fields.get("id").ok_or("redirect ID missing")?)?;
                        let condition = fields.get("condition").filter(|v| !v.is_null()).map(|value| conditions.decode(value)).transpose()?;
                        Ok((target, condition))
                    }).collect::<Result<Vec<_>, String>>()?;
                    for (target, condition) in redirects {
                        if condition.as_ref().map(|c| conditions.test(c)).transpose()?.unwrap_or(true) {
                            // Return through the selection below without decoding the unused modifier.
                            return Ok(Some(target));
                        }
                    }
                }
                if let Some(condition) = fields.get("condition") {
                    if !conditions.test(&conditions.decode(condition)?)? { return Ok(None); }
                }
                let modifier = Modifier::decode(name.clone(), value)?;
                self.dynamic.insert(name.clone(), Arc::new(modifier));
                Ok(None)
            })();
            match selected {
                Ok(Some(target)) => { redirects.insert(name.clone(), target); }
                Err(error) => self.problems.push(format!("{name}: {error}")),
                Ok(None) => {}
            }
        }
        // Resolve against concrete static/dynamic definitions before adding any redirects.
        let resolved: Vec<_> = redirects.into_iter().filter_map(|(from, to)| {
            match self.get(&to).cloned() {
                Some(modifier) => Some((from, modifier)),
                None => { self.problems.push(format!("invalid modifier redirect {from} -> {to}")); None }
            }
        }).collect();
        self.dynamic.extend(resolved);
        for id in &self.expected {
            if !self.dynamic.contains_key(id) { self.problems.push(format!("missing expected modifier {id}")); }
        }
        self.definitions_loaded = true;
    }
    pub fn load_tags(&mut self, stacks: &[pluto_lib::JsonResourceStack]) {
        let mut problems = Vec::new();
        self.tags = crate::materials::load_resource_tags(stacks, |id| {
            let modifier = self.get(id)?;
            if modifier.empty_sentinel && id.to_string() != "tconstruct:empty" { return None; }
            Some(modifier.id.clone())
        }, &mut problems, "modifier");
        self.problems.extend(problems);
        self.reverse_tags.clear();
        for (tag, members) in &self.tags {
            for member in members { self.reverse_tags.entry(member.clone()).or_default().insert(tag.clone()); }
        }
    }

    /// Resolve before merging. Optional missing references are omitted; required
    /// missing implementations error instead of being silently replaced by empty hooks.
    pub fn resolve(&self, entries: &[crate::material_traits::ModifierEntry]) -> Result<Vec<(Entry, Arc<Modifier>)>, String> {
        self.resolve_with_upgrades(&crate::modifier_entries::List::default(), entries)
    }
    /// TraitBuilder: adds each modifier and, recursively, the traits its modules bring. A modifier
    /// met inside itself is skipped (the original logs an error), an entry with no loaded
    /// definition is dropped like an unbound entry, and the result merges and sorts by priority.
    pub fn expand_traits(&self, before: &[(Entry, Arc<Modifier>)], facts: &crate::tool_predicate::Facts) -> Result<Vec<(Entry, Arc<Modifier>)>, String> {
        struct State<'a> { registry: &'a Registry, facts: &'a crate::tool_predicate::Facts<'a>, built: Vec<(Entry, Arc<Modifier>)>,
            seen: BTreeSet<Identifier>, stack: Vec<Identifier> }
        fn add(state: &mut State, entry: Entry, modifier: Arc<Modifier>) -> Result<(), String> {
            if modifier.empty_sentinel { return Ok(()); }
            match state.built.iter_mut().find(|(current, _)| current.id == entry.id) {
                Some((current, _)) => *current = current.merge(&entry)?,
                None => state.built.push((entry.clone(), modifier.clone())),
            }
            if !modifier.has_trait_modules() { return Ok(()); }
            if state.stack.contains(&entry.id) { eprintln!("Tinkers: {} is a child of itself", entry.id); return Ok(()); }
            state.stack.push(entry.id.clone());
            let first = !state.seen.contains(&entry.id);
            for nested in modifier.trait_entries(&entry, state.facts, first) {
                // An entry with no loaded definition is not bound and is skipped.
                if let Some(definition) = state.registry.get(&nested.id).cloned() { add(state, nested, definition)?; }
            }
            state.seen.insert(entry.id.clone());
            state.stack.pop();
            Ok(())
        }
        let mut state = State { registry: self, facts, built: Vec::new(), seen: BTreeSet::new(), stack: Vec::new() };
        for (entry, modifier) in before { add(&mut state, entry.clone(), modifier.clone())?; }
        let mut result = state.built;
        result.sort_by_key(|(_, modifier)| modifier.priority.wrapping_neg());
        Ok(result)
    }
    pub fn resolve_with_upgrades(&self, upgrades: &crate::modifier_entries::List, traits: &[crate::material_traits::ModifierEntry]) -> Result<Vec<(Entry, Arc<Modifier>)>, String> {
        if !self.definitions_loaded { return Err("modifier definitions are not loaded".into()); }
        let mut result: Vec<(Entry, Arc<Modifier>)> = Vec::new();
        let inputs = upgrades.entries().iter().cloned().map(|entry| (entry, false)).chain(
            traits.iter().map(|entry| (Entry::new(entry.id.clone(), entry.level), entry.optional)));
        for (entry, optional) in inputs {
            let Some(modifier) = self.get(&entry.id) else {
                if optional { continue; }
                return Err(format!("required modifier {} unavailable", entry.id));
            };
            if modifier.empty_sentinel { continue; }
            if let Some((current, _)) = result.iter_mut().find(|(current, _)| current.id == entry.id) { *current = current.merge(&entry)?; }
            else { result.push((entry, modifier.clone())); }
        }
        // ModifierNBT.Builder sorts stably by negated Java int priority, not compareTo's path tie-breaker.
        result.sort_by_key(|(_, modifier)| modifier.priority.wrapping_neg());
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    struct BooleanConditions;
    impl Conditions for BooleanConditions {
        type Parsed = bool;
        fn decode(&self, value: &Value) -> Result<bool, String> { value.as_bool().ok_or("condition must be boolean".into()) }
        fn test(&self, value: &bool) -> Result<bool, String> { Ok(*value) }
    }
    fn name(value: &str) -> Identifier { Identifier::parse(value).unwrap() }
    fn reference(value: &str, level: i32) -> crate::material_traits::ModifierEntry {
        crate::material_traits::ModifierEntry { id: name(value), level, optional: false }
    }
    #[test]
    fn static_priority_redirect_identity_and_generation_owned_resolution() {
        let mut registry = Registry::default();
        assert!(registry.resolve(&[]).is_err());
        registry.register_static(Modifier::decode(name("test:static"), &json!({"priority":150})).unwrap()).unwrap();
        assert!(registry.expect_dynamic(name("test:static")).is_err());
        registry.expect_dynamic(name("test:expected")).unwrap();
        assert!(registry.contains_static(&name("test:expected")));
        assert!(registry.get(&name("test:expected")).is_none(), "expectation is not an implemented modifier");
        assert!(registry.register_static(Modifier::decode(name("test:expected"), &json!({})).unwrap()).is_err());
        registry.reload(&[
            (name("test:static"), json!({"priority":-10})),
            (name("test:expected"), json!({"priority":200,"modules":[{"type":"tconstruct:volatile_flag","flag":"test:flag","modifier_level":{"min":2}}]})),
            (name("test:alias"), json!({"redirects":[{"id":"test:expected"}],"condition":false,"modules":"unused"})),
            (name("test:chain"), json!({"redirects":[{"id":"test:alias"}]})),
            (name("test:empty_alias"), json!({"redirects":[{"id":"tconstruct:empty"}]})),
            (name("test:invalid"), json!({"redirects":[{"id":"test:expected"},{"id":"test:static","condition":"bad"}]})),
        ], &BooleanConditions);
        assert_eq!(registry.get(&name("test:static")).unwrap().priority, 150);
        assert!(Arc::ptr_eq(registry.get(&name("test:alias")).unwrap(), registry.get(&name("test:expected")).unwrap()));
        assert!(registry.get(&name("test:chain")).is_none());
        assert!(registry.get(&name("test:invalid")).is_none(), "decode all redirects before selecting");
        let resolved = registry.resolve(&[reference("test:static",1), reference("test:alias",1), reference("test:expected",1), reference("test:alias",1)]).unwrap();
        assert_eq!(resolved.iter().map(|(entry,_)| (entry.id.to_string(),entry.level)).collect::<Vec<_>>(),
            vec![("test:alias".into(),2),("test:expected".into(),1),("test:static".into(),1)]);
        let mut data = NbtCompound::new();
        for (entry, modifier) in &resolved { modifier.apply_volatile(entry, &mut data); }
        assert_eq!(data["test:flag"], Nbt::Byte(1));
        registry.load_tags(&[
            pluto_lib::JsonResourceStack { id: name("test:members"), layers: vec![Ok(json!({"values":["test:alias","test:expected","tconstruct:empty"]}))] },
            pluto_lib::JsonResourceStack { id: name("test:invalid_empty"), layers: vec![Ok(json!({"values":["test:empty_alias"]}))] },
        ]);
        assert_eq!(registry.tags[&name("test:members")], vec![name("test:expected"), name("tconstruct:empty")]);
        assert!(!registry.tags.contains_key(&name("test:invalid_empty")), "only an explicit empty ID may reference the default object");
        assert!(registry.reverse_tags[&name("test:expected")].contains(&name("test:members")));
        assert!(!registry.reverse_tags.contains_key(&name("test:alias")), "reverse membership uses resolved modifier identity");
        registry.reload(&[], &BooleanConditions);
        assert!(registry.resolve(&[reference("test:expected",1)]).is_err());
        let mut optional = reference("test:expected",1); optional.optional = true;
        assert!(registry.resolve(&[optional]).unwrap().is_empty());
        assert_eq!(resolved[0].1.id, name("test:expected"), "owned resolved values survive a reload");
        registry.clear_dynamic();
        assert!(!registry.definitions_loaded);
        assert!(registry.get(&name("test:static")).is_some());
    }
    #[test]
    fn composable_flags_validate_full_definitions_and_level_ranges() {
        let modifier = Modifier::decode(name("test:flag"), &json!({"level_display":"tconstruct:no_levels","tooltip_display":"never",
            "modules":[{"type":"tconstruct:volatile_flag","flag":"test:flag","modifier_level":2}]})).unwrap();
        assert_eq!(modifier.level_display, name("tconstruct:no_levels"));
        assert!(!modifier.tooltips.contains("tool")); assert!(modifier.tooltips.contains("book"));
        let mut data = NbtCompound::new();
        modifier.apply_volatile(&Entry::new(name("test:flag"),1), &mut data);
        assert!(data.is_empty());
        modifier.apply_volatile(&Entry::new(name("test:flag"),2), &mut data);
        assert_eq!(data["test:flag"], Nbt::Byte(1));
        for invalid in [
            json!({"modules":[{"type":"tconstruct:volatile_flag","flag":"test:x"},{"type":"test:missing"}]}),
            json!({"modules":[{"type":"tconstruct:volatile_flag","flag":"test:x","modifier_level":0}]}),
            json!({"modules":[{"type":"tconstruct:volatile_flag","flag":"test:x","modifier_level":{"min":3,"max":2}}]}),
            json!({"modules":[{"type":"tconstruct:empty","hooks":["tconstruct:volatile_data"]}]}),
        ] { assert!(Modifier::decode(name("test:bad"), &invalid).is_err()); }
        // A predicate this port lacks, or another hook binding, is recorded as unbound, never installed.
        for gated in [
            json!({"modules":[{"type":"tconstruct:volatile_flag","flag":"test:x","tool":{"type":"tconstruct:has_hook","hook":"a:b"}}]}),
            json!({"modules":[{"type":"tconstruct:volatile_flag","flag":"test:x","hooks":["tconstruct:projectile_launch"]}]}),
        ] {
            let modifier = Modifier::decode(name("test:gated"), &gated).unwrap();
            assert_eq!(modifier.unbound.iter().map(Identifier::to_string).collect::<Vec<_>>(), vec!["tconstruct:volatile_flag"]);
            let mut data = NbtCompound::new();
            modifier.apply_volatile(&Entry::new(name("test:gated"), 1), &mut data);
            assert!(data.is_empty(), "an unbound flag writes nothing");
        }
    }
    #[test]
    fn nested_traits_scale_with_the_owner_join_once_when_fixed_and_cannot_recurse() {
        use crate::tool_predicate::{Check, Facts};
        let mut registry = Registry::default();
        let module = |name: &str, level: i64, fixed: bool| json!({"type":"tconstruct:trait","name":name,"level":level,"fixed_level":fixed});
        registry.reload(&[
            (name("test:owner"), json!({"modules":[module("test:scaled", 2, false), module("test:once", 1, true), module("test:missing", 1, false)]})),
            (name("test:other"), json!({"priority":50,"modules":[module("test:once", 1, true)]})),
            (name("test:wrapper"), json!({"priority":200,"modules":[module("test:owner", 1, false)]})),
            (name("test:scaled"), json!({})), (name("test:once"), json!({})),
            (name("test:loop"), json!({"modules":[module("test:loop", 1, false), module("test:scaled", 1, false)]})),
            (name("test:gated"), json!({"modules":[{"type":"tconstruct:trait","name":"test:scaled","level":1,"fixed_level":false,
                "tool":{"type":"tconstruct:has_modifier","check":"all","level":0,"modifier":{"type":"tconstruct:single","modifier":"test:other"}}}]})),
        ], &BooleanConditions);
        assert!(registry.problems.is_empty(), "{:?}", registry.problems);
        let facts = Facts { item: None, in_tag: &|_, _| false, has_persistent_key: &|_| false, tag_level: &|_, _| 0,
            level: &|modifier, _check| if modifier.path() == "other" { 1 } else { 0 }, persistent_string: &|_| String::new(), persistent_numbers: &|_| Vec::new(), hook_traits: &|_| Vec::new(), has_hook: &|_| false };
        let before = |levels: &[(&str, i32)]| levels.iter().map(|(id, level)| (Entry::new(name(id), *level), registry.get(&name(id)).unwrap().clone())).collect::<Vec<_>>();
        let level_of = |built: &[(Entry, Arc<Modifier>)], id: &str| built.iter().find(|(entry, _)| entry.id == name(id)).map(|(entry, _)| entry.level);
        // Owner level 3: the scaled trait is 2 * 3, the fixed one joins once even with a second owner.
        let built = registry.expand_traits(&before(&[("test:owner", 3), ("test:other", 1)]), &facts).unwrap();
        assert_eq!(level_of(&built, "test:scaled"), Some(6));
        assert_eq!(level_of(&built, "test:once"), Some(2), "each distinct owner adds its fixed trait once");
        assert_eq!(level_of(&built, "test:missing"), None, "an unbound nested entry is skipped, not an error");
        // Reached twice (nested under a wrapper, then listed itself), an owner adds its fixed trait only the first time.
        let built = registry.expand_traits(&before(&[("test:wrapper", 1), ("test:owner", 1)]), &facts).unwrap();
        assert_eq!(level_of(&built, "test:once"), Some(1));
        assert_eq!(level_of(&built, "test:scaled"), Some(4), "the scaled trait is added on each encounter");
        // Owner level 1 uses the trait's own level; scaled traits of two owners merge by id.
        let built = registry.expand_traits(&before(&[("test:owner", 1), ("test:scaled", 4)]), &facts).unwrap();
        assert_eq!(level_of(&built, "test:scaled"), Some(6), "4 saved + 2 from the owner");
        // A modifier that lists itself as a trait terminates: the nested copy still merges its level
        // (the builder adds before the stack check) but its traits are not expanded again.
        let built = registry.expand_traits(&before(&[("test:loop", 1)]), &facts).unwrap();
        assert_eq!((level_of(&built, "test:loop"), level_of(&built, "test:scaled")), (Some(2), Some(1)));
        // A tool predicate reads the tool's modifiers: this one needs test:other to be absent, and it is present.
        let built = registry.expand_traits(&before(&[("test:gated", 1)]), &facts).unwrap();
        assert_eq!(level_of(&built, "test:scaled"), None);
        assert!(registry.get(&name("test:owner")).unwrap().has_trait_modules());
        assert!(!registry.get(&name("test:scaled")).unwrap().has_trait_modules());
        let _ = Check::All;
    }
    #[test]
    fn rarity_keeps_the_largest_ordinal_and_ignores_level_conditions() {
        let rare = Modifier::decode(name("test:rare"), &json!({"modules":[{"type":"tconstruct:rarity","rarity":"rare"}]})).unwrap();
        let common = Modifier::decode(name("test:common"), &json!({"modules":[{"type":"tconstruct:rarity","rarity":"common"}]})).unwrap();
        let mut data = NbtCompound::new();
        // Common never writes (ordinal 0 is not greater than the absent default 0).
        common.apply_volatile(&Entry::new(name("test:common"), 1), &mut data);
        assert!(data.is_empty());
        rare.apply_volatile(&Entry::new(name("test:rare"), 1), &mut data);
        assert_eq!(data["tconstruct:rarity"], Nbt::Int(2));
        let epic = Modifier::decode(name("test:epic"), &json!({"modules":[{"type":"tconstruct:rarity","rarity":"epic"}]})).unwrap();
        epic.apply_volatile(&Entry::new(name("test:epic"), 1), &mut data);
        common.apply_volatile(&Entry::new(name("test:common"), 1), &mut data);
        rare.apply_volatile(&Entry::new(name("test:rare"), 1), &mut data);
        assert_eq!(data["tconstruct:rarity"], Nbt::Int(3));
        for invalid in [json!({"type":"tconstruct:rarity"}), json!({"type":"tconstruct:rarity","rarity":"legendary"}),
            json!({"type":"tconstruct:rarity","rarity":"Rare"})] {
            assert!(Modifier::decode(name("test:bad"), &json!({"modules":[invalid]})).is_err());
        }
    }
}

/// The modifiers the original defines only in code (a Java class, no data file) that a material trait names and Tinkers has not ported:
/// DwarvenModifier (copper), StrongBonesModifier (bone), ConductingModifier (blazing bone), EnderportingModifier (ender pearl and ender slime
/// vine), InsatiableModifier (jadeite), LaceratingModifier (nahuatl), MomentumModifier (hepatizon) and SelfDestructiveModifier (gunpowder).
/// Each is registered with no behaviour and its own id in `unbound`, so a tool of those materials builds (as the original's lookup of a modifier with
/// no behaviour does) and the missing behaviour is listed rather than made up; the traits' effects wait for their classes.
pub(crate) const CODE_MODIFIERS: &[&str] = &["conducting", "dwarven", "enderporting", "insatiable", "lacerating", "momentum", "self_destructive", "strong_bones"];

pub(crate) fn unported_code_modifiers() -> Result<Vec<Modifier>, String> {
    CODE_MODIFIERS.iter().map(|name| {
        let id = Identifier::new("tconstruct", name).map_err(|e| e.to_string())?;
        let mut modifier = Modifier::decode(id.clone(), &serde_json::json!({}))?;
        modifier.unbound = vec![id];
        Ok(modifier)
    }).collect()
}

pub(crate) fn register<C: Conditions + 'static>(context: impl Fn() -> Result<C, String> + Send + Sync + 'static) -> Result<Arc<RwLock<Registry>>, String> {
    // Overslime is defined in code in the original; it survives every reload as a static modifier.
    let mut registry = Registry::default();
    for modifier in crate::overslime::code_modifiers()? { registry.register_static(modifier)?; }
    for modifier in unported_code_modifiers()? { registry.register_static(modifier)?; }
    let state = Arc::new(RwLock::new(registry));
    let output = state.clone();
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "modifier_tags").unwrap(), TAG_FOLDER, |_| Ok(()),
    )?;
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "modifiers").unwrap(), FOLDER, move |files| {
            let context = context().and_then(|context| {
                let tags = ResourceManagerHelper::reload_json_stacks(TAG_FOLDER).ok_or("modifier tag folder was not captured")?;
                Ok((context, tags))
            });
            let mut state = output.write().map_err(|_| "modifier registry poisoned")?;
            match context {
                Ok((context, tags)) => {
                    state.reload(files, &context);
                    state.load_tags(&tags);
                    for problem in &state.problems { eprintln!("Tinkers: {problem}"); }
                    Ok(())
                },
                Err(error) => { state.clear_dynamic(); state.problems.push(error.clone()); Err(error) },
            }
        })?;
    let stopped = state.clone();
    ServerLifecycleEvents::SERVER_STOPPED.register(move |_| stopped.write().expect("modifier registry poisoned").clear_dynamic());
    Ok(state)
}
