//! The dynamic layer's contents: the bright end of the HYG catalog as a
//! PlutoLib sky layer. The star rows live in `stars_table` (generated from
//! the catalog extract, `data/bright_stars.json` in the Stellarsky
//! workspace); this module names the layer's phase, its one darkening, and
//! the assembly the registry is handed.

use pluto_lib::{SkyDarkening, SkyLayer, SkyStar, Twinkle, VanillaStars};

use crate::stars_table::BRIGHT_STARS;

/// The right ascension standing on the meridian when the host's star angle
/// is 0 (noon), in hours. This is the phase the static custom-sky texture is
/// painted with (the pipeline's PHASE_H), so the bright stars stand where
/// the texture's sky puts the sun's path and the galactic centre: at
/// midnight Sagittarius A* culminates high in the south.
pub const MERIDIAN_RA_HOURS: f32 = 5.75;

/// Sagittarius A*, the galactic centre: a black hole's shadow laid over the
/// brightest core of the Milky Way. Drawn by the surface as concentric quads
/// of rising alpha, so the edge is soft.
pub const GALACTIC_CENTRE: SkyDarkening = SkyDarkening {
    ra_hours: 17.75,
    dec_deg: -29.0,
    radius_deg: 3.2,
    depth: 0.85,
};

/// The Overworld's dynamic layer: every bright star with its colour and
/// twinkle, the galactic centre's dark patch, and the vanilla star field
/// hidden — this field replaces it (the faint stars stay in the static
/// texture, which draws beneath).
pub fn layer() -> SkyLayer {
    SkyLayer {
        meridian_ra_hours: MERIDIAN_RA_HOURS,
        stars: BRIGHT_STARS
            .iter()
            .map(|&(ra_hours, dec_deg, magnitude, rgb, amplitude, rate, phase)| SkyStar {
                ra_hours,
                dec_deg,
                magnitude,
                rgb,
                twinkle: Twinkle { amplitude, rate, phase },
            })
            .collect(),
        darkenings: vec![GALACTIC_CENTRE],
        vanilla_stars: VanillaStars::Hide,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::{SkyFrame, SkyRegistry, celestial_direction, frame_quads, twinkle_factor};

    #[test]
    fn the_layer_holds_the_whole_bright_catalog() {
        let layer = layer();
        assert_eq!(layer.stars.len(), 523, "every star of magnitude 4 and brighter");
        for star in &layer.stars {
            assert!(star.magnitude <= 4.0, "fainter than the window: {star:?}");
            assert!(star.magnitude >= -1.44, "brighter than Sirius: {star:?}");
            assert!((0.0..24.0).contains(&star.ra_hours), "{star:?}");
            assert!((-90.0..=90.0).contains(&star.dec_deg), "{star:?}");
            assert!(star.rgb.iter().all(|c| (0.0..=1.0).contains(c)), "{star:?}");
        }
        // The brightest row is Sirius, at its catalog place.
        let sirius = layer.stars.iter().min_by(|a, b| a.magnitude.total_cmp(&b.magnitude)).unwrap();
        assert_eq!(sirius.magnitude, -1.44);
        assert!((sirius.ra_hours - 6.75248).abs() < 1e-4, "{sirius:?}");
        assert!((sirius.dec_deg - -16.71612).abs() < 1e-4, "{sirius:?}");
        assert_eq!(sirius.rgb, [0.79, 0.84, 1.0]);
    }

    #[test]
    fn twinkle_parameters_stay_inside_the_registry_rules() {
        for star in &layer().stars {
            let twinkle = star.twinkle;
            assert!((0.0..=1.0).contains(&twinkle.amplitude), "{star:?}");
            assert!((0.0..=10.0).contains(&twinkle.rate) && twinkle.rate > 0.0, "{star:?}");
            assert!((0.0..1.0).contains(&twinkle.phase), "{star:?}");
        }
    }

    #[test]
    fn twinkle_factor_never_leaves_its_amplitude() {
        // Sample the catalog: every star's flicker stays between its trough
        // (1 - amplitude) and its crest (1) across ten seconds.
        for star in layer().stars.iter().step_by(37) {
            for i in 0..200 {
                let factor = twinkle_factor(&star.twinkle, i as f32 * 0.05);
                assert!(
                    (1.0 - star.twinkle.amplitude..=1.0).contains(&factor),
                    "{star:?} at step {i}: {factor}"
                );
            }
        }
    }

    #[test]
    fn the_meridian_is_the_static_texture_phase() {
        // The two halves of the sky agree only while this stays the phase
        // the static layers are painted with (pipeline PHASE_H = 5.75 h).
        assert_eq!(MERIDIAN_RA_HOURS, 5.75);
        assert_eq!(layer().meridian_ra_hours, 5.75);
        assert_eq!(layer().vanilla_stars, VanillaStars::Hide);
    }

    #[test]
    fn sagittarius_a_star_stands_high_in_the_south_at_midnight() {
        let d = celestial_direction(
            GALACTIC_CENTRE.ra_hours,
            GALACTIC_CENTRE.dec_deg,
            MERIDIAN_RA_HOURS,
            180.0,
        );
        assert!((d[1] - 0.8746).abs() < 1e-3, "{d:?}");
        assert!((d[2] - 0.4848).abs() < 1e-3, "{d:?}");
        assert!(d[0].abs() < 1e-3, "{d:?}");
    }

    #[test]
    fn the_darkening_rings_deepen_toward_the_centre_at_midnight() {
        let midnight = SkyFrame { star_angle_deg: 180.0, star_brightness: 1.0, seconds: 0.0 };
        let quads = frame_quads(&layer(), &midnight);
        assert!(quads.len() > 100, "the bright field is up at midnight: {}", quads.len());
        // Stars first, then the darkening's four rings, alpha rising inward.
        let rings = &quads[quads.len() - 4..];
        for ring in rings {
            assert_eq!(ring.rgba[..3], [0.0, 0.0, 0.0], "a darkening is black: {ring:?}");
        }
        assert!(
            rings.windows(2).all(|w| w[0].rgba[3] < w[1].rgba[3]),
            "rings deepen toward the centre: {rings:?}"
        );
        // The innermost ring reaches most of the declared depth, and the
        // patch stands where Sagittarius A* stands: high in the south.
        assert!(rings[3].rgba[3] > 0.45, "{rings:?}");
        let centre_y = rings[3].corners.iter().map(|c| c[1]).sum::<f32>() / 4.0;
        let centre_z = rings[3].corners.iter().map(|c| c[2]).sum::<f32>() / 4.0;
        assert!((centre_y - 87.46).abs() < 3.0, "{rings:?}");
        assert!((centre_z - 48.48).abs() < 3.0, "{rings:?}");
    }

    #[test]
    fn day_frames_draw_nothing() {
        let day = SkyFrame { star_angle_deg: 0.0, star_brightness: 0.0, seconds: 0.0 };
        assert!(frame_quads(&layer(), &day).is_empty());
    }

    #[test]
    fn the_layer_registers_whole_for_the_overworld() {
        // The exact call lib.rs's register() makes on the mod's behalf.
        SkyRegistry::register(0, layer()).unwrap();
        let registered = SkyRegistry::layer(0).expect("the overworld layer");
        assert_eq!(registered.stars.len(), 523);
        assert_eq!(registered.darkenings, vec![GALACTIC_CENTRE]);
        assert_eq!(registered.vanilla_stars, VanillaStars::Hide);
        assert!(SkyRegistry::registered_worlds().contains(&0));
    }
}
