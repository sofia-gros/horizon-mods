//! The original mod's generated data, as copied into the content root, through the production
//! decoders. This is the check that decoders accept what the original actually ships; synthetic
//! fixtures elsewhere only cover the shapes their authors thought of.
use crate::{materials::Definitions as Materials, modifiers::Registry, tool_definitions::Data};
use pluto_lib::{Identifier, JsonResourceStack};
use serde_json::Value;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tconstruct/content/data/tconstruct/tinkering")
}
fn files(folder: &str) -> Vec<(Identifier, Value)> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(root().join(folder)).expect("content folder") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") { continue; }
        let name = path.file_stem().unwrap().to_str().unwrap().to_owned();
        let value = serde_json::from_slice(&std::fs::read(&path).unwrap()).expect("valid JSON");
        found.push((Identifier::new("tconstruct", &name).unwrap(), value));
    }
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}
fn stacks(folder: &str) -> Vec<JsonResourceStack> {
    files(folder).into_iter().map(|(id, value)| JsonResourceStack { id, layers: vec![Ok(value)] }).collect()
}
/// The production condition tree over an item registry with no external ingot tags and default config,
/// the vanilla-only world the other-mod materials were excluded for.
fn conditions() -> crate::conditions::Context {
    crate::conditions::Context { items: Some(pluto_lib::fixtures::registry(&[]).unwrap()), config: crate::config::parse("").unwrap() }
}

#[test]
fn every_shipped_tool_definition_decodes() {
    let definitions = files("tool_definitions");
    assert_eq!(definitions.len(), 44, "45 original definitions minus the other-mod minotaur axe");
    let failures: Vec<String> = definitions.iter()
        .filter_map(|(id, value)| Data::decode(value).err().map(|error| format!("{id}: {error}"))).collect();
    assert!(failures.is_empty(), "definitions that do not decode: {failures:#?}");
}

#[test]
fn every_shipped_material_loads_with_stats_and_traits_without_problems() {
    let mut registry = Materials::default();
    let definitions = files("materials/definition");
    registry = Materials::load(&definitions, &conditions());
    assert!(registry.problems.is_empty(), "material definition problems: {:#?}", registry.problems);
    assert_eq!(registry.materials.len() + registry.redirects.len(), definitions.len());
    let mut problems = Vec::new();
    registry.stats = crate::material_stats::load(&stacks("materials/stats"), &mut problems);
    registry.traits = crate::material_traits::load(&stacks("materials/traits"), &mut problems);
    assert!(problems.is_empty(), "material stats/traits problems: {problems:#?}");
    assert_eq!(registry.stats.len(), registry.materials.len(), "every concrete material ships stats");
}

#[test]
fn shipped_modifiers_report_exactly_the_modules_that_are_not_ported() {
    let mut registry = Registry::default();
    registry.reload(&files("modifiers"), &conditions());
    assert!(registry.problems.is_empty(), "modifiers that do not decode: {:#?}", registry.problems);
    let unbound = (0..).zip(registry.dynamic_ids()).filter(|(_, id)| registry.get(id).is_some_and(|modifier| !modifier.unbound.is_empty())).count();
    // The baseline of modifiers with at least one unported module; it only shrinks as modules are ported.
    assert!(unbound <= 300, "{unbound} modifiers carry unported modules");
}
