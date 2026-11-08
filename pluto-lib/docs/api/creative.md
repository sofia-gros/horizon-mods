# Creative inventory tabs

Add tabs to the creative inventory, with stacks that carry data (a finished tool with its materials, a potion with its effects) that the player takes out like any other stack, and add entries to tabs that already exist. More tabs than fit the screen are paged by the game, ten mod tabs to a page; every stack is also findable in the search tab.

Register in your mod's `hooks()`, before the creative inventory is loaded.

### `ItemStack`

A stack of an item, optionally with data. `item` is the item, `count` how many, `components` the stack's data components as an SNBT compound (None for a plain stack).

```rust
use pluto_lib::{Identifier, ItemStack};

let plain = ItemStack::new(Identifier::new("minecraft", "stick").unwrap(), 1);
assert_eq!(plain.count, 1);
assert!(plain.components.is_none());
assert_eq!(plain.item.to_string(), "minecraft:stick");
```

### `ItemStack::new`

`fn new(item: Identifier, count: u32) -> ItemStack`. A plain stack.

```rust
use pluto_lib::{Identifier, ItemStack};

let ten_apples = ItemStack::new(Identifier::new("minecraft", "apple").unwrap(), 10);
assert_eq!(ten_apples.count, 10);
```

### `ItemStack::with_components`

`fn with_components(self, snbt_compound: &str) -> ItemStack`. The same stack with data components, given as an SNBT compound.

```rust
use pluto_lib::{Identifier, ItemStack};

let named = ItemStack::new(Identifier::new("minecraft", "stick").unwrap(), 1)
    .with_components("{\"minecraft:custom_name\":'\"Wand\"'}");
assert!(named.components.is_some());
```

### `ItemStack::to_snbt`

`fn to_snbt(&self) -> String`. The stack in the SNBT form the creative inventory data uses.

```rust
use pluto_lib::{Identifier, ItemStack};

let stack = ItemStack::new(Identifier::new("minecraft", "stick").unwrap(), 2);
assert_eq!(stack.to_snbt(), "{count:2,id:\"minecraft:stick\"}");
let named = stack.with_components("{\"minecraft:damage\":3}");
assert_eq!(named.to_snbt(), "{count:2,id:\"minecraft:stick\",components:{\"minecraft:damage\":3}}");
```

### `ItemGroup`

A creative tab. `id` names it, `display_name` is the title shown, `icon` the stack drawn on its button, `entries` the stacks it holds in order.

```rust
use pluto_lib::{Identifier, ItemGroup, ItemStack};

let id = Identifier::new("example", "tools").unwrap();
let icon = ItemStack::new(Identifier::new("minecraft", "iron_pickaxe").unwrap(), 1);
let group = ItemGroup::new(id, "Example Tools", icon);
assert_eq!(group.display_name, "Example Tools");
assert!(group.entries.is_empty());
assert_eq!(group.id.to_string(), "example:tools");
assert_eq!(group.icon.count, 1);
```

### `ItemGroup::new`

`fn new(id: Identifier, display_name: &str, icon: ItemStack) -> ItemGroup`. A tab with no entries yet.

```rust
use pluto_lib::{Identifier, ItemGroup, ItemStack};

let icon = ItemStack::new(Identifier::new("minecraft", "stick").unwrap(), 1);
let group = ItemGroup::new(Identifier::new("example", "misc").unwrap(), "Misc", icon);
assert_eq!(group.entries.len(), 0);
```

### `ItemGroup::entry`

`fn entry(self, stack: ItemStack) -> ItemGroup`. One more entry at the end.

```rust
use pluto_lib::{Identifier, ItemGroup, ItemStack};

let stick = ItemStack::new(Identifier::new("minecraft", "stick").unwrap(), 1);
let group = ItemGroup::new(Identifier::new("example", "misc").unwrap(), "Misc", stick.clone()).entry(stick);
assert_eq!(group.entries.len(), 1);
```

### `ItemGroup::entries`

`fn entries(self, stacks: impl IntoIterator<Item = ItemStack>) -> ItemGroup`. Several more entries at the end.

```rust
use pluto_lib::{Identifier, ItemGroup, ItemStack};

let item = |name: &str| ItemStack::new(Identifier::new("minecraft", name).unwrap(), 1);
let group = ItemGroup::new(Identifier::new("example", "mix").unwrap(), "Mix", item("stick"))
    .entries(["apple", "bread", "carrot"].map(item));
assert_eq!(group.entries.len(), 3);
```

### `ItemGroupRegistry`

Registers tabs (Fabric: `Registry.register(Registries.ITEM_GROUP, ...)`).

```rust
use pluto_lib::{Identifier, ItemGroup, ItemGroupRegistry, ItemStack};

let icon = ItemStack::new(Identifier::new("minecraft", "stick").unwrap(), 1);
ItemGroupRegistry::register(ItemGroup::new(Identifier::new("example", "first").unwrap(), "First", icon));
```

### `ItemGroupRegistry::register`

`fn register(group: ItemGroup)`. Adds the tab to the creative inventory. Registering the same id again replaces the earlier tab.

```rust
use pluto_lib::{Identifier, ItemGroup, ItemGroupRegistry, ItemStack};

let id = Identifier::new("example", "replaced").unwrap();
let icon = ItemStack::new(Identifier::new("minecraft", "stick").unwrap(), 1);
ItemGroupRegistry::register(ItemGroup::new(id.clone(), "Old", icon.clone()));
ItemGroupRegistry::register(ItemGroup::new(id, "New", icon));
let mine: Vec<_> = ItemGroupRegistry::registered().into_iter().filter(|g| g.id.path() == "replaced").collect();
assert_eq!(mine.len(), 1);
assert_eq!(mine[0].display_name, "New");
```

### `ItemGroupRegistry::registered`

`fn registered() -> Vec<ItemGroup>`. The tabs registered so far, in registration order.

```rust
use pluto_lib::{Identifier, ItemGroup, ItemGroupRegistry, ItemStack};

let icon = ItemStack::new(Identifier::new("minecraft", "stick").unwrap(), 1);
ItemGroupRegistry::register(ItemGroup::new(Identifier::new("example", "listed").unwrap(), "Listed", icon));
assert!(ItemGroupRegistry::registered().iter().any(|g| g.id.path() == "listed"));
```

### `ModifyEntries`

What a modify-entries callback receives. `group` is the tab being filled, `added` the stacks to add after the tab's own entries (callbacks push onto it with `borrow_mut`).

```rust
use pluto_lib::{Identifier, ItemGroupEvents, ItemStack, ModifyEntries};

ItemGroupEvents::MODIFY_ENTRIES_ALL.register(|entries: &ModifyEntries| {
    if entries.group.to_string() == "minecraft:ingredients" {
        let gem = ItemStack::new(Identifier::new("example", "gem").unwrap(), 1);
        entries.added.borrow_mut().push(gem);
    }
});
```

### `ItemGroupEvents`

The events that edit tab entries. Fabric: `ItemGroupEvents`.

```rust
use pluto_lib::ItemGroupEvents;

ItemGroupEvents::MODIFY_ENTRIES_ALL.register(|_| {});
```

### `ItemGroupEvents::MODIFY_ENTRIES_ALL`

Runs for every tab, vanilla's and mods', as the creative inventory is built. Check `entries.group` to pick the tab.

```rust
use pluto_lib::{Identifier, ItemGroupEvents, ItemStack};
use std::cell::RefCell;
use pluto_lib::ModifyEntries;

ItemGroupEvents::MODIFY_ENTRIES_ALL.register(|e| e.added.borrow_mut().push(ItemStack::new(Identifier::new("example", "extra").unwrap(), 1)));
let context = ModifyEntries { group: Identifier::new("example", "any").unwrap(), added: RefCell::new(Vec::new()) };
ItemGroupEvents::MODIFY_ENTRIES_ALL.invoke(&context);
assert_eq!(context.added.borrow().len(), 1);
```

### `ItemGroupEvents::modify_entries_event`

`fn modify_entries_event(group: Identifier, callback: impl Fn(&ModifyEntries) + Send + Sync + 'static)`. Runs `callback` for one tab only, vanilla's (`minecraft:ingredients`) or a mod's.

```rust
use pluto_lib::{Identifier, ItemGroupEvents, ItemStack, ModifyEntries};
use std::cell::RefCell;

let tab = Identifier::new("minecraft", "ingredients").unwrap();
ItemGroupEvents::modify_entries_event(tab.clone(), |e| {
    e.added.borrow_mut().push(ItemStack::new(Identifier::new("example", "dust").unwrap(), 1));
});
let here = ModifyEntries { group: tab, added: RefCell::new(Vec::new()) };
ItemGroupEvents::MODIFY_ENTRIES_ALL.invoke(&here);
assert_eq!(here.added.borrow().len(), 1);
let elsewhere = ModifyEntries { group: Identifier::new("minecraft", "food").unwrap(), added: RefCell::new(Vec::new()) };
ItemGroupEvents::MODIFY_ENTRIES_ALL.invoke(&elsewhere);
assert!(elsewhere.added.borrow().is_empty());
```
