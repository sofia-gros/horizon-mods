// Bounded port of TConstruct f4e77d43's four slimy foliage injectors.
// Only a verified stock table, verified original injector and enabled config
// are eligible. The pool index mapping is documented in the companion guide.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub(super) const CONFIG_KEY: &str = "loot_table/!tconstruct:slimy_loot_config";
pub(super) const PREFIX: &str = "loot_table/!tconstruct:slimy_loot_";
pub(super) const SPECS: [(&str, &str, &str, &str); 4] = [
    (
        "minecraft:chests/simple_dungeon",
        "dungeon",
        "b7e42634dee7bde63c212837976e63add9041fff1f61780a366a9900d086d049",
        "74629ee8fbb7039f1c1b4cf593a6bffbd5d92f716ada621f3360dc985d44e6ca",
    ),
    (
        "minecraft:chests/nether_bridge",
        "nether_fortress",
        "8d681cec5418b8f042af2e97beb7c91bc0e85401f6b84077d5778322a61ab773",
        "2076664ec2938e5b41855ff7c53caa7fba4bb427064fcf6ae569518938205eff",
    ),
    (
        "minecraft:chests/bastion_bridge",
        "bastion",
        "873b9842707725a4036b743fd96a7bc5e5f0bd5b7d653444c6c033c20b467ac9",
        "9d4f8f067e28e242879b54d97285cf3fcab2fd9eaa5a71f7fbdeca790b2851ca",
    ),
    (
        "minecraft:chests/end_city_treasure",
        "end_city",
        "8b02ba8e8f8bd9b536f2f3d6b68137edf8d837fcbb0a3c59daf08ddf6c8d377a",
        "e49b79c136d4e935a6d6663fb530fdeed9c08812db4cde4d169880ada07270cd",
    ),
];
pub(super) fn hash(value: &Value) -> Option<String> {
    serde_json::to_vec(value)
        .ok()
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
}
pub(super) fn canonical(bytes: &[u8], expected: &str) -> bool {
    serde_json::from_slice::<Value>(bytes)
        .ok()
        .and_then(|v| hash(&v))
        .as_deref()
        == Some(expected)
}
pub(super) fn proven(files: &[(String, Vec<Vec<u8>>)], key: &str) -> bool {
    let mut values = files.iter().filter(|(path, _)| path == key);
    let found = values
        .next()
        .is_some_and(|(_, layers)| layers.last().is_some_and(|v| v == b"true"));
    found && values.next().is_none()
}
fn entry(name: &str, weight: u32, seeds: bool) -> Value {
    let mut item =
        json!({"type":"minecraft:item", "name":format!("tconstruct:{name}"), "weight":weight});
    if seeds {
        item["modifier"] = json!({"type":"minecraft:set_count", "count":{"type":"minecraft:uniform", "min":2, "max":4}});
    }
    item
}
pub(super) fn inject(id: &str, table: &mut Value, files: &[(String, Vec<Vec<u8>>)]) -> bool {
    let Some((_, key, table_hash, _)) = SPECS.iter().find(|(name, _, _, _)| *name == id) else {
        return false;
    };
    if !proven(files, CONFIG_KEY)
        || !proven(files, &format!("{PREFIX}{key}"))
        || files.iter().any(|(p, _)| p == &format!("loot_table/{id}"))
        || hash(table).as_deref() != Some(*table_hash)
    {
        return false;
    }
    let Some(pools) = table.get_mut("pools").and_then(Value::as_array_mut) else {
        return false;
    };
    let additions: Vec<(usize, Value)> = match *key {
        "dungeon" => vec![
            (0, entry("earth_slime_sapling", 3, false)),
            (0, entry("sky_slime_sapling", 7, false)),
            (1, entry("earth_slime_grass_seeds", 3, true)),
            (1, entry("sky_slime_grass_seeds", 7, true)),
        ],
        "nether_fortress" => vec![(0, entry("blood_slime_grass_seeds", 5, true))],
        "bastion" => vec![(0, entry("blood_slime_sapling", 1, false))],
        "end_city" => vec![
            (0, entry("ender_slime_grass_seeds", 5, true)),
            (0, entry("ender_slime_sapling", 3, false)),
        ],
        _ => return false,
    };
    if additions.iter().any(|(idx, _)| {
        pools
            .get(*idx)
            .and_then(|p| p.get("entries"))
            .and_then(Value::as_array)
            .is_none()
    }) {
        return false;
    }
    for (idx, item) in additions {
        pools[idx]["entries"].as_array_mut().unwrap().push(item);
    }
    true
}
