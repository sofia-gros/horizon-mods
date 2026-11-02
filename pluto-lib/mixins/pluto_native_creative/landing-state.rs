// Per-player transient landing data, shared by equipment and motion hooks.
use super::PlayerMotion;

#[derive(Clone, Default)]
pub struct LandingState {
    pub enabled: bool,
    pub damage_multiplier: Option<f32>,
    pub rebound_y: Option<f64>,
}

pub fn get(player: &PlayerMotion) -> &LandingState {
    player.__horizon_mixins_pluto_native_creative_landing_state()
}

pub fn get_mut(player: &mut PlayerMotion) -> &mut LandingState {
    player.__horizon_mixins_pluto_native_creative_landing_state_mut()
}
