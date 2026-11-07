include!("pluto_storage_fixture.rs");
// Real PlutoLib graph engine under the native library export name.
#[path = "../../../pluto-lib/mixins/pluto_native_creative/tag-graph.rs"]
pub mod pluto_tag_graph;
// Integration fixture: compile against a host staged with this mod and its native loot hook.
use horizon_world::loot::LootStack;
#[path = "../../../pluto-lib/mixins/pluto_native_creative/stack-fluid.rs"]
mod pluto_stack_fluid;
#[path = "../../mixins/tconstruct/source-fluid-display.rs"]
mod tconstruct_source_fluid_display;
#[path = "../../mixins/tconstruct/source-melter-state.rs"]
mod tconstruct_source_melter_state;
#[path = "../../mixins/tconstruct/source-smeltery-tank.rs"]
mod tconstruct_source_smeltery_tank;
mod tconstruct_source_smeltery_fuel {
    pub(super) fn potion_fluid_name(tag: Option<&horizon_world::nbt::Tag>) -> Option<String> {
        match tag?.get("Potion")? {
            horizon_world::nbt::Tag::String(id)
                if [
                    "minecraft:swiftness",
                    "minecraft:long_swiftness",
                    "minecraft:strong_swiftness",
                ]
                .contains(&id.to_string_lossy().as_str()) =>
            {
                Some("item.minecraft.potion.effect.swiftness".into())
            }
            _ => None,
        }
    }
    pub(super) fn item_capacity(id: &str) -> Option<u32> {
        match id {
            "tconstruct:seared_fuel_tank"
            | "tconstruct:seared_fuel_gauge"
            | "tconstruct:scorched_fuel_tank"
            | "tconstruct:scorched_fuel_gauge"
            | "tconstruct:seared_casting_tank" => Some(4000),
            "tconstruct:seared_ingot_tank"
            | "tconstruct:seared_ingot_gauge"
            | "tconstruct:scorched_ingot_tank"
            | "tconstruct:scorched_ingot_gauge" => Some(4320),
            "tconstruct:seared_lantern" | "tconstruct:scorched_lantern" => Some(50),
            "tconstruct:seared_fluid_cannon" => Some(2000),
            _ => None,
        }
    }
}
#[path = "../../mixins/tconstruct/source-blazing-barter.rs"]
mod source_blazing_barter;
#[path = "../../mixins/tconstruct/source-fluid-container.rs"]
mod tconstruct_source_fluid_container;
#[path = "../../mixins/tconstruct/source-melting-lookup.rs"]
mod tconstruct_source_melting_lookup;
#[path = "../../mixins/tconstruct/source-melting-recipes.rs"]
mod tconstruct_source_melting_recipes;
#[path = "../../mixins/tconstruct/source-tags.rs"]
mod tconstruct_source_tags;
#[cfg(test)]
mod barter_integration {
    use super::*;
    use horizon_core::random::LegacyRandom;
    use horizon_items::items::{ItemStack, Items};
    use horizon_world::loot::PackOverlay;
    #[test]
    fn real_items_engine_rolls_filled_lantern_and_drains_one() {
        let mut table = source_blazing_barter::testing_stock_table();
        assert!(source_blazing_barter::inject(
            "minecraft:gameplay/piglin_bartering",
            &mut table
        ));
        let items = Items::tags_only(std::path::Path::new("/tmp"), &PackOverlay::NONE).unwrap();
        let mut random = LegacyRandom::new(137);
        let mut found = None;
        for _ in 0..100 {
            let result = items.inline_loot_table(&table, &mut random);
            if let Some(stack) = result
                .into_iter()
                .find(|s| &*s.item == "tconstruct:scorched_lantern")
            {
                found = Some(stack);
                break;
            }
        }
        let stack: ItemStack = found.expect("no injected lantern");
        assert!((1..=4).contains(&stack.count));
        let custom = stack
            .data()
            .unwrap()
            .other("minecraft:custom_data")
            .unwrap();
        assert!(matches!(
            custom.get("tconstruct:tank").and_then(|t| t.get("amount")),
            Some(horizon_world::nbt::Tag::Int(50))
        ));
        let mut fuel_tank = tconstruct_source_melter_state::Tank::new(4000);
        let mut fuel_slots = [Some(stack.clone()), None];
        assert!(tconstruct_source_fluid_container::transfer_single(
            &mut fuel_tank,
            &mut fuel_slots,
            tconstruct_source_fluid_container::Direction::EmptyItem,
            &|_| true,
            &|_| true
        ));
        assert_eq!(fuel_tank.fluid.as_deref(), Some("tconstruct:blazing_blood"));
        assert_eq!(fuel_tank.amount, 50);
        let recipes=std::collections::BTreeMap::from([("fuel".into(),br#"{"type":"tconstruct:melting_fuel","duration":150,"fluid":{"amount":50,"fluid":"tconstruct:blazing_blood"},"rate":15,"temperature":1500}"#.to_vec())]);
        let tags = std::collections::HashMap::new();
        let known_items = std::collections::HashSet::new();
        let known_fluids = std::collections::HashSet::from(["tconstruct:blazing_blood".into()]);
        let catalog = tconstruct_source_melting_lookup::Catalog {
            recipes: &recipes,
            item_tags: &tags,
            fluid_tags: &tags,
            known_items: &known_items,
            known_fluids: &known_fluids,
        };
        let fuel = catalog
            .fluid_fuel("tconstruct:blazing_blood", fuel_tank.amount, &|_| true)
            .unwrap();
        assert_eq!(
            (fuel.temperature, fuel.duration, fuel.rate),
            (1500, 150, 15)
        );
        let mut tank = tconstruct_source_smeltery_tank::SmelteryTank::new(1000);
        let mut slots = [Some(stack), None];
        assert!(tconstruct_source_fluid_container::transfer(
            &mut tank,
            &mut slots,
            tconstruct_source_fluid_container::Direction::EmptyItem,
            &|_| true,
            &|_| true
        ));
        assert_eq!(tank.amount("tconstruct:blazing_blood"), 50);
        assert_eq!(
            slots[1].as_ref().unwrap().item.as_ref(),
            "tconstruct:scorched_lantern"
        );
        assert!(
            slots[1]
                .as_ref()
                .unwrap()
                .data()
                .and_then(|d| d.other("minecraft:custom_data"))
                .and_then(|d| d.get("tconstruct:tank"))
                .is_none()
        );
    }
    #[test]
    fn pack_proof_controls_real_shared_tables_and_cache() {
        use std::fs;
        let root =
            std::env::temp_dir().join(format!("blazing-barter-proof-{}", std::process::id()));
        let table_dir = root.join("data/minecraft/loot_table/gameplay");
        fs::create_dir_all(&table_dir).unwrap();
        let table = source_blazing_barter::testing_stock_table();
        fs::write(
            table_dir.join("piglin_bartering.json"),
            serde_json::to_vec(&table).unwrap(),
        )
        .unwrap();
        let id = "minecraft:gameplay/piglin_bartering";
        let files = |pack: bool, native: bool| {
            vec![
                (
                    "loot_table/!tconstruct:blazing_barter_pack_state".to_owned(),
                    vec![pack.to_string().into_bytes()],
                ),
                (
                    "loot_table/!tconstruct:blazing_barter_native_state".to_owned(),
                    vec![native.to_string().into_bytes()],
                ),
            ]
        };
        let yes_files = files(true, true);
        let yes =
            horizon_world::loot::shared_tables(&root, &PackOverlay { files: &yes_files }).unwrap();
        assert_eq!(yes[id]["pools"][0]["entries"].as_array().unwrap().len(), 20);
        assert!(!yes.contains_key("!tconstruct:blazing_barter_pack_state"));
        assert!(!yes.contains_key("!tconstruct:blazing_barter_native_state"));
        let no_files = files(false, true);
        let no =
            horizon_world::loot::shared_tables(&root, &PackOverlay { files: &no_files }).unwrap();
        assert_eq!(no[id], table);
        let no_native_files = files(true, false);
        let no_native = horizon_world::loot::shared_tables(
            &root,
            &PackOverlay {
                files: &no_native_files,
            },
        )
        .unwrap();
        assert_eq!(no_native[id], table);
        let absent = horizon_world::loot::shared_tables(&root, &PackOverlay::NONE).unwrap();
        assert_eq!(absent[id], table);
        fs::remove_dir_all(root).unwrap();
    }
}
