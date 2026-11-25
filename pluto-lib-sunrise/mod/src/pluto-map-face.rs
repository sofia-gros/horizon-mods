// The flush of the map faces a mod's drawing queued (pluto-map-face-api): drawn here, where
// the game is at hand, at the end of the HUD's draw. A face is the map's own texture — the
// game's map texture for the id the stack carries, uploaded by the game when it has changed —
// blitted into the queued rect through the batch, so it lands over the HUD elements the mod
// drew under it (a minimap's panel). ModifierClientEvents' map rendering, in the same order:
// the paper background behind the face (the checkerboard when the map has data, the plain sheet
// when it does not), the face, then the map's decorations — all of them, the player arrow
// included, because the minimap is not an item frame (the game's own map screen draws only the
// frame-shown ones). The decorations ride the batch here, not the game's 3D map pipeline.
use super::Game;
use sunrise_client::hud::HudBatch;

impl Game<'_> {
    pub(crate) fn draw_queued_map_faces(&mut self, batch: &mut HudBatch) {
        for face in sunrise_client::game::pluto_map_face::take_queued() {
            let Some(id) = crate::game::maps::map_id(&face.stack) else { continue; };
            let face_texture = self.map_texture(id);
            let scale = face.rect[2] / 128.0;
            let x = face.rect[0];
            let y = face.rect[1];
            // The paper panel behind the face, a 7-pixel border around it (142 a side): the
            // checkerboard when the map has data, the plain sheet when it does not. Drawn even
            // when the face texture is missing, as the original's background always draws.
            let background = if face_texture.is_some() {
                "map/map_background_checkerboard"
            } else {
                "map/map_background"
            };
            if let Some(r) = self.gui.atlas.sprites.get(background).copied() {
                batch.rect_uv(x - 7.0 * scale, y - 7.0 * scale, 142.0 * scale, 142.0 * scale, r, 0xFFFF_FFFF);
            }
            let Some(atlas) = face_texture else { continue; };
            batch.rect_uv(face.rect[0], face.rect[1], face.rect[2], face.rect[3], atlas, 0xFFFF_FFFF);
            self.draw_minimap_decorations(batch, id, x, y, scale);
        }
    }

    /// The map's decorations over its image at `(x, y)` drawn `scale` GUI pixels per map pixel:
    /// the game's `draw_map_decorations`, with the frame filter dropped (a minimap is not a
    /// frame, so the player arrow and the markers draw too), then a native mod's own icons.
    fn draw_minimap_decorations(&self, batch: &mut HudBatch, id: i32, x: f32, y: f32, scale: f32) {
        let Some(map) = self.maps.data.get(&id) else {
            return;
        };
        for d in &map.decorations {
            let Some(s) = self.gui.atlas.sprites.get(d.sprite).copied() else {
                continue;
            };
            batch.quad(
                decoration_corners(d, x, y, scale),
                [[s.u0, s.v0], [s.u1, s.v0], [s.u1, s.v1], [s.u0, s.v1]],
                u32::MAX,
            );
            if let Some(name) = &d.name {
                let text = sunrise_client::text::Text::literal(name);
                let f = self.gui.atlas.font.width_text(&text);
                if f > 0.0 {
                    let f1 = (25.0 / f).clamp(0.0, 6.0 / 9.0);
                    let mut temp = HudBatch::new([1, 1]);
                    temp.name_tag(&self.gui.atlas, &text, 0.0, 0xFFFF_FFFF, 0x8000_0000);
                    batch.patches.extend(temp.patches);
                    let tm = glam::Mat4::from_translation(glam::Vec3::new(
                        x + (f32::from(d.x) / 2.0 + 64.0) * scale,
                        y + (f32::from(d.y) / 2.0 + 64.0 + 4.0) * scale,
                        0.0,
                    )) * glam::Mat4::from_scale(glam::Vec3::splat(scale * f1));
                    for quad in temp.vertices.as_chunks::<6>().0 {
                        let corners = [quad[0], quad[1], quad[2], quad[5]].map(|v| {
                            let p = tm.transform_point3(glam::Vec3::new(v.pos[0], v.pos[1], 0.0));
                            [p.x, p.y]
                        });
                        let uvs = [quad[0].uv, quad[1].uv, quad[2].uv, quad[5].uv];
                        batch.quad(corners, uvs, quad[0].color);
                    }
                }
            }
        }
        // A native mod's own icons (`native_map_decorations`).
        for d in self.mod_map_decorations(id, false) {
            let Some(s) = self.gui.atlas.sprites.get(d.sprite.as_str()).copied() else {
                continue;
            };
            batch.quad(
                pose_corners(
                    crate::game::maps::decoration_pose_at(d.x, d.y, d.rot, 0.0),
                    x,
                    y,
                    scale,
                ),
                [[s.u0, s.v0], [s.u1, s.v0], [s.u1, s.v1], [s.u0, s.v1]],
                u32::MAX,
            );
        }
    }
}

/// Decoration `d`'s corners on screen, for a map drawn at `(x, y)` with
/// `scale` GUI pixels per map pixel: (-1, 1), (1, 1), (1, -1), (-1, -1)
/// under `MapRenderer.render`'s pose.
fn decoration_corners(d: &crate::game::maps::Decoration, x: f32, y: f32, scale: f32) -> [[f32; 2]; 4] {
    pose_corners(crate::game::maps::decoration_pose(d, 0.0), x, y, scale)
}

/// The corners of a decoration drawn under `pose` (`decoration_pose`), like
/// [`decoration_corners`].
fn pose_corners(pose: glam::Mat4, x: f32, y: f32, scale: f32) -> [[f32; 2]; 4] {
    let m = glam::Mat4::from_translation(glam::Vec3::new(x, y, 0.0))
        * glam::Mat4::from_scale(glam::Vec3::new(scale, scale, 1.0))
        * pose;
    [(-1.0, 1.0), (1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)].map(|(cx, cy)| {
        let p = m.transform_point3(glam::Vec3::new(cx, cy, 0.0));
        [p.x, p.y]
    })
}
