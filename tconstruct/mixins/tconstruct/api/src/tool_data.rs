//! What a tool made by the library keeps in its stack's data, read into one value and written back by rebuilding: the item, its materials, the
//! upgrades the player added, the extra slots `/tconstruct slots` gave, and how worn it is. Every change to a tool is a change to this value
//! followed by `rebuild`, so the stats, modifiers and free slots always follow from the data and never drift.
use std::collections::BTreeMap;

use pluto_lib::{Nbt, StackData};
use tconstruct_library::materials::MaterialId;
use tconstruct_library::modifiers::{ModifierEntry, ModifierId};
use tconstruct_library::resource::ResourceId;

use crate::tools::ToolLibrary;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolData {
    /// The tool item, such as `tconstruct:pickaxe`.
    pub item: String,
    /// The tool definition the item belongs to.
    pub definition: ResourceId,
    pub materials: Vec<MaterialId>,
    /// The modifiers the player added, with their levels (not the traits of the materials).
    pub upgrades: Vec<ModifierEntry>,
    /// Slots added (or taken away, when negative) by `/tconstruct slots`.
    pub extra_slots: BTreeMap<String, i32>,
    pub damage: u32,
}

impl ToolData {
    /// Reads a tool stack. Err with the reason when it is not a tool made by the library.
    pub fn read(tool: &StackData) -> Result<Self, String> {
        let Some(Nbt::List(materials)) = tool.custom_data.get("tconstruct:materials") else { return Err("this tool has no materials".to_owned()) };
        let materials = materials.iter().filter_map(|m| if let Nbt::String(s) = m { MaterialId::try_parse(s) } else { None }).collect();
        let upgrades = match tool.custom_data.get("tconstruct:upgrades") {
            Some(Nbt::List(list)) => list
                .iter()
                .filter_map(|u| {
                    let Nbt::String(s) = u else { return None };
                    let (name, level) = s.split_once('#')?;
                    Some(ModifierEntry { modifier: ModifierId::try_parse(name)?, level: level.parse().ok()? })
                })
                .collect(),
            _ => Vec::new(),
        };
        let extra_slots = match tool.custom_data.get("tconstruct:extra_slots") {
            Some(Nbt::Compound(map)) => map.iter().filter_map(|(k, v)| if let Nbt::Int(n) = v { Some((k.clone(), *n)) } else { None }).collect(),
            _ => BTreeMap::new(),
        };
        let definition = tconstruct_library::tools::definition::tool_of_item(&ResourceId::try_parse(&tool.item).ok_or("not a tool")?);
        Ok(Self { item: tool.item.clone(), definition, materials, upgrades, extra_slots, damage: tool.damage })
    }

    /// The level of a modifier among the upgrades.
    pub fn level_of(&self, modifier: &ModifierId) -> i32 {
        self.upgrades.iter().find(|u| &u.modifier == modifier).map_or(0, |u| u.level)
    }

    /// Sets the level of an upgrade; 0 takes it off.
    pub fn set_level(&mut self, modifier: &ModifierId, level: i32) {
        self.upgrades.retain(|u| &u.modifier != modifier);
        if level > 0 {
            self.upgrades.push(ModifierEntry { modifier: modifier.clone(), level });
        }
    }

    /// The tool as a stack again: stats, modifiers and free slots rebuilt from this data, the item kept and the damage kept (but not past the
    /// new durability). Err with the reason when the tool cannot be built or would have more modifiers than slots.
    pub fn rebuild(&self, library: &ToolLibrary) -> Result<StackData, String> {
        let mut made = library.make_with(&self.definition, self.materials.clone(), &self.upgrades, &self.extra_slots)?;
        if let Some(Nbt::Compound(free)) = made.custom_data.get("tconstruct:slots") {
            if let Some((slot, _)) = free.iter().find(|(_, v)| matches!(v, Nbt::Int(n) if *n < 0)) {
                return Err(format!("the tool would have too many {slot} modifiers"));
            }
        }
        made.item = self.item.clone();
        made.damage = made.max_damage.map_or(self.damage, |max| self.damage.min(max.max(0) as u32));
        Ok(made)
    }
}
