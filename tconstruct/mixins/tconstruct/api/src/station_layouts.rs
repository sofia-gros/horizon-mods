//! The tinker station slot layouts (`tinkering/station_layouts`, StationSlotLayoutLoader): per tool, where
//! the window's input slots sit and what they take; the three layouts named after a station block are that
//! block's main layout. SlimeKnights (MIT; see LICENSE) defines the data and the loader's rules.
//!
//! The window (station_menu) reads positions from here. Two of the original's behaviours do not carry
//! over and are recorded there and here: a layout's slot filters cannot be enforced (PlutoLib's slot rule
//! sees one stack at a time, with no layout context), and a slot a layout leaves out is hidden here,
//! where the original keeps it dormant (take-only). The tool slot is never hidden: a layout without a
//! tool slot of its own (every tool layout) borrows the station's main tool slot, so a tool can always
//! be put in and taken out.
use crate::{material_recipes::Ingredient, materials::Conditions};
use pluto_lib::{Identifier, ResourceManagerHelper, ResourceType, ServerLifecycleEvents};
use serde_json::Value;
use std::{collections::BTreeMap, sync::{Arc, RwLock}};

pub(crate) const FOLDER: &str = "tinkering/station_layouts";

/// One slot of a layout (`LayoutSlot`): its window position and the pattern icon it shows while empty.
/// A slot at (-1, -1) is hidden.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Slot {
    pub icon: Option<Identifier>,
    pub translation_key: String,
    pub x: i32,
    pub y: i32,
    /// What the slot takes; decoded for completeness, not enforced (see the module note).
    pub filter: Option<Ingredient>,
}

impl Slot {
    pub(crate) fn hidden(&self) -> bool { self.x == -1 && self.y == -1 }

    fn decode(value: &Value) -> Result<Self, String> {
        let fields = value.as_object().ok_or("layout slot must be an object")?;
        let icon = match fields.get("icon") {
            None => None,
            Some(Value::String(icon)) => Some(Identifier::parse(icon).map_err(|e| e.to_string())?),
            Some(_) => return Err("layout slot icon must be a pattern id string".into()),
        };
        let filter = match fields.get("filter") {
            None => None,
            Some(filter) => Some(Ingredient::decode(filter)?),
        };
        let coordinate = |key: &str| fields.get(key).and_then(Value::as_i64).ok_or_else(|| format!("layout slot {key} must be an integer"));
        Ok(Self {
            icon,
            translation_key: fields.get("translation_key").and_then(Value::as_str).unwrap_or_default().to_owned(),
            x: i32::try_from(coordinate("x")?).map_err(|_| format!("layout slot x out of range"))?,
            y: i32::try_from(coordinate("y")?).map_err(|_| format!("layout slot y out of range"))?,
            filter,
        })
    }
}

/// A station slot layout (`StationSlotLayout`): the tool slot (absent in the data of a tool layout,
/// which then borrows the main layout's) and the input slots in order.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Layout {
    pub name: Identifier,
    pub translation_key: String,
    /// The layout's own icon: an item (tool layouts) or a pattern (the main layouts' repair button).
    pub icon_item: Option<Identifier>,
    pub icon_pattern: Option<Identifier>,
    /// Absent for a station's main layout (`isMain`).
    pub sort_index: Option<i32>,
    pub tool_slot: Option<Slot>,
    pub input_slots: Vec<Slot>,
}

impl Layout {
    pub(crate) fn is_main(&self) -> bool { self.sort_index.is_none() }

    fn decode(name: &Identifier, value: &Value) -> Result<Self, String> {
        let fields = value.as_object().ok_or("station layout must be an object")?;
        let (mut icon_item, mut icon_pattern) = (None, None);
        if let Some(icon) = fields.get("icon") {
            let icon = icon.as_object().ok_or("layout icon must be an object")?;
            if let Some(item) = icon.get("item") {
                icon_item = Some(Identifier::parse(item.as_str().ok_or("layout icon item must be a string")?).map_err(|e| e.to_string())?);
            }
            if let Some(pattern) = icon.get("pattern") {
                icon_pattern = Some(Identifier::parse(pattern.as_str().ok_or("layout icon pattern must be a string")?).map_err(|e| e.to_string())?);
            }
        }
        let tool_slot = match fields.get("tool_slot") {
            None => None,
            Some(slot) => Some(Slot::decode(slot)?),
        };
        let input_slots = fields.get("input_slots").and_then(Value::as_array).ok_or("input_slots must be an array")?
            .iter().map(Slot::decode).collect::<Result<Vec<_>, _>>()?;
        // The loader's size rule: the visible slots (inputs plus a shown tool slot) must number at least two.
        let visible = input_slots.len() + usize::from(tool_slot.as_ref().is_some_and(|slot| !slot.hidden()));
        if visible < 2 { return Err(format!("too few slots for layout {name}, must have at least 2")); }
        Ok(Self {
            name: name.clone(),
            translation_key: fields.get("translation_key").and_then(Value::as_str).unwrap_or_default().to_owned(),
            icon_item,
            icon_pattern,
            sort_index: match fields.get("sortIndex") {
                None => None,
                Some(index) => Some(i32::try_from(index.as_i64().ok_or("sortIndex must be an integer")?).map_err(|_| "sortIndex out of range")?),
            },
            tool_slot,
            input_slots,
        })
    }

    /// The window position of every station slot under this layout: the tool slot first, then the
    /// inputs, `None` for a slot the layout leaves out. The tool slot falls back to `main`'s when this
    /// layout has none (see the module note); inputs beyond this layout's list stay hidden.
    pub(crate) fn positions(&self, main: &Layout, input_count: usize) -> Vec<Option<(i32, i32)>> {
        let tool = self.tool_slot.as_ref().filter(|slot| !slot.hidden()).or_else(|| main.tool_slot.as_ref().filter(|slot| !slot.hidden()));
        let mut positions = vec![tool.map(|slot| (slot.x, slot.y))];
        for index in 0..input_count {
            positions.push(self.input_slots.get(index).filter(|slot| !slot.hidden()).map(|slot| (slot.x, slot.y)));
        }
        positions
    }
}

/// The main layout of a station block, baked from the shipped data: the window needs positions before
/// the first reload lands (and a datapack that drops a required layout must not break the station,
/// which is why the original registers these three as required).
pub(crate) fn baked_main(block: &str) -> Layout {
    let slot = |icon: &str, x: i32, y: i32| Slot { icon: Some(Identifier::new("tconstruct", icon).expect("valid pattern")), translation_key: String::new(), x, y, filter: None };
    let (tool, inputs) = match block {
        "tinkers_anvil" | "scorched_anvil" => ((33, 41), vec![slot("dust", 11, 37), slot("lapis", 33, 19), slot("ingot", 55, 37), slot("gem", 51, 62), slot("quartz", 15, 62)]),
        _ => ((53, 41), vec![slot("quartz", 11, 41), slot("dust", 31, 30), slot("lapis", 31, 50)]),
    };
    Layout {
        name: Identifier::new("tconstruct", block).expect("valid block id"),
        translation_key: String::new(),
        icon_item: None,
        icon_pattern: Some(Identifier::new("tconstruct", "button_repair").expect("valid pattern")),
        sort_index: None,
        tool_slot: Some(Slot { icon: Some(Identifier::new("tconstruct", "pickaxe").expect("valid pattern")), translation_key: String::new(), x: tool.0, y: tool.1, filter: None }),
        input_slots: inputs,
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Layouts {
    pub layouts: BTreeMap<Identifier, Layout>,
    pub problems: Vec<String>,
    pub loaded: bool,
}

impl Layouts {
    pub fn load<C: Conditions>(files: &[(Identifier, Value)], conditions: &C) -> Self {
        let mut result = Self { loaded: true, ..Self::default() };
        let mut sorted: Vec<&(Identifier, Value)> = files.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for (id, value) in sorted {
            let outcome = (|| -> Result<Option<Layout>, String> {
                let fields = value.as_object().ok_or("station layout must be an object")?;
                // An empty object disables a layout at a lower datapack priority (the loader skips it silently).
                if fields.is_empty() { return Ok(None); }
                if let Some(list) = fields.get("conditions") {
                    let parsed = list.as_array().ok_or("conditions must be an array")?.iter().map(|c| conditions.decode(c)).collect::<Result<Vec<_>, _>>()?;
                    for condition in &parsed { if !conditions.test(condition)? { return Ok(None); } }
                }
                Layout::decode(id, value).map(Some)
            })();
            match outcome {
                Ok(Some(layout)) => { result.layouts.insert(layout.name.clone(), layout); }
                Ok(None) => {}
                Err(error) => result.problems.push(format!("{id}: {error}")),
            }
        }
        result
    }

    pub fn get(&self, name: &Identifier) -> Option<&Layout> { self.layouts.get(name) }

    /// The main layout for a station block: the loaded one when there is one, else the baked layout.
    pub fn main_for(&self, block: &str) -> Layout {
        let id = Identifier::new("tconstruct", block).expect("valid block id");
        self.layouts.get(&id).filter(|layout| layout.is_main()).cloned().unwrap_or_else(|| baked_main(block))
    }

    /// The tab layouts in the loader's order: by sort index, then name (the loader sorts by index
    /// alone; the name settles ties deterministically here).
    pub fn tabs(&self) -> Vec<&Layout> {
        let mut tabs: Vec<&Layout> = self.layouts.values().filter(|layout| !layout.is_main()).collect();
        tabs.sort_by(|a, b| (a.sort_index.unwrap_or(255), a.name.to_string()).cmp(&(b.sort_index.unwrap_or(255), b.name.to_string())));
        tabs
    }
}

/// Reload listener, shaped like the other data listeners.
pub(crate) fn register<C: Conditions + 'static>(context: impl Fn() -> Result<C, String> + Send + Sync + 'static) -> Result<Arc<RwLock<Layouts>>, String> {
    let state = Arc::new(RwLock::new(Layouts::default()));
    let output = state.clone();
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "station_layouts").unwrap(), FOLDER, move |files| {
            let context = context();
            let mut state = output.write().map_err(|_| "station layouts poisoned")?;
            match context {
                Ok(context) => {
                    *state = Layouts::load(files, &context);
                    for problem in &state.problems { eprintln!("Tinkers: {problem}"); }
                    Ok(())
                }
                Err(error) => { *state = Layouts { problems: vec![error.clone()], ..Layouts::default() }; Err(error) }
            }
        })?;
    let stopped = state.clone();
    ServerLifecycleEvents::SERVER_STOPPED.register(move |_| *stopped.write().expect("station layouts poisoned") = Layouts::default());
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct Always;
    impl Conditions for Always {
        type Parsed = ();
        fn decode(&self, _: &Value) -> Result<(), String> { Ok(()) }
        fn test(&self, _: &()) -> Result<bool, String> { Ok(true) }
    }

    fn load(files: Vec<(Identifier, Value)>) -> Layouts { Layouts::load(&files, &Always) }

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }

    #[test]
    fn layouts_decode_and_the_main_layout_lends_its_tool_slot() {
        let layouts = load(vec![
            (id("tconstruct:tinker_station"), json!({"icon": {"pattern": "tconstruct:button_repair"},
                "tool_slot": {"filter": {"tag": "tconstruct:modifiable"}, "icon": "tconstruct:pickaxe", "x": 53, "y": 41},
                "input_slots": [{"icon": "tconstruct:quartz", "x": 11, "y": 41}, {"icon": "tconstruct:dust", "x": 31, "y": 30}, {"icon": "tconstruct:lapis", "x": 31, "y": 50}],
                "translation_key": "gui.tconstruct.tinker_station.repair_limited"})),
            (id("tconstruct:pickaxe"), json!({"icon": {"item": "tconstruct:pickaxe", "nbt": "{Damage:0}"},
                "input_slots": [
                    {"filter": {"item": "tconstruct:pick_head"}, "icon": "tconstruct:pick_head", "translation_key": "item.tconstruct.pick_head", "x": 53, "y": 22},
                    {"filter": {"item": "tconstruct:tool_handle"}, "icon": "tconstruct:tool_handle", "translation_key": "item.tconstruct.tool_handle", "x": 15, "y": 60}],
                "sortIndex": 6, "translation_key": "item.tconstruct.pickaxe"})),
        ]);
        assert!(layouts.problems.is_empty(), "{:?}", layouts.problems);
        let main = layouts.main_for("tinker_station");
        assert!(main.is_main());
        let pickaxe = layouts.get(&id("tconstruct:pickaxe")).unwrap();
        assert_eq!(pickaxe.icon_item, Some(id("tconstruct:pickaxe")));
        assert_eq!(pickaxe.sort_index, Some(6));
        // The pickaxe layout has no tool slot: the tool stays at the main layout's (53, 41); its two
        // inputs sit at their own positions and the station's remaining inputs are hidden.
        assert_eq!(pickaxe.positions(&main, 4), vec![Some((53, 41)), Some((53, 22)), Some((15, 60)), None, None]);
        assert_eq!(layouts.tabs().iter().map(|layout| layout.name.to_string()).collect::<Vec<_>>(), ["tconstruct:pickaxe"]);
    }

    #[test]
    fn empty_and_undersized_layouts_are_not_loaded() {
        let layouts = load(vec![
            (id("tconstruct:disabled"), json!({})),
            (id("tconstruct:tiny"), json!({"input_slots": [{"x": 1, "y": 2}]})),
        ]);
        assert!(layouts.layouts.is_empty());
        assert_eq!(layouts.problems.len(), 1, "the empty object is a silent disable, the one-slot layout a problem");
        assert!(layouts.problems[0].contains("too few slots"));
    }

    #[test]
    fn baked_mains_match_the_shipped_main_layouts() {
        let station = baked_main("tinker_station");
        assert_eq!(station.positions(&station, 4), vec![Some((53, 41)), Some((11, 41)), Some((31, 30)), Some((31, 50)), None]);
        let anvil = baked_main("tinkers_anvil");
        assert_eq!(anvil.positions(&anvil, 6), vec![Some((33, 41)), Some((11, 37)), Some((33, 19)), Some((55, 37)), Some((51, 62)), Some((15, 62)), None]);
        assert_eq!(baked_main("scorched_anvil").positions(&anvil, 6), anvil.positions(&anvil, 6));
    }
}
