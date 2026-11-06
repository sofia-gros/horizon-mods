//! Explicit pair staging. Native Menu consumption and automatic hooks are not executed.
extern crate self as horizon_client;
pub use horizon_items::{item_data, items};
pub use pluto_inventory_api as pluto_inventory;
pub mod crafting;
pub mod recipe_book;
pub mod advancements {
    include!("item-view.rs");
}
#[path = "products-proposal.rs"]
mod consumer;
#[path = "component-view.rs"]
pub mod pluto_component_view;
#[path = "nbt-storage.rs"]
pub mod pluto_nbt_storage;
#[path = "stack-storage.rs"]
pub mod pluto_stack_storage;
#[path = "products-reference.rs"]
mod reference;
#[path = "products-source-helpers.rs"]
mod tconstruct_source_overslime;
#[cfg(test)]
mod tests {
    use super::*;
    use crafting::pluto_products::{Preparation, Prepared};
    use horizon_world::{
        loot::PackOverlay,
        nbt::{JavaString, Tag},
    };
    use items::{ItemStack, Items};
    use pluto_inventory::advancements::predicates::{ComponentText, ComponentValue};
    use pluto_inventory::crafting::products::{
        CompoundPatch, Error, MissingCompound, Preparer, Product, Proposal, Source,
    };
    fn items() -> Items {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("pluto-products-{}-{serial}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let ids = [
            "tconstruct:pickaxe",
            "tconstruct:earth_slime_bottle",
            "tconstruct:sky_slime_bottle",
            "tconstruct:ichor_slime_bottle",
            "tconstruct:ender_slime_bottle",
            "minecraft:glass_bottle",
            "example:token",
            "example:catalyst",
        ];
        let values: serde_json::Map<String, serde_json::Value> = ids
            .into_iter()
            .map(|id| {
                (
                    id.into(),
                    serde_json::json!({"minecraft:max_stack_size":64}),
                )
            })
            .collect();
        std::fs::write(
            root.join("items.json"),
            serde_json::to_vec(&values).unwrap(),
        )
        .unwrap();
        let items = Items::catalog(&root, &PackOverlay::NONE).unwrap();
        std::fs::remove_dir_all(root).unwrap();
        items
    }
    fn tool(current: i32, cap: i32, active: bool) -> ItemStack {
        let mut stack = ItemStack::new("tconstruct:pickaxe", 1);
        let mut custom = Tag::compound([
            ("tic_overslime", Tag::Int(current)),
            ("tic_overslime_capacity", Tag::Int(cap)),
            ("tic_overslime_active", Tag::Byte(i8::from(active))),
            ("tic_overslime_overworked", Tag::Int(0)),
            (
                "unknown",
                Tag::List(vec![
                    Tag::Float(0x7fc00001),
                    Tag::String(JavaString(vec![0xd800])),
                ]),
            ),
        ]);
        if let Tag::Compound(fields) = &mut custom {
            fields.insert(JavaString(vec![0xdc00]), Tag::Long(i64::MIN));
        }
        stack
            .data_mut()
            .set_other("minecraft:custom_data", Some(custom));
        stack.data_mut().bundle_selected.0 = 7;
        stack.damage = 13;
        stack
    }
    fn bottle(id: &str) -> ItemStack {
        let mut s = ItemStack::new(id, 5);
        s.data_mut().set_other("example:extra", Some(Tag::Int(9)));
        s
    }
    fn grid(current: i32, cap: i32, active: bool) -> Vec<Option<ItemStack>> {
        let mut grid = vec![None; 16];
        grid[5] = Some(tool(current, cap, active));
        for at in [7, 13, 15] {
            grid[at] = Some(bottle("tconstruct:earth_slime_bottle"));
        }
        grid
    }
    fn compare(items: &Items, grid: Vec<Option<ItemStack>>) {
        let before = grid.clone();
        let expected = reference::plan(&grid);
        let cap = Preparation::new(&grid, 4, items).unwrap();
        let proposal = consumer::propose(&cap);
        assert_eq!(proposal.is_some(), expected.is_some());
        if let (Some(p), Some((expected, remainders))) = (proposal, expected) {
            let pair = cap
                .stage(&p)
                .unwrap()
                .release_if_unchanged(Some(&grid))
                .unwrap();
            assert!(pluto_stack_storage::same_item_snapshot(
                &expected,
                &pair.output
            ));
            assert!(pluto_stack_storage::inventory_snapshot_matches(
                &remainders,
                Some(&pair.remainders)
            ));
            assert_eq!(pair.output.data().unwrap().bundle_selected.0, 7);
            assert_eq!(
                pair.output
                    .data()
                    .unwrap()
                    .other("minecraft:custom_data")
                    .unwrap()
                    .get("unknown"),
                before[5]
                    .as_ref()
                    .unwrap()
                    .data()
                    .unwrap()
                    .other("minecraft:custom_data")
                    .unwrap()
                    .get("unknown")
            );
        }
        assert!(pluto_stack_storage::inventory_snapshot_matches(
            &before,
            Some(&grid)
        ));
    }
    #[test]
    fn actual_overslime_reference_matches_bounded_explicit_pair_staging() {
        let items = items();
        for (current, cap, active) in [
            (0, 50, true),
            (45, 50, true),
            (50, 50, true),
            (50, 50, false),
            (0, 0, true),
        ] {
            compare(&items, grid(current, cap, active));
        }
        let mut mixed = grid(0, 50, true);
        mixed[7] = Some(bottle("tconstruct:sky_slime_bottle"));
        compare(&items, mixed);
        let mut two = grid(0, 50, true);
        two[6] = Some(tool(0, 50, true));
        compare(&items, two);
        let mut stacked = grid(0, 50, true);
        stacked[5].as_mut().unwrap().count = 2;
        compare(&items, stacked);
    }
    fn simple() -> Proposal {
        Proposal {
            output: Product {
                source: Source::Input(0),
                count: 1,
                patches: vec![],
            },
            remainders: vec![None],
        }
    }
    #[test]
    fn independent_mod_can_stage_fresh_result_and_preserved_catalyst_patch() {
        let items = items();
        let mut catalyst = ItemStack::new("example:catalyst", 2);
        catalyst.data_mut().set_other(
            "example:charge",
            Some(Tag::compound([
                ("uses", Tag::Int(3)),
                ("keep", Tag::Byte(9)),
            ])),
        );
        let input = [Some(catalyst)];
        let cap = Preparation::new(&input, 1, &items).unwrap();
        let mut p = simple();
        p.output.source = Source::Fresh("example:token".into());
        p.output.patches.push(CompoundPatch {
            component: "example:origin".into(),
            missing: MissingCompound::Create,
            fields: vec![(
                ComponentText(vec![0xd800]),
                Some(ComponentValue::Double(0x7ff8000000000001)),
            )],
        });
        p.remainders[0] = Some(Product {
            source: Source::Input(0),
            count: 1,
            patches: vec![CompoundPatch {
                component: "example:charge".into(),
                missing: MissingCompound::Reject,
                fields: vec![("uses".into(), Some(ComponentValue::Int(2)))],
            }],
        });
        let pair = cap
            .stage(&p)
            .unwrap()
            .release_if_unchanged(Some(&input))
            .unwrap();
        assert_eq!(&*pair.output.item, "example:token");
        let rest = pair.remainders[0].as_ref().unwrap();
        assert_eq!(rest.count, 1);
        assert_eq!(
            rest.data()
                .unwrap()
                .other("example:charge")
                .unwrap()
                .get("uses"),
            Some(&Tag::Int(2))
        );
        assert_eq!(
            rest.data()
                .unwrap()
                .other("example:charge")
                .unwrap()
                .get("keep"),
            Some(&Tag::Byte(9))
        );
        assert_eq!(input[0].as_ref().unwrap().count, 2);
    }
    fn fail(result: Result<Prepared, Error>, expected: Error) {
        match result {
            Err(actual) => assert_eq!(actual, expected),
            Ok(_) => panic!("unexpected admitted pair; wanted {expected:?}"),
        }
    }
    #[test]
    fn invalid_or_stale_pairs_never_release_partial_values() {
        let items = items();
        let input = [Some(tool(0, 50, true))];
        let cap = Preparation::new(&input, 1, &items).unwrap();
        let mut p = simple();
        p.remainders[0] = Some(Product {
            source: Source::Fresh("missing:item".into()),
            count: 1,
            patches: vec![],
        });
        fail(cap.stage(&p), Error::UnknownItem);
        p = simple();
        p.output.count = 65;
        fail(cap.stage(&p), Error::InvalidCount);
        p = simple();
        p.output.patches.push(CompoundPatch {
            component: "missing:data".into(),
            missing: MissingCompound::Reject,
            fields: vec![],
        });
        fail(cap.stage(&p), Error::MissingCompound);
        p = simple();
        p.output.patches.push(CompoundPatch {
            component: "minecraft:custom_name".into(),
            missing: MissingCompound::Create,
            fields: vec![(
                "text".into(),
                Some(ComponentValue::String("new name".into())),
            )],
        });
        fail(cap.stage(&p), Error::UnrepresentableProduct);
        let prepared = cap.stage(&simple()).unwrap();
        let mut changed = input.clone();
        changed[0].as_mut().unwrap().data_mut().bundle_selected.0 = 8;
        assert!(matches!(
            prepared.release_if_unchanged(Some(&changed)),
            Err(Error::StaleInput)
        ));
        let prepared = cap.stage(&simple()).unwrap();
        assert!(matches!(
            prepared.release_if_unchanged(None),
            Err(Error::StaleInput)
        ));
        assert_eq!(
            input[0].as_ref().unwrap().data().unwrap().bundle_selected.0,
            7
        );
    }
    #[test]
    fn native_stack_override_and_removal_limit_finished_products() {
        let items = items();
        for (override_, removed, count, accepted) in [
            (1, false, 64, false),
            (99, false, 65, true),
            (99, true, 65, false),
            (100, false, 65, false),
            (0, false, 64, true),
        ] {
            let mut stack = ItemStack::new("example:token", 1);
            stack
                .data_mut()
                .set_other("minecraft:max_stack_size", Some(Tag::Int(override_)));
            if removed {
                stack.data_mut().set_other(
                    "!minecraft:max_stack_size",
                    Some(Tag::compound([] as [(&str, Tag); 0])),
                );
            }
            let input = [Some(stack)];
            let cap = Preparation::new(&input, 1, &items).unwrap();
            let mut p = simple();
            p.output.count = count;
            let result = cap.stage(&p);
            if accepted {
                assert_eq!(
                    result
                        .unwrap()
                        .release_if_unchanged(Some(&input))
                        .unwrap()
                        .output
                        .count,
                    count
                );
            } else {
                fail(result, Error::InvalidCount);
            }
        }
    }
    #[test]
    fn conditional_modeled_components_and_unsorted_other_are_not_guessed() {
        let items = items();
        let mut ominous = ItemStack::new("example:token", 1);
        ominous.data_mut().ominous_banner = true;
        // Setting the flag alone leaves banner_patterns empty; native decoding
        // materializes the layers. Assert rejection, then use a canonical test
        // input to reach the later conditional component checks.
        let invalid = [Some(ominous.clone())];
        fail(
            Preparation::new(&invalid, 1, &items)
                .unwrap()
                .stage(&simple()),
            Error::UnrepresentableProduct,
        );
        let (_, canonical) =
            item_data::decode_components(item_data::encode_components(&ominous).as_ref());
        *ominous.data_mut() = *canonical.unwrap();
        assert!(pluto_stack_storage::encode_item_lossless(&ominous).is_some());
        let input = [Some(ominous)];
        let cap = Preparation::new(&input, 1, &items).unwrap();
        for component in [
            "minecraft:item_name",
            "minecraft:rarity",
            "minecraft:tooltip_display",
        ] {
            let mut p = simple();
            p.output.patches.push(CompoundPatch {
                component: component.into(),
                missing: MissingCompound::Create,
                fields: vec![],
            });
            fail(cap.stage(&p), Error::UnsupportedComponent);
        }
        let mut unsorted = ItemStack::new("example:token", 1);
        unsorted.data_mut().other = vec![
            ("z:unknown".into(), Tag::Int(1)),
            ("a:unknown".into(), Tag::Int(2)),
        ];
        let input = [Some(unsorted)];
        let cap = Preparation::new(&input, 1, &items).unwrap();
        fail(cap.stage(&simple()), Error::UnrepresentableProduct);
    }
    #[test]
    fn opaque_or_ambiguous_sources_fail_without_reconstruction() {
        let items = items();
        let mut opaque = tool(0, 50, true);
        let decoded = opaque.clone();
        opaque.data_mut().wire = item_data::Wire::new(decoded, std::sync::Arc::from([1u8]));
        assert!(matches!(
            Preparation::new(&[Some(opaque.clone())], 1, &items),
            Err(Error::OpaqueInput)
        ));
        let mut parent = tool(0, 50, true);
        parent.data_mut().container.push((0, opaque));
        assert!(matches!(
            Preparation::new(&[Some(parent)], 1, &items),
            Err(Error::OpaqueInput)
        ));
        let mut duplicate = tool(0, 50, true);
        duplicate.data_mut().other.push((
            "minecraft:custom_data".into(),
            Tag::compound([("x", Tag::Int(1))]),
        ));
        let source = [Some(duplicate)];
        let cap = Preparation::new(&source, 1, &items).unwrap();
        fail(cap.stage(&simple()), Error::UnrepresentableProduct);
        let input = grid(0, 50, true);
        let cap = Preparation::new(&input, 4, &items).unwrap();
        let mut proposal = consumer::propose(&cap).unwrap();
        proposal.remainders[1] = Some(Product {
            source: Source::Fresh("example:token".into()),
            count: 1,
            patches: vec![],
        });
        fail(cap.stage(&proposal), Error::EmptyRemainderCell);
    }
}
