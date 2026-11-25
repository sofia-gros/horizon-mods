//! Sky layers a mod draws itself, over the pack custom-sky layers: bright stars that twinkle, and
//! dark patches on the celestial sphere (Minecraft has no equivalent; the original this follows is
//! the night sky itself, as the Stellarsky design specifies it).
//!
//! A texture cannot twinkle, and a texture cannot darken what another layer drew under it, so the
//! bright end of the sky is not a custom-sky layer: a mod registers a [`SkyLayer`] — its stars with
//! a [`Twinkle`] each, its [`SkyDarkening`]s, and whether the vanilla procedural stars may stay —
//! and every frame the game asks the registry for the layer's quads ([`frame_quads`]) and draws
//! them with the sun and moon. Positions are equatorial (right ascension in hours, declination in
//! degrees, J2000 as catalogs give them); the layer names the right ascension that stands on the
//! meridian when the host's star angle is 0 (noon), which is the same phase a mod's static
//! custom-sky texture is painted with, so the two halves of a mod's sky agree.
//!
//! The host's vanilla stars are drawn from a fixed procedural buffer with one scalar brightness:
//! no per-star colour, no twinkle, and the pack custom-sky layers all draw *before* them, so no
//! texture can cover them. A layer that brings its own star field therefore also says
//! [`VanillaStars::Hide`], and the game drops the vanilla star frame for that world's frames.

use std::collections::BTreeMap;
use std::sync::RwLock;

/// How one star's light flickers (scintillation: the air over the star, not the star). The factor
/// [`twinkle_factor`] returns moves between `1 - amplitude` and 1 at about `rate` cycles a second,
/// starting from `phase` (in turns). An `amplitude` of 0 is a steady star.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Twinkle {
    /// How far the light dips, 0 (steady) to 1 (out at the trough).
    pub amplitude: f32,
    /// Cycles per second, roughly; two detuned waves are summed, so the flicker never repeats
    /// mechanically.
    pub rate: f32,
    /// Where in its cycle the star starts, in turns.
    pub phase: f32,
}

/// One bright star of a layer, in the catalog's own terms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkyStar {
    /// Right ascension, in hours (0 to 24).
    pub ra_hours: f32,
    /// Declination, in degrees (-90 to 90).
    pub dec_deg: f32,
    /// Visual magnitude; the layer's brightness and size curves are keyed to it.
    pub magnitude: f32,
    /// The star's colour, linear RGB (a spectral class colour, as the catalogs give it).
    pub rgb: [f32; 3],
    /// The star's scintillation.
    pub twinkle: Twinkle,
}

/// A patch of sky darker than its surroundings: a black hole's shadow on the bright core behind
/// it, or a dark nebula. Drawn as concentric quads of rising alpha, so the edge is soft.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkyDarkening {
    /// Right ascension of the centre, in hours (0 to 24).
    pub ra_hours: f32,
    /// Declination of the centre, in degrees (-90 to 90).
    pub dec_deg: f32,
    /// The dark patch's radius, in degrees (at most 45).
    pub radius_deg: f32,
    /// How dark the centre gets, 0 (nothing) to 1 (black).
    pub depth: f32,
}

/// Whether the host's vanilla procedural stars show in a world that has a registered layer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VanillaStars {
    /// The vanilla stars stay (the layer adds to them).
    #[default]
    Keep,
    /// The vanilla stars are dropped: the layer's own field replaces them.
    Hide,
}

/// One mod's dynamic sky for one world: its bright stars, its dark patches, and its say over the
/// vanilla stars.
#[derive(Clone, Debug, PartialEq)]
pub struct SkyLayer {
    /// The right ascension (hours) standing on the meridian when the host's star angle is 0
    /// (noon): the phase the layer's positions are read against. A mod whose static sky texture
    /// was painted with a phase uses the same number here.
    pub meridian_ra_hours: f32,
    /// The layer's bright stars (the faint end belongs in a custom-sky texture; a layer holds at
    /// most 4096).
    pub stars: Vec<SkyStar>,
    /// The layer's dark patches, drawn after its stars.
    pub darkenings: Vec<SkyDarkening>,
    /// Whether the vanilla procedural stars show under this layer.
    pub vanilla_stars: VanillaStars,
}

/// What one frame asks the layer for: the host's own sky numbers for the frame, and the time the
/// twinkles are read at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkyFrame {
    /// The host's star angle for the frame, in degrees (0 at noon, 180 at midnight).
    pub star_angle_deg: f32,
    /// The host's star brightness for the frame, 0 to 1 (its twilight and weather fade).
    pub star_brightness: f32,
    /// Seconds since an arbitrary fixed point, for the twinkles; only differences matter.
    pub seconds: f32,
}

/// One quad of a frame's layer, ready to draw: four corners on the celestial sphere (at the
/// celestial bodies' own distance, 100) in the sun quad's corner order, and its colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkyQuad {
    /// The quad's corners, camera-relative world positions.
    pub corners: [[f32; 3]; 4],
    /// The quad's colour, linear RGBA; a star's light in RGB with its brightness in A, a
    /// darkening's black with its alpha.
    pub rgba: [f32; 4],
}

static LAYERS: RwLock<BTreeMap<i32, SkyLayer>> = RwLock::new(BTreeMap::new());

/// The registered dynamic sky layers, by world number (the host's custom-sky numbering: 0 the
/// Overworld, 1 the End). Registration is process-lifetime and re-registering a world replaces
/// its layer.
pub struct SkyRegistry;

impl SkyRegistry {
    /// Registers `layer` for `world`, replacing any layer that world had. The layer is validated
    /// whole first — positions in range, colours finite, a twinkle's amplitude at most 1, a
    /// darkening's radius in (0, 45] and depth at most 1, at most 4096 stars — so a refused layer
    /// leaves the previous registration intact. Right ascensions are taken modulo 24 hours.
    pub fn register(world: i32, layer: SkyLayer) -> Result<(), String> {
        let layer = validate(layer)?;
        let mut layers = LAYERS.write().map_err(|_| "sky registry poisoned")?;
        layers.insert(world, layer);
        Ok(())
    }

    /// The layer registered for `world`, or None when no mod registered one.
    pub fn layer(world: i32) -> Option<SkyLayer> {
        LAYERS.read().ok()?.get(&world).cloned()
    }

    /// Every world that has a layer, sorted (the registry's own census, for tests and tools).
    pub fn registered_worlds() -> Vec<i32> {
        LAYERS.read().map(|layers| layers.keys().copied().collect()).unwrap_or_default()
    }
}

fn validate(mut layer: SkyLayer) -> Result<SkyLayer, String> {
    if !layer.meridian_ra_hours.is_finite() {
        return Err("sky layer meridian right ascension must be finite".into());
    }
    layer.meridian_ra_hours = layer.meridian_ra_hours.rem_euclid(24.0);
    if layer.stars.len() > 4096 {
        return Err(format!("sky layer holds {} stars, over the 4096 cap", layer.stars.len()));
    }
    for star in &mut layer.stars {
        if !star.ra_hours.is_finite() || !star.dec_deg.is_finite() || !star.magnitude.is_finite() {
            return Err("sky star position and magnitude must be finite".into());
        }
        if !(-90.0..=90.0).contains(&star.dec_deg) {
            return Err(format!("sky star declination {} outside -90..=90", star.dec_deg));
        }
        if star.rgb.iter().any(|c| !c.is_finite() || *c < 0.0) {
            return Err("sky star colour must be finite and non-negative".into());
        }
        let twinkle = &mut star.twinkle;
        if !(0.0..=1.0).contains(&twinkle.amplitude) || !(0.0..=10.0).contains(&twinkle.rate)
            || !twinkle.phase.is_finite()
        {
            return Err("sky star twinkle amplitude must be within 0..=1 and rate within 0..=10".into());
        }
        star.ra_hours = star.ra_hours.rem_euclid(24.0);
        twinkle.phase = twinkle.phase.rem_euclid(1.0);
    }
    for darkening in &mut layer.darkenings {
        if !darkening.ra_hours.is_finite() || !(-90.0..=90.0).contains(&darkening.dec_deg) {
            return Err("sky darkening position must be finite, declination within -90..=90".into());
        }
        if !(0.0 < darkening.radius_deg && darkening.radius_deg <= 45.0) {
            return Err(format!("sky darkening radius {} outside (0, 45]", darkening.radius_deg));
        }
        if !(0.0..=1.0).contains(&darkening.depth) {
            return Err(format!("sky darkening depth {} outside 0..=1", darkening.depth));
        }
        darkening.ra_hours = darkening.ra_hours.rem_euclid(24.0);
    }
    Ok(layer)
}

/// The world direction of an equatorial position at a star angle: the celestial sphere of an
/// equatorial observer (the game's sun rises due east, crosses the zenith and sets due west, so
/// the poles stand at the north and south horizon points). `meridian_ra_hours` is the right
/// ascension on the meridian at angle 0, as a [`SkyLayer`] declares it. The result is a unit
/// vector, x east, y up, z south.
pub fn celestial_direction(ra_hours: f32, dec_deg: f32, meridian_ra_hours: f32, star_angle_deg: f32) -> [f32; 3] {
    let h = ((ra_hours - meridian_ra_hours) * 15.0 + star_angle_deg).to_radians();
    let dec = dec_deg.to_radians();
    let cd = dec.cos();
    [-cd * h.sin(), cd * h.cos(), -dec.sin()]
}

/// One star's twinkle factor at `seconds`: 1 at the crest, down to `1 - amplitude` at the trough.
/// Two detuned sine waves are summed so the flicker does not read as a mechanism.
pub fn twinkle_factor(twinkle: &Twinkle, seconds: f32) -> f32 {
    if twinkle.amplitude <= 0.0 || twinkle.rate <= 0.0 {
        return 1.0;
    }
    let tau = std::f32::consts::TAU;
    let w1 = 0.5 + 0.5 * (tau * (twinkle.rate * seconds + twinkle.phase)).sin();
    let w2 = 0.5 + 0.5 * (tau * (2.63 * twinkle.rate * seconds + 1.37 * twinkle.phase + 0.31)).sin();
    1.0 - twinkle.amplitude * (0.62 * w1 + 0.38 * w2)
}

/// The quads of `layer` for one frame: its stars first (each a billboard on the celestial sphere,
/// sized and lit by its magnitude, dimmed by the frame's star brightness, by the air near the
/// horizon, and by its twinkle — which strengthens low in the sky, as scintillation does), then
/// its darkenings' rings, so a dark patch falls over the layer's own stars as well as the sky
/// beneath. Stars below the horizon and quads too faint to see are not emitted.
pub fn frame_quads(layer: &SkyLayer, frame: &SkyFrame) -> Vec<SkyQuad> {
    let mut out = Vec::new();
    let bright = frame.star_brightness.clamp(0.0, 1.0);
    for star in &layer.stars {
        let dir = celestial_direction(star.ra_hours, star.dec_deg, layer.meridian_ra_hours, frame.star_angle_deg);
        if dir[1] < -0.02 {
            continue;
        }
        // Atmospheric extinction: a star at the horizon is read through the whole air.
        let extinction = ((dir[1] + 0.02) / 0.22).clamp(0.0, 1.0);
        let mut twinkle = star.twinkle;
        twinkle.amplitude = (twinkle.amplitude * (1.0 + 1.6 * (1.0 - dir[1].clamp(0.0, 1.0)))).min(0.95);
        let flux = 0.9 * 10f32.powf(-0.28 * (star.magnitude - 1.0));
        let alpha = (flux * bright * extinction * twinkle_factor(&twinkle, frame.seconds)).clamp(0.0, 1.0);
        if alpha <= 0.003 {
            continue;
        }
        let half_deg = 0.14 + 0.55 * 10f32.powf(-0.13 * (star.magnitude + 1.46));
        out.push(quad(dir, half_deg, [star.rgb[0], star.rgb[1], star.rgb[2], alpha]));
    }
    // A darkening is read against the night sky; the host's star brightness is the frame's own
    // measure of how dark the sky is, so the patch follows it (and stays out of the day).
    let night = (bright * 1.25).clamp(0.0, 1.0);
    if night > 0.0 {
        for darkening in &layer.darkenings {
            let dir = celestial_direction(darkening.ra_hours, darkening.dec_deg, layer.meridian_ra_hours, frame.star_angle_deg);
            if dir[1] <= -darkening.radius_deg.to_radians().sin() {
                continue;
            }
            for (fraction, ring) in [(1.0, 0.16), (0.74, 0.28), (0.50, 0.42), (0.27, 0.60)] {
                let alpha = darkening.depth * ring * night;
                if alpha > 0.003 {
                    out.push(quad(dir, darkening.radius_deg * fraction, [0.0, 0.0, 0.0, alpha]));
                }
            }
        }
    }
    out
}

/// A billboard quad on the celestial sphere: centre `dir` at the bodies' distance (100), in the
/// plane across the line of sight, `half_deg` the half-size in degrees.
fn quad(dir: [f32; 3], half_deg: f32, rgba: [f32; 4]) -> SkyQuad {
    let reference = if dir[1].abs() < 0.95 { [0.0, 1.0, 0.0] } else { [0.0, 0.0, -1.0] };
    let right = normalize(cross(dir, reference));
    let up = cross(right, dir);
    let half = 100.0 * half_deg.to_radians().tan();
    let centre = [dir[0] * 100.0, dir[1] * 100.0, dir[2] * 100.0];
    let corner = |sx: f32, sy: f32| {
        [
            centre[0] + (right[0] * sx + up[0] * sy) * half,
            centre[1] + (right[1] * sx + up[1] * sy) * half,
            centre[2] + (right[2] * sx + up[2] * sy) * half,
        ]
    };
    SkyQuad { corners: [corner(-1.0, -1.0), corner(1.0, -1.0), corner(1.0, 1.0), corner(-1.0, 1.0)], rgba }
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > 0.0 { [v[0] / len, v[1] / len, v[2] / len] } else { [0.0, 0.0, 0.0] }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn star(ra: f32, dec: f32, mag: f32) -> SkyStar {
        SkyStar { ra_hours: ra, dec_deg: dec, magnitude: mag, rgb: [1.0, 1.0, 1.0], twinkle: Twinkle { amplitude: 0.0, rate: 1.0, phase: 0.0 } }
    }

    fn layer() -> SkyLayer {
        SkyLayer { meridian_ra_hours: 5.75, stars: vec![], darkenings: vec![], vanilla_stars: VanillaStars::Keep }
    }

    #[test]
    fn the_sun_stands_at_the_zenith_at_noon_and_in_the_west_at_sunset() {
        // A body at the meridian right ascension, on the equator, is the sun's own path.
        let noon = celestial_direction(5.75, 0.0, 5.75, 0.0);
        assert!(noon[1] > 0.999 && noon[0].abs() < 1e-3, "{noon:?}");
        let sunset = celestial_direction(5.75, 0.0, 5.75, 90.0);
        assert!(sunset[0] < -0.999 && sunset[1].abs() < 1e-3, "{sunset:?}");
        let midnight = celestial_direction(5.75, 0.0, 5.75, 180.0);
        assert!(midnight[1] < -0.999, "{midnight:?}");
    }

    #[test]
    fn sagittarius_a_star_culminates_high_in_the_south_at_midnight() {
        // The phase the Stellarsky static layer is painted with: at midnight the galactic centre
        // (17.75 h, -29 deg) stands on the meridian, 61 degrees up, due south.
        let d = celestial_direction(17.75, -29.0, 5.75, 180.0);
        assert!((d[1] - 0.8746).abs() < 1e-3, "{d:?}");
        assert!((d[2] - 0.4848).abs() < 1e-3, "{d:?}");
        assert!(d[0].abs() < 1e-3, "{d:?}");
    }

    #[test]
    fn the_north_pole_never_moves() {
        for angle in [0.0, 90.0, 180.0, 270.0] {
            let d = celestial_direction(3.0, 90.0, 5.75, angle);
            assert!(d[2] < -0.999 && d[0].abs() < 1e-3 && d[1].abs() < 1e-3, "{d:?}");
        }
    }

    #[test]
    fn twinkle_stays_within_its_amplitude() {
        let twinkle = Twinkle { amplitude: 0.4, rate: 1.3, phase: 0.2 };
        for i in 0..200 {
            let f = twinkle_factor(&twinkle, i as f32 * 0.05);
            assert!((0.6..=1.0).contains(&f), "{f}");
        }
        assert_eq!(twinkle_factor(&Twinkle { amplitude: 0.0, rate: 1.0, phase: 0.0 }, 3.0), 1.0);
    }

    #[test]
    fn registered_layers_read_back_and_invalid_ones_are_refused_whole() {
        let mut good = layer();
        good.stars.push(star(6.75, -16.7, -1.44));
        good.vanilla_stars = VanillaStars::Hide;
        SkyRegistry::register(41, good.clone()).unwrap();
        assert_eq!(SkyRegistry::layer(41), Some(good));
        assert!(SkyRegistry::registered_worlds().contains(&41));

        let mut bad = layer();
        bad.darkenings.push(SkyDarkening { ra_hours: 17.75, dec_deg: -29.0, radius_deg: 0.0, depth: 0.5 });
        assert!(SkyRegistry::register(41, bad).is_err());
        assert_eq!(SkyRegistry::layer(41).map(|l| l.stars.len()), Some(1), "the refused layer changed nothing");
    }

    #[test]
    fn frame_quads_draw_bright_stars_and_darkening_rings_in_order() {
        let mut l = layer();
        l.stars.push(star(17.75, -29.0, 1.0)); // on the meridian at midnight
        l.stars.push(star(5.75, 0.0, 1.0)); // the nadir at midnight: not drawn
        l.darkenings.push(SkyDarkening { ra_hours: 17.75, dec_deg: -29.0, radius_deg: 3.2, depth: 0.85 });
        let frame = SkyFrame { star_angle_deg: 180.0, star_brightness: 1.0, seconds: 0.0 };
        let quads = frame_quads(&l, &frame);
        assert_eq!(quads.len(), 1 + 4, "{quads:?}");
        let star_quad = quads[0];
        assert!(star_quad.rgba[3] > 0.5, "a magnitude 1 star at full brightness: {star_quad:?}");
        // The star stands 100 out, high in the south.
        assert!((star_quad.corners[0][1] - 87.46).abs() < 2.0, "{star_quad:?}");
        // Rings shrink and deepen toward the centre.
        let spreads: Vec<f32> = quads[1..].iter().map(|q| (q.corners[1][0] - q.corners[0][0]).abs().max((q.corners[1][2] - q.corners[0][2]).abs())).collect();
        assert!(spreads.windows(2).all(|w| w[0] > w[1]), "{spreads:?}");
        assert!(quads[1].rgba[3] < quads[4].rgba[3], "outer ring faintest: {quads:?}");
        // In the day (no star brightness) nothing is emitted at all.
        let day = SkyFrame { star_brightness: 0.0, ..frame };
        assert!(frame_quads(&l, &day).is_empty());
    }
}

