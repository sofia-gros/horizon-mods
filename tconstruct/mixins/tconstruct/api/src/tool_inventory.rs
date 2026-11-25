//! InventoryModule, InventoryMenuModule, InventorySlotMenuModule and ToolInventoryCapability, from SlimeKnights (MIT; see LICENSE): the item
//! slots a modifier adds to a tool, kept in the tool's persistent data. This file is the data model and the pure rules (slot counts, the
//! item filter, the stack limit, insert and extract, validation and removal, what a quick-move does); the menu that shows the slots is a
//! screen and not here. The layout is the original's: under the module's key (the modifier's ID when none is given) the persistent data
//! holds a list of `{id, Count (byte), tag, Slot (int)}` compounds, one per filled slot. A stack here also has parts the original kept in
//! its `tag` (damage, enchantments) or did not have (a per-stack tool or attribute component); those are stored beside `tag` under
//! `pluto_components`, so that a stack leaves and returns unchanged. A module with a filter this port cannot evaluate stays unbound.
use crate::{break_speed_hook::with_held_tool, common, modifier_entries::Entry, tool_stack};
use pluto_lib::{Identifier, ItemSlot, Nbt, NbtCompound, StackData};
use serde_json::{Map, Value};

/// ToolInventoryCapability.TOTAL_SLOTS: the volatile int holding the slot count over all modifiers.
pub(crate) const TOTAL_SLOTS: &str = "tconstruct:total_item_slots";
const TAG_SLOT: &str = "Slot";
const COMPONENTS: &str = "pluto_components";
/// TinkerTags.Items.TOOL_INVENTORY_BLACKLIST.
const BLACKLIST: &str = "tconstruct:inventory_blacklist";
/// TinkerTags.MenuTypes.TOOL_INVENTORY_REPLACEMENTS.
const REPLACEMENT_MENUS: &str = "tconstruct:tool_inventory_replacements";
/// The error when the items in a tool's inventory stop a modifier from going (InventoryModule.HAS_ITEMS).
pub(crate) const HAS_ITEMS: &str = "modifier.tconstruct.inventory_cannot_remove";

/// A LevelingInt: `flat + each_level * level`, both parts shorts.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Leveling { flat: i32, each: i32 }

impl Leveling {
    fn decode(fields: &Map<String, Value>, key: &str, default: Option<Self>) -> Result<Self, String> {
        match fields.get(key).filter(|value| !value.is_null()) {
            None => default.ok_or_else(|| format!("{key} missing")),
            Some(Value::Object(value)) => {
                let short = |name: &str| match value.get(name).filter(|v| !v.is_null()) {
                    None => Ok(0),
                    Some(Value::Number(number)) => {
                        let int = crate::materials::number_int(&number.to_string());
                        if (-32768..=32767).contains(&int) { Ok(int) } else { Err(format!("{key}.{name} must fit in a short")) }
                    }
                    Some(_) => Err(format!("{key}.{name} must be a number")),
                };
                Ok(Self { flat: short("flat")?, each: short("each_level")? })
            }
            Some(_) => Err(format!("{key} must be an object")),
        }
    }
    /// compute(float): `(int)(flat + each * level)`.
    fn compute(self, level: f32) -> i32 { (self.flat as f32 + self.each as f32 * level) as i32 }
    fn compute_for_level(self, level: f32) -> i32 { if level > 0.0 { self.compute(level) } else { 0 } }
}

/// The item predicates the shipped inventories use: any, a tag, a set, combinations of those and `tconstruct:map` (a filled map).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ItemFilter { Any, Tag(Identifier), Set(Vec<Identifier>), Map, Not(Box<ItemFilter>), And(Vec<ItemFilter>), Or(Vec<ItemFilter>) }

impl ItemFilter {
    /// `Ok(None)` for a predicate type this port cannot evaluate.
    fn decode(value: &Value) -> Result<Option<Self>, String> {
        match value {
            Value::String(kind) => Self::typed(&Map::new(), kind),
            Value::Object(object) => Self::typed(object, object.get("type").and_then(Value::as_str).ok_or("filter type missing")?),
            _ => Err("filter must be a string or an object".into()),
        }
    }
    fn typed(object: &Map<String, Value>, kind: &str) -> Result<Option<Self>, String> {
        let id = |value: &Value| Identifier::parse(value.as_str().ok_or("expected an identifier string")?).map_err(|e| e.to_string());
        let list = |key: &str| -> Result<Option<Vec<Self>>, String> {
            let values = object.get(key).and_then(Value::as_array).filter(|values| !values.is_empty()).ok_or_else(|| format!("{key} must be a nonempty array"))?;
            values.iter().map(Self::decode).collect::<Result<Vec<_>, _>>().map(|all| all.into_iter().collect())
        };
        Ok(Some(match kind {
            "mantle:any" => Self::Any,
            "tconstruct:map" => Self::Map,
            "mantle:tag" => Self::Tag(id(object.get("tag").ok_or("tag missing")?)?),
            "mantle:set" => Self::Set(object.get("items").and_then(Value::as_array).ok_or("items must be an array")?.iter().map(id).collect::<Result<_, _>>()?),
            "mantle:inverted" => {
                let inner = object.get("inverted_type").and_then(Value::as_str).ok_or("inverted_type missing")?;
                if inner == "mantle:inverted" { return Err("nested inversion is not shipped".into()); }
                let Some(inner) = Self::typed(object, inner)? else { return Ok(None); };
                Self::Not(Box::new(inner))
            }
            "mantle:and" => match list("predicates")? { Some(all) => Self::And(all), None => return Ok(None) },
            "mantle:or" => match list("predicates")? { Some(any) => Self::Or(any), None => return Ok(None) },
            _ => return Ok(None),
        }))
    }
    fn matches(&self, item: &Identifier, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> bool {
        match self {
            Self::Any => true,
            Self::Tag(tag) => in_tag(item, tag),
            Self::Set(items) => items.contains(item),
            Self::Map => item.to_string() == "minecraft:filled_map",
            Self::Not(inner) => !inner.matches(item, in_tag),
            Self::And(all) => all.iter().all(|filter| filter.matches(item, in_tag)),
            Self::Or(any) => any.iter().any(|filter| filter.matches(item, in_tag)),
        }
    }
}

/// What the rules ask about items, which a stack does not carry itself.
pub(crate) struct Items<'a> {
    /// The item's largest stack.
    pub max_stack: &'a dyn Fn(&Identifier) -> u32,
    /// Whether an item is in an item tag.
    pub in_tag: &'a dyn Fn(&Identifier, &Identifier) -> bool,
    /// Item.canFitInsideContainerItems.
    pub can_fit_inside_container: &'a dyn Fn(&Identifier) -> bool,
    /// Whether the stack has an item handler of its own (such as a tool with an inventory): no nesting.
    pub has_item_handler: &'a dyn Fn(&ItemSlot) -> bool,
}

impl Items<'_> {
    /// ToolInventoryCapability.isBlacklisted.
    pub fn is_blacklisted(&self, stack: &ItemSlot) -> bool {
        !(self.can_fit_inside_container)(&stack.item) || (self.in_tag)(&stack.item, &Identifier::parse(BLACKLIST).expect("valid tag")) || (self.has_item_handler)(stack)
    }
    fn stack_limit(&self, stack: &ItemSlot) -> i32 { (self.max_stack)(&stack.item) as i32 }
    /// ItemStack.isStackable: a damaged stack is not.
    fn stackable(&self, stack: &ItemSlot) -> bool { self.stack_limit(stack) > 1 && stack.damage == 0 }
}

/// InventoryModule: the slots one modifier adds.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Inventory { key: Option<Identifier>, slots: Leveling, limit: Leveling, filter: ItemFilter, pattern: Option<Identifier>, validation: (i32, i32), quiver: bool }

impl Inventory {
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        let filter = match fields.get("filter").filter(|value| !value.is_null()) {
            None => ItemFilter::Any,
            Some(value) => match ItemFilter::decode(value)? { Some(filter) => filter, None => return Ok(None) },
        };
        let optional = |key: &str| match fields.get(key).filter(|value| !value.is_null()) {
            None => Ok(None),
            Some(value) => Identifier::parse(value.as_str().ok_or_else(|| format!("{key} must be an ID string"))?).map(Some).map_err(|e| e.to_string()),
        };
        Ok(Some(Self {
            key: optional("key")?,
            slots: Leveling::decode(fields, "slots", None)?,
            limit: Leveling::decode(fields, "limit", Some(Leveling { flat: 64, each: 0 }))?,
            filter,
            pattern: optional("pattern")?,
            validation: crate::modifiers::range(fields.get("validation_level"))?,
            quiver: false,
        }))
    }

    /// QuiverInventoryModule: the inventory loader's fields but no filter — the slots take the quiver's
    /// ammunition instead (see Part::valid). The pattern still draws the slot's background.
    pub(crate) fn decode_quiver(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        Ok(Self::decode(fields)?.map(|mut module| { module.quiver = true; module }))
    }

    /// The persistent-data key: the module's, else the modifier's ID.
    pub(crate) fn key(&self, modifier: &Identifier) -> String { self.key.as_ref().unwrap_or(modifier).to_string() }
    /// The key the module JSON set, when it set one.
    pub(crate) fn named_key(&self) -> Option<&Identifier> { self.key.as_ref() }
    /// This inventory with another key.
    pub(crate) fn with_key(&self, key: Identifier) -> Self { let mut out = self.clone(); out.key = Some(key); out }
    /// SmeltingModule's output inventory over this (input) inventory: the same slots, limit and
    /// validation, no filter, and the given key and pattern. The key stays unset when the caller
    /// resolves the default (the modifier's ID with an `_output` suffix) at use.
    pub(crate) fn as_output(&self, key: Option<Identifier>, pattern: Option<Identifier>) -> Self {
        Self { key, slots: self.slots, limit: self.limit, filter: ItemFilter::Any, pattern, validation: self.validation, quiver: false }
    }
    fn potential_slots(&self, level: f32) -> i32 { self.slots.compute_for_level(level).max(0) }
    /// The slots to add to the volatile total when the module's condition holds (even when that is none).
    pub(crate) fn volatile_slots(&self, entry: &Entry) -> i32 { self.potential_slots(entry.effective_level()) }
}

/// A bound inventory module of one modifier on one tool, with its conditions already evaluated against that tool.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Part { module: Inventory, entry: Entry, active: bool, validates: bool, crossbow: bool, ballista: bool }

impl Part {
    /// `active`: the module's condition (modifier level range and tool predicate) holds. `tool_matches`: its tool predicate alone holds.
    pub(crate) fn new(module: Inventory, entry: Entry, active: bool, tool_matches: bool) -> Self {
        let validates = tool_matches && module.validation.0 <= entry.level && entry.level <= module.validation.1;
        Self { module, entry, active, validates, crossbow: false, ballista: false }
    }
    /// QuiverInventoryModule.isItemValid asks about the tool: whether it is a crossbow (fireworks count as
    /// ammunition) and whether it is a ballista (ModifiableBowItem.isBallista: the ballistas tag and the
    /// `tconstruct:ballista` volatile flag set). The caller notes both once the tool is known.
    pub(crate) fn note_tool(&mut self, crossbow: bool, ballista: bool) { self.crossbow = crossbow; self.ballista = ballista; }
    /// The ballista half of the note, which reads the tool's volatile data the part's facts do not carry.
    pub(crate) fn note_ballista(&mut self) { self.ballista = true; }
    pub(crate) fn is_quiver(&self) -> bool { self.module.quiver }
    fn key(&self) -> String { self.module.key(&self.entry.id) }
    fn slots(&self) -> usize { if self.active { self.module.potential_slots(self.entry.effective_level()) as usize } else { 0 } }
    fn limit(&self) -> i32 { self.module.limit.compute(self.entry.effective_level()) }
    fn valid(&self, item: &Identifier, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> bool {
        if !self.active { return false; }
        if !self.module.quiver { return self.module.filter.matches(item, in_tag); }
        in_tag(item, &Identifier::parse("minecraft:arrows").expect("valid tag"))
            || (self.crossbow && item.to_string() == "minecraft:firework_rocket")
            || (self.ballista && in_tag(item, &Identifier::parse("tconstruct:modifiable/ballista_ammo").expect("valid tag")))
    }
}

// ---- the stored stacks -------------------------------------------------------------------------------------------------------------

fn compound_list<'a>(persistent: &'a NbtCompound, key: &str) -> Option<Vec<&'a NbtCompound>> {
    let Some(Nbt::List(list)) = persistent.get(key) else { return None; };
    let found: Vec<&NbtCompound> = list.iter().filter_map(|tag| if let Nbt::Compound(compound) = tag { Some(compound) } else { None }).collect();
    // getList(key, TAG_COMPOUND) is empty for a list of anything else.
    Some(if found.len() == list.len() { found } else { Vec::new() })
}
fn slot_number(compound: &NbtCompound) -> i64 { compound.get(TAG_SLOT).and_then(Nbt::as_i64).unwrap_or(0) }

/// ItemStack.save plus the slot, for a stack in slot `slot`.
pub(crate) fn write_stack(stack: &ItemSlot, slot: usize) -> NbtCompound {
    let mut compound = NbtCompound::new();
    compound.insert("id".into(), Nbt::String(stack.item.to_string()));
    compound.insert("Count".into(), Nbt::Byte(stack.count as i8));
    if !stack.custom_data.is_empty() { compound.insert("tag".into(), Nbt::Compound(stack.custom_data.clone())); }
    let mut extra = NbtCompound::new();
    if stack.damage > 0 { extra.insert("damage".into(), Nbt::Int(stack.damage as i32)); }
    if let Some(max) = stack.max_damage { extra.insert("max_damage".into(), Nbt::Int(max)); }
    if let Some(tool) = &stack.tool { extra.insert("tool".into(), Nbt::Compound(tool.clone())); }
    if let Some(modifiers) = &stack.attribute_modifiers { extra.insert("attribute_modifiers".into(), Nbt::List(modifiers.iter().cloned().map(Nbt::Compound).collect())); }
    if let Some(enchantments) = &stack.enchantments {
        extra.insert("enchantments".into(), Nbt::Compound(enchantments.iter().map(|(name, level)| (name.clone(), Nbt::Int(*level))).collect()));
    }
    // A filled map's id: the map a minimap inventory stores keeps its explored data through it.
    if let Some(id) = stack.map_id { extra.insert("map_id".into(), Nbt::Int(id)); }
    if !extra.is_empty() { compound.insert(COMPONENTS.into(), Nbt::Compound(extra)); }
    compound.insert(TAG_SLOT.into(), Nbt::Int(slot as i32));
    compound
}

/// ItemStack.of: None for an empty stack (no valid item, air or a count below one).
pub(crate) fn read_stack(compound: &NbtCompound) -> Option<ItemSlot> {
    let Some(Nbt::String(name)) = compound.get("id") else { return None; };
    let item = Identifier::parse(name).ok().filter(|item| item.to_string() != "minecraft:air")?;
    let count = compound.get("Count").and_then(Nbt::as_i64).unwrap_or(0) as i8;
    if count <= 0 { return None; }
    let mut stack = ItemSlot::new(item, count as u32);
    if let Some(Nbt::Compound(tag)) = compound.get("tag") { stack.custom_data = tag.clone(); }
    if let Some(Nbt::Compound(extra)) = compound.get(COMPONENTS) {
        stack.damage = extra.get("damage").and_then(Nbt::as_i64).unwrap_or(0).max(0) as u32;
        stack.max_damage = extra.get("max_damage").and_then(Nbt::as_i64).map(|value| value as i32);
        if let Some(Nbt::Compound(tool)) = extra.get("tool") { stack.tool = Some(tool.clone()); }
        if let Some(Nbt::List(list)) = extra.get("attribute_modifiers") {
            stack.attribute_modifiers = Some(list.iter().filter_map(|tag| if let Nbt::Compound(c) = tag { Some(c.clone()) } else { None }).collect());
        }
        if let Some(Nbt::Compound(map)) = extra.get("enchantments") {
            stack.enchantments = Some(map.iter().filter_map(|(name, level)| level.as_i64().map(|level| (name.clone(), level as i32))).collect());
        }
        stack.map_id = extra.get("map_id").and_then(Nbt::as_i64).map(|value| value as i32);
    }
    // The model selection is derived from the material data, not saved.
    crate::tool_materials::sync_model(&mut stack);
    Some(stack)
}

/// The item slot view of a stack, for the tool damage pipeline and the carried-stack write-backs
/// (which work on slots): the item, count, damage, max damage, custom data, custom model data and
/// enchantments. None for an item id that does not parse.
pub(crate) fn slot_of(stack: &StackData) -> Option<ItemSlot> {
    let mut slot = ItemSlot::new(Identifier::parse(&stack.item).ok()?, stack.count);
    slot.damage = stack.damage;
    slot.max_damage = stack.max_damage;
    slot.custom_data = stack.custom_data.clone();
    slot.custom_model_data = stack.custom_model_data.clone();
    if !stack.enchantments.is_empty() {
        slot.enchantments = Some(stack.enchantments.clone());
    }
    Some(slot)
}

/// The stack a slot view writes back as: the item, count, damage, max damage, custom data, custom
/// model data and enchantments.
pub(crate) fn stack_of(slot: &ItemSlot) -> StackData {
    StackData {
        item: slot.item.to_string(),
        count: slot.count,
        damage: slot.damage,
        max_damage: slot.max_damage,
        custom_data: slot.custom_data.clone(),
        custom_model_data: slot.custom_model_data.clone(),
        enchantments: slot.enchantments.clone().unwrap_or_default(),
    }
}

/// Item.canFitInsideContainerItems: a shulker box (any colour) does not fit inside a container,
/// and every other item does. The host offers no item property for this, so the vanilla rule is
/// spelled out here (ShulkerBoxItem is the one vanilla override).
pub(crate) fn can_fit_inside_container(item: &Identifier) -> bool { !item.path().ends_with("shulker_box") }

/// Whether the stack carries an item handler of its own: a built Tinkers tool with inventory
/// slots does (the capability exists only above zero slots). The port knows no other mod's
/// handlers, so anything that is not such a tool answers false.
pub(crate) fn has_item_handler(stack: &ItemSlot) -> bool {
    let data = stack_of(stack);
    with_held_tool(&data, |env, tool| tool_stack::inventory(&tool.custom_data, env).slots() > 0).unwrap_or(false)
}

/// The item facts for the blacklist and the slot-validity checks, from the captured item
/// snapshot (the tag answers) and the item rules above. The max stack answers 64: neither
/// check reads it (the blacklist reads the tag, the fit rule and the handler; slot validity
/// reads the blacklist and the module's filter), and no vanilla max-stack surface exists to
/// answer it from — do not use this `Items` for slot limits or stackability.
pub(crate) fn with_snapshot_items<R>(run: impl FnOnce(&Items<'_>) -> R) -> R {
    let snapshot = common::item_snapshot();
    let in_tag = |item: &Identifier, tag: &Identifier| snapshot.as_deref().and_then(|tags| tags.get_tag(tag)).is_some_and(|members| members.contains(item));
    let max_stack = |_: &Identifier| 64;
    let items = Items { max_stack: &max_stack, in_tag: &in_tag, can_fit_inside_container: &can_fit_inside_container, has_item_handler: &has_item_handler };
    run(&items)
}

/// ItemStack.isSameItem: only the item counts.
fn same_item(a: &ItemSlot, b: &ItemSlot) -> bool { a.item == b.item }
/// ItemStack.isSameItemSameTags: everything but the count.
pub(crate) fn same_stack(a: &ItemSlot, b: &ItemSlot) -> bool {
    let mut left = write_stack(a, 0);
    let mut right = write_stack(b, 0);
    left.remove("Count");
    right.remove("Count");
    left == right
}

impl Part {
    /// InventoryModule.getStack.
    fn get(&self, persistent: &NbtCompound, slot: usize) -> Option<ItemSlot> {
        if slot >= self.slots() { return None; }
        compound_list(persistent, &self.key())?.into_iter().find(|compound| slot_number(compound) == slot as i64).and_then(read_stack)
    }

    /// Whether the part is active on its tool (its module's condition held).
    pub(crate) fn is_active(&self) -> bool { self.active }
    /// The limit of one of this part's slots.
    pub(crate) fn slot_limit(&self) -> i32 { self.limit() }
    /// The part's stored entries, in stored order, as (slot, the entry compound as stored): a
    /// consumer that keeps its own fields in the entries (a smelting module's cooking times)
    /// reads and rewrites them here, where the stack accessors would drop those fields.
    pub(crate) fn entries(&self, persistent: &NbtCompound) -> Vec<(usize, NbtCompound)> {
        compound_list(persistent, &self.key()).map(|list| list.iter().filter_map(|compound| {
            let slot = slot_number(compound);
            (slot >= 0).then(|| (slot as usize, (*compound).clone()))
        }).collect()).unwrap_or_default()
    }
    /// Replaces the part's stored list with `entries`, each compound's slot set from its pair.
    pub(crate) fn set_entries(&self, persistent: &mut NbtCompound, entries: Vec<(usize, NbtCompound)>) {
        let list = entries.into_iter().map(|(slot, mut compound)| {
            compound.insert(TAG_SLOT.into(), Nbt::Int(slot as i32));
            Nbt::Compound(compound)
        }).collect();
        persistent.insert(self.key(), Nbt::List(list));
    }

    /// InventoryModule.setStack: replaces the entry for the slot, or adds one after the last smaller slot; an empty stack removes it.
    fn set(&self, persistent: &mut NbtCompound, slot: usize, stack: Option<&ItemSlot>) {
        if slot >= self.slots() { return; }
        let key = self.key();
        let stack = stack.filter(|stack| stack.count > 0);
        if !matches!(persistent.get(&key), Some(Nbt::List(_))) {
            let Some(stack) = stack else { return; };
            persistent.insert(key, Nbt::List(vec![Nbt::Compound(write_stack(stack, slot))]));
            return;
        }
        let Some(Nbt::List(list)) = persistent.get_mut(&key) else { return; };
        let mut insert_at = 0;
        for index in 0..list.len() {
            let Nbt::Compound(existing) = &list[index] else { continue; };
            let listed = slot_number(existing);
            if listed == slot as i64 {
                match stack { None => { list.remove(index); } Some(stack) => list[index] = Nbt::Compound(write_stack(stack, slot)) }
                return;
            } else if listed < slot as i64 { insert_at = index + 1; }
        }
        if let Some(stack) = stack { list.insert(insert_at.min(list.len()), Nbt::Compound(write_stack(stack, slot))); }
    }
}

// ---- the tool's inventory ------------------------------------------------------------------------------------------------------------

/// The ItemHandler of a tool: the inventory modules of its modifiers, last modifier first (so that a shield strap or tool belt can come
/// later in the screen without moving the key), their slots one after the other.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct ToolInventory { parts: Vec<Part> }

impl ToolInventory {
    pub(crate) fn new(parts: Vec<Part>) -> Self { Self { parts } }
    /// The number of slots, over all modules.
    pub(crate) fn slots(&self) -> usize { self.parts.iter().map(Part::slots).sum() }

    /// The module and the slot inside it for a slot of the whole inventory.
    fn find(&self, slot: usize) -> Option<(&Part, usize)> {
        let mut start = 0;
        for part in &self.parts {
            let size = part.slots();
            if slot < start + size { return Some((part, slot - start)); }
            start += size;
        }
        None
    }

    pub(crate) fn stack(&self, persistent: &NbtCompound, slot: usize) -> Option<ItemSlot> {
        let (part, local) = self.find(slot)?;
        part.get(persistent, local)
    }
    pub(crate) fn set_stack(&self, persistent: &mut NbtCompound, slot: usize, stack: Option<&ItemSlot>) {
        if let Some((part, local)) = self.find(slot) { part.set(persistent, local, stack); }
    }
    pub(crate) fn slot_limit(&self, slot: usize) -> i32 { self.find(slot).map_or(0, |(part, _)| part.limit()) }
    /// The slot's pattern to draw while it is empty (`has_stack` false) and its module has one.
    pub(crate) fn pattern(&self, slot: usize, has_stack: bool) -> Option<&Identifier> {
        if has_stack { return None; }
        self.find(slot)?.0.module.pattern.as_ref()
    }
    /// IItemHandler.isItemValid: no nesting, then the module's filter.
    pub(crate) fn is_item_valid(&self, items: &Items, slot: usize, stack: &ItemSlot) -> bool {
        if stack.count > 0 && items.is_blacklisted(stack) { return false; }
        self.find(slot).is_some_and(|(part, _)| part.valid(&stack.item, items.in_tag))
    }

    /// IItemHandler.insertItem: what is left over (None when it all went in).
    pub(crate) fn insert_item(&self, items: &Items, persistent: &mut NbtCompound, slot: usize, stack: ItemSlot, simulate: bool) -> Option<ItemSlot> {
        if stack.count == 0 { return None; }
        if items.is_blacklisted(&stack) { return Some(stack); }
        let Some((part, local)) = self.find(slot) else { return Some(stack); };
        if !part.valid(&stack.item, items.in_tag) { return Some(stack); }
        let limit = part.limit();
        let count = stack.count as i32;
        let leftover;
        match part.get(persistent, local) {
            None => {
                let can_insert = count.min(items.stack_limit(&stack).min(limit));
                leftover = count - can_insert;
                if !simulate {
                    let mut placed = stack.clone();
                    placed.count = can_insert.max(0) as u32;
                    part.set(persistent, local, Some(&placed));
                }
            }
            Some(mut current) => {
                let top = items.stack_limit(&current).min(limit);
                if current.count as i32 >= top || !same_item(&current, &stack) { return Some(stack); }
                let total = current.count as i32 + count;
                let size = total.min(top);
                leftover = total - size;
                if !simulate { current.count = size as u32; part.set(persistent, local, Some(&current)); }
            }
        }
        if leftover <= 0 { return None; }
        let mut rest = stack;
        rest.count = leftover as u32;
        Some(rest)
    }

    /// IItemHandler.extractItem.
    pub(crate) fn extract_item(&self, persistent: &mut NbtCompound, slot: usize, amount: i32, simulate: bool) -> Option<ItemSlot> {
        if amount <= 0 { return None; }
        let (part, local) = self.find(slot)?;
        let mut current = part.get(persistent, local)?;
        let amount = amount.min(current.count as i32);
        let mut taken = current.clone();
        taken.count = amount as u32;
        if !simulate {
            if amount == current.count as i32 { part.set(persistent, local, None); }
            else { current.count -= amount as u32; part.set(persistent, local, Some(&current)); }
        }
        Some(taken)
    }

    /// The first stack the predicate accepts, with its slot in the whole inventory (the hook's findStack, module after module).
    pub(crate) fn find_stack(&self, persistent: &NbtCompound, predicate: &dyn Fn(&ItemSlot) -> bool) -> Option<(ItemSlot, usize)> {
        let mut start = 0;
        for part in &self.parts {
            let max = part.slots();
            if max > 0 {
                if let Some(list) = compound_list(persistent, &part.key()) {
                    for compound in list {
                        let slot = slot_number(compound);
                        if slot < 0 || slot as usize >= max { continue; }
                        if let Some(stack) = read_stack(compound).filter(|stack| predicate(stack)) { return Some((stack, start + slot as usize)); }
                    }
                }
            }
            start += max;
        }
        None
    }

    /// The first stack a quiver draw takes: the first stored stack its own part accepts as ammunition
    /// (Part::valid — arrows, and what the tool's form adds), in slot order over the parts. The original's
    /// findStack tests the game's ammo predicate, which for the port's tools is the part's own validity:
    /// a quiver part holds nothing else, as inserts are validated the same way.
    pub(crate) fn find_ammo(&self, persistent: &NbtCompound, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Option<(ItemSlot, usize)> {
        let mut start = 0;
        for part in &self.parts {
            let max = part.slots();
            if max > 0 {
                if let Some(list) = compound_list(persistent, &part.key()) {
                    for compound in list {
                        let slot = slot_number(compound);
                        if slot < 0 || slot as usize >= max { continue; }
                        if let Some(stack) = read_stack(compound).filter(|stack| part.valid(&stack.item, in_tag)) { return Some((stack, start + slot as usize)); }
                    }
                }
            }
            start += max;
        }
        None
    }

    /// Every stack, slot order inside each module.
    pub(crate) fn all_stacks(&self, persistent: &NbtCompound) -> Vec<ItemSlot> {
        let mut stacks = Vec::new();
        for part in &self.parts {
            let max = part.slots();
            if max == 0 { continue; }
            let Some(list) = compound_list(persistent, &part.key()) else { continue; };
            let mut parsed: Vec<Option<ItemSlot>> = vec![None; max];
            for compound in list {
                let slot = slot_number(compound);
                if slot >= 0 && (slot as usize) < max { parsed[slot as usize] = read_stack(compound); }
            }
            stacks.extend(parsed.into_iter().flatten());
        }
        stacks
    }

    /// What a shift-click of `stack` into the tool's inventory does (`AbstractContainerMenu.moveItemStackTo` over its slots, front to back):
    /// the stack's remainder, which is `None` when it all moved, and whether anything moved.
    pub(crate) fn quick_move_into(&self, items: &Items, persistent: &mut NbtCompound, mut stack: ItemSlot) -> (Option<ItemSlot>, bool) {
        let mut moved = false;
        let size = self.slots();
        // A slot's limit for a stack: the module's limit, never above the item's own.
        let room = |slot: usize, of: &ItemSlot| self.slot_limit(slot).min(items.stack_limit(of));
        if items.stackable(&stack) {
            for slot in 0..size {
                if stack.count == 0 { break; }
                let Some(mut current) = self.stack(persistent, slot) else { continue; };
                if !same_stack(&stack, &current) { continue; }
                let max = room(slot, &current);
                let total = current.count as i32 + stack.count as i32;
                if total <= max {
                    stack.count = 0;
                    current.count = total as u32;
                } else if (current.count as i32) < max {
                    stack.count -= (max - current.count as i32) as u32;
                    current.count = max as u32;
                } else { continue; }
                self.set_stack(persistent, slot, Some(&current));
                moved = true;
            }
        }
        if stack.count > 0 {
            for slot in 0..size {
                if self.stack(persistent, slot).is_some() || !self.is_item_valid(items, slot, &stack) { continue; }
                let max = self.slot_limit(slot).min(items.stack_limit(&stack));
                let mut placed = stack.clone();
                placed.count = stack.count.min(max.max(0) as u32);
                stack.count -= placed.count;
                self.set_stack(persistent, slot, Some(&placed));
                moved = true;
                break;
            }
        }
        (Some(stack).filter(|stack| stack.count > 0), moved)
    }

    /// InventoryModule.validate for every module: stacks that sit above the slots a module now has move to a free slot below them.
    /// `Err` when there is no room or the module has no slots left (the tool cannot be built that way).
    pub(crate) fn validate(&self, persistent: &mut NbtCompound) -> Result<(), &'static str> {
        for part in self.parts.iter().filter(|part| part.validates) {
            let key = part.key();
            let max = part.slots();
            let Some(Nbt::List(list)) = persistent.get_mut(&key) else { continue; };
            if list.is_empty() { continue; }
            if max == 0 { return Err(HAS_ITEMS); }
            let mut free = vec![true; max];
            for tag in list.iter() {
                if let Nbt::Compound(compound) = tag {
                    let slot = slot_number(compound);
                    if slot >= 0 && (slot as usize) < max { free[slot as usize] = false; }
                }
            }
            for tag in list.iter_mut() {
                let Nbt::Compound(compound) = tag else { continue; };
                if slot_number(compound) >= max as i64 {
                    let Some(target) = free.iter().position(|free| *free) else { return Err(HAS_ITEMS); };
                    free[target] = false;
                    compound.insert(TAG_SLOT.into(), Nbt::Int(target as i32));
                }
            }
        }
        Ok(())
    }
}

/// InventoryModule.onRemoved: the stored stacks block removal; an empty list's key is dropped.
pub(crate) fn on_removed(module: &Inventory, modifier: &Identifier, persistent: &mut NbtCompound) -> Result<(), &'static str> {
    let key = module.key(modifier);
    if compound_list(persistent, &key).is_some_and(|list| !list.is_empty()) { return Err(HAS_ITEMS); }
    persistent.remove(&key);
    Ok(())
}

/// ToolInventoryCapability.addSlots: the volatile total grows by `count`.
pub(crate) fn add_slots(volatile: &mut NbtCompound, count: i32) {
    let total = crate::modifier_entries::nbt_int(volatile.get(TOTAL_SLOTS)).wrapping_add(count);
    volatile.insert(TOTAL_SLOTS.into(), Nbt::Int(total));
}

// ---- opening the screen --------------------------------------------------------------------------------------------------------------

/// InventoryMenuModule: which key opens the inventory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MenuKey { Any, Normal, Shift, Control, Alt }

impl MenuKey {
    /// One key name, as the loaders spell it (`TooltipKey` in the original).
    pub(crate) fn from_name(name: &str) -> Result<Self, String> {
        Ok(match name {
            "any" => Self::Any, "normal" => Self::Normal, "shift" => Self::Shift,
            "control" => Self::Control, "alt" => Self::Alt,
            _ => return Err("on_key must be any, normal, shift, control or alt".into()),
        })
    }
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Self, String> {
        match fields.get("on_key").and_then(Value::as_str) {
            Some(name) => Self::from_name(name),
            _ => Err("on_key must be any, normal, shift, control or alt".into()),
        }
    }
    /// The armor keybind: opens when the key modifier held is the required one (any opens for every key).
    pub(crate) fn opens_on_keybind(self, held: MenuKey) -> bool { self == Self::Any || self == held }
    /// A right click: the key is shift when the player crouches, else normal.
    pub(crate) fn opens_on_use(self, crouching: bool) -> bool { self == Self::Any || self == if crouching { Self::Shift } else { Self::Normal } }
}

/// InventorySlotMenuModule.isValidContainer: the player's own menu (no menu type) or a menu type of the replacement tag.
pub(crate) fn is_valid_container(menu_type: Option<&Identifier>, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> bool {
    menu_type.is_none_or(|menu| in_tag(menu, &Identifier::parse(REPLACEMENT_MENUS).expect("valid tag")))
}
/// InventorySlotMenuModule.overrideOtherStackedOnMe: an empty cursor on a tool in the player's own inventory slots, in a menu that allows it.
pub(crate) fn slot_click_opens(cursor_empty: bool, slot_in_player_inventory: bool, menu_valid: bool) -> bool { cursor_empty && slot_in_player_inventory && menu_valid }

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
    fn module(value: Value) -> Option<Inventory> { Inventory::decode(value.as_object().unwrap()).unwrap() }
    fn part(inventory: Inventory, modifier: &str, level: i32) -> Part { Part::new(inventory, Entry::new(id(modifier), level), true, true) }
    fn stack(item: &str, count: u32) -> ItemSlot { ItemSlot::new(id(item), count) }
    fn max_stack(item: &Identifier) -> u32 { if item.path().ends_with("sword") { 1 } else if item.path() == "ender_pearl" { 16 } else { 64 } }
    fn in_tag(item: &Identifier, tag: &Identifier) -> bool {
        (tag.path() == "throwable" && item.path() == "snowball") || (tag.path() == "inventory_blacklist" && item.path() == "shulker_box")
    }
    fn fits(item: &Identifier) -> bool { item.path() != "shulker_box" }
    fn handler(stack: &ItemSlot) -> bool { stack.custom_data.contains_key("tic_stats") }
    fn items() -> Items<'static> { Items { max_stack: &max_stack, in_tag: &in_tag, can_fit_inside_container: &fits, has_item_handler: &handler } }

    #[test]
    fn a_stack_round_trips_through_its_slot_view() {
        let mut data = StackData { item: "minecraft:diamond_sword".into(), count: 1, damage: 12, max_damage: Some(1561), enchantments: [("minecraft:sharpness".to_owned(), 3)].into_iter().collect(), ..StackData::default() };
        data.custom_data.insert("tconstruct:test".to_owned(), Nbt::Int(1));
        let slot = slot_of(&data).expect("a parsed id");
        assert_eq!((slot.damage, slot.max_damage, slot.enchantments.as_ref().map(|e| e.len())), (12, Some(1561), Some(1)));
        assert_eq!(stack_of(&slot), data, "the write-back is the stack read");
        assert!(slot_of(&StackData { item: "not an id".into(), ..StackData::default() }).is_none());
    }

    #[test]
    fn only_a_shulker_box_does_not_fit_inside_a_container() {
        assert!(!can_fit_inside_container(&id("minecraft:shulker_box")));
        assert!(!can_fit_inside_container(&id("minecraft:red_shulker_box")));
        assert!(can_fit_inside_container(&id("minecraft:chest")));
        assert!(can_fit_inside_container(&id("minecraft:bundle")));
        // A plain stack carries no handler: only a built tool with inventory slots does.
        assert!(!has_item_handler(&stack("minecraft:torch", 1)));
    }
    fn pockets(level: i32) -> ToolInventory {
        ToolInventory::new(vec![part(module(json!({"key":"tconstruct:pockets","slots":{"each_level":3}})).unwrap(), "tconstruct:pocket", level)])
    }

    #[test]
    fn a_quiver_takes_arrows_and_for_the_right_tool_fireworks_and_ballista_ammo() {
        let quiver_items = Items { max_stack: &max_stack, can_fit_inside_container: &fits, has_item_handler: &handler,
            in_tag: &|item: &Identifier, tag: &Identifier| (tag.to_string() == "minecraft:arrows" && matches!(item.path(), "arrow" | "tipped_arrow"))
                || (tag.to_string() == "tconstruct:modifiable/ballista_ammo" && item.path() == "iron_ballista_bolt") };
        // The shipped bulk quiver: two slots a level, no limit of its own; the trick quiver caps at 32.
        let bulk = Inventory::decode_quiver(json!({"pattern":"tconstruct:arrow","slots":{"each_level":2}}).as_object().unwrap()).unwrap().unwrap();
        let mut bulk_part = part(bulk, "tconstruct:bulk_quiver", 1);
        let inventory = ToolInventory::new(vec![bulk_part.clone()]);
        assert_eq!(inventory.slots(), 2);
        assert!(inventory.is_item_valid(&quiver_items, 0, &stack("minecraft:arrow", 1)));
        assert!(inventory.is_item_valid(&quiver_items, 0, &stack("minecraft:tipped_arrow", 1)));
        assert!(!inventory.is_item_valid(&quiver_items, 0, &stack("minecraft:stick", 1)), "no filter items, only ammunition");
        assert!(!inventory.is_item_valid(&quiver_items, 0, &stack("minecraft:firework_rocket", 1)), "fireworks need a crossbow");
        assert!(!inventory.is_item_valid(&quiver_items, 0, &stack("tconstruct:iron_ballista_bolt", 1)), "ballista ammo needs a ballista");
        bulk_part.note_tool(true, false);
        assert!(ToolInventory::new(vec![bulk_part.clone()]).is_item_valid(&quiver_items, 0, &stack("minecraft:firework_rocket", 1)), "a crossbow's fireworks");
        bulk_part.note_ballista();
        assert!(ToolInventory::new(vec![bulk_part]).is_item_valid(&quiver_items, 0, &stack("tconstruct:iron_ballista_bolt", 1)), "a ballista's ammo");
        let trick = Inventory::decode_quiver(json!({"pattern":"tconstruct:tipped_arrow","limit":{"flat":32},"slots":{"each_level":3}}).as_object().unwrap()).unwrap().unwrap();
        assert_eq!(ToolInventory::new(vec![part(trick, "tconstruct:trick_quiver", 1)]).slot_limit(0), 32);
    }

    #[test]
    fn slots_follow_the_level_and_the_limit_defaults_to_a_stack() {
        assert_eq!(pockets(2).slots(), 6);
        let belt = module(json!({"slots":{"each_level":1,"flat":3}})).unwrap();
        assert_eq!(belt.volatile_slots(&Entry::new(id("tconstruct:tool_belt"), 2)), 5);
        assert_eq!(pockets(1).slot_limit(0), 64);
        assert_eq!(pockets(1).slot_limit(3), 0, "past the last slot");
        let mut volatile = NbtCompound::new();
        add_slots(&mut volatile, 3);
        add_slots(&mut volatile, 5);
        assert_eq!(volatile.get(TOTAL_SLOTS), Some(&Nbt::Int(8)));
        assert!(Inventory::decode(json!({"limit":{"flat":1}}).as_object().unwrap()).is_err(), "slots are required");
    }

    #[test]
    fn stacks_are_stored_as_the_original_wrote_them_ordered_by_slot() {
        let inventory = pockets(1);
        let mut persistent = NbtCompound::new();
        inventory.set_stack(&mut persistent, 2, Some(&stack("minecraft:stick", 5)));
        inventory.set_stack(&mut persistent, 0, Some(&stack("minecraft:apple", 3)));
        let Some(Nbt::List(list)) = persistent.get("tconstruct:pockets") else { panic!("no list") };
        let Nbt::Compound(first) = &list[0] else { panic!() };
        assert_eq!(first.get("id"), Some(&Nbt::String("minecraft:apple".into())));
        assert_eq!(first.get("Count"), Some(&Nbt::Byte(3)));
        assert_eq!(first.get("Slot"), Some(&Nbt::Int(0)));
        assert_eq!(inventory.stack(&persistent, 2).unwrap().count, 5);
        inventory.set_stack(&mut persistent, 3, Some(&stack("minecraft:stick", 1)));
        assert!(inventory.stack(&persistent, 3).is_none(), "outside the slots nothing is written");
        inventory.set_stack(&mut persistent, 0, None);
        assert_eq!(inventory.all_stacks(&persistent).len(), 1);
    }

    #[test]
    fn a_stack_with_data_returns_as_it_left() {
        let mut tool = stack("tconstruct:pickaxe", 1);
        tool.custom_data.insert("tic_stats".into(), Nbt::Compound(NbtCompound::new()));
        tool.damage = 7;
        tool.tool = Some(NbtCompound::new());
        tool.enchantments = Some([("minecraft:fortune".to_owned(), 3)].into());
        let written = write_stack(&tool, 4);
        assert_eq!(read_stack(&written), Some(tool));
        let mut empty = write_stack(&stack("minecraft:stick", 1), 0);
        empty.insert("Count".into(), Nbt::Byte(0));
        assert_eq!(read_stack(&empty), None, "a count of zero is no stack");
    }

    #[test]
    fn a_stored_map_keeps_its_id_and_a_plain_map_keeps_none() {
        let mut map = stack("minecraft:filled_map", 1);
        map.map_id = Some(12);
        assert_eq!(read_stack(&write_stack(&map, 0)), Some(map), "the id survives the persistent round-trip");
        let plain = write_stack(&stack("minecraft:filled_map", 1), 0);
        assert_eq!(read_stack(&plain).unwrap().map_id, None, "a map with no id reads back none");
        assert!(plain.get(COMPONENTS).is_none(), "a plain map writes no components compound at all");
    }

    #[test]
    fn insert_fills_a_slot_up_to_the_limit_and_returns_the_rest() {
        let inventory = ToolInventory::new(vec![part(module(json!({"limit":{"flat":16},"slots":{"each_level":3}})).unwrap(), "tconstruct:sleeves", 1)]);
        let mut persistent = NbtCompound::new();
        assert_eq!(inventory.insert_item(&items(), &mut persistent, 0, stack("minecraft:snowball", 20), false).unwrap().count, 4);
        assert_eq!(inventory.stack(&persistent, 0).unwrap().count, 16);
        let rest = inventory.insert_item(&items(), &mut persistent, 0, stack("minecraft:snowball", 1), false);
        assert_eq!(rest.unwrap().count, 1, "a full slot takes nothing");
        inventory.extract_item(&mut persistent, 0, 6, false);
        assert_eq!(inventory.insert_item(&items(), &mut persistent, 0, stack("minecraft:snowball", 3), true), None);
        assert_eq!(inventory.stack(&persistent, 0).unwrap().count, 10, "a simulation changes nothing");
        assert_eq!(inventory.insert_item(&items(), &mut persistent, 0, stack("minecraft:stick", 1), false).unwrap().item, id("minecraft:stick"), "another item does not merge");
        assert_eq!(inventory.insert_item(&items(), &mut persistent, 1, stack("minecraft:ender_pearl", 40), false).unwrap().count, 24, "the item's own stack size caps it");
    }

    #[test]
    fn the_filter_and_the_nesting_rules_refuse_items() {
        let filtered = module(json!({"filter":{"type":"mantle:tag","tag":"tconstruct:throwable"},"slots":{"each_level":3}})).unwrap();
        let inventory = ToolInventory::new(vec![part(filtered, "tconstruct:sleeves", 1)]);
        assert!(inventory.is_item_valid(&items(), 0, &stack("minecraft:snowball", 1)));
        assert!(!inventory.is_item_valid(&items(), 0, &stack("minecraft:stick", 1)));
        let open = pockets(1);
        assert!(!open.is_item_valid(&items(), 0, &stack("minecraft:shulker_box", 1)), "a blacklisted item");
        let mut tool = stack("tconstruct:pickaxe", 1);
        tool.custom_data.insert("tic_stats".into(), Nbt::Compound(NbtCompound::new()));
        assert!(!open.is_item_valid(&items(), 0, &tool), "an item with its own inventory");
        let mut persistent = NbtCompound::new();
        assert!(open.insert_item(&items(), &mut persistent, 0, stack("minecraft:shulker_box", 1), false).is_some());
        assert!(persistent.is_empty());
        let map = module(json!({"filter":"tconstruct:map","limit":{"flat":1},"slots":{"each_level":3}})).unwrap();
        assert!(map.filter.matches(&id("minecraft:filled_map"), &in_tag) && !map.filter.matches(&id("minecraft:paper"), &in_tag));
    }

    #[test]
    fn extract_takes_part_of_a_stack_or_clears_the_slot() {
        let inventory = pockets(1);
        let mut persistent = NbtCompound::new();
        inventory.insert_item(&items(), &mut persistent, 1, stack("minecraft:apple", 10), false);
        assert_eq!(inventory.extract_item(&mut persistent, 1, 4, true).unwrap().count, 4);
        assert_eq!(inventory.extract_item(&mut persistent, 1, 4, false).unwrap().count, 4);
        assert_eq!(inventory.stack(&persistent, 1).unwrap().count, 6);
        assert_eq!(inventory.extract_item(&mut persistent, 1, 50, false).unwrap().count, 6);
        assert!(inventory.stack(&persistent, 1).is_none());
        assert!(inventory.extract_item(&mut persistent, 1, 1, false).is_none());
        assert!(inventory.extract_item(&mut persistent, 0, 0, false).is_none());
    }

    #[test]
    fn modules_stack_up_in_the_order_given() {
        let first = part(module(json!({"slots":{"flat":1}})).unwrap(), "tconstruct:a", 1);
        let second = part(module(json!({"key":"tconstruct:b_items","slots":{"flat":2}})).unwrap(), "tconstruct:b", 1);
        // The handler lists the later modifier first.
        let inventory = ToolInventory::new(vec![second, first]);
        assert_eq!(inventory.slots(), 3);
        let mut persistent = NbtCompound::new();
        inventory.insert_item(&items(), &mut persistent, 2, stack("minecraft:stick", 1), false);
        assert!(persistent.contains_key("tconstruct:a"), "slot 2 is the first modifier's only slot");
        inventory.insert_item(&items(), &mut persistent, 0, stack("minecraft:apple", 1), false);
        assert!(persistent.contains_key("tconstruct:b_items"));
        assert_eq!(inventory.find_stack(&persistent, &|s| s.item.path() == "stick").unwrap().1, 2);
        assert_eq!(inventory.all_stacks(&persistent).len(), 2);
    }

    #[test]
    fn a_tool_that_lost_slots_moves_stacks_down_or_refuses() {
        let mut persistent = NbtCompound::new();
        let wide = pockets(2);
        wide.set_stack(&mut persistent, 5, Some(&stack("minecraft:stick", 1)));
        wide.set_stack(&mut persistent, 4, Some(&stack("minecraft:apple", 1)));
        let narrow = pockets(1);
        assert_eq!(narrow.validate(&mut persistent), Ok(()));
        assert_eq!(narrow.stack(&persistent, 0).unwrap().item, id("minecraft:apple"), "list order picks the first free slot");
        assert_eq!(narrow.stack(&persistent, 1).unwrap().item, id("minecraft:stick"));
        let none = ToolInventory::new(vec![part(module(json!({"key":"tconstruct:pockets","slots":{"each_level":0}})).unwrap(), "tconstruct:pocket", 1)]);
        assert_eq!(none.validate(&mut persistent), Err(HAS_ITEMS));
        let mut crowded = NbtCompound::new();
        for (slot, item) in ["minecraft:a", "minecraft:b", "minecraft:c", "minecraft:d"].into_iter().enumerate() {
            wide.set_stack(&mut crowded, slot, Some(&stack(item, 1)));
        }
        assert_eq!(narrow.validate(&mut crowded), Err(HAS_ITEMS), "no free slot below");
    }

    #[test]
    fn removing_the_modifier_needs_an_empty_inventory() {
        let inventory = module(json!({"key":"tconstruct:pockets","slots":{"each_level":3}})).unwrap();
        let mut persistent = NbtCompound::new();
        pockets(1).set_stack(&mut persistent, 0, Some(&stack("minecraft:stick", 1)));
        assert_eq!(on_removed(&inventory, &id("tconstruct:pocket"), &mut persistent), Err(HAS_ITEMS));
        pockets(1).set_stack(&mut persistent, 0, None);
        assert_eq!(on_removed(&inventory, &id("tconstruct:pocket"), &mut persistent), Ok(()));
        assert!(!persistent.contains_key("tconstruct:pockets"));
    }

    #[test]
    fn a_quick_move_merges_into_matching_stacks_then_the_first_empty_slot() {
        let inventory = ToolInventory::new(vec![part(module(json!({"limit":{"flat":16},"slots":{"each_level":3}})).unwrap(), "tconstruct:sliver", 1)]);
        let mut persistent = NbtCompound::new();
        inventory.set_stack(&mut persistent, 1, Some(&stack("minecraft:slime_ball", 10)));
        let (rest, moved) = inventory.quick_move_into(&items(), &mut persistent, stack("minecraft:slime_ball", 30));
        assert!(moved);
        assert_eq!(inventory.stack(&persistent, 1).unwrap().count, 16);
        assert_eq!(inventory.stack(&persistent, 0).unwrap().count, 16, "the first empty slot takes what the limit allows");
        assert_eq!(rest.unwrap().count, 8, "one stack goes into one empty slot only");
        let (rest, moved) = inventory.quick_move_into(&items(), &mut persistent, stack("minecraft:shulker_box", 1));
        assert!(!moved && rest.is_some());
        let mut sword = stack("minecraft:iron_sword", 1);
        sword.damage = 5;
        let (rest, moved) = inventory.quick_move_into(&items(), &mut persistent, sword);
        assert!(moved && rest.is_none(), "an unstackable item takes an empty slot");
    }

    #[test]
    fn menu_keys_and_slot_clicks() {
        assert!(MenuKey::Any.opens_on_use(true) && MenuKey::Any.opens_on_use(false));
        assert!(MenuKey::Shift.opens_on_use(true) && !MenuKey::Shift.opens_on_use(false));
        assert!(MenuKey::Normal.opens_on_use(false) && !MenuKey::Normal.opens_on_use(true));
        assert!(MenuKey::Alt.opens_on_keybind(MenuKey::Alt) && !MenuKey::Alt.opens_on_keybind(MenuKey::Shift));
        assert!(MenuKey::decode(json!({"on_key":"hold"}).as_object().unwrap()).is_err());
        let replace = |menu: &Identifier, tag: &Identifier| menu.path() == "crafting" && tag.path() == "tool_inventory_replacements";
        assert!(is_valid_container(None, &replace) && is_valid_container(Some(&id("minecraft:crafting")), &replace));
        assert!(!is_valid_container(Some(&id("minecraft:chest")), &replace));
        assert!(slot_click_opens(true, true, true) && !slot_click_opens(false, true, true) && !slot_click_opens(true, false, true));
    }

    #[test]
    fn a_filter_this_port_cannot_evaluate_leaves_the_module_unbound() {
        assert!(module(json!({"filter":"tconstruct:can_melt","slots":{"flat":1}})).is_none());
        assert!(module(json!({"filter":{"type":"mantle:or","predicates":["tconstruct:map","tconstruct:bucket"]},"slots":{"flat":1}})).is_none());
        assert!(module(json!({"filter":{"type":"mantle:or","predicates":["tconstruct:map","mantle:any"]},"slots":{"flat":1}})).is_some());
    }
}
