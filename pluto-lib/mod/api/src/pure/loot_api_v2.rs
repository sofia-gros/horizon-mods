//! Counterpart of Fabric's `fabric-loot-api-v2`: change loot tables as they load.
//!
//! Fabric hands the callback a `LootTable.Builder`; Horizon's loot tables are JSON, so PlutoLib hands over the table's JSON and takes back
//! whatever the callbacks leave in it.

use std::cell::RefCell;

use super::base::Event;

/// A loot table being loaded. Callbacks edit `table` in place.
pub struct LootTableContext {
    /// The table's id, such as `minecraft:blocks/stone`.
    pub id: String,
    pub table: RefCell<serde_json::Value>,
}

static MODIFY: Event<LootTableContext, ()> = Event::void();

/// Fabric: `LootTableEvents`.
pub struct LootTableEvents;

impl LootTableEvents {
    /// Fabric: `MODIFY`: runs for every loot table as it loads; edit `context.table`.
    pub const MODIFY: &'static Event<LootTableContext, ()> = &MODIFY;
}

/// Runs the `MODIFY` callbacks over `table` and returns the (possibly edited) table.
pub(crate) fn modify(id: &str, table: serde_json::Value) -> serde_json::Value {
    let context = LootTableContext { id: id.to_owned(), table: RefCell::new(table) };
    MODIFY.invoke(&context);
    context.table.into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_callback_edit_comes_back_and_only_for_the_tables_it_targets() {
        LootTableEvents::MODIFY.register(|c| {
            if c.id == "minecraft:blocks/stone" {
                c.table.borrow_mut()["pools"] = json!([{"rolls": 1}]);
            }
        });
        let edited = modify("minecraft:blocks/stone", json!({"type": "minecraft:block"}));
        assert_eq!(edited["pools"][0]["rolls"], 1);
        let untouched = modify("minecraft:blocks/dirt", json!({"type": "minecraft:block"}));
        assert!(untouched.get("pools").is_none());
    }
}
