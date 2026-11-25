// Exact official RecipeIndex/Known declarations and lookup methods are prepended
// by the fixture runner. The outer publication example is a model, not Game.
#[cfg(test)]
mod tests {
    use super::*;
    use pluto_inventory_api::recipe_book::index::{Allocation, Definition, Error, Patch, Snapshot};
    use std::collections::BTreeMap;

    fn native() -> RecipeIndex {
        RecipeIndex {
            recipes: BTreeMap::from([
                (
                    "other:a".into(),
                    Known {
                        special: false,
                        displays: 2,
                        first_display: 1,
                        station: "minecraft:furnace",
                        notify: false,
                        result: Some("other:output".into()),
                    },
                ),
                (
                    "other:b".into(),
                    Known {
                        special: true,
                        displays: 0,
                        first_display: 8,
                        station: "custom:untouched",
                        notify: false,
                        result: None,
                    },
                ),
            ]),
        }
    }
    fn snapshot(index: &mut RecipeIndex) -> Snapshot {
        pluto_index_patch::with_index(index, |api| api.snapshot().unwrap())
    }
    fn patch(index: &mut RecipeIndex) -> Patch {
        Patch {
            expected: snapshot(index),
            remove: Default::default(),
            insert: BTreeMap::from([(
                "example:new".into(),
                Definition {
                    special: false,
                    displays: 1,
                    station: "minecraft:crafting_table".into(),
                    notify: true,
                    result: Some("example:result".into()),
                },
            )]),
            allocation: Allocation::FirstGap,
        }
    }
    #[test]
    fn real_native_lookups_read_successful_records_and_unrelated_fields_survive() {
        let mut index = native();
        let p = patch(&mut index);
        let after =
            pluto_index_patch::with_index(&mut index, |api| api.compare_and_apply(&p)).unwrap();
        assert_eq!(index.display_ids("example:new"), 0..1);
        assert_eq!(index.recipe_of_display(0), Some("example:new"));
        assert_eq!(index.result("example:new"), Some("example:result"));
        assert_eq!(
            index.toast_station("example:new"),
            Some("minecraft:crafting_table")
        );
        assert!(!index.special("example:new"));
        assert_eq!(after["other:a"], p.expected["other:a"]);
        assert_eq!(after["other:b"], p.expected["other:b"]);
        assert_eq!(index.recipes["other:b"].station, "custom:untouched");
    }
    #[test]
    fn unsupported_station_and_stale_fields_leave_native_index_unchanged() {
        let mut index = native();
        let mut p = patch(&mut index);
        let before = snapshot(&mut index);
        p.insert.get_mut("example:new").unwrap().station = "custom:new".into();
        assert_eq!(
            pluto_index_patch::with_index(&mut index, |a| a.compare_and_apply(&p)),
            Err(Error::UnsupportedStation("custom:new".into()))
        );
        assert_eq!(snapshot(&mut index), before);
        p = patch(&mut index);
        index.recipes.get_mut("other:a").unwrap().notify = true;
        let changed = snapshot(&mut index);
        assert_eq!(
            pluto_index_patch::with_index(&mut index, |a| a.compare_and_apply(&p)),
            Err(Error::StaleSnapshot)
        );
        assert_eq!(snapshot(&mut index), changed);
    }
    #[test]
    fn unchanged_index_clone_is_not_a_durable_owner_token() {
        let mut index = native();
        let p = patch(&mut index);
        let mut replacement = index.clone();
        assert!(
            pluto_index_patch::with_index(&mut replacement, |a| a.compare_and_apply(&p)).is_ok()
        );
        assert!(!index.contains("example:new"));
        assert!(replacement.contains("example:new"));
    }
    #[test]
    fn caller_stages_all_fields_before_no_fail_publication() {
        // Modeled Vec/meta payloads stand in for cloned crafting/meta fields.
        // The index and adapter are the exact native structures/source.
        struct Tables {
            crafting: Vec<String>,
            meta: BTreeMap<String, String>,
            index: RecipeIndex,
        }
        fn publish(t: &mut Tables, p: &Patch) -> Result<(), Error> {
            let mut crafting = t.crafting.clone();
            crafting.push("example:new".into());
            let mut meta = t.meta.clone();
            meta.insert("example:new".into(), "group".into());
            let mut candidate = t.index.clone();
            pluto_index_patch::with_index(&mut candidate, |a| a.compare_and_apply(p))?;
            // No callbacks, errors, reload or observers between these moves.
            (t.crafting, t.meta, t.index) = (crafting, meta, candidate);
            Ok(())
        }
        let mut t = Tables {
            crafting: vec!["other:a".into()],
            meta: BTreeMap::new(),
            index: native(),
        };
        let mut p = patch(&mut t.index);
        p.insert.get_mut("example:new").unwrap().station = "bad:station".into();
        let before = snapshot(&mut t.index);
        assert!(publish(&mut t, &p).is_err());
        assert_eq!(t.crafting, ["other:a"]);
        assert!(t.meta.is_empty());
        assert_eq!(snapshot(&mut t.index), before);
        p.insert.get_mut("example:new").unwrap().station = "minecraft:crafting_table".into();
        publish(&mut t, &p).unwrap();
        assert_eq!(t.crafting, ["other:a", "example:new"]);
        assert_eq!(t.meta["example:new"], "group");
        assert!(t.index.contains("example:new"));
    }
}
