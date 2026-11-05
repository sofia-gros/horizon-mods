// Only the tags used by the explicitly ported acquisition criteria use the
// resolved world snapshot. Unrelated host tag semantics remain untouched.
const TAGS: &[&str] = &[
    "tconstruct:modifiable/multipart",
    "tconstruct:seared_blocks",
    "tconstruct:anvil_metal",
    "forge:rods/wooden",
    "forge:ingots/copper",
    "forge:ingots/gold",
    "forge:glass/colorless",
    "tconstruct:greenheart_logs",
    "tconstruct:skyroot_logs",
    "tconstruct:bloodshroom_logs",
    "tconstruct:enderbark_logs",
    "forge:slimeball/earth",
    "forge:slimeball/sky",
    "forge:slimeball/ichor",
    "forge:slimeball/ender",
];

pub(super) fn managed(tag: &str) -> bool { TAGS.contains(&tag) }

pub(super) fn members<'a>(game: &'a crate::game::Game<'_>, tag: &str) -> Vec<&'a str> {
    game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()
        .and_then(|snapshot| snapshot.as_ref().ok()).and_then(|tags| tags.get(tag))
        .map(|items| items.iter().map(String::as_str).collect()).unwrap_or_default()
}

impl crate::game::Game<'_> {
    pub(crate) fn tconstruct_invalidate_advancement_inventory_index(&mut self) {
        // Preserve the host inventory-event gate: a tag-only reload does not
        // synthesize an inventory change or revoke earned progress.
        self.adv.inventory_index = None;
    }
}
