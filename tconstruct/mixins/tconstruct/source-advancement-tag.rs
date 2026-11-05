// Bound only the Tinker Tool criterion's source tag. All other host tag
// semantics remain untouched. Missing/invalid snapshots must match nothing.
const TAG: &str = "tconstruct:modifiable/multipart";

pub(super) fn members<'a>(game: &'a crate::game::Game<'_>) -> Vec<&'a str> {
    game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()
        .and_then(|snapshot| snapshot.as_ref().ok()).and_then(|tags| tags.get(TAG))
        .map(|items| items.iter().map(String::as_str).collect()).unwrap_or_default()
}

impl crate::game::Game<'_> {
    pub(crate) fn tconstruct_invalidate_advancement_inventory_index(&mut self) {
        self.adv.inventory_index = None;
    }
}
