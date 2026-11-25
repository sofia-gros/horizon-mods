//! Counterpart of Fabric's `fabric-item-group-api-v1`: creative inventory tabs a mod adds, and entries added to existing tabs.
//!
//! A tab holds item stacks, including stacks that carry data (a finished tool with its materials), and the player takes them out of
//! the creative screen like any other stack. More tabs than fit the screen are paged by the game (ten mod tabs to a page).
//!
//! Fabric: `FabricItemGroup.builder().displayName(..).icon(..).entries(..)` registered in `Registries.ITEM_GROUP`, and
//! `ItemGroupEvents.modifyEntriesEvent(key)`. Here: [`ItemGroup`], [`ItemGroupRegistry::register`] and [`ItemGroupEvents`].

use std::cell::RefCell;
use std::sync::Mutex;

use serde_json::{json, Value};

use super::base::{Event, Identifier};

/// A stack of an item, optionally with data (Minecraft: `ItemStack`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemStack {
    /// The item.
    pub item: Identifier,
    /// How many.
    pub count: u32,
    /// The stack's data components as an SNBT compound, such as `{"minecraft:custom_name":'"Excalibur"'}`; None for a plain stack.
    pub components: Option<String>,
}

impl ItemStack {
    /// A plain stack of `count` of `item`.
    pub fn new(item: Identifier, count: u32) -> Self {
        Self { item, count, components: None }
    }

    /// The same stack with data components given as an SNBT compound.
    pub fn with_components(mut self, snbt_compound: &str) -> Self {
        self.components = Some(snbt_compound.to_owned());
        self
    }

    /// The stack in the SNBT form the creative inventory data uses: `{count:1,id:"minecraft:stick"}`.
    pub fn to_snbt(&self) -> String {
        match &self.components {
            Some(components) => format!("{{count:{},id:\"{}\",components:{}}}", self.count, self.item, components),
            None => format!("{{count:{},id:\"{}\"}}", self.count, self.item),
        }
    }
}

/// A creative inventory tab (Fabric: `ItemGroup`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemGroup {
    /// The tab's id, such as `tconstruct:tools`.
    pub id: Identifier,
    /// The title shown on the tab.
    pub display_name: String,
    /// The stack drawn on the tab's button.
    pub icon: ItemStack,
    /// The stacks in the tab, in order.
    pub entries: Vec<ItemStack>,
}

impl ItemGroup {
    /// A tab with no entries yet.
    pub fn new(id: Identifier, display_name: &str, icon: ItemStack) -> Self {
        Self { id, display_name: display_name.to_owned(), icon, entries: Vec::new() }
    }

    /// The same tab with one more entry at the end.
    pub fn entry(mut self, stack: ItemStack) -> Self {
        self.entries.push(stack);
        self
    }

    /// The same tab with several more entries at the end.
    pub fn entries(mut self, stacks: impl IntoIterator<Item = ItemStack>) -> Self {
        self.entries.extend(stacks);
        self
    }
}

static GROUPS: Mutex<Vec<ItemGroup>> = Mutex::new(Vec::new());

/// Registers tabs (Fabric: `Registry.register(Registries.ITEM_GROUP, id, group)`).
pub struct ItemGroupRegistry;

impl ItemGroupRegistry {
    /// Adds a tab to the creative inventory. Registering an id again replaces the earlier tab.
    pub fn register(group: ItemGroup) {
        if let Ok(mut all) = GROUPS.lock() {
            all.retain(|g| g.id != group.id);
            all.push(group);
        }
    }

    /// The tabs registered so far, in registration order.
    pub fn registered() -> Vec<ItemGroup> {
        GROUPS.lock().map(|all| all.clone()).unwrap_or_default()
    }
}

/// What a modify-entries callback receives: the tab and its entries to edit.
pub struct ModifyEntries {
    /// The tab being filled, such as `minecraft:ingredients` or one of the mod tabs.
    pub group: Identifier,
    /// Stacks to add after the tab's own entries; callbacks push onto it with `borrow_mut`.
    pub added: RefCell<Vec<ItemStack>>,
}

static MODIFY_ENTRIES: Event<ModifyEntries, ()> = Event::void();

/// Fabric: `ItemGroupEvents`.
pub struct ItemGroupEvents;

impl ItemGroupEvents {
    /// Fabric: `ItemGroupEvents.MODIFY_ENTRIES_ALL`: runs for every tab as the creative inventory is built.
    pub const MODIFY_ENTRIES_ALL: &'static Event<ModifyEntries, ()> = &MODIFY_ENTRIES;

    /// Fabric: `ItemGroupEvents.modifyEntriesEvent(key)`: `callback` runs for the one tab `group`, vanilla's or a mod's.
    pub fn modify_entries_event(group: Identifier, callback: impl Fn(&ModifyEntries) + Send + Sync + 'static) {
        MODIFY_ENTRIES.register(move |entries| {
            if entries.group == group {
                callback(entries);
            }
        });
    }
}

fn snbt_list(stacks: &[ItemStack]) -> Vec<Value> {
    stacks.iter().map(|stack| Value::String(stack.to_snbt())).collect()
}

fn display_path(stack: &ItemStack) -> String {
    stack.item.path().to_owned()
}

/// The creative inventory data (`creative-inventory.json` form) that adds PlutoLib's contribution to the tabs the game already has:
/// the entries callbacks add to existing tabs, the registered tabs (appended after the existing ones, ten to a page as the game pages
/// mod tabs) and a search tab entry carrying all their stacks. `existing` are the ids of the tabs already there, `mod_tabs` how many of
/// them are mod tabs (page one and up). Called by the game-facing half when the creative inventory is built.
pub(crate) fn document(existing: &[String], mod_tabs: usize) -> Value {
    let mut tabs: Vec<Value> = Vec::new();
    let mut for_search: Vec<ItemStack> = Vec::new();
    let mut mod_tabs = mod_tabs;

    // entries added to tabs the game or other mods made
    for id in existing {
        let Ok(group) = Identifier::parse(id) else { continue };
        let context = ModifyEntries { group, added: RefCell::new(Vec::new()) };
        MODIFY_ENTRIES.invoke(&context);
        let added = context.added.into_inner();
        if added.is_empty() {
            continue;
        }
        tabs.push(json!({"id": id, "type": "CATEGORY", "items": snbt_list(&added)}));
        for_search.extend(added);
    }

    // the registered tabs
    for group in ItemGroupRegistry::registered() {
        if existing.contains(&group.id.to_string()) {
            continue;
        }
        let context = ModifyEntries { group: group.id.clone(), added: RefCell::new(Vec::new()) };
        MODIFY_ENTRIES.invoke(&context);
        let mut entries = group.entries.clone();
        entries.extend(context.added.into_inner());
        let slot = mod_tabs % 10;
        tabs.push(json!({
            "id": group.id.to_string(),
            "type": "CATEGORY",
            "row": if slot < 5 { "TOP" } else { "BOTTOM" },
            "column": slot % 5,
            "page": 1 + mod_tabs / 10,
            "title": group.display_name,
            "icon": group.icon.to_snbt(),
            "visible": true,
            "items": snbt_list(&entries),
        }));
        mod_tabs += 1;
        for_search.extend(entries);
    }

    // all of them are findable in the search tab
    tabs.push(json!({
        "id": "pluto_lib:search_additions",
        "type": "SEARCH",
        "items": snbt_list(&for_search),
        "lines": for_search.iter().map(|s| json!([display_path(s)])).collect::<Vec<_>>(),
        "tags": for_search.iter().map(|_| json!([])).collect::<Vec<_>>(),
    }));
    json!({ "tabs": tabs })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as TestLock;

    /// The registry is global, so the tests that fill it run one at a time and start from an empty one.
    static ONE_AT_A_TIME: TestLock<()> = TestLock::new(());

    fn start() -> std::sync::MutexGuard<'static, ()> {
        let guard = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        GROUPS.lock().unwrap().clear();
        guard
    }

    fn id(text: &str) -> Identifier {
        Identifier::parse(text).unwrap()
    }

    #[test]
    fn registered_tabs_follow_the_existing_ones_with_stacks_that_carry_data_and_reach_the_search_tab() {
        let _only = start();
        ItemGroupRegistry::register(
            ItemGroup::new(id("test:tools"), "Test Tools", ItemStack::new(id("minecraft:stick"), 1))
                .entry(ItemStack::new(id("test:pick"), 1).with_components("{\"minecraft:damage\":3}")),
        );
        ItemGroupEvents::modify_entries_event(id("minecraft:ingredients"), |entries| {
            entries.added.borrow_mut().push(ItemStack::new(id("test:gem"), 1));
        });
        let ingredients = vec!["minecraft:ingredients".to_owned()];
        let doc = document(&ingredients, 0);
        let tabs = doc["tabs"].as_array().unwrap();
        let mine = tabs.iter().find(|t| t["id"] == "test:tools").unwrap();
        assert_eq!((mine["page"].as_u64(), mine["row"].as_str(), mine["column"].as_u64()), (Some(1), Some("TOP"), Some(0)));
        assert_eq!(mine["items"][0], "{count:1,id:\"test:pick\",components:{\"minecraft:damage\":3}}");
        let existing = tabs.iter().find(|t| t["id"] == "minecraft:ingredients").unwrap();
        assert_eq!(existing["items"][0], "{count:1,id:\"test:gem\"}", "an entry added to an existing tab is listed for that tab");
        let search = tabs.iter().find(|t| t["type"] == "SEARCH").unwrap();
        assert_eq!(search["items"].as_array().unwrap().len(), 2);
        assert_eq!(search["lines"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn more_than_ten_mod_tabs_continue_on_the_next_page_and_a_known_id_is_not_added_twice() {
        let _only = start();
        for n in 0..11 {
            ItemGroupRegistry::register(ItemGroup::new(id(&format!("many:tab{n}")), "T", ItemStack::new(id("minecraft:stick"), 1)));
        }
        let doc = document(&[], 0);
        let tabs = doc["tabs"].as_array().unwrap();
        let eleventh = tabs.iter().find(|t| t["id"] == "many:tab10").unwrap();
        assert_eq!(eleventh["page"], 2, "ten to a page, the eleventh starts page two");
        let known = vec!["many:tab0".to_owned()];
        let again = document(&known, 5);
        assert!(again["tabs"].as_array().unwrap().iter().all(|t| t["id"] != "many:tab0"), "ids already present are skipped");
        let first = again["tabs"].as_array().unwrap().iter().find(|t| t["id"] == "many:tab1").unwrap();
        assert_eq!((first["row"].as_str(), first["column"].as_u64()), (Some("BOTTOM"), Some(0)), "numbering continues after five mod tabs");
    }
}
