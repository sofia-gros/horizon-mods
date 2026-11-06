// Internal official-host construction only. This module does not spawn a cloud.
use super::{Breath, CloudSettings, PotionCloud};
use horizon_client::{pluto_cloud::{Attribution, Label, Validated}, text::Text};
use horizon_client_core::who::Who;

/// A native object with no world association or allocated entity ID. Its fields
/// stay private; only the game-private insertion boundary consumes the object.
#[allow(dead_code)]
pub(in crate::game) struct StagedCloud {
    native: Breath,
}
impl StagedCloud {
    /// Consume without cloning or allocating. This grants no actor authority;
    /// the insertion boundary must complete its checks before publication.
    #[allow(dead_code)]
    pub(in crate::game) fn into_native(self) -> Breath { self.native }
}

/// Materialize a validated explicit request without selecting an actor, reading
/// Game, allocating an entity ID or inserting anything into the world.
#[allow(dead_code)]
pub(in crate::game) fn construct(request: Validated) -> StagedCloud {
    let d = request.descriptor();
    let (owner, guest) = match d.attribution {
        Attribution::Unowned => (None, None),
        Attribution::Player { guest_connection } => (Some(Who::Player), guest_connection),
        Attribution::Mob { entity_id } => (Some(Who::Mob(entity_id)), None),
    };
    let source = match &d.label {
        Label::Literal(value) => Text::literal(value.clone()),
        Label::Translation(key) => Text::translate(key, &[]),
    };
    let mut hidden = Vec::new();
    let mut ambient = Vec::new();
    let effects = request.effects().iter().map(|effect| {
        if !effect.visible && !hidden.contains(&effect.raw_id) { hidden.push(effect.raw_id); }
        if effect.ambient && !ambient.contains(&effect.raw_id) { ambient.push(effect.raw_id); }
        (effect.raw_id, effect.duration_ticks, effect.amplifier)
    }).collect();
    StagedCloud { native: Breath {
        id: 0, at: d.position, radius: d.radius, age: d.timing.age,
        duration: d.timing.duration_ticks, from_dragon: false, victims: Vec::new(),
        potion: Some(PotionCloud {
            settings: CloudSettings {
                wait_time: d.timing.wait_ticks,
                reapplication_delay: d.timing.reapplication_ticks,
                radius_on_use: d.radius_on_use,
                duration_on_use: d.timing.duration_on_use,
                duration_scale: d.timing.effect_duration_scale,
            },
            effects, hidden, ambient, radius_per_tick: d.radius_per_tick,
            color: d.color_rgb, victims: Vec::new(), source, owner, guest,
        }),
    } }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_client::pluto_cloud::{self, Descriptor, Effect, Timing};

    fn requested(attribution: Attribution) -> Validated {
        pluto_cloud::validate(Descriptor {
            position: [1.25, 64.0, -3.5], radius: 1.0, radius_per_tick: -1.0 / 600.0,
            radius_on_use: -0.5, timing: Timing { age: 0, duration_ticks: 600,
                wait_ticks: 10, reapplication_ticks: 20, duration_on_use: -2,
                effect_duration_scale: 0.25 },
            color_rgb: 0x123456, label: Label::Literal("explicit label".into()), attribution,
            effects: vec![Effect { id: "speed".into(), duration_ticks: 400, amplifier: 2,
                visible: false, ambient: true }],
        }).unwrap()
    }

    #[test]
    fn real_native_fields_match_explicit_request_and_fresh_bookkeeping() {
        let staged = construct(requested(Attribution::Unowned));
        let cloud = staged.native;
        assert_eq!(cloud.id, 0);
        assert_eq!(cloud.at, [1.25, 64.0, -3.5]);
        assert_eq!((cloud.radius, cloud.age, cloud.duration), (1.0, 0, 600));
        assert!(!cloud.from_dragon && cloud.victims.is_empty());
        let potion = cloud.potion.unwrap();
        assert_eq!(potion.effects, [("speed", 400, 2)]);
        assert_eq!(potion.hidden, ["speed"]);
        assert_eq!(potion.ambient, ["speed"]);
        assert_eq!(potion.radius_per_tick.to_bits(), (-1.0f32 / 600.0).to_bits());
        assert_eq!((potion.settings.wait_time, potion.settings.reapplication_delay,
            potion.settings.duration_on_use), (10, 20, -2));
        assert_eq!(potion.settings.radius_on_use, -0.5);
        assert_eq!(potion.settings.duration_scale, 0.25);
        assert_eq!(potion.color, 0x123456);
        assert_eq!(potion.source.plain(), "explicit label");
        assert!(potion.victims.is_empty());
        assert_eq!((potion.owner, potion.guest), (None, None));
    }

    #[test]
    fn real_native_attribution_preserves_only_requested_identity() {
        for (request, expected) in [
            (Attribution::Unowned, (None, None)),
            (Attribution::Player { guest_connection: None }, (Some(Who::Player), None)),
            (Attribution::Player { guest_connection: Some(17) }, (Some(Who::Player), Some(17))),
            (Attribution::Mob { entity_id: 42 }, (Some(Who::Mob(42)), None)),
        ] {
            let potion = construct(requested(request)).native.potion.unwrap();
            assert_eq!((potion.owner, potion.guest), expected);
        }
    }
}
