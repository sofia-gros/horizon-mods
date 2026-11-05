// Bounded source Wither Bone global modifier. Proof is rebuilt per world load.
use serde_json::Value;
pub(super) const TABLE: &str = "minecraft:entities/wither_skeleton";
pub(super) const CONFIG: &str = "loot_table/!tconstruct:wither_bone_config";
pub(super) const PACK: &str = "loot_table/!tconstruct:wither_bone_pack";
pub(super) const MARKER: &str = "!tconstruct:wither_bone_enabled";
pub(super) const SOURCE: &str = "data/tconstruct/loot_modifiers/wither_bone.json";
pub(super) const TABLE_PATH: &str = "data/minecraft/loot_table/entities/wither_skeleton.json";
pub(super) fn canonical(bytes: &[u8]) -> bool {
    use sha2::{Digest, Sha256};
    serde_json::from_slice::<Value>(bytes)
        .ok()
        .and_then(|v| serde_json::to_vec(&v).ok())
        .is_some_and(|v| {
            format!("{:x}", Sha256::digest(v))
                == "6df1481b296271aa3d6d44f05291e3552a97a37695bb9b3ef520628fe7716a56"
        })
}
pub(super) fn proven(files: &[(String, Vec<Vec<u8>>)], key: &str) -> bool {
    let mut matching = files.iter().filter(|(p, _)| p == key);
    let yes = matching
        .next()
        .is_some_and(|(_, v)| v.last().is_some_and(|v| v == b"true"));
    yes && matching.next().is_none()
}
pub(super) fn enabled(files: &[(String, Vec<Vec<u8>>)]) -> bool {
    proven(files, CONFIG)
        && proven(files, PACK)
        && !files
            .iter()
            .any(|(p, _)| p.contains("loot_modifiers") || p == &format!("loot_table/{TABLE}"))
}
