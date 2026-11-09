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
    /// What loading found wrong, as the original would log it.
    pub problems: Vec<String>,
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
        Self { stats, materials, modifiers, definitions, item_tags, salvage, problems }
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
    /// The tool stack with one more level of `modifier`, if the tool has the slots and the modifier goes on it. Err with the reason.
    pub fn upgrade(&self, tool: &pluto_lib::StackData, modifier: &str) -> Result<pluto_lib::StackData, String> {
        use pluto_lib::Nbt;
        use tconstruct_library::modifiers::{ModifierEntry, ModifierId};
        let id = ModifierId::try_parse(&format!("tconstruct:{}", modifier.trim_start_matches("tconstruct:"))).ok_or("bad modifier id")?;
        let (cost, tag) = self.salvage.get(&id).ok_or_else(|| format!("{id} is not a modifier a recipe adds"))?;
        let definition_id = tconstruct_library::tools::definition::tool_of_item(&ResourceId::try_parse(&tool.item).ok_or("not a tool")?);
        let item = ToolItem { library: self, item: definition_id.clone() };
        if !item.item_in_tag(tag) {
            return Err(format!("{id} does not go on this tool"));
        }
        let Some(Nbt::List(materials)) = tool.custom_data.get("tconstruct:materials") else { return Err("this tool has no materials".to_owned()) };
        let materials: Vec<MaterialId> = materials.iter().filter_map(|m| if let Nbt::String(s) = m { MaterialId::try_parse(s) } else { None }).collect();
        let mut upgrades: Vec<ModifierEntry> = match tool.custom_data.get("tconstruct:upgrades") {
            Some(Nbt::List(list)) => list.iter().filter_map(|u| if let Nbt::String(s) = u { let (name, level) = s.split_once('#')?; Some(ModifierEntry { modifier: ModifierId::try_parse(name)?, level: level.parse().ok()? }) } else { None }).collect(),
            _ => Vec::new(),
        };
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
        match upgrades.iter_mut().find(|u| u.modifier == id) {
            Some(existing) => existing.level += 1,
            None => upgrades.push(ModifierEntry { modifier: id, level: 1 }),
        }
        let mut made = self.make(&definition_id, materials, &upgrades)?;
        made.item = tool.item.clone();
        made.damage = tool.damage;
        Ok(made)
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
        match library().upgrade(&tool, modifier) {
            Ok(upgraded) => {
                if ctx.source.update_stack(slot, &upgraded) {
                    println!("tconstruct: upgraded {} with {modifier} in slot {slot}, max damage {:?}, slots {:?}", upgraded.item, upgraded.max_damage, upgraded.custom_data.get("tconstruct:slots"));
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
