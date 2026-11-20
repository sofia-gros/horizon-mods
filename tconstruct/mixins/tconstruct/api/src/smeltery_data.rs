//! The smeltery domain's loaded data: its recipe files (`recipes/fluids`) and fluid tags (`tinkering/fluid_tags`),
//! read by every machine that melts, burns fuel or casts (SlimeKnights Tinkers' Construct, MIT; see LICENSE).
//! The original asks the recipe manager and the tag manager; this keeps both in one server-side state that a
//! reload replaces and a server stop clears, so a machine never works from half a reload.
use crate::{fluid_ingredient::FluidTags, fluid_recipes::{self, Recipes}, materials::Conditions};
use pluto_lib::{Identifier, ResourceManagerHelper, ResourceType, ServerLifecycleEvents};
use serde_json::Value;
use std::sync::{Arc, RwLock};

/// Where the fluid tags are: `<namespace>/<path>.json` below this folder is the tag `<namespace>:<path>`.
pub(crate) const TAG_FOLDER: &str = "tinkering/fluid_tags";

#[derive(Clone, Debug, Default)]
pub(crate) struct Data {
    pub recipes: Recipes,
    pub tags: FluidTags,
    pub recipes_loaded: bool,
    pub tags_loaded: bool,
}

impl Data {
    /// True once both halves of the current reload have arrived.
    pub(crate) fn loaded(&self) -> bool { self.recipes_loaded && self.tags_loaded }
}

/// The tag a file in the tag folder stands for: its first folder is the namespace (the original keeps Forge's and Mantle's
/// tags beside its own), the rest the path. A file directly in the folder has no namespace and is left out.
pub(crate) fn tag_files(files: &[(Identifier, Value)]) -> Vec<(Identifier, Value)> {
    files.iter().filter_map(|(id, value)| {
        let (namespace, path) = id.path().split_once('/')?;
        Some((Identifier::new(namespace, path).ok()?, value.clone()))
    }).collect()
}

pub(crate) fn register<C: Conditions + 'static>(context: impl Fn() -> Result<C, String> + Send + Sync + 'static) -> Result<Arc<RwLock<Data>>, String> {
    let state = Arc::new(RwLock::new(Data::default()));
    let recipes_out = state.clone();
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "fluid_recipes").unwrap(), fluid_recipes::FOLDER, move |files| {
            let context = context();
            let mut state = recipes_out.write().map_err(|_| "smeltery data poisoned")?;
            match context {
                Ok(context) => {
                    state.recipes = Recipes::load(files, &context);
                    state.recipes_loaded = true;
                    for problem in &state.recipes.problems { eprintln!("Tinkers: {problem}"); }
                    Ok(())
                }
                Err(error) => { state.recipes = Recipes { problems: vec![error.clone()], ..Recipes::default() }; state.recipes_loaded = false; Err(error) }
            }
        })?;
    let tags_out = state.clone();
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "fluid_tags").unwrap(), TAG_FOLDER, move |files| {
            let mut state = tags_out.write().map_err(|_| "smeltery data poisoned")?;
            state.tags = FluidTags::load(&tag_files(files));
            state.tags_loaded = true;
            // Two shipped references name tags of other mods; every other problem is worth saying.
            for problem in &state.tags.problems { eprintln!("Tinkers: {problem}"); }
            Ok(())
        })?;
    let stopped = state.clone();
    ServerLifecycleEvents::SERVER_STOPPED.register(move |_| *stopped.write().expect("smeltery data poisoned") = Data::default());
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }

    #[test]
    fn a_tag_file_is_named_by_its_first_folder_and_a_loose_file_is_left_out() {
        let files = vec![
            (id("tconstruct:forge/molten_iron"), json!({"values": ["#tconstruct:molten_iron"]})),
            (id("tconstruct:tconstruct/molten_iron"), json!({"values": ["tconstruct:molten_iron"]})),
            (id("tconstruct:stray"), json!({"values": []})),
        ];
        let tags = FluidTags::load(&tag_files(&files));
        assert_eq!(tags.len(), 2);
        assert!(tags.contains(&id("tconstruct:molten_iron"), &id("forge:molten_iron")));
    }
}
