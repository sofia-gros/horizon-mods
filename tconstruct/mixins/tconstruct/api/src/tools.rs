//! Tools, on the original's library (`tconstruct-library`): the registries the tool code works from, loaded from the original's data.
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, OnceLock};

use serde_json::Value;
use tconstruct_library::condition::ConditionContext;
use tconstruct_library::materials::MaterialId;
use tconstruct_library::modifiers::stat_modules::ToolContext;
use tconstruct_library::tools::stack::rebuild_stats;
use tconstruct_library::tools::stat::StatValue;
use tconstruct_library::materials::manager::MaterialManager;
use tconstruct_library::materials::MaterialRegistry;
use tconstruct_library::modifiers::{ModifierManager, ModuleRegistry};
use tconstruct_library::resource::ResourceId;
use tconstruct_library::tools::definition::{ToolDefinitionData, ToolModuleRegistry};
use tconstruct_library::tools::stat::{HarvestTiers, ToolStats};

use crate::tinkering_data;
use crate::tool_data::ToolData;

/// What the data's conditions ask the game. The port has no mod integration: nothing else is installed, no tag of another mod has
/// members and the integration switches are off.
struct GameConditions;

impl ConditionContext for GameConditions {
    fn mod_loaded(&self, _: &str) -> bool {
        false
    }
    fn tag_filled(&self, _: &str) -> bool {
        false
    }
    fn config(&self, _: &str) -> bool {
        false
    }
}

/// Everything the tool code reads: stats, materials (with their stats and traits), modifiers and tool definitions.
pub struct ToolLibrary {
    pub stats: Arc<ToolStats>,
    pub materials: MaterialRegistry,
    pub modifiers: ModifierManager,
    pub definitions: HashMap<ResourceId, ToolDefinitionData>,
    /// The members of the `modifiable/*` item tags, with nested tags resolved (an item that does not exist is not checked).
    pub item_tags: BTreeMap<ResourceId, Vec<ResourceId>>,
    /// For each modifier a recipe adds: the slots it costs and the item tag of the tools it goes on.
    pub salvage: HashMap<tconstruct_library::modifiers::ModifierId, (BTreeMap<String, i32>, String)>,
    /// The `forge:` item tags the recipes name, with nested tags resolved.
    pub forge_tags: BTreeMap<ResourceId, Vec<ResourceId>>,
    /// For each modifier: the recipes that add a level of it, with the items each costs.
    pub recipes: HashMap<tconstruct_library::modifiers::ModifierId, Vec<ModifierRecipe>>,
    /// What loading found wrong, as the original would log it.
    pub problems: Vec<String>,
}

/// What one level of a modifier costs in items: an item (`minecraft:leather`) or an item tag (`#forge:string`) and how many.
#[derive(Clone, Debug, PartialEq)]
pub enum Ingredient {
    Item(String),
    Tag(String),
}

/// A recipe that adds one level of a modifier: the levels it can give and the items it takes.
#[derive(Clone, Debug, PartialEq)]
pub struct ModifierRecipe {
    pub min_level: i32,
    pub max_level: i32,
    pub inputs: Vec<(Ingredient, u32)>,
}

/// The recipe that adds a level of a modifier, in the form `/tconstruct upgrade` uses: the levels it can give and the items it takes, from the
/// library's `ModifierRecipe`. None for a recipe that is not carried: a kind that is not used here (swappable, overslime), data conditions that
/// are not met, or an input that is not one plain item or tag.
pub fn parse_recipe(id: ResourceId, json: &Value) -> Option<(tconstruct_library::modifiers::ModifierId, ModifierRecipe)> {
    use tconstruct_library::recipes::modifier::{Kind, ModifierRecipe as Library};
    use tconstruct_library::smeltery::melting::ItemMatch;
    let recipe = Library::from_json(id, json, &GameConditions).ok()??;
    let modifier = recipe.result.clone()?;
    let (min_level, max_level) = match &recipe.kind {
        Kind::Swappable { .. } | Kind::Overslime { .. } => return None,
        Kind::Multilevel { levels } | Kind::MultilevelIncremental { levels, .. } => (levels.iter().map(|(r, _)| r.min).min()?, levels.iter().map(|(r, _)| r.max).max()?),
        _ => (recipe.levels.min, recipe.levels.max),
    };
    let per_level = recipe.items_per_level();
    let mut inputs: Vec<(Ingredient, u32)> = Vec::new();
    for input in &recipe.inputs {
        let [one] = input.item.as_slice() else { return None };
        let what = match one {
            ItemMatch::Item(item) => Ingredient::Item(item.to_string()),
            ItemMatch::Tag(tag) => Ingredient::Tag(tag.to_string()),
            ItemMatch::Except { .. } => return None,
        };
        let count = per_level.map_or(input.amount, |n| n as u32);
        match inputs.iter_mut().find(|(i, _)| *i == what) {
            Some((_, n)) => *n += count,
            None => inputs.push((what, count)),
        }
    }
    Some((modifier, ModifierRecipe { min_level, max_level, inputs }))
}

fn files(kind: &str) -> Vec<(ResourceId, Value)> {
    tinkering_data::FILES
        .iter()
        .filter(|(k, _, _)| *k == kind)
        .filter_map(|(_, id, json)| Some((ResourceId::try_parse(id)?, serde_json::from_str(json).ok()?)))
        .collect()
}

impl ToolLibrary {
    /// Loads the library from the data that is built into the mod.
    pub fn load() -> Self {
        let stats = Arc::new(ToolStats::tinkers(HarvestTiers::vanilla()));
        let mut problems = Vec::new();
        let mut materials = MaterialRegistry::with_tinkers_stats();
        let (manager, material_problems) = MaterialManager::load(&files("materials/definition"), &files("tags/materials"), &GameConditions);
        materials.materials = manager;
        problems.extend(material_problems);
        problems.extend(materials.stats.load(&files("materials/stats")));
        problems.extend(materials.traits.load(&files("materials/traits")));

        let mut modules = ModuleRegistry::with_original_types();
        modules.register_stat_modules(stats.clone());
        let enchantments: Vec<Value> = serde_json::from_str(tinkering_data::ENCHANTMENTS_TO_MODIFIERS).into_iter().collect();
        let (modifiers, modifier_problems) = ModifierManager::load(&files("modifiers"), &files("tags/modifiers"), &enchantments, &modules, &GameConditions);
        problems.extend(modifier_problems);

        let tool_modules = ToolModuleRegistry::with_original_types();
        let mut definitions = HashMap::new();
        for (id, json) in files("tool_definitions") {
            match ToolDefinitionData::from_json(&json, &tool_modules, &stats) {
                Ok(data) => {
                    definitions.insert(id, data);
                }
                Err(error) => problems.push(format!("Failed to load tool definition {id}: {error}")),
            }
        }
        let (item_tags, tag_problems) = tconstruct_library::tags::load(&files("tags/item"), &|_| true);
        problems.extend(tag_problems);
        let mut salvage = HashMap::new();
        for (_, json) in files("modifier_salvage") {
            let (Some(modifier), Some(slots)) = (json.get("modifier").and_then(Value::as_str).and_then(tconstruct_library::modifiers::ModifierId::try_parse), json.get("slots").and_then(Value::as_object)) else { continue };
            let slots = slots.iter().filter_map(|(k, v)| Some((k.clone(), i32::try_from(v.as_i64()?).ok()?))).collect();
            let tag = json.get("tools").and_then(|t| t.get("tag")).and_then(Value::as_str).unwrap_or("tconstruct:modifiable").to_owned();
            salvage.insert(modifier, (slots, tag));
        }
        let (forge_tags, forge_problems) = tconstruct_library::tags::load(&files("tags/forge_item"), &|_| true);
        problems.extend(forge_problems);
        let mut recipes: HashMap<_, Vec<ModifierRecipe>> = HashMap::new();
        for (id, json) in files("modifier_recipes") {
            if let Some((modifier, recipe)) = parse_recipe(id, &json) {
                recipes.entry(modifier).or_default().push(recipe);
            }
        }
        Self { stats, materials, modifiers, definitions, item_tags, salvage, recipes, forge_tags, problems }
    }
}

/// The item tags of one tool item, for the conditions of the modules.
struct ToolItem<'a> {
    library: &'a ToolLibrary,
    item: ResourceId,
}

impl ToolContext for ToolItem<'_> {
    fn item_in_tag(&self, tag: &str) -> bool {
        ResourceId::try_parse(tag).and_then(|tag| self.library.item_tags.get(&tag)).is_some_and(|members| members.contains(&self.item))
    }
}

impl ToolLibrary {
    /// The stack of a tool made of `material` in every part: the tool's item, its durability as the stack's max damage and its materials,
    /// modifiers and stats as the stack's data (`tconstruct:materials`, `tconstruct:modifiers`, `tconstruct:stats` in the custom data).
    pub fn build(&self, tool: &str, given: &[&str]) -> Result<pluto_lib::StackData, String> {
        use pluto_lib::Nbt;
        let id = ResourceId::try_parse_in("tconstruct", tool).ok_or("bad tool id")?;
        let definition = self.definitions.get(&id).ok_or_else(|| format!("no tool {id}"))?;
        // one material for each part in order; the last one given covers the parts after it
        let mut materials = Vec::new();
        for name in given {
            let material = MaterialId::try_parse(&format!("tconstruct:{}", name.trim_start_matches("tconstruct:"))).ok_or("bad material id")?;
            if self.materials.materials.material(&material).is_none() {
                return Err(format!("no material {material}"));
            }
            materials.push(material);
        }
        let last = materials.last().cloned().ok_or("give at least one material")?;
        materials.resize(definition.parts().len().max(1), last);
        materials.truncate(definition.parts().len());
        self.make(&id, materials, &[])
    }

    /// The stack of the tool `id` made of the given materials, one for each part in order.
    pub fn make(&self, id: &ResourceId, materials: Vec<MaterialId>, upgrades: &[tconstruct_library::modifiers::ModifierEntry]) -> Result<pluto_lib::StackData, String> {
        self.make_with(id, materials, upgrades, &BTreeMap::new())
    }

    /// Like [`make`](Self::make), with `extra` slots added to the tool's own (what `/tconstruct slots` gives; kept on the stack as
    /// `tconstruct:extra_slots` so a rebuild keeps them).
    pub fn make_with(&self, id: &ResourceId, materials: Vec<MaterialId>, upgrades: &[tconstruct_library::modifiers::ModifierEntry], extra: &BTreeMap<String, i32>) -> Result<pluto_lib::StackData, String> {
        use pluto_lib::Nbt;
        let definition = self.definitions.get(id).ok_or_else(|| format!("no tool {id}"))?;
        let rebuilt = rebuild_stats(definition, &materials, upgrades, &self.materials, &self.modifiers, &self.stats, &ToolItem { library: self, item: id.clone() });
        let mut stats = pluto_lib::NbtCompound::new();
        for (stat, value) in &rebuilt.stats {
            stats.insert(stat.0.to_string(), match value {
                StatValue::Float(v) => Nbt::Float(*v),
                StatValue::Tier(t) => Nbt::String(t.to_string()),
            });
        }
        let durability = match rebuilt.stats.get(&tconstruct_library::tools::stat::ToolStatId::tconstruct("durability")) {
            Some(StatValue::Float(v)) => Some(*v as i32),
            _ => None,
        };
        let mut data = pluto_lib::NbtCompound::new();
        // the form the renderer reads the materials of a tool in (`tic_materials`, as the original keeps them)
        data.insert("tic_materials".to_owned(), Nbt::List(materials.iter().map(|m| Nbt::String(m.to_string())).collect()));
        data.insert("tconstruct:materials".to_owned(), Nbt::List(materials.iter().map(|m| Nbt::String(m.to_string())).collect()));
        data.insert("tconstruct:modifiers".to_owned(), Nbt::List(rebuilt.modifiers.iter().map(|m| Nbt::String(format!("{}#{}", m.modifier, m.level))).collect()));
        data.insert("tconstruct:stats".to_owned(), Nbt::Compound(stats));
        // the upgrades the player added (not the traits), to rebuild from; the slots left are those of the tool less what they cost
        data.insert("tconstruct:upgrades".to_owned(), Nbt::List(upgrades.iter().map(|m| Nbt::String(format!("{}#{}", m.modifier, m.level))).collect()));
        let mut free = pluto_lib::NbtCompound::new();
        for (slot, count) in &rebuilt.slots {
            let used: i32 = upgrades.iter().filter_map(|u| Some((self.salvage.get(&u.modifier)?, u.level))).map(|((slots, _), level)| slots.get(slot).copied().unwrap_or(0) * level).sum();
            free.insert(slot.clone(), Nbt::Int(count - used));
        }
        for (slot, count) in extra {
            let now = match free.get(slot) {
                Some(Nbt::Int(v)) => *v,
                _ => 0,
            };
            free.insert(slot.clone(), Nbt::Int(now + count));
        }
        if !extra.is_empty() {
            data.insert("tconstruct:extra_slots".to_owned(), Nbt::Compound(extra.iter().map(|(k, v)| (k.clone(), Nbt::Int(*v))).collect()));
        }
        data.insert("tconstruct:slots".to_owned(), Nbt::Compound(free));
        Ok(pluto_lib::StackData { item: tconstruct_library::tools::definition::item_of_tool(id), count: 1, damage: 0, max_damage: durability, custom_data: data })
    }

    /// The tool definitions whose parts can all be taken from `available` (part item and its material, one entry for each stack), each
    /// with the stacks to take from, tried in the order of the tools' ids; the first is what a station would make.
    pub fn tool_for_parts(&self, available: &[(ResourceId, MaterialId)]) -> Option<(ResourceId, Vec<usize>)> {
        let mut ids: Vec<&ResourceId> = self.definitions.keys().collect();
        ids.sort();
        'tools: for id in ids {
            let parts = self.definitions[id].parts();
            if parts.is_empty() {
                continue;
            }
            let mut used = Vec::new();
            for part in &parts {
                match available.iter().enumerate().find(|(i, (item, _))| *item == part.item && !used.contains(i)) {
                    Some((i, _)) => used.push(i),
                    None => continue 'tools,
                }
            }
            return Some((id.clone(), used));
        }
        None
    }

    /// A tool of `definition` (a tool item such as `tconstruct:pickaxe`) with every part made of `material`: the stats and the modifiers it
    /// would have, as one line of text. Err with the reason when the tool or the material is unknown.
    pub fn preview(&self, tool: &str, material: &str) -> Result<String, String> {
        let id = ResourceId::try_parse_in("tconstruct", tool).ok_or("bad tool id")?;
        let definition = self.definitions.get(&id).ok_or_else(|| format!("no tool {id}"))?;
        let material = MaterialId::try_parse(&format!("tconstruct:{}", material.trim_start_matches("tconstruct:"))).ok_or("bad material id")?;
        if self.materials.materials.material(&material).is_none() {
            return Err(format!("no material {material}"));
        }
        let materials = vec![material; definition.parts().len()];
        let rebuilt = rebuild_stats(definition, &materials, &[], &self.materials, &self.modifiers, &self.stats, &ToolItem { library: self, item: id.clone() });
        let stats: Vec<String> = rebuilt
            .stats
            .iter()
            .map(|(stat, value)| match value {
                StatValue::Float(v) => format!("{}={v}", stat.0.path()),
                StatValue::Tier(t) => format!("{}={t}", stat.0.path()),
            })
            .collect();
        let modifiers: Vec<String> = rebuilt.modifiers.iter().map(|m| format!("{}x{}", m.modifier.resource().path(), m.level)).collect();
        Ok(format!("{id}: {}; modifiers {}; slots {:?}", stats.join(", "), modifiers.join(", "), rebuilt.slots))
    }
}

/// The library, loaded once.
pub fn library() -> &'static ToolLibrary {
    static LIBRARY: OnceLock<ToolLibrary> = OnceLock::new();
    LIBRARY.get_or_init(ToolLibrary::load)
}

/// Called once at start: loads the library and says how much it holds, so a broken link or a broken data file shows in the log.
pub fn register() {
    register_break_speed();
    register_tooltip();
    register_attack_damage();
    let library = library();
    println!(
        "tconstruct: library linked, {} tool stats, {} material stat types",
        library.stats.all().count(),
        library.materials.stats.stat_type_ids().count()
    );
    println!(
        "tconstruct: library loaded, {} materials, {} modifiers, {} tool definitions, {} problems",
        library.materials.materials.all_materials().count(),
        library.modifiers.ids().len(),
        library.definitions.len(),
        library.problems.len()
    );
    for problem in library.problems.iter().take(10) {
        println!("tconstruct: library problem: {problem}");
    }
}

/// `/tconstruct preview <tool> <material>`: what a tool made of one material would be.
pub fn preview_command(ctx: &pluto_lib::CommandContext) -> i32 {
    let (Some(tool), Some(material)) = (ctx.args.first(), ctx.args.get(1)) else { return 0 };
    match library().preview(tool, material) {
        Ok(text) => {
            println!("tconstruct: preview {tool} {material}: {text}");
            ctx.source.send_message(&text);
            1
        }
        Err(reason) => {
            ctx.source.send_message(&reason);
            0
        }
    }
}

/// `/tconstruct build <tool> <material> [more materials]`: gives a tool with its stats on the stack; one material for each part in order,
/// the last one given covering the parts after it.
pub fn build_command(ctx: &pluto_lib::CommandContext) -> i32 {
    let Some(tool) = ctx.args.first() else { return 0 };
    let materials: Vec<&str> = ctx.args[1..].iter().map(String::as_str).collect();
    let material = materials.join(" ");
    match library().build(tool, &materials) {
        Ok(stack) => {
            let (item, max) = (stack.item.clone(), stack.max_damage);
            if ctx.source.give_stack(stack).is_some() {
                ctx.source.send_message("no room for the tool");
                return 0;
            }
            println!("tconstruct: built {item} of {material} with max damage {max:?}");
            1
        }
        Err(reason) => {
            ctx.source.send_message(&reason);
            0
        }
    }
}

/// A tool digs with the mining speed of its own stats (`tconstruct:stats` in the stack's data) the blocks its definition calls effective
/// (`is_effective`) when its harvest tier can harvest them (the block is not in `incorrect_for_<tier>_tool`); a block it is effective on but
/// cannot harvest digs at hand speed. Other blocks keep the game's speed. Haste and fatigue are not combined yet.
pub fn register_break_speed() {
    use pluto_lib::{BreakSpeedCallback, Nbt};
    use std::collections::HashSet;
    use std::sync::Mutex;
    static SAID: Mutex<Option<HashSet<&'static str>>> = Mutex::new(None);
    let say = |case: &'static str, text: String| {
        if let Ok(mut said) = SAID.lock() {
            if said.get_or_insert_with(HashSet::new).insert(case) {
                println!("tconstruct: break speed {case}: {text}");
            }
        }
    };
    BreakSpeedCallback::EVENT.register(move |dig| {
        let Some(held) = &dig.held else { return };
        let Some(definition) = ResourceId::try_parse(&held.item).and_then(|id| library().definitions.get(&tconstruct_library::tools::definition::tool_of_item(&id))) else { return };
        let Some(Nbt::Compound(stats)) = held.custom_data.get("tconstruct:stats") else { return };
        let Some(Nbt::Float(mining)) = stats.get("tconstruct:mining_speed") else { return };
        if !definition.is_effective(&|tag| dig.block.is_in_tag(tag)) {
            return;
        }
        let tier = match stats.get("tconstruct:harvest_tier") {
            Some(Nbt::String(tier)) => tier.clone(),
            _ => "minecraft:wood".to_owned(),
        };
        let name = tier.strip_prefix("minecraft:").unwrap_or(&tier);
        let tag = format!("minecraft:incorrect_for_{}_tool", if name == "wood" { "wooden" } else { name });
        if dig.block.is_in_tag(&tag) {
            dig.speed.set(1.0);
            say("tier too low", format!("{tier} on {}", dig.block.block));
        } else {
            dig.speed.set(*mining);
            say("effective", format!("original={} now={mining} on {}", dig.original, dig.block.block));
        }
    });
}

/// A player's attack with a tool hits for the attack damage of the tool's stats plus the 1 a hand gives. The game's cooldown scaling,
/// critical hits and enchantments are not combined yet (the game's amount is replaced).
pub fn register_attack_damage() {
    use pluto_lib::{Nbt, ServerLivingEntityEvents};
    use std::sync::atomic::{AtomicBool, Ordering};
    static SAID: AtomicBool = AtomicBool::new(false);
    ServerLivingEntityEvents::MODIFY_DAMAGE.register(|hit| {
        let Some(stack) = &hit.attacker_stack else { return };
        if ResourceId::try_parse(&stack.item).and_then(|id| library().definitions.get(&tconstruct_library::tools::definition::tool_of_item(&id))).is_none() {
            return;
        }
        let Some(Nbt::Compound(stats)) = stack.custom_data.get("tconstruct:stats") else { return };
        let Some(Nbt::Float(attack)) = stats.get("tconstruct:attack_damage") else { return };
        let amount = attack + 1.0;
        hit.amount.set(amount);
        if !SAID.swap(true, Ordering::Relaxed) {
            println!("tconstruct: attack damage original={} now={amount} with {}", hit.damage.amount, stack.item);
        }
    });
}

/// `/tconstruct part <part> <material>`: gives a tool part made of a material, which carries the material as the original's parts do
/// (`Material` in the stack's data).
pub fn part_command(ctx: &pluto_lib::CommandContext) -> i32 {
    use pluto_lib::Nbt;
    let (Some(part), Some(material)) = (ctx.args.first(), ctx.args.get(1)) else { return 0 };
    let Some(item) = ResourceId::try_parse_in("tconstruct", part) else { return 0 };
    let Some(material) = MaterialId::try_parse(&format!("tconstruct:{}", material.trim_start_matches("tconstruct:"))) else { return 0 };
    if library().materials.materials.material(&material).is_none() || tconstruct_library::tools::definition::part_stat_type(&item).is_none() {
        ctx.source.send_message("no such part or material");
        return 0;
    }
    let stack = pluto_lib::StackData {
        item: item.to_string(),
        count: 1,
        damage: 0,
        max_damage: None,
        custom_data: [("Material".to_owned(), Nbt::String(material.to_string()))].into(),
    };
    if ctx.source.give_stack(stack).is_some() {
        ctx.source.send_message("no room for the part");
        return 0;
    }
    println!("tconstruct: part {item} of {material}");
    1
}

/// `/tconstruct craft`: makes the first tool whose parts are in the inventory (parts that carry a material), using one of each.
pub fn craft_command(ctx: &pluto_lib::CommandContext) -> i32 {
    use pluto_lib::Nbt;
    let library = library();
    let mut found = Vec::new(); // (slot, stack, part item, material)
    for slot in 0..36 {
        let Some(stack) = ctx.source.stack(slot) else { continue };
        let Some(Nbt::String(material)) = stack.custom_data.get("Material") else { continue };
        let (Some(item), Some(material)) = (ResourceId::try_parse(&stack.item), MaterialId::try_parse(material)) else { continue };
        if tconstruct_library::tools::definition::part_stat_type(&item).is_none() {
            continue;
        }
        found.push((slot, stack.clone(), item, material));
    }
    let available: Vec<(ResourceId, MaterialId)> = found.iter().map(|(_, _, item, material)| (item.clone(), material.clone())).collect();
    let Some((tool, used)) = library.tool_for_parts(&available) else {
        ctx.source.send_message("no tool can be made of the parts you carry");
        return 0;
    };
    let materials: Vec<MaterialId> = used.iter().map(|i| found[*i].3.clone()).collect();
    let Ok(made) = library.make(&tool, materials, &[]) else { return 0 };
    let item = made.item.clone();
    for i in &used {
        let (slot, stack, _, _) = &found[*i];
        let mut left = stack.clone();
        left.count -= 1;
        ctx.source.update_stack(*slot, &left);
    }
    if ctx.source.give_stack(made).is_some() {
        ctx.source.send_message("no room for the tool");
    }
    println!("tconstruct: crafted {item} from {} parts", used.len());
    1
}

impl ToolLibrary {
    /// Whether `item` is in the `forge:` item tag `tag` (false for a tag the library does not carry).
    pub fn in_forge_tag(&self, item: &str, tag: &str) -> bool {
        let (Some(tag), Some(item)) = (ResourceId::try_parse(tag), ResourceId::try_parse(item)) else { return false };
        self.forge_tags.get(&tag).is_some_and(|members| members.contains(&item))
    }
}

impl ToolLibrary {
    /// The tool stack with one more level of `modifier`, if the tool has the slots and the modifier goes on it. Err with the reason.
    pub fn upgrade(&self, tool: &pluto_lib::StackData, modifier: &str) -> Result<pluto_lib::StackData, String> {
        self.upgrade_paying(tool, modifier, &mut |_| Ok(()))
    }

    /// Like [`upgrade`](Self::upgrade), and the items of the recipe for the new level are paid through `pay` once the slots and the tool fit:
    /// `pay` gets the inputs of one recipe, returns Ok when it took them, Err with the reason when it could not. Each recipe that can give the
    /// level is tried in turn; a modifier without a carried recipe costs no items.
    pub fn upgrade_paying(&self, tool: &pluto_lib::StackData, modifier: &str, pay: &mut dyn FnMut(&[(Ingredient, u32)]) -> Result<(), String>) -> Result<pluto_lib::StackData, String> {
        use pluto_lib::Nbt;
        use tconstruct_library::modifiers::{ModifierEntry, ModifierId};
        let id = ModifierId::try_parse(&format!("tconstruct:{}", modifier.trim_start_matches("tconstruct:"))).ok_or("bad modifier id")?;
        let (cost, tag) = self.salvage.get(&id).ok_or_else(|| format!("{id} is not a modifier a recipe adds"))?;
        let definition_id = tconstruct_library::tools::definition::tool_of_item(&ResourceId::try_parse(&tool.item).ok_or("not a tool")?);
        let item = ToolItem { library: self, item: definition_id.clone() };
        if !item.item_in_tag(tag) {
            return Err(format!("{id} does not go on this tool"));
        }
        let mut data = ToolData::read(tool)?;
        let mut upgrades = data.upgrades.clone();
        let Some(Nbt::Compound(free)) = tool.custom_data.get("tconstruct:slots") else { return Err("this tool has no slot data".to_owned()) };
        for (slot, count) in cost {
            let left = match free.get(slot) {
                Some(Nbt::Int(v)) => *v,
                _ => 0,
            };
            if left < *count {
                return Err(format!("not enough {slot} slots: {left} left, {count} needed"));
            }
        }
        let new_level = upgrades.iter().find(|u| u.modifier == id).map_or(1, |u| u.level + 1);
        if let Some(candidates) = self.recipes.get(&id) {
            let fitting: Vec<&ModifierRecipe> = candidates.iter().filter(|r| r.min_level <= new_level && new_level <= r.max_level).collect();
            if fitting.is_empty() {
                return Err(format!("no recipe gives {id} level {new_level}"));
            }
            let mut first_error = None;
            let mut paid = false;
            for recipe in fitting {
                match pay(&recipe.inputs) {
                    Ok(()) => {
                        paid = true;
                        break;
                    }
                    Err(error) => {
                        first_error.get_or_insert(error);
                    }
                }
            }
            if !paid {
                return Err(first_error.unwrap_or_default());
            }
        }
        match upgrades.iter_mut().find(|u| u.modifier == id) {
            Some(existing) => existing.level += 1,
            None => upgrades.push(ModifierEntry { modifier: id, level: 1 }),
        }
        data.upgrades = upgrades;
        data.rebuild(self)
    }
}

/// `/tconstruct upgrade <modifier>`: adds a level of a modifier to the first tool in the inventory that has the slots for it.
pub fn upgrade_command(ctx: &pluto_lib::CommandContext) -> i32 {
    let Some(modifier) = ctx.args.first() else { return 0 };
    let mut reason = String::from("you carry no tool made by /tconstruct build or craft");
    for slot in 0..36 {
        let Some(tool) = ctx.source.stack(slot) else { continue };
        if !tool.custom_data.contains_key("tconstruct:stats") {
            continue;
        }
        let player = &ctx.source;
        let mut pay = |inputs: &[(Ingredient, u32)]| -> Result<(), String> {
            let have = |what: &Ingredient| match what {
                Ingredient::Item(item) => player.count_item(item),
                Ingredient::Tag(tag) => player.count_in_tag(tag.trim_start_matches('#')),
            };
            for (what, need) in inputs {
                if have(what) < *need {
                    return Err(format!("needs {need} of {what:?}, you carry {}", have(what)));
                }
            }
            for (what, need) in inputs {
                match what {
                    Ingredient::Item(item) => player.take_item(item, *need),
                    Ingredient::Tag(tag) => player.take_in_tag(tag.trim_start_matches('#'), *need),
                };
            }
            Ok(())
        };
        match library().upgrade_paying(&tool, modifier, &mut pay) {
            Ok(upgraded) => {
                if ctx.source.update_stack(slot, &upgraded) {
                    println!("tconstruct: upgraded {} with {modifier} in slot {slot}: {}", upgraded.item, tooltip_lines(&upgraded).join(" | "));
                    return 1;
                }
            }
            Err(error) => reason = error,
        }
    }
    ctx.source.send_message(&reason);
    println!("tconstruct: upgrade refused: {reason}");
    0
}

/// The lines that describe a tool made by the library: its durability, its stats, its modifiers and the slots it has left, read from the
/// stack's data. Empty for a stack that is not such a tool.
pub fn tooltip_lines(stack: &pluto_lib::StackData) -> Vec<String> {
    use pluto_lib::Nbt;
    let data = &stack.custom_data;
    let Some(Nbt::Compound(stats)) = data.get("tconstruct:stats") else { return Vec::new() };
    let mut lines = Vec::new();
    if let Some(max) = stack.max_damage {
        lines.push(format!("Durability: {} / {max}", max - stack.damage as i32));
    }
    for (name, value) in stats {
        let Some(name) = name.strip_prefix("tconstruct:") else { continue };
        if name == "durability" {
            continue;
        }
        match value {
            Nbt::Float(v) => lines.push(format!("{}: {}", name.replace('_', " "), (v * 100.0).round() / 100.0)),
            Nbt::String(t) => lines.push(format!("{}: {t}", name.replace('_', " "))),
            _ => {}
        }
    }
    if let Some(Nbt::List(modifiers)) = data.get("tconstruct:modifiers") {
        let names: Vec<String> = modifiers.iter().filter_map(|m| if let Nbt::String(s) = m { Some(s.replace("tconstruct:", "").replace('#', " x")) } else { None }).collect();
        if !names.is_empty() {
            lines.push(format!("Modifiers: {}", names.join(", ")));
        }
    }
    if let Some(Nbt::Compound(free)) = data.get("tconstruct:slots") {
        let slots: Vec<String> = free.iter().filter_map(|(k, v)| if let Nbt::Int(n) = v { Some(format!("{k} {n}")) } else { None }).collect();
        lines.push(format!("Slots left: {}", slots.join(", ")));
    }
    lines
}

/// The tooltip of a tool made by the library.
pub fn register_tooltip() {
    use pluto_lib::{ItemTooltipCallback, TooltipLine};
    ItemTooltipCallback::EVENT.register(|tip| {
        let mut added = tip.added.borrow_mut();
        for line in tooltip_lines(&tip.stack) {
            added.push(TooltipLine::grey(line));
        }
    });
}

impl ToolLibrary {
    /// The tool stack with `change` levels of `modifier` added (positive) or taken off (negative; `i32::MIN` takes all of it), as
    /// `/tconstruct modifiers` does: no slot cost and no recipe, but the tool must still fit its slots. Err with the reason.
    pub fn change_modifier(&self, tool: &pluto_lib::StackData, modifier: &str, change: i32) -> Result<pluto_lib::StackData, String> {
        use pluto_lib::Nbt;
        use tconstruct_library::modifiers::{ModifierEntry, ModifierId};
        let id = ModifierId::try_parse(&format!("tconstruct:{}", modifier.trim_start_matches("tconstruct:"))).ok_or("bad modifier id")?;
        if self.modifiers.get(&id).is_none() {
            return Err(format!("no modifier {id}"));
        }
        let definition_id = tconstruct_library::tools::definition::tool_of_item(&ResourceId::try_parse(&tool.item).ok_or("not a tool")?);
        let mut data = ToolData::read(tool)?;
        let current = data.level_of(&id);
        if change < 0 && current == 0 {
            return Err(format!("the tool has no {id} to take off"));
        }
        data.set_level(&id, if change == i32::MIN { 0 } else { (current + change).max(0) });
        data.rebuild(self)
    }
}

/// `/tconstruct modifiers add|remove <modifier> [level]`: changes the modifiers of the first tool in the inventory (the original changes
/// the tool in the hand of each target).
pub fn modifiers_command(ctx: &pluto_lib::CommandContext, adding: bool) -> i32 {
    let Some(modifier) = ctx.args.first() else { return 0 };
    let level: Option<i32> = ctx.args.get(1).and_then(|l| l.parse().ok());
    let change = match (adding, level) {
        (true, level) => level.unwrap_or(1).max(1),
        (false, Some(level)) => -level.max(1),
        (false, None) => i32::MIN,
    };
    let mut reason = String::from("you carry no tool made by /tconstruct build or craft");
    for slot in 0..36 {
        let Some(tool) = ctx.source.stack(slot) else { continue };
        if !tool.custom_data.contains_key("tconstruct:stats") {
            continue;
        }
        match library().change_modifier(&tool, modifier, change) {
            Ok(changed) => {
                if ctx.source.update_stack(slot, &changed) {
                    println!("tconstruct: modifiers {} {modifier}: {}", if adding { "add" } else { "remove" }, tooltip_lines(&changed).join(" | "));
                    return 1;
                }
            }
            Err(error) => reason = error,
        }
    }
    ctx.source.send_message(&reason);
    println!("tconstruct: modifiers refused: {reason}");
    0
}

/// `/tconstruct durability add|set <amount>`: adds durability to the first tool in the inventory (a negative amount wears it), or sets the
/// durability it has left, as the original's `DurabilityCommand` does on the held tool.
pub fn durability_command(ctx: &pluto_lib::CommandContext, adding: bool) -> i32 {
    let Some(amount) = ctx.args.first().and_then(|a| a.parse::<i64>().ok()) else { return 0 };
    if adding && amount == 0 {
        ctx.source.send_message("the amount to add must not be 0");
        return 0;
    }
    if !adding && amount < 0 {
        ctx.source.send_message("the durability to set must not be negative");
        return 0;
    }
    for slot in 0..36 {
        let Some(mut tool) = ctx.source.stack(slot) else { continue };
        let Some(max) = tool.max_damage.filter(|_| tool.custom_data.contains_key("tconstruct:stats")) else { continue };
        // damage counts up from 0, so adding durability lowers it; the original clamps it at 0
        let damage = if adding { i64::from(tool.damage) - amount } else { i64::from(max) - amount };
        tool.damage = damage.clamp(0, i64::from(max)) as u32;
        if ctx.source.update_stack(slot, &tool) {
            println!("tconstruct: durability {}: {} / {max} left", if adding { "add" } else { "set" }, i64::from(max) - i64::from(tool.damage));
            return 1;
        }
    }
    ctx.source.send_message("you carry no tool made by /tconstruct build or craft");
    0
}

/// `/tconstruct slots add|set <slot_type> [count]`: gives the first tool in the inventory more slots of a type (`add`), or makes it have
/// exactly `count` free (`set`), as the original's `SlotsCommand` does on the held tool.
pub fn slots_command(ctx: &pluto_lib::CommandContext, adding: bool) -> i32 {
    use pluto_lib::Nbt;
    let Some(slot_type) = ctx.args.first().filter(|t| ["upgrades", "abilities", "defense"].contains(&t.as_str())) else {
        ctx.source.send_message("the slot type must be upgrades, abilities or defense");
        return 0;
    };
    let count: i32 = ctx.args.get(1).and_then(|c| c.parse().ok()).unwrap_or(1);
    if adding && count == 0 {
        ctx.source.send_message("the number of slots to add must not be 0");
        return 0;
    }
    if !adding && count < 0 {
        ctx.source.send_message("the number of slots to set must not be negative");
        return 0;
    }
    let mut reason = String::from("you carry no tool made by /tconstruct build or craft");
    for slot in 0..36 {
        let Some(tool) = ctx.source.stack(slot) else { continue };
        let Some(Nbt::Compound(free)) = tool.custom_data.get("tconstruct:slots") else { continue };
        let left = match free.get(slot_type.as_str()) {
            Some(Nbt::Int(v)) => *v,
            _ => 0,
        };
        let rebuilt = ToolData::read(&tool).and_then(|mut data| {
            *data.extra_slots.entry(slot_type.clone()).or_insert(0) += if adding { count } else { count - left };
            data.rebuild(library())
        });
        match rebuilt {
            Ok(made) => {
                if ctx.source.update_stack(slot, &made) {
                    println!("tconstruct: slots {} {slot_type}: {}", if adding { "add" } else { "set" }, tooltip_lines(&made).join(" | "));
                    return 1;
                }
            }
            Err(error) => reason = error,
        }
    }
    ctx.source.send_message(&reason);
    println!("tconstruct: slots refused: {reason}");
    0
}

impl ToolLibrary {
    /// The tool with the material of part `index` replaced, rebuilt with its upgrades and extra slots. Err with the reason.
    pub fn set_material(&self, tool: &pluto_lib::StackData, index: usize, material: &str) -> Result<pluto_lib::StackData, String> {
        use pluto_lib::Nbt;
        use tconstruct_library::modifiers::{ModifierEntry, ModifierId};
        let material = MaterialId::try_parse(&format!("tconstruct:{}", material.trim_start_matches("tconstruct:"))).ok_or("bad material id")?;
        if self.materials.materials.material(&material).is_none() {
            return Err(format!("no material {material}"));
        }
        let mut data = ToolData::read(tool)?;
        if index >= data.materials.len() {
            return Err(format!("the tool has {} parts, there is no part {index}", data.materials.len()));
        }
        data.materials[index] = material;
        data.rebuild(self)
    }
}

/// `/tconstruct materials set <index> <material>`: replaces the material of one part of the first tool in the inventory, as the original's
/// `MaterialsCommand set` does on the held tool.
pub fn materials_set_command(ctx: &pluto_lib::CommandContext) -> i32 {
    let (Some(index), Some(material)) = (ctx.args.first().and_then(|i| i.parse::<usize>().ok()), ctx.args.get(1)) else { return 0 };
    let mut reason = String::from("you carry no tool made by /tconstruct build or craft");
    for slot in 0..36 {
        let Some(tool) = ctx.source.stack(slot) else { continue };
        if !tool.custom_data.contains_key("tconstruct:stats") {
            continue;
        }
        match library().set_material(&tool, index, material) {
            Ok(changed) => {
                if ctx.source.update_stack(slot, &changed) {
                    println!("tconstruct: materials set part {index} to {material}: {}", tooltip_lines(&changed).join(" | "));
                    return 1;
                }
            }
            Err(error) => reason = error,
        }
    }
    ctx.source.send_message(&reason);
    println!("tconstruct: materials refused: {reason}");
    0
}
