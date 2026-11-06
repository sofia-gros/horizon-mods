// TConstruct cloud content policy over Pluto's guarded native insertion boundary.
// Only an explicitly captured, attached player-action frame is supported here.
use crate::game::{Game, pluto_cloud_insertion};
use horizon_client::{mod_api::EntityRef, pluto_cloud::{self, Attribution, Descriptor, Effect, Label, Timing, Validated}, pluto_potions};

pub(in crate::game) use crate::game::pluto_cloud_insertion::AttachedPlayerFrame;

#[derive(Debug)]
pub(in crate::game) enum Error {
    Unsupported(pluto_cloud_insertion::Error),
    Invalid(pluto_cloud::Error),
    DurationOverflow { index: usize },
}

/// Called only from the existing held-item block-action route. Observation
/// does not authorize a new action; detached guest frames remain unsupported.
pub(in crate::game) fn capture(game: &Game<'_>) -> Result<AttachedPlayerFrame, Error> {
    pluto_cloud_insertion::attached_player_frame(game).map_err(Error::Unsupported)
}

pub(in crate::game) fn require_current(game: &Game<'_>, frame: &AttachedPlayerFrame) -> Result<(), Error> {
    pluto_cloud_insertion::require_attached_player_frame(game, frame).map_err(Error::Unsupported)
}

// Preserve this port's 4x input / 0.25 native duration pair and 600-tick lifetime.
// Source makeCloud sets radius=1, radius-on-use=-0.5 and wait=10; the native
// lingering settings supply the existing reapplication=20/duration-on-use=0.
// These are TConstruct policy, never defaults in the shared library.
fn request(at: [f64; 3], effects: Vec<(&'static str, i32, i32)>, attribution: Attribution)
    -> Result<Validated, Error> {
    if effects.is_empty() || effects.len() > pluto_cloud::MAX_EFFECTS {
        return Err(Error::Invalid(pluto_cloud::Error::EffectBudget));
    }
    let (mut red, mut green, mut blue, mut total) = (0u32, 0u32, 0u32, 0u32);
    let mut requested = Vec::with_capacity(effects.len());
    for (index, (id, duration, amplifier)) in effects.into_iter().enumerate() {
        if duration < 0 || !(0..=255).contains(&amplifier) {
            return Err(Error::Invalid(pluto_cloud::Error::InvalidEffect { index }));
        }
        let Some(fact) = pluto_potions::effect(id) else {
            return Err(Error::Invalid(pluto_cloud::Error::UnknownEffect { index }));
        };
        let duration_ticks = duration.checked_mul(4).ok_or(Error::DurationOverflow { index })?;
        // Exact native weighted RGB for the admitted <=256 effects / amp<=255
        // domain: each channel sum is at most 256*256*255, with no u32 overflow.
        let weight = amplifier as u32 + 1;
        red += weight * (fact.color_rgb >> 16 & 255);
        green += weight * (fact.color_rgb >> 8 & 255);
        blue += weight * (fact.color_rgb & 255);
        total += weight;
        requested.push(Effect { id: id.into(), duration_ticks, amplifier, visible: true, ambient: false });
    }
    let color_rgb = (red / total << 16) | (green / total << 8) | (blue / total);
    pluto_cloud::validate(Descriptor {
        position: at, radius: 1.0, radius_per_tick: -1.0 / 600.0, radius_on_use: -0.5,
        timing: Timing { age: 0, duration_ticks: 600, wait_ticks: 10,
            reapplication_ticks: 20, duration_on_use: 0, effect_duration_scale: 0.25 },
        color_rgb, label: Label::Translation("entity.minecraft.area_effect_cloud".into()),
        attribution, effects: requested,
    }).map_err(Error::Invalid)
}

/// Validation, same-frame admission, capacity and native publication are all
/// delegated. Failure is not a successful spawn and grants no fluid charge.
pub(in crate::game) fn spawn(game: &mut Game<'_>, frame: &AttachedPlayerFrame,
    at: [f64; 3], effects: Vec<(&'static str, i32, i32)>) -> Result<EntityRef, Error> {
    let request = request(at, effects, frame.attribution())?;
    pluto_cloud_insertion::insert_attached_player_frame(game, frame, request).map_err(Error::Unsupported)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_policy_preserves_duration_pair_order_and_flags() {
        let owner = Attribution::Player { guest_connection: Some(7) };
        let value = request([1.25, 64.0, -2.0], vec![("speed", 80, 2), ("poison", 20, 0)], owner).unwrap();
        let d = value.descriptor();
        assert_eq!(d.attribution, owner);
        assert_eq!(d.position, [1.25,64.0,-2.0]);
        assert_eq!((d.radius,d.radius_on_use,d.radius_per_tick),(1.0,-0.5,-1.0/600.0));
        assert_eq!(d.timing, Timing { age:0,duration_ticks:600,wait_ticks:10,
            reapplication_ticks:20,duration_on_use:0,effect_duration_scale:0.25 });
        assert_eq!(value.effects().iter().map(|e|(e.raw_id,e.duration_ticks,e.amplifier,e.visible,e.ambient)).collect::<Vec<_>>(),
            [("speed",320,2,true,false),("poison",80,0,true,false)]);
    }

    #[test]
    fn weighted_color_matches_real_native_function_at_admitted_edges() {
        for effects in [vec![("speed",11,0)],vec![("poison",32,0),("speed",75,2)],
            vec![("poison",11,255);pluto_cloud::MAX_EFFECTS]] {
            let expected=horizon_client_core::potions::effects_color(effects.iter().map(|(id,_,amp)|(*id,*amp)));
            let value=request([0.0;3],effects,Attribution::Player { guest_connection:Some(4) }).unwrap();
            assert_eq!(value.descriptor().color_rgb,expected);
        }
    }

    #[test]
    fn overflow_unknown_budget_and_invalid_values_reject_before_publication() {
        let owner=Attribution::Player { guest_connection:Some(4) };
        assert!(matches!(request([0.0;3],vec![("speed",i32::MAX,0)],owner),Err(Error::DurationOverflow { index:0 })));
        for effects in [vec![],vec![("missing:unknown",11,0)],vec![("speed",11,-1)],vec![("speed",11,256)],
            vec![("speed",-1,0)],vec![("speed",11,0);pluto_cloud::MAX_EFFECTS+1]] {
            assert!(request([0.0;3],effects,owner).is_err());
        }
        assert!(request([f64::NAN,0.0,0.0],vec![("speed",11,0)],owner).is_err());
        assert_eq!(request([0.0;3],vec![("speed",i32::MAX/4,0)],owner).unwrap().effects()[0].duration_ticks,(i32::MAX/4)*4);
    }
}
