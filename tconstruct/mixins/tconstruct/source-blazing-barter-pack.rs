// Per-load proof for the fixed Mantle injector. The invalid namespace keeps
// this transport key separate from real loot table resource locations.
pub(super) const SENTINEL_ID: &str = "!tconstruct:blazing_barter_pack_state";
pub(super) const SENTINEL_PATH: &str = "loot_table/!tconstruct:blazing_barter_pack_state";
pub(super) const NATIVE_SENTINEL_ID: &str = "!tconstruct:blazing_barter_native_state";
pub(super) const NATIVE_SENTINEL_PATH: &str = "loot_table/!tconstruct:blazing_barter_native_state";
pub(super) const INJECTOR_PATH: &str =
    "data/tconstruct/mantle/loot_injectors/piglin_bartering.json";
pub(super) const TABLE_PATH: &str = "data/minecraft/loot_table/gameplay/piglin_bartering.json";
pub(super) const TABLE_KEY: &str = "loot_table/minecraft:gameplay/piglin_bartering";
const SOURCE_SHA256: &str = "1775e5aa49c6ac07d1484e8b5829b894fdec77901a5623392aee3fbf69caa9a5";

pub(super) type Files = Vec<(String, Vec<Vec<u8>>)>;

pub(super) fn canonical(bytes: &[u8]) -> bool {
    use sha2::{Digest, Sha256};
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return false;
    };
    serde_json::to_vec(&value)
        .ok()
        .is_some_and(|bytes| format!("{:x}", Sha256::digest(bytes)) == SOURCE_SHA256)
}

// Absence is not evidence of the pinned injector. It can also mean that the
// resource was omitted, removed, or its pack was disabled. Both an absent and
// an unreadable definition fail closed; only the highest definition controls.
pub(super) fn enabled(highest: Option<Option<Vec<u8>>>) -> bool {
    match highest {
        Some(Some(bytes)) => canonical(&bytes),
        None | Some(None) => false,
    }
}

pub(super) fn observe(highest: &mut Option<Option<Vec<u8>>>, listed: bool, bytes: Option<Vec<u8>>) {
    if listed || bytes.is_some() {
        *highest = Some(bytes);
    }
}

pub(super) fn stamp(mut files: Files, enabled: bool) -> Files {
    // A pack cannot manufacture either proof, including the separate proof
    // that native contributions contained no injector or barter replacement.
    files.retain(|(path, _)| path != SENTINEL_PATH && path != NATIVE_SENTINEL_PATH);
    files.push((
        SENTINEL_PATH.into(),
        vec![if enabled {
            b"true".to_vec()
        } else {
            b"false".to_vec()
        }],
    ));
    files
}

pub(super) fn gate(files: &[(String, Vec<Vec<u8>>)]) -> bool {
    let proven = |key: &str| {
        let mut matches = files.iter().filter(|(path, _)| path == key);
        let enabled = matches
            .next()
            .is_some_and(|(_, layers)| layers.last().is_some_and(|bytes| bytes == b"true"));
        enabled && matches.next().is_none()
    };
    proven(SENTINEL_PATH)
        && proven(NATIVE_SENTINEL_PATH)
        && !files
            .iter()
            .any(|(path, _)| path.contains("loot_injectors") || path == TABLE_KEY)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_and_unreadable_definitions_fail_closed() {
        assert!(!enabled(None));
        assert!(!enabled(Some(None)));
        assert!(!enabled(Some(Some(b"{}".to_vec()))));
        assert!(!enabled(Some(Some(b"{broken".to_vec()))));
        assert!(!enabled(Some(Some(
            br#"{"conditions":[],"pools":[]}"#.to_vec()
        ))));
    }

    #[test]
    fn highest_layer_wins_without_sticky_state() {
        let mut highest = None;
        observe(&mut highest, false, None);
        assert!(!enabled(highest.clone()));
        observe(&mut highest, true, Some(b"{}".to_vec()));
        observe(&mut highest, false, None);
        assert!(!enabled(highest));
        assert!(!enabled(None));
    }

    #[test]
    fn sentinel_is_explicit_unique_and_changes_cache_input() {
        assert!(!gate(&[]));
        let native = (NATIVE_SENTINEL_PATH.into(), vec![b"true".to_vec()]);
        let mut yes = stamp(Vec::new(), true);
        yes.push(native.clone());
        let mut no = stamp(yes.clone(), false);
        no.push(native.clone());
        assert!(gate(&yes));
        assert!(!gate(&no));
        assert_ne!(yes, no);
        let mut restamped = stamp(yes.clone(), true);
        restamped.push(native);
        assert_eq!(restamped, yes);
        let mut duplicated = yes.clone();
        duplicated.extend(yes);
        assert!(!gate(&duplicated));
    }
}
