// The logical height of each loaded level (DimensionType.logicalHeight), published by the
// game's own height read so the API side can clamp by the level's logical top: a random
// teleport's ceiling is the level's lowest y plus its logical height, less one, which may
// sit below the build limit (the nether's logical height is 128 of its 256). The build
// range has a Host call (API 381, level_height); the logical height has none, so it is
// read from the level's shape where the game itself answers the height.
use std::collections::BTreeMap;
use std::sync::RwLock;

static LOGICAL_HEIGHTS: RwLock<BTreeMap<String, i32>> = RwLock::new(BTreeMap::new());

pub fn publish_logical_height(dimension: &str, logical_height: i32) {
    if let Ok(mut heights) = LOGICAL_HEIGHTS.write() {
        heights.insert(dimension.to_owned(), logical_height);
    }
}

pub fn logical_height(dimension: &str) -> Option<i32> {
    LOGICAL_HEIGHTS.read().ok()?.get(dimension).copied()
}
