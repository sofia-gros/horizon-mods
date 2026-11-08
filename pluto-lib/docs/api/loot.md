# Loot tables: change them as they load

Horizon's loot tables are JSON. PlutoLib hands the callback the table's JSON and takes back whatever the callback leaves in it.

### `LootTableContext`

A loot table being loaded. `id` is the table id such as `minecraft:blocks/stone`; `table` holds the table's JSON in a `RefCell`, so a callback edits it in place with `borrow_mut`.

```rust
use pluto_lib::{LootTableContext, LootTableEvents};
use serde_json::json;
use std::cell::RefCell;

LootTableEvents::MODIFY.register(|context: &LootTableContext| {
    if context.id == "minecraft:blocks/stone" {
        let mut table = context.table.borrow_mut();
        table["pools"] = json!([]); // stone drops nothing
    }
});
let context = LootTableContext { id: "minecraft:blocks/stone".into(), table: RefCell::new(json!({"pools": [1, 2]})) };
LootTableEvents::MODIFY.invoke(&context);
assert_eq!(context.table.into_inner(), json!({"pools": []}));
```

### `LootTableEvents`

Holder of the loot table event.

```rust
use pluto_lib::LootTableEvents;

LootTableEvents::MODIFY.register(|_| {});
```

### `LootTableEvents::MODIFY`

Runs for every loot table as it loads. Callbacks edit `context.table`; several callbacks apply in registration order.

```rust
use pluto_lib::{LootTableContext, LootTableEvents};
use serde_json::json;
use std::cell::RefCell;

LootTableEvents::MODIFY.register(|c| {
    c.table.borrow_mut()["random_sequence"] = json!("example:added");
});
let context = LootTableContext { id: "example:any".into(), table: RefCell::new(json!({})) };
LootTableEvents::MODIFY.invoke(&context);
assert_eq!(context.table.borrow()["random_sequence"], "example:added");
```
