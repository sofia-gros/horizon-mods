// The flush of the map faces a mod's drawing queued (pluto-map-face-api): drawn here, where
// the game is at hand, at the end of the HUD's draw. A face is the map's own texture — the
// game's map texture for the id the stack carries, uploaded by the game when it has changed —
// blitted into the queued rect through the batch, so it lands over the HUD elements the mod
// drew under it (a minimap's panel). The map's decorations ride the game's 3D map pipeline,
// not this batch, and are not drawn.
use super::Game;
use horizon_client::hud::HudBatch;

impl Game<'_> {
    pub(crate) fn draw_queued_map_faces(&mut self, batch: &mut HudBatch) {
        for face in horizon_client::game::pluto_map_face::take_queued() {
            let Some(id) = super::maps::map_id(&face.stack) else { continue; };
            let Some(atlas) = self.map_texture(id) else { continue; };
            batch.rect_uv(face.rect[0], face.rect[1], face.rect[2], face.rect[3], atlas, 0xFFFF_FFFF);
        }
    }
}
