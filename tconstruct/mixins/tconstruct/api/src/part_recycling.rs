//! Part builder tool recycling (`tconstruct:part_builder_tool_recycling`), from SlimeKnights PartBuilderToolRecycle
//! (MIT; see LICENSE): a tool without upgrades in the material slot and a pattern item break into one of its
//! parts, in the tool's material for that part, with a chance of a second part when the tool is worn.
use crate::{
    material_recipes::Ingredient,
    materials::{Conditions, Definitions},
    modifier_recipes::Sized,
    tool_materials::{set_part_material, MaterialVariant, ToolPart},
    tool_stack,
};
use pluto_lib::{Identifier, ItemSlot, ResourceManagerHelper, ResourceType, ServerLifecycleEvents};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

pub(crate) const FOLDER: &str = "recipes/tools/recycling";
const RECYCLE: &str = "tconstruct:part_builder_tool_recycling";

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Recycle {
    pub id: Identifier,
    /// The tools this recipe takes (default: any multipart tool).
    pub tools: Sized,
    pub pattern: Ingredient,
    /// The parts to recover, overriding the tool definition's (empty: use the definition's).
    pub parts: Vec<Identifier>,
}

impl Recycle {
    fn decode(id: &Identifier, value: &Value) -> Result<Self, String> {
        let fields = value.as_object().ok_or("recipe must be an object")?;
        let tools = match fields.get("tools") {
            None => Sized { ingredient: Ingredient::Tag(Identifier::new("tconstruct", "multipart_tool").expect("valid tag")), amount: 1 },
            Some(value) => Sized::decode(value)?,
        };
        let pattern = Ingredient::decode(fields.get("pattern").ok_or("pattern missing")?)?;
        let parts = match fields.get("parts") {
            None => Vec::new(),
            Some(Value::Array(list)) => list.iter().map(|part| match part {
                Value::String(item) => Identifier::parse(item).map_err(|e| e.to_string()),
                _ => Err("a part must be an item ID string".to_owned()),
            }).collect::<Result<Vec<_>, _>>()?,
            Some(_) => return Err("parts must be an array".into()),
        };
        Ok(Self { id: id.clone(), tools, pattern, parts })
    }

    /// partialMatch: the pattern and the tool requirement match.
    pub fn partial_match(&self, tool: &(Identifier, u32), pattern: &Identifier, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> bool {
        self.pattern.matches(pattern, in_tag) && self.tools.test(tool, in_tag)
    }

    /// matches: also, the tool carries no upgrades.
    pub fn matches(&self, tool: &ItemSlot, pattern: &Identifier, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> bool {
        self.partial_match(&(tool.item.clone(), tool.count), pattern, in_tag) && tool_stack::upgrades(&tool.custom_data).entries().is_empty()
    }

    /// The parts this recipe recovers from a tool of the given definition.
    fn parts_for<'a>(&'a self, definition_parts: &'a [Identifier]) -> &'a [Identifier] {
        if self.parts.is_empty() { definition_parts } else { &self.parts }
    }

    /// The part slots of the tool whose part can still take the tool's material there, as (index, part item).
    fn usable(&self, tool: &ItemSlot, definition_parts: &[Identifier], registry: &BTreeMap<Identifier, ToolPart>, materials: &Definitions) -> Vec<(usize, Identifier)> {
        let list = tool_stack::materials(&tool.custom_data, true);
        let parts = self.parts_for(definition_parts);
        let size = parts.len().min(list.as_slice().len());
        (0..size).filter(|index| {
            let material = list.get(*index as i32);
            registry.get(&parts[*index]).is_some_and(|part| part.can_use(&material, materials))
        }).map(|index| (index, parts[index].clone())).collect()
    }

    /// getPatterns: each part the tool can give back, once each, in the tool's part order.
    pub fn buttons(&self, tool: &ItemSlot, definition_parts: &[Identifier], registry: &BTreeMap<Identifier, ToolPart>, materials: &Definitions) -> Vec<Identifier> {
        let mut buttons: Vec<Identifier> = Vec::new();
        for (_, part) in self.usable(tool, definition_parts, registry, materials) { if !buttons.contains(&part) { buttons.push(part); } }
        buttons
    }

    fn part_stack(&self, tool: &ItemSlot, index: usize, part: &Identifier) -> ItemSlot {
        let variant: MaterialVariant = tool_stack::materials(&tool.custom_data, true).get(index as i32);
        let mut stack = ItemSlot::new(part.clone(), 1);
        set_part_material(&mut stack, &variant);
        stack
    }

    /// assemble: the chosen part, in the tool's material for it.
    pub fn assemble(&self, tool: &ItemSlot, chosen: &Identifier, definition_parts: &[Identifier], registry: &BTreeMap<Identifier, ToolPart>, materials: &Definitions) -> Option<ItemSlot> {
        let (index, part) = self.usable(tool, definition_parts, registry, materials).into_iter().find(|(_, part)| part == chosen)?;
        Some(self.part_stack(tool, index, &part))
    }

    /// getLeftover: a worn tool gives nothing extra with a chance equal to its worn share; otherwise one of the parts that
    /// were not taken, picked at random (`random` is a value in [0, 1)).
    pub fn leftover(&self, tool: &ItemSlot, chosen: &Identifier, definition_parts: &[Identifier], registry: &BTreeMap<Identifier, ToolPart>,
        materials: &Definitions, random: &mut dyn FnMut() -> f32) -> Option<ItemSlot> {
        let damage = tool_stack::damage(&tool.custom_data);
        if damage > 0 {
            let max = tool_stack::durability(&tool.custom_data).max(1);
            if ((random() * max as f32) as i32) < damage { return None; }
        }
        // Every usable part but the first one that matches the choice.
        let mut found = false;
        let others: Vec<(usize, Identifier)> = self.usable(tool, definition_parts, registry, materials).into_iter().filter(|(_, part)| {
            if !found && part == chosen { found = true; false } else { true }
        }).collect();
        if others.is_empty() { return None; }
        let pick = ((random() * others.len() as f32) as usize).min(others.len() - 1);
        let (index, part) = &others[pick];
        Some(self.part_stack(tool, *index, part))
    }
}

/// A cheap random float in [0, 1) for the leftover's chances.
pub(crate) fn random() -> f32 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static STATE: AtomicU64 = AtomicU64::new(0x2545_F491_4F6C_DD1D);
    let mut x = STATE.load(Ordering::Relaxed) ^ std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.subsec_nanos() as u64);
    x ^= x << 13; x ^= x >> 7; x ^= x << 17;
    STATE.store(x, Ordering::Relaxed);
    (x >> 40) as f32 / (1u64 << 24) as f32
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Recycles { recipes: Vec<Recycle>, pub problems: Vec<String>, pub loaded: bool }

impl Recycles {
    pub fn load<C: Conditions>(files: &[(Identifier, Value)], conditions: &C) -> Self {
        let mut result = Self { loaded: true, ..Self::default() };
        let mut sorted: Vec<&(Identifier, Value)> = files.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for (id, value) in sorted {
            let outcome = (|| -> Result<Option<Recycle>, String> {
                let fields = value.as_object().ok_or("recipe must be an object")?;
                if fields.get("type").and_then(Value::as_str) != Some(RECYCLE) { return Ok(None); }
                if let Some(list) = fields.get("conditions") {
                    let parsed = list.as_array().ok_or("conditions must be an array")?.iter().map(|c| conditions.decode(c)).collect::<Result<Vec<_>, _>>()?;
                    for condition in &parsed { if !conditions.test(condition)? { return Ok(None); } }
                }
                Recycle::decode(id, value).map(Some)
            })();
            match outcome {
                Ok(Some(recipe)) => result.recipes.push(recipe),
                Ok(None) => {}
                Err(error) => result.problems.push(format!("{id}: {error}")),
            }
        }
        result
    }
    pub fn len(&self) -> usize { self.recipes.len() }
    /// The first recipe (in ID order) that fully matches the tool and the pattern.
    pub fn find(&self, tool: &ItemSlot, pattern: &Identifier, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Option<&Recycle> {
        self.recipes.iter().find(|recipe| recipe.matches(tool, pattern, in_tag))
    }
}

/// Reload listener, shaped like the other recipe listeners.
pub(crate) fn register<C: Conditions + 'static>(context: impl Fn() -> Result<C, String> + Send + Sync + 'static) -> Result<Arc<RwLock<Recycles>>, String> {
    let state = Arc::new(RwLock::new(Recycles::default()));
    let output = state.clone();
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "part_recycling").unwrap(), FOLDER, move |files| {
            let context = context();
            let mut state = output.write().map_err(|_| "part recycling poisoned")?;
            match context {
                Ok(context) => {
                    *state = Recycles::load(files, &context);
                    for problem in &state.problems { eprintln!("Tinkers: {problem}"); }
                    Ok(())
                }
                Err(error) => { *state = Recycles { problems: vec![error.clone()], ..Recycles::default() }; Err(error) }
            }
        })?;
    let stopped = state.clone();
    ServerLifecycleEvents::SERVER_STOPPED.register(move |_| *stopped.write().expect("part recycling poisoned") = Recycles::default());
    Ok(state)
}
