// The flush of the living-entity previews a mod's screen queued during its draw
// (pluto-living-preview-api): drawn here, where the game is at hand, through the game's own
// screen-preview path — the smithing screen's armour stand, whose renderer places the item
// by its own equipment slot (a worn piece on its part of the figure, anything else in the
// left hand) at full light. Only the armour stand has such a path; another kind is left
// undrawn. The stand the path draws stands as the smithing preview's does (turned to 210
// degrees, arms shown, no base plate); a queued preview's own stance fields wait for a
// general living renderer.
use crate::game::Game;
use sunrise_client_render::hud::HudBatch;

impl Game<'_> {
    pub(crate) fn draw_queued_living_previews(&self, batch: &mut HudBatch) {
        for preview in crate::game::pluto_living_preview::take_queued() {
            if preview.kind != "minecraft:armor_stand" {
                continue;
            }
            self.draw_smithing_armor_stand(batch, preview.item.as_ref(), preview.rect);
        }
    }
}
