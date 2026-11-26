// Transport for owner-resolved advancement tag snapshots: the submitting
// library installs a source, and the advancement tick applies the snapshots
// it returns to the current world's tag store (see
// pluto_advancement_tags::drive). No world state lives here.
use crate::pluto_inventory::advancements::tags::TagSnapshot;
use std::sync::OnceLock;

/// One owner's resolved item-tag snapshot, owned for transport from the
/// submitting library to the native sink. Neither the owner nor the tags
/// are interpreted here.
pub struct OwnedSnapshot {
    pub owner: String,
    pub tags: TagSnapshot,
}

/// The installed snapshot transport: a cheap generation probe plus the
/// snapshot pull. The generation is the submitting library's publication
/// counter; the driver compares it against the generation last applied to
/// the current world's store, so the pull runs only when publications
/// changed or a fresh store has not been driven yet.
pub struct TagSource {
    pub generation: fn() -> u64,
    pub snapshots: fn() -> Vec<OwnedSnapshot>,
}

static TAG_SOURCE: OnceLock<TagSource> = OnceLock::new();

/// Install the process-lifetime source of owner tag snapshots. Installing
/// this native package alone publishes nothing: without a source the
/// driver stays inert and every tag remains native.
pub fn set_tag_source(source: TagSource) -> Result<(), String> {
    TAG_SOURCE
        .set(source)
        .map_err(|_| "advancement tag source already installed".into())
}

/// The submitting library's current publication generation; 0 when no
/// source is installed or nothing was ever published.
pub(crate) fn generation() -> u64 {
    TAG_SOURCE.get().map_or(0, |source| (source.generation)())
}

/// Pull every owner's current snapshot. The driver calls this only when
/// the generation moved or the current store was never driven.
pub(crate) fn take_snapshots() -> Vec<OwnedSnapshot> {
    TAG_SOURCE.get().map_or_else(Vec::new, |source| (source.snapshots)())
}
