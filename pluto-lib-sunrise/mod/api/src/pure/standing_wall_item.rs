//! StandingAndWallBlockItem declarations. Native item-binding application is
//! not connected yet; do not approximate it with a cross-block placement answer.
use std::collections::BTreeMap;
use std::sync::Mutex;
use super::base::Identifier;
use super::block_api_v1::Direction;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StandingWallBinding {
    pub item: Identifier,
    pub standing: Identifier,
    pub wall: Identifier,
    pub attachment: Direction,
}
#[derive(Default)]
struct Registry {
    frozen: bool,
    bindings: BTreeMap<String, StandingWallBinding>,
}
impl Registry {
    fn register(&mut self, binding: StandingWallBinding) -> Result<(), String> {
        if self.frozen { return Err("item placement bindings have already been collected".into()); }
        let item = binding.item.to_string();
        if self.bindings.contains_key(&item) { return Err(format!("item placement binding {item} already exists")); }
        self.bindings.insert(item, binding);
        Ok(())
    }
    fn snapshot(&mut self) -> Vec<StandingWallBinding> {
        self.frozen = true;
        self.bindings.values().cloned().collect()
    }
}
static REGISTRY: Mutex<Registry> = Mutex::new(Registry { frozen: false, bindings: BTreeMap::new() });

/// Minecraft's StandingAndWallBlockItem: an already declared item placing a
/// standing block or its wall partner. This registers placement metadata only;
/// it does not declare content, copy another item's properties, or place blocks.
/// Native application is currently an explicit stub in the support matrix.
pub struct StandingAndWallBlockItem;
impl StandingAndWallBlockItem {
    /// Records an item's two blocks and attachment direction (Down for heads,
    /// signs and torches). Skip the opposite direction during candidate choice;
    /// the attachment direction selects standing, other directions select wall.
    /// Duplicate and post-collection registrations return an
    /// error without replacing any earlier entry. IDs are checked against the
    /// actual native registries by the future adapter, not guessed here.
    pub fn register(item: Identifier, standing: Identifier, wall: Identifier, attachment: Direction) -> Result<(), String> {
        REGISTRY.lock().map_err(|_| "item placement registry poisoned")?
            .register(StandingWallBinding { item, standing, wall, attachment })
    }
}

/// Snapshot under the lock; native resolution/validation must happen after it
/// is released, against each resource snapshot, before publishing any changes.
pub(crate) fn standing_wall_bindings() -> Result<Vec<StandingWallBinding>, String> {
    Ok(REGISTRY.lock().map_err(|_| "item placement registry poisoned")?.snapshot())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn binding(item: &str, wall: &str) -> StandingWallBinding {
        StandingWallBinding { item: Identifier::parse(item).unwrap(),
            standing: Identifier::parse("example:head").unwrap(), wall: Identifier::parse(wall).unwrap(), attachment: Direction::Down }
    }
    #[test]
    fn bindings_are_owned_and_duplicates_or_late_changes_do_not_replace_them() {
        let mut registry = Registry::default();
        let original = binding("example:head", "example:wall_head");
        registry.register(original.clone()).unwrap();
        assert!(registry.register(binding("example:head", "example:wrong_wall")).is_err());
        // The original constructor does not forbid identical block arguments.
        let same = binding("example:same", "example:head");
        registry.register(same.clone()).unwrap();
        let first = registry.snapshot();
        assert_eq!(first, [original.clone(), same.clone()]);
        let mut edited_copy = first;
        edited_copy[0].attachment = Direction::Up;
        assert_eq!(registry.snapshot(), [original, same]);
        assert!(registry.register(binding("example:late", "example:other_wall")).is_err());
        assert_eq!(registry.snapshot().len(), 2);
    }
}
