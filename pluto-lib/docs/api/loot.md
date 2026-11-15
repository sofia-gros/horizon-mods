# Loot tables: change them as they load

Horizon's loot tables are JSON. PlutoLib hands the callback the table's JSON and takes back whatever the callback leaves in it.

## Basic Example

Change a loot table as it loads. The table is JSON; edit it in place:

```rust
use pluto_lib::LootTableEvents;
use serde_json::json;

LootTableEvents::MODIFY.register(|context| {
    if context.id == "minecraft:blocks/stone" {
        context.table.borrow_mut()["random_sequence"] = json!("example:stone");
    }
});
```

## Choosing Which Tables

The callback runs for every table, so check `context.id` first and return quickly for the others. Several callbacks edit the same table in registration order, each seeing what the earlier ones left.

> **Tip.** To add a drop, push a pool onto `table["pools"]`; to remove drops, set the pools to an empty list. The shape of the JSON is the game's own loot table format.

## Living Death Drops

`LivingEntityDropsCallback` registers ID-ordered callbacks over a borrowed death context, separately from loading loot-table JSON. A callback can inspect owned death metadata, ask the native killer's head-drop capability, claim it once, and append a reduced `StackData` without rewriting untouched native drops.

```rust
use pluto_lib::{Identifier, LivingEntityDropsCallback, StackData};
LivingEntityDropsCallback::register(Identifier::parse("example:heads").unwrap(), |drops| {
    if drops.info().entity.kind == "example:creature" && drops.can_drop_head()? && drops.claim_head_drop()? {
        drops.append_stack(StackData { item: "example:creature_head".into(), count: 1, ..Default::default() })?;
    }
    Ok(())
}).unwrap();
```

> **Use with Caution.** The registry, dispatcher, actual Tinkers consumer and host-independent gateway/state helpers are pure-tested. Native collection/counter callers are source-declared but remain blocked by native attachment review; registration does not establish a working drop. Nav clones share an unsaved spent-head flag; CombatTracker owns removed-exploder claims with an explicit wave reset. The scoped loot gate never clears powered state or changes ordinary loot properties. Retained explosion type IDs are normalized without replacing custom namespaces. The draft native adapter publishes appended stacks immediately during callbacks, before captured ordinary drops; both use the pinned host's item-owned RNG constructor for initial motion. Player loot context is optional independently of player-kill credit. It does not expose arbitrary removal/cancellation. The source-bound diagnostic binary passed an isolated Wither death check: one nether star retained negative Age, and collecting the native XP orbs raised the player's total from 0 to the pinned Wither reward of 50. This verifies extended-age initialization, not the complete despawn duration, callback ordering, head claims or combined attachment contracts. Errors stop later callbacks without rolling back earlier claims/appends, including a claim whose append fails.

## Reference

### `LivingDropInfo`

`LivingDropInfo { entity: EntityInfo, position: [f64; 3], damage_type: Option<String>, killer_type: Option<String> }` owns the victim identity/dimension, actual position, optional damage type and damage owner's type. `killer_type` is not the direct projectile and does not establish a class capability: a derived creeper may have a custom type ID.

```rust
use pluto_lib::{EntityInfo, LivingDropInfo};
let death = LivingDropInfo { entity: EntityInfo { id: 0, kind: "example:creature".into(), dimension: "minecraft:overworld".into() },
    position: [-1.0, 70.0, 2.0], damage_type: Some("minecraft:explosion".into()), killer_type: Some("minecraft:creeper".into()) };
assert_eq!(death.entity.kind, "example:creature");
assert_eq!(death.position[1], 70.0);
assert_eq!(death.damage_type.as_deref(), Some("minecraft:explosion"));
assert_eq!(death.killer_type.as_deref(), Some("minecraft:creeper"));
```

### `LivingEntityDrops`

A borrowed host interface. `info(&self) -> &LivingDropInfo` returns metadata; `can_drop_head(&self) -> Result<bool, String>` queries authoritative class/power/counter eligibility; `claim_head_drop(&mut self) -> Result<bool, String>` consumes it once; `append_stack(&mut self, StackData) -> Result<(), String>` appends without rewriting untouched native stacks. Unsupported capabilities and failed writes return errors rather than fabricated values. This memory host demonstrates the methods, not native attachment:

```rust
use pluto_lib::{EntityInfo, LivingDropInfo, LivingEntityDrops, StackData};
struct Memory { info: LivingDropInfo, spent: bool, items: Vec<StackData> }
impl LivingEntityDrops for Memory {
    fn info(&self) -> &LivingDropInfo { &self.info }
    fn can_drop_head(&self) -> Result<bool, String> { Ok(!self.spent) }
    fn claim_head_drop(&mut self) -> Result<bool, String> { Ok(!std::mem::replace(&mut self.spent, true)) }
    fn append_stack(&mut self, stack: StackData) -> Result<(), String> { self.items.push(stack); Ok(()) }
}
let mut host = Memory { info: LivingDropInfo { entity: EntityInfo { id: 1, kind: "example:creature".into(), dimension: "minecraft:overworld".into() },
    position: [0.0; 3], damage_type: None, killer_type: None }, spent: false, items: vec![] };
assert_eq!(host.info().entity.id, 1);
assert!(host.can_drop_head().unwrap());
assert!(host.claim_head_drop().unwrap());
assert!(!host.claim_head_drop().unwrap());
host.append_stack(StackData { item: "example:head".into(), count: 1, ..Default::default() }).unwrap();
assert_eq!(host.items.len(), 1);
```

### `LivingEntityDropsCallback`

Holds the ordered borrowed-context callbacks. Registration is functional; native dispatch remains a documented stub.

```rust
use pluto_lib::{Identifier, LivingEntityDropsCallback};
LivingEntityDropsCallback::register(Identifier::parse("example:observe_death").unwrap(), |_| Ok(())).unwrap();
```

### `LivingEntityDropsCallback::register`

`fn register(id: Identifier, callback: impl Fn(&mut dyn LivingEntityDrops) -> Result<(), String> + Send + Sync + 'static) -> Result<(), String>`. IDs define lexical dispatch order; duplicate IDs and a poisoned registry reject registration. Callbacks run outside its mutex from a retained callback snapshot; errors stop later callbacks without undoing earlier operations.

```rust
use pluto_lib::{Identifier, LivingEntityDropsCallback};
let id = Identifier::parse("example:unique_death_callback").unwrap();
LivingEntityDropsCallback::register(id.clone(), |_| Ok(())).unwrap();
assert!(LivingEntityDropsCallback::register(id, |_| Ok(())).is_err());
```

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
