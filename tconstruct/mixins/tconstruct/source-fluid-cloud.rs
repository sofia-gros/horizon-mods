// TConstruct cloud content policy over Pluto's guarded native insertion boundary.
// Only the source-pinned current native held-item action is supported here.
use crate::game::{Game, pluto_cloud_insertion};
use horizon_client::{mod_api::EntityRef, pluto_cloud::{self, Attribution, Descriptor, Effect, Label, Timing, Validated}, pluto_potions};

pub(in crate::game) use crate::game::pluto_cloud_insertion::NativeItemFrame;

#[derive(Debug)]
pub(in crate::game) enum Error {
    Unsupported(pluto_cloud_insertion::Error),
    Invalid(pluto_cloud::Error),
    DurationOverflow { index: usize },
}

/// Called only from the existing held-item block-action route. Observation
/// does not authorize a new action. Completed native guest and local/fake
/// frames retain their distinct native Player/guest attribution.
pub(in crate::game) fn capture(game: &Game<'_>) -> Result<NativeItemFrame, Error> {
    pluto_cloud_insertion::native_item_frame(game).map_err(Error::Unsupported)
}

pub(in crate::game) fn require_current(game: &Game<'_>, frame: &NativeItemFrame) -> Result<(), Error> {
    pluto_cloud_insertion::require_native_item_frame(game, frame).map_err(Error::Unsupported)
}

// Preserve this port's 4x input / 0.25 native duration pair and 600-tick lifetime.
// Source makeCloud sets radius=1, radius-on-use=-0.5 and wait=10; the native
// lingering settings supply the existing reapplication=20/duration-on-use=0.
// These are TConstruct policy, never defaults in the shared library.
fn request(at: [f64; 3], effects: Vec<(&'static str, i32, i32)>, attribution: Attribution)
    -> Result<Validated, Error> {
    if effects.is_empty() {
        return Err(Error::Invalid(pluto_cloud::Error::EffectBudget));
    }
    // The legacy parser already admitted/owns this complete sequence. Keep its
    // count/order/duplicates instead of silently imposing the default256 cap.
    let effect_budget = effects.len();
    let mut requested = Vec::with_capacity(effects.len());
    for (index, &(id, duration, amplifier)) in effects.iter().enumerate() {
        if duration < 0 || !(0..=255).contains(&amplifier) {
            return Err(Error::Invalid(pluto_cloud::Error::InvalidEffect { index }));
        }
        let Some(_) = pluto_potions::effect(id) else {
            return Err(Error::Invalid(pluto_cloud::Error::UnknownEffect { index }));
        };
        let duration_ticks = duration.checked_mul(4).ok_or(Error::DurationOverflow { index })?;
        requested.push(Effect { id: id.into(), duration_ticks, amplifier, visible: true, ambient: false });
    }
    pluto_cloud::validate_with_effect_budget(Descriptor {
        position: at, radius: 1.0, radius_per_tick: -1.0 / 600.0, radius_on_use: -0.5,
        timing: Timing { age: 0, duration_ticks: 600, wait_ticks: 10,
            reapplication_ticks: 20, duration_on_use: 0, effect_duration_scale: 0.25 },
        color_rgb: 0, label: Label::Translation("entity.minecraft.area_effect_cloud".into()),
        attribution, effects: requested,
    }, effect_budget).map(|value| value.with_native_effect_color()).map_err(Error::Invalid)
}

/// Validation, same-frame admission, capacity and native publication are all
/// delegated. Failure is not a successful spawn and grants no fluid charge.
pub(in crate::game) fn spawn(game: &mut Game<'_>, frame: &NativeItemFrame,
    at: [f64; 3], effects: Vec<(&'static str, i32, i32)>) -> Result<EntityRef, Error> {
    let request = request(at, effects, frame.attribution())?;
    pluto_cloud_insertion::insert_native_item_frame(game, frame, request).map_err(Error::Unsupported)
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
            vec![("poison",11,255);pluto_cloud::MAX_EFFECTS],vec![("speed",11,255);257],
            vec![("poison",20,0),("speed",11,255)].into_iter().cycle().take(512).collect()] {
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
            vec![("speed",-1,0)]] {
            assert!(request([0.0;3],effects,owner).is_err());
        }
        // Default256 remains unchanged. Explicit budget validates the entire
        // accepted sequence, including a duplicate/flag error past index255.
        let value=request([0.0;3],vec![("speed",11,0);257],owner).unwrap();
        assert_eq!(value.effects().len(),257);
        assert!(matches!(pluto_cloud::validate(value.descriptor().clone()),Err(pluto_cloud::Error::EffectBudget)));
        for budget in [0,256,usize::MAX] {
            assert!(matches!(pluto_cloud::validate_with_effect_budget(value.descriptor().clone(),budget),Err(pluto_cloud::Error::EffectBudget)));
        }
        assert_eq!(pluto_cloud::validate_with_effect_budget(value.descriptor().clone(),257).unwrap().effects(),value.effects());
        // Arbitrary high raw color stays invalid, even with an explicit budget.
        let mut raw_color=value.descriptor().clone();raw_color.color_rgb=0xffffefff;
        assert!(matches!(pluto_cloud::validate(raw_color.clone()),Err(pluto_cloud::Error::InvalidColor)));
        assert!(matches!(pluto_cloud::validate_with_effect_budget(raw_color,257),Err(pluto_cloud::Error::InvalidColor)));
        // Only the consuming native operation computes its own result from
        // validated visible effects; no raw u32 override is exposed.
        let mut visible=value.descriptor().clone();
        visible.effects.truncate(3);
        visible.effects[1].id="poison".into();visible.effects[1].visible=false;
        visible.effects[2].id="regeneration".into();
        let visible=pluto_cloud::validate(visible).unwrap().with_native_effect_color();
        let expected=horizon_client_core::potions::effects_color([("speed",0),("regeneration",0)].into_iter());
        assert_eq!(visible.descriptor().color_rgb,expected);

        let mut conflicting=value.descriptor().clone();
        conflicting.effects[256].ambient=true;
        assert!(matches!(pluto_cloud::validate_with_effect_budget(conflicting,257),Err(pluto_cloud::Error::ConflictingFlags { index:256 })));
        let mut invalid=value.descriptor().clone();
        invalid.effects[256].amplifier=256;
        assert!(matches!(pluto_cloud::validate_with_effect_budget(invalid,257),Err(pluto_cloud::Error::InvalidEffect { index:256 })));
        let mut empty=value.descriptor().clone();empty.effects.clear();
        assert!(matches!(pluto_cloud::validate_with_effect_budget(empty,0),Err(pluto_cloud::Error::EffectBudget)));
        assert!(request([f64::NAN,0.0,0.0],vec![("speed",11,0)],owner).is_err());
        assert_eq!(request([0.0;3],vec![("speed",i32::MAX/4,0)],owner).unwrap().effects()[0].duration_ticks,(i32::MAX/4)*4);
    }
}
