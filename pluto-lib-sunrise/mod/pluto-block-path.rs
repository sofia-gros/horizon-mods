// Native path-node query with the same borrowed BlockView contract as other
// block callbacks. No global state-id table or guessed unloaded state is used.
use super::*;
use sunrise_world::level::pluto_block_access::{self as transport, LevelView, StateInfo};

struct View<'a, 'w>(&'a MobWorld<'w>);
impl View<'_, '_> {
    fn faces(&self, pos: [i32; 3]) -> u8 {
        let state = self.0.world.block(pos[0], pos[1], pos[2]);
        self.0.collision.cached_shapes().cached_shape(state).ok().flatten()
            .and_then(sunrise_world::level::data::pluto_block_view_shapes::full_faces).unwrap_or(0)
    }
}
impl LevelView for View<'_, '_> {
    fn state(&self, pos: [i32; 3]) -> Option<StateInfo> {
        if !self.0.world.loaded(pos[0], pos[2]) { return None; }
        let state = self.0.registry.state(self.0.world.block(pos[0], pos[1], pos[2]))?;
        Some(StateInfo { block: state.name.to_string(), properties: state.properties.iter()
            .map(|(key, value)| (key.to_string(), value.to_string())).collect() })
    }
    fn is_air(&self, pos: [i32; 3]) -> bool {
        self.0.facts.is_air(self.0.world.block(pos[0], pos[1], pos[2]))
    }
    fn replaceable(&self, pos: [i32; 3]) -> bool {
        self.0.facts.get(self.0.world.block(pos[0], pos[1], pos[2])).replaceable
    }
    fn face_sturdy(&self, pos: [i32; 3], face: u8) -> bool { face < 6 && self.faces(pos) & (1 << face) != 0 }
    fn in_tag(&self, pos: [i32; 3], tag: &str) -> bool {
        self.0.block_tags.get(tag).is_some_and(|blocks| blocks.contains(self.0.block_name(pos[0], pos[1], pos[2])))
    }
    fn brightness(&self, pos: [i32; 3]) -> u8 {
        let light = self.0.world.light(pos[0], pos[1], pos[2]);
        (i32::from(light >> 4)-self.0.sky_darken()).max(i32::from(light & 15)).clamp(0, 15) as u8
    }
    fn light_passes_up(&self, pos: [i32; 3]) -> bool {
        let Some(y) = pos[1].checked_add(1) else { return false };
        let above = [pos[0], y, pos[2]];
        let from = self.0.facts.get(self.0.world.block(pos[0], pos[1], pos[2]));
        let to = self.0.facts.get(self.0.world.block(above[0], above[1], above[2]));
        let blocked = from.can_occlude && from.shape_occlusion && self.faces(pos) & (1 << 1) != 0
            || to.can_occlude && to.shape_occlusion && self.faces(above) & 1 != 0;
        let attenuation = if blocked { 16 } else { i32::from(to.dampening) };
        attenuation < 15
    }
    fn is_water(&self, pos: [i32; 3]) -> bool {
        self.state(pos).is_some_and(|state| state.block == "minecraft:water"
            || state.properties.get("waterlogged").is_some_and(|value| value == "true"))
    }
    fn fluid_full(&self, pos: [i32; 3]) -> bool {
        self.state(pos).is_some_and(|state| state.properties.get("waterlogged").is_some_and(|value| value == "true")
            || matches!(state.block.as_str(), "minecraft:water" | "minecraft:lava")
                && state.properties.get("level").is_some_and(|value| value == "0"))
    }
    fn area_loaded(&self, pos: [i32; 3], radius: i32) -> bool {
        if radius < 0 { return false; }
        [-radius, radius].into_iter().all(|dx| [-radius, radius].into_iter().all(|dz| {
            pos[0].checked_add(dx).zip(pos[2].checked_add(dz))
                .is_some_and(|(x, z)| self.0.world.loaded(x, z))
        }))
    }
    fn full_block(&self, pos: [i32; 3]) -> bool { self.faces(pos) & 0x3f == 0x3f }
    fn build_height(&self) -> [i32; 2] { [self.0.world.bottom_y(), self.0.world.top_y()] }
}

pub(super) fn classify(world: &MobWorld<'_>, pos: [i32; 3]) -> Option<PathType> {
    let view = View(world);
    let state = view.state(pos)?;
    let name = transport::dispatch_path_node(&view, &state, pos, None)?;
    // Do not allow a malformed transport result to become an implicit Blocked.
    let answer = PathType::from_name(&name);
    if name != "BLOCKED" && answer == PathType::Blocked { return None; }
    Some(answer)
}
