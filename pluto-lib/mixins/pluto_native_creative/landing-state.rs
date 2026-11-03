// Per-player transient landing data, shared by equipment and motion hooks.
use super::ChunkMap;

#[derive(Clone, Default)]
pub struct LandingState {
    pub enabled: bool,
    pub damage_multiplier: Option<f32>,
    pub rebound_y: Option<f64>,
    pub guest: bool,
    pub movement_y: f64,
}

pub fn get(world: &ChunkMap, entity: i32) -> Option<&LandingState> {
    world.__horizon_mixins_pluto_native_creative_landing_state().get(&entity)
}

pub fn get_mut(world: &mut ChunkMap, entity: i32) -> &mut LandingState {
    world.__horizon_mixins_pluto_native_creative_landing_state_mut().entry(entity).or_default()
}

pub fn remove(world: &mut ChunkMap, entity: i32) {
    world.__horizon_mixins_pluto_native_creative_landing_state_mut().remove(&entity);
}
