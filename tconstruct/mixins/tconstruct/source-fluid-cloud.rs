// Construct a source-compatible MobEffectFluidEffect cloud through the
// existing Horizon cloud tick and tracking path. Kept under game::potions so
// its private PotionCloud fields do not require a new shared host framework.
use crate::game::Game;
use crate::game::dragon::Breath;
use super::PotionCloud;

pub(in crate::game) fn spawn(game: &mut Game<'_>, at: [f64; 3], effects: Vec<(&'static str, i32, i32)>) {
    let color = horizon_client::brewing::effects_color(effects.iter().map(|(id, _, amplifier)| (*id, *amplifier)));
    // Horizon's lingering cloud path applies a quarter-duration scale. Source
    // custom cloud effects are already scaled, so store 4x durations to retain
    // the intended result, rejecting overflow at the caller.
    let effects = effects.into_iter().map(|(id, time, amplifier)| (id, time * 4, amplifier)).collect();
    let id = game.take_entity_id();
    game.breath_clouds.push(Breath {
        id, at, radius: 1.0, age: 0, duration: 600, from_dragon: false,
        potion: Some(PotionCloud {
            effects, hidden: Vec::new(), ambient: Vec::new(),
            radius_per_tick: -1.0 / 600.0, color, victims: Vec::new(),
            source: horizon_client::text::Text::translate("entity.minecraft.area_effect_cloud", &[]),
        }),
    });
}
