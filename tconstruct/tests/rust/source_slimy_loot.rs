#[path = "../../mixins/tconstruct/source-slimy-loot.rs"]
mod source_slimy_loot;
#[cfg(test)]
mod tests {
    use super::source_slimy_loot as s;
    use serde_json::Value;
    #[test]
    fn pinned_injectors_and_stock_tables() {
        let root = std::path::PathBuf::from(
            std::env::var("HORIZON_RESOURCES")
                .expect("set HORIZON_RESOURCES to the prepared 26.3 root"),
        );
        for (id, name, table_hash, injector_hash) in s::SPECS {
            let injector = std::fs::read(format!(
                "tconstruct/content/data/tconstruct/mantle/loot_injectors/slimy_foliage_{name}.json"
            ))
            .unwrap();
            assert!(
                s::canonical(&injector, injector_hash),
                "{name} source mismatch"
            );
            let table_path = root.join(format!(
                "data/minecraft/loot_table/{}.json",
                id.strip_prefix("minecraft:").unwrap()
            ));
            let mut table: Value =
                serde_json::from_slice(&std::fs::read(table_path).unwrap()).unwrap();
            assert_eq!(s::hash(&table).as_deref(), Some(table_hash));
            let original = table.clone();
            let files = vec![
                (s::CONFIG_KEY.to_owned(), vec![b"true".to_vec()]),
                (format!("{}{name}", s::PREFIX), vec![b"true".to_vec()]),
            ];
            assert!(s::inject(id, &mut table, &files));
            let src: Value = serde_json::from_slice(&injector).unwrap();
            for source_pool in src["pools"].as_array().unwrap() {
                let index = if source_pool["name"] == "main" {
                    0
                } else if source_pool["name"] == "pool1" {
                    1
                } else {
                    panic!("unknown source pool")
                };
                let base = original["pools"][index]["entries"]
                    .as_array()
                    .unwrap()
                    .len();
                let actual = &table["pools"][index]["entries"].as_array().unwrap()[base..];
                let source = source_pool["entries"].as_array().unwrap();
                assert_eq!(actual.len(), source.len());
                for (actual, expected) in actual.iter().zip(source) {
                    assert_eq!(actual["name"], expected["name"]);
                    assert_eq!(
                        actual["weight"].as_u64().unwrap_or(1),
                        expected["weight"].as_u64().unwrap_or(1)
                    );
                    if expected.get("functions").is_some() {
                        assert_eq!(actual["modifier"]["type"], "minecraft:set_count");
                        assert_eq!(actual["modifier"]["count"]["min"], 2);
                        assert_eq!(actual["modifier"]["count"]["max"], 4);
                    } else {
                        assert!(actual.get("modifier").is_none());
                    }
                }
            }
            assert!(!s::inject(id, &mut table, &files));
            for (before, after) in original["pools"]
                .as_array()
                .unwrap()
                .iter()
                .zip(table["pools"].as_array().unwrap())
            {
                assert!(
                    after["entries"]
                        .as_array()
                        .unwrap()
                        .starts_with(before["entries"].as_array().unwrap())
                );
            }
            let mut off = original.clone();
            let mut disabled = files.clone();
            disabled[0].1[0] = b"false".to_vec();
            assert!(!s::inject(id, &mut off, &disabled));
            assert_eq!(off, original);
            let mut off = original.clone();
            let mut override_files = files.clone();
            override_files.push((format!("loot_table/{id}"), vec![b"{}".to_vec()]));
            assert!(!s::inject(id, &mut off, &override_files));
            assert_eq!(off, original);
            let mut off = original.clone();
            off["pools"][0]["rolls"] = serde_json::json!(999);
            let before = off.clone();
            assert!(!s::inject(id, &mut off, &files));
            assert_eq!(off, before);
        }
    }
}
