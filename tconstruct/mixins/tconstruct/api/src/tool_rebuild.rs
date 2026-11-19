//! ToolStack.rebuildStats ordering from SlimeKnights (MIT; see LICENSE).
//! Pure rebuild of derived tool state. Persisting it into stack NBT, clamping damage
//! to the new durability and RAW_DATA hooks belong to the caller that owns the stack.
use crate::{
    material_traits::ModifierEntry, materials::Definitions as Materials, modifier_entries::{Entry, List},
    modifiers::{Modifier, Registry}, tool_definitions::Data, tool_materials::MaterialList,
    tool_predicate::{Check, Facts}, tool_stats::Stat,
};
use pluto_lib::{Identifier, Nbt, NbtCompound};
use std::{collections::BTreeMap, sync::Arc};

/// The four gates checked before any state is touched.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Readiness { pub definition: bool, pub materials: bool, pub modifiers: bool, pub item_tags: bool }
impl Readiness {
    pub fn complete(self) -> bool { self.definition && self.materials && self.modifiers && self.item_tags }
}

#[derive(Debug)]
pub(crate) struct Rebuilt {
    /// Final modifier list (upgrades + traits), stably sorted by priority.
    pub modifiers: Vec<(Entry, Arc<Modifier>)>,
    pub volatile: NbtCompound,
    pub stats: BTreeMap<Stat, f32>,
    pub multipliers: BTreeMap<Stat, f32>,
    /// Harvest tier when any module contributed one (StatsNBT stores it as a string).
    pub tier: Option<Identifier>,
}

/// What tool predicates and stat support can ask of the tool being rebuilt.
pub(crate) struct Tool<'a> {
    pub item: Option<&'a Identifier>,
    pub item_tags: Option<&'a pluto_lib::RegistryLookup>,
    /// The persistent mod data compound (tic_persistent) for has_persistent_key.
    pub persistent: &'a NbtCompound,
    /// The definition's and materials' traits for an alternate ToolTraitHook; None outside a stack.
    pub hook_traits: Option<&'a dyn Fn(&Identifier) -> Vec<ModifierEntry>>,
    /// Whether the definition carries a hook (HasToolHookPredicate); None outside a stack.
    pub has_hook: Option<&'a dyn Fn(&Identifier) -> bool>,
    /// Modifier -> the modifier tags it is in (Registry.reverse_tags), for tag matches on modifiers.
    pub modifier_tags: Option<&'a std::collections::BTreeMap<Identifier, std::collections::BTreeSet<Identifier>>>,
}

/// `Ok(None)` mirrors the original early return: not ready, so leave all tool state alone.
/// An `Err` means a required modifier reference is unresolved, which must not silently
/// become a tool with missing traits.
pub(crate) fn rebuild(
    readiness: Readiness, definition: &Data, materials: &MaterialList, upgrades: &List,
    material_registry: &Materials, modifiers: &Registry, tool: &Tool,
) -> Result<Option<Rebuilt>, String> {
    if !readiness.complete() { return Ok(None); }
    // Upgrades first, then tool/material traits (ToolHooks.TOOL_TRAITS).
    let traits: Vec<ModifierEntry> = definition.traits(materials, material_registry)?;
    // No modifier defines MODIFIER_TRAITS yet (such modules reject at decode), so the
    // TraitBuilder pass adds each bound entry exactly once, in this order.
    let before = modifiers.resolve_with_upgrades(upgrades, &traits)?;
    // Modifier traits are expanded against the tool as it stands before them ("modifiers does not include traits").
    let resolved = with_facts(tool, &before, upgrades, |facts| modifiers.expand_traits(&before, facts))?;
    // Volatile data is built before stats so stat hooks can read it: definition, then modifiers.
    with_facts(tool, &resolved, upgrades, |facts| {
        let mut volatile = definition.build_volatile_for(materials, material_registry);
        for (entry, modifier) in &resolved { modifier.apply_volatile_with(entry, facts, &mut volatile); }
        // Stats come last so they can read the volatile data: definition, then each modifier in order.
        let mut builder = definition.build_stats(materials, material_registry);
        for (entry, modifier) in &resolved { modifier.apply_stats(entry, facts, &mut builder); }
        Ok(Some(Rebuilt { modifiers: resolved.clone(), volatile, stats: builder.build(), multipliers: builder.build_multipliers(), tier: builder.touched_tier() }))
    })
}

/// Facts about the tool for predicates, with `entries` as the "all" modifier list.
pub(crate) fn with_facts<R>(tool: &Tool, entries: &[(Entry, Arc<Modifier>)], upgrades: &List, run: impl FnOnce(&Facts) -> R) -> R {
    let in_tag = |item: &Identifier, tag: &Identifier| tool.item_tags.and_then(|tags| tags.get_tag(tag)).is_some_and(|members| members.contains(item));
    let level = |modifier: &Identifier, check: Check| -> i32 {
        let sum = |entries: &mut dyn Iterator<Item = &Entry>| entries.filter(|entry| &entry.id == modifier).fold(0_i32, |sum, entry| sum.wrapping_add(entry.int_effective_level()));
        match check {
            Check::All => sum(&mut entries.iter().map(|(entry, _)| entry)),
            Check::Upgrades => sum(&mut upgrades.entries().iter()),
        }
    };
    let persistent_string = |key: &Identifier| match tool.persistent.get(&key.to_string()) { Some(Nbt::String(text)) => text.clone(), _ => String::new() };
    let persistent_numbers = |key: &Identifier| -> Vec<(String, f64)> {
        match tool.persistent.get(&key.to_string()) {
            Some(Nbt::Compound(data)) => data.iter().filter_map(|(name, value)| match value { Nbt::Byte(n) => Some((name.clone(), f64::from(*n))), Nbt::Short(n) => Some((name.clone(), f64::from(*n))),
                Nbt::Int(n) => Some((name.clone(), f64::from(*n))), Nbt::Long(n) => Some((name.clone(), *n as f64)),
                Nbt::Float(n) => Some((name.clone(), f64::from(*n))), Nbt::Double(n) => Some((name.clone(), *n)), _ => None }).collect(),
            _ => Vec::new(),
        }
    };
    let hook_traits = |hook: &Identifier| tool.hook_traits.map(|lookup| lookup(hook)).unwrap_or_default();
    let has_hook = |hook: &Identifier| tool.has_hook.is_some_and(|lookup| lookup(hook));
    let has_persistent_key = |key: &Identifier| tool.persistent.contains_key(&key.to_string());
    // A tag match sums the levels of every checked modifier that is in the tag.
    let tag_level = |tag: &Identifier, check: Check| -> i32 {
        let in_modifier_tag = |id: &Identifier| tool.modifier_tags.and_then(|tags| tags.get(id)).is_some_and(|member_of| member_of.contains(tag));
        let sum = |entries: &mut dyn Iterator<Item = &Entry>| entries.filter(|entry| in_modifier_tag(&entry.id)).fold(0_i32, |sum, entry| sum.wrapping_add(entry.int_effective_level()));
        match check {
            Check::All => sum(&mut entries.iter().map(|(entry, _)| entry)),
            Check::Upgrades => sum(&mut upgrades.entries().iter()),
        }
    };
    run(&Facts { item: tool.item, in_tag: &in_tag, level: &level, tag_level: &tag_level, has_persistent_key: &has_persistent_key, persistent_string: &persistent_string, persistent_numbers: &persistent_numbers, hook_traits: &hook_traits, has_hook: &has_hook })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use pluto_lib::Nbt;
    struct Always;
    impl crate::materials::Conditions for Always {
        type Parsed = ();
        fn decode(&self, _: &serde_json::Value) -> Result<(), String> { Ok(()) }
        fn test(&self, _: &()) -> Result<bool, String> { Ok(true) }
    }
    fn no_tool() -> Tool<'static> { static EMPTY: std::sync::OnceLock<NbtCompound> = std::sync::OnceLock::new(); Tool { item: None, item_tags: None, persistent: EMPTY.get_or_init(NbtCompound::new), hook_traits: None, has_hook: None, modifier_tags: None } }
    fn ready() -> Readiness { Readiness { definition: true, materials: true, modifiers: true, item_tags: true } }
    #[test]
    fn any_missing_gate_leaves_state_untouched() {
        let data = Data::default();
        for gate in 0..4 {
            let mut r = ready();
            match gate { 0 => r.definition = false, 1 => r.materials = false, 2 => r.modifiers = false, _ => r.item_tags = false }
            let result = rebuild(r, &data, &MaterialList::default(), &List::default(), &Materials::default(), &Registry::default(), &no_tool());
            assert!(matches!(result, Ok(None)), "gate {gate}");
        }
    }
    #[test]
    fn rebuild_resolves_traits_then_volatile_then_stats() {
        let data = Data::decode(&json!({"modules":[
            {"type":"tconstruct:traits","traits":[{"name":"test:rare","level":1}]},
            {"type":"tconstruct:volatile_flag","flag":"test:definition"},
            {"type":"tconstruct:base_stats","stats":{"tconstruct:durability":100}}
        ]})).unwrap();
        let mut registry = Registry::default();
        registry.reload(&[(Identifier::parse("test:rare").unwrap(),
            json!({"modules":[{"type":"tconstruct:rarity","rarity":"rare"},{"type":"tconstruct:volatile_flag","flag":"test:modifier"}]}))],
            &Always);
        let rebuilt = rebuild(ready(), &data, &MaterialList::default(), &List::default(), &Materials::default(), &registry, &no_tool())
            .unwrap().unwrap();
        assert_eq!(rebuilt.modifiers.len(), 1);
        assert_eq!(rebuilt.volatile["test:definition"], Nbt::Byte(1));
        assert_eq!(rebuilt.volatile["test:modifier"], Nbt::Byte(1));
        assert_eq!(rebuilt.volatile["tconstruct:rarity"], Nbt::Int(2));
        assert_eq!(rebuilt.stats[&Stat::Durability], 100.0);
        // Required but unknown trait: error, never a silently missing trait.
        let missing = Data::decode(&json!({"modules":[{"type":"tconstruct:traits","traits":[{"name":"test:absent","level":1}]}]})).unwrap();
        assert!(rebuild(ready(), &missing, &MaterialList::default(), &List::default(), &Materials::default(), &registry, &no_tool()).is_err());
    }
}
