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

/// How a textured layer mixes with what is drawn under it: the host's own custom-sky blends
/// (OptiFine's `blend=`), in the host's order. `Add` is the format's default. The declaration
/// order is the transport's blend number, so it must not change.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SkyBlend {
    /// The layer's light is added to the sky under it.
    #[default]
    Add,
    /// The layer's light is taken away.
    Subtract,
    /// The sky under it is multiplied by the layer.
    Multiply,
    /// Colour dodge.
    Dodge,
    /// Colour burn.
    Burn,
    /// Screen.
    Screen,
    /// The layer's pixels wherever its brightness is above 0, unfaded.
    Replace,
    /// Overlay.
    Overlay,
    /// A weighted average by the layer's alpha.
    Alpha,
}

/// The weather of a frame, as a textured layer's admission reads it: thunder is a storm's
/// rain, so it is only `Thunder`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkyNow {
    /// No rain.
    Clear,
    /// Raining.
    Rain,
    /// Thundering.
    Thunder,
}

/// The weathers a textured layer shows in (`weather=` in the pack format); clear alone by
/// default, so a layer without a say hides in the rain, like the vanilla stars.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkyWeather {
    /// Shows in clear weather.
    pub clear: bool,
    /// Shows in rain.
    pub rain: bool,
    /// Shows in thunder.
    pub thunder: bool,
}

impl Default for SkyWeather {
    fn default() -> Self {
        Self { clear: true, rain: false, thunder: false }
    }
}

impl SkyWeather {
    /// Whether the layer shows in the frame's weather.
    pub fn admits(self, now: SkyNow) -> bool {
        match now {
            SkyNow::Clear => self.clear,
            SkyNow::Rain => self.rain,
            SkyNow::Thunder => self.thunder,
        }
    }
}

/// A clock time as ticks of the day, as the pack format's fade times are written: 6:00 is
/// tick 0 (sunrise), 12:00 is 6000, 18:00 is 12000, 0:00 is 18000. `None` off the clock.
pub fn fade_ticks(hour: u32, minute: u32) -> Option<i64> {
    if hour >= 24 || minute >= 60 {
        return None;
    }
    let (h, m) = (i64::from(hour), i64::from(minute));
    Some((h * 1000 + m * 1000 / 60 - 6000).rem_euclid(24_000))
}

/// The fade times of a textured layer, in ticks of the day (0 at sunrise, 6:00). Between
/// `start_in` and `end_in` the brightness rises from 0 to 1, stays 1 until `start_out`, falls
/// to 0 at `end_out`, and stays 0 until `start_in` again; each span may cross midnight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkyFade {
    /// When the fade in starts.
    pub start_in: i64,
    /// When the fade in completes.
    pub end_in: i64,
    /// When the fade out starts.
    pub start_out: i64,
    /// When the fade out completes.
    pub end_out: i64,
}

impl SkyFade {
    /// A fade from its in-times and its out end: the fade out starts as long before `end_out`
    /// as the fade in takes, as the pack format derives it when `startFadeOut` is absent.
    pub fn new(start_in: i64, end_in: i64, end_out: i64) -> Self {
        let start_out = (end_out - (end_in - start_in).rem_euclid(24_000)).rem_euclid(24_000);
        Self { start_in, end_in, start_out, end_out }
    }

    /// The brightness at `day_time` (ticks; any day).
    pub fn brightness(&self, day_time: i64) -> f32 {
        let since = |t: i64| (t - self.start_in).rem_euclid(24_000);
        let now = since(day_time);
        let (end_in, start_out) = (since(self.end_in), since(self.start_out));
        // A fade out that ends where the fade in starts ends a whole day later, not at once.
        let end_out = match since(self.end_out) {
            0 => 24_000,
            end_out => end_out,
        };
        if now < end_in {
            now as f32 / end_in as f32
        } else if now < start_out {
            1.0
        } else if now < end_out {
            1.0 - (now - start_out) as f32 / (end_out - start_out) as f32
        } else {
            0.0
        }
    }
}

/// One textured sky layer a mod draws itself: a skybox texture (the host's custom-sky layout:
/// one PNG, six faces in a 3x2 grid — bottom, top and south in the top row, west, north and
/// east below) with the blend, fade, turning and weather of the pack format's layer, but
/// carried by the mod itself instead of a resource pack. Layers of a world draw in
/// registration order, over the pack's layers and under the dynamic layers' quads.
#[derive(Clone, Debug, PartialEq)]
pub struct SkyTextureLayer {
    /// The skybox texture, PNG bytes.
    pub png: Vec<u8>,
    /// How the layer mixes with what is under it.
    pub blend: SkyBlend,
    /// The fade times; `None` is always at full brightness.
    pub fade: Option<SkyFade>,
    /// Whether the layer turns with the time of day around `axis`.
    pub rotate: bool,
    /// Turns per day at `rotate`, as a multiple of the sun's own turn.
    pub speed: f32,
    /// The axis it turns around, normalised at registration; south (`0 0 1`) by default, as
    /// the sun.
    pub axis: [f32; 3],
    /// The weathers it shows in.
    pub weather: SkyWeather,
    /// Seconds the brightness takes to follow a change of the weather admission.
    pub transition: f32,
}

impl SkyTextureLayer {
    /// A layer of just a texture, with the pack format's defaults: `add`, no fade, turning
    /// with the sun about the south-north axis, clear weather only, a second's transition.
    pub fn new(png: Vec<u8>) -> Self {
        Self {
            png,
            blend: SkyBlend::Add,
            fade: None,
            rotate: true,
            speed: 1.0,
            axis: [0.0, 0.0, 1.0],
            weather: SkyWeather::default(),
            transition: 1.0,
        }
    }

    /// Whether the frame's weather admits the layer; the brightness it then shows at is
    /// [`SkyTextureLayer::brightness`], smoothed by the frame evaluation's transition.
    pub fn shows(&self, now: SkyNow) -> bool {
        self.weather.admits(now)
    }

    /// The brightness of the fade times at `day_time`; a `replace` layer is all or nothing.
    pub fn brightness(&self, day_time: i64) -> f32 {
        let b = self.fade.map_or(1.0, |f| f.brightness(day_time));
        if self.blend == SkyBlend::Replace {
            if b > 0.0 { 1.0 } else { 0.0 }
        } else {
            b
        }
    }

    /// How far the layer has turned around its axis, in turns, when the sun stands at
    /// `sun_turns` (the sun's angle over 360°, 0 at noon). A turn a day at speed 1, so the
    /// layer keeps pace with the sun; counted from noon of day 0, so a slower speed carries
    /// on across days. 0 for a layer that does not rotate.
    pub fn turns(&self, day_time: i64, sun_turns: f32) -> f32 {
        if !self.rotate {
            return 0.0;
        }
        let days = (day_time - 6000).div_euclid(24_000);
        // Whole turns only matter at a speed that is not whole; keep the f32 small.
        let days = (days as f64 * f64::from(self.speed)).rem_euclid(1.0);
        (days + f64::from(sun_turns) * f64::from(self.speed)).rem_euclid(1.0) as f32
    }
}

/// The size a PNG declares in its header, without decoding it: the registry reads sizes to
/// hold layers to the host's texture budget before any decode is paid for.
pub fn png_size(png: &[u8]) -> Option<(u32, u32)> {
    const SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    if png.len() < 24 || &png[0..8] != SIGNATURE || &png[12..16] != b"IHDR" {
        return None;
    }
    let width = u32::from_be_bytes(png[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(png[20..24].try_into().ok()?);
    (width > 0 && height > 0).then_some((width, height))
}

static TEXTURE_LAYERS: RwLock<BTreeMap<i32, Vec<SkyTextureLayer>>> = RwLock::new(BTreeMap::new());
static TEXTURE_GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// How the registry's textured layers changed, for the transport's decode cache: bumped by
/// every taken registration, never reused.
pub(crate) fn texture_generation() -> u64 {
    TEXTURE_GENERATION.load(std::sync::atomic::Ordering::Relaxed)
}

/// The registered textured sky layers, by world number (the host's custom-sky numbering: 0
/// the Overworld, 1 the End). Registration is process-lifetime; a world's layers draw in
/// registration order. Layers are read when the game assembles a pack, so a mod registers
/// its layers at startup, from its hooks.
pub struct SkyTextureRegistry;

impl SkyTextureRegistry {
    /// Appends `layer` to `world`'s textured layers. The layer is validated whole first —
    /// its PNG header must declare a size, the registry's whole census must stay within the
    /// host's custom-sky texture budget (64M texels over every world and layer), fade ticks
    /// on the day, a finite speed, an axis that is not the zero vector (it is normalised),
    /// a transition that is not negative — so a refused layer changes nothing.
    pub fn register(world: i32, layer: SkyTextureLayer) -> Result<(), String> {
        let layer = validate_texture_layer(layer)?;
        let mut layers = TEXTURE_LAYERS.write().map_err(|_| "sky texture registry poisoned")?;
        let mut texels = texels_of(&layer)?;
        for world_layers in layers.values() {
            for held in world_layers {
                texels += texels_of(held)?;
            }
        }
        if texels > 64 << 20 {
            return Err(format!("sky texture layers hold {texels} texels, over the host's 64M budget"));
        }
        layers.entry(world).or_default().push(layer);
        TEXTURE_GENERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    /// The textured layers registered for `world`, in drawing order.
    pub fn layers(world: i32) -> Vec<SkyTextureLayer> {
        TEXTURE_LAYERS.read().map(|layers| layers.get(&world).cloned().unwrap_or_default()).unwrap_or_default()
    }

    /// Every world that has textured layers, sorted: the registry's own census.
    pub fn registered_worlds() -> Vec<i32> {
        TEXTURE_LAYERS
            .read()
            .map(|layers| layers.iter().filter(|(_, held)| !held.is_empty()).map(|(world, _)| *world).collect())
            .unwrap_or_default()
    }
}

fn texels_of(layer: &SkyTextureLayer) -> Result<u64, String> {
    let (width, height) = png_size(&layer.png).ok_or("sky texture layer's PNG declares no size")?;
    Ok(u64::from(width) * u64::from(height))
}

fn validate_texture_layer(mut layer: SkyTextureLayer) -> Result<SkyTextureLayer, String> {
    texels_of(&layer)?;
    if let Some(fade) = layer.fade {
        for tick in [fade.start_in, fade.end_in, fade.start_out, fade.end_out] {
            if !(0..24_000).contains(&tick) {
                return Err(format!("sky texture fade tick {tick} outside the day"));
            }
        }
    }
    if !layer.speed.is_finite() {
        return Err("sky texture layer speed must be finite".into());
    }
    let axis = layer.axis;
    if axis.iter().any(|c| !c.is_finite()) {
        return Err("sky texture layer axis must be finite".into());
    }
    let len = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
    if len <= 1e-6 {
        return Err("sky texture layer axis is the zero vector".into());
    }
    layer.axis = [axis[0] / len, axis[1] / len, axis[2] / len];
    if !layer.transition.is_finite() || layer.transition < 0.0 {
        return Err("sky texture layer transition must be finite and not negative".into());
    }
    Ok(layer)
}

/// What one frame asks the textured layers for: the day's ticks (counting the days) and the
/// frame's weather. (Biome and height gating stay with pack layers; a mod layer that needs
/// them is a pack layer.)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkyTextureConditions {
    /// Ticks of the day, counting the days.
    pub day_time: i64,
    /// The frame's weather.
    pub weather: SkyNow,
}

/// One textured layer of a frame, ready to draw: which texture (an index over every
/// registered layer's texture, in census order), with which blend, how bright and turned how.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkyTextureDraw {
    /// The layer's texture index: `first_image` plus its place in its world's layers.
    pub image: usize,
    /// How the layer mixes with what is under it.
    pub blend: SkyBlend,
    /// The layer's brightness this frame, fade and transition applied.
    pub brightness: f32,
    /// The axis it is turned around.
    pub axis: [f32; 3],
    /// How far it has turned, in turns.
    pub turns: f32,
}

/// A layer's eased brightness moves toward `target` by `seconds` of its transition
/// (linearly), so it does not pop when its weather admission flips.
fn ease(value: f32, target: f32, seconds: f32, transition: f32) -> f32 {
    if transition <= 0.0 {
        target
    } else {
        let step = seconds / transition;
        if value < target { (value + step).min(target) } else { (value - step).max(target) }
    }
}

/// The textured draws of one world's layers for one frame: each layer the weather admits
/// eases toward full brightness over its transition (a layer the weather refuses eases out),
/// is lit by its fade times, and turns as its rotation says. `first_image` is the texture
/// index of the world's first layer over the whole census; `eased` is the caller's per-layer
/// eased brightness, kept between frames (its length is the layer count; a mismatch — a new
/// world or pack — snaps every layer to its target, as the host's own evaluation does, and
/// `snap` asks for the same on a world change).
pub fn texture_draws(
    layers: &[SkyTextureLayer],
    first_image: usize,
    conditions: &SkyTextureConditions,
    sun_turns: f32,
    seconds: f32,
    snap: bool,
    eased: &mut Vec<f32>,
) -> Vec<SkyTextureDraw> {
    let snap = snap || eased.len() != layers.len();
    if snap {
        eased.clear();
        eased.resize(layers.len(), 0.0);
    }
    let mut out = Vec::new();
    for (i, layer) in layers.iter().enumerate() {
        let target = if layer.shows(conditions.weather) { 1.0 } else { 0.0 };
        let shown = if snap {
            eased[i] = target;
            target
        } else {
            eased[i] = ease(eased[i], target, seconds, layer.transition);
            eased[i]
        };
        let brightness = shown * layer.brightness(conditions.day_time);
        if brightness <= 0.0 {
            continue;
        }
        out.push(SkyTextureDraw {
            image: first_image + i,
            blend: layer.blend,
            brightness,
            axis: layer.axis,
            turns: layer.turns(conditions.day_time, sun_turns),
        });
    }
    out
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

    /// A PNG header declaring `width` x `height` (nothing after the header: the
    /// registry reads sizes from headers and never decodes).
    fn png_header(width: u32, height: u32) -> Vec<u8> {
        let mut png = Vec::new();
        png.extend_from_slice(b"\x89PNG\r\n\x1a\n");
        png.extend_from_slice(&13u32.to_be_bytes());
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&width.to_be_bytes());
        png.extend_from_slice(&height.to_be_bytes());
        png.extend_from_slice(&[8, 6, 0, 0, 0]);
        png
    }

    fn texture_layer(width: u32, height: u32) -> SkyTextureLayer {
        SkyTextureLayer::new(png_header(width, height))
    }

    #[test]
    fn fade_ticks_are_ticks_from_sunrise() {
        assert_eq!(fade_ticks(6, 0), Some(0));
        assert_eq!(fade_ticks(12, 0), Some(6000));
        assert_eq!(fade_ticks(18, 0), Some(12000));
        assert_eq!(fade_ticks(0, 0), Some(18000));
        assert_eq!(fade_ticks(5, 30), Some(23500));
        assert_eq!(fade_ticks(24, 0), None);
        assert_eq!(fade_ticks(6, 60), None);
    }

    #[test]
    fn a_textured_night_layer_fades_across_midnight() {
        // In from 18:00 to 20:00, out by 6:00: the fade out starts at 4:00, two hours before.
        let fade = SkyFade::new(fade_ticks(18, 0).unwrap(), fade_ticks(20, 0).unwrap(), fade_ticks(6, 0).unwrap());
        assert_eq!(fade.start_out, 22000);
        let mut layer = texture_layer(6, 4);
        layer.fade = Some(fade);
        assert_eq!(layer.brightness(12000), 0.0); // 18:00
        assert_eq!(layer.brightness(13000), 0.5); // 19:00
        assert_eq!(layer.brightness(18000), 1.0); // midnight
        assert_eq!(layer.brightness(23000), 0.5); // 5:00
        assert_eq!(layer.brightness(0), 0.0); // 6:00
        assert_eq!(layer.brightness(6000), 0.0); // noon
        assert_eq!(layer.brightness(24000 * 3 + 13000), 0.5, "any day");
    }

    #[test]
    fn no_fade_means_always_and_replace_is_unfaded() {
        assert_eq!(texture_layer(6, 4).brightness(9999), 1.0);
        let mut layer = texture_layer(6, 4);
        layer.blend = SkyBlend::Replace;
        layer.fade = Some(SkyFade::new(0, 2000, 14000));
        assert_eq!(layer.brightness(500), 1.0, "half way in, yet full");
        assert_eq!(layer.brightness(20000), 0.0);
    }

    #[test]
    fn textured_layers_turn_with_the_sun_and_slower_ones_carry_over_days() {
        let layer = texture_layer(6, 4);
        assert_eq!(layer.turns(6000, 0.0), 0.0);
        assert_eq!(layer.turns(12000, 0.25), 0.25);
        let mut still = texture_layer(6, 4);
        still.rotate = false;
        assert_eq!(still.turns(12000, 0.25), 0.0);
        // Half speed: noon of day 1 is half a turn on, and a quarter day later 5/8.
        let mut half = texture_layer(6, 4);
        half.speed = 0.5;
        assert_eq!(half.turns(6000, 0.0), 0.0);
        assert_eq!(half.turns(30000, 0.0), 0.5);
        assert_eq!(half.turns(36000, 0.25), 0.625);
        assert_eq!(half.turns(54000, 0.0), 0.0);
    }

    #[test]
    fn textured_weather_defaults_to_clear_only() {
        let layer = texture_layer(6, 4);
        assert!(layer.shows(SkyNow::Clear));
        assert!(!layer.shows(SkyNow::Rain));
        assert!(!layer.shows(SkyNow::Thunder));
        let mut wet = texture_layer(6, 4);
        wet.weather = SkyWeather { clear: false, rain: true, thunder: true };
        assert!(!wet.shows(SkyNow::Clear));
        assert!(wet.shows(SkyNow::Thunder));
    }

    #[test]
    fn png_size_reads_the_header_and_nothing_else() {
        assert_eq!(png_size(&png_header(3072, 2048)), Some((3072, 2048)));
        assert_eq!(png_size(&[]), None);
        assert_eq!(png_size(b"not a png at all, nope, no"), None);
        assert_eq!(png_size(&png_header(0, 10)), None);
    }

    #[test]
    fn textured_layers_register_in_order_and_refusals_change_nothing() {
        let world = 91;
        SkyTextureRegistry::register(world, texture_layer(6, 4)).unwrap();
        let mut second = texture_layer(6, 4);
        second.blend = SkyBlend::Alpha;
        SkyTextureRegistry::register(world, second).unwrap();
        let held = SkyTextureRegistry::layers(world);
        assert_eq!(held.len(), 2);
        assert_eq!(held[0].blend, SkyBlend::Add);
        assert_eq!(held[1].blend, SkyBlend::Alpha);
        assert!(SkyTextureRegistry::registered_worlds().contains(&world));
        // A layer with no PNG header is refused whole.
        assert!(SkyTextureRegistry::register(world, SkyTextureLayer::new(vec![1, 2, 3])).is_err());
        // A layer whose header alone busts the texel budget is refused whole.
        assert!(SkyTextureRegistry::register(world, texture_layer(8192, 8192)).is_err());
        // A zero axis is refused; a long axis is normalised.
        let mut bad_axis = texture_layer(6, 4);
        bad_axis.axis = [0.0, 0.0, 0.0];
        assert!(SkyTextureRegistry::register(world, bad_axis).is_err());
        let mut long_axis = texture_layer(6, 4);
        long_axis.axis = [0.0, 2.0, 0.0];
        SkyTextureRegistry::register(world, long_axis).unwrap();
        assert_eq!(SkyTextureRegistry::layers(world)[2].axis, [0.0, 1.0, 0.0]);
        assert_eq!(SkyTextureRegistry::layers(world).len(), 3, "the refusals added nothing");
    }

    #[test]
    fn texture_draws_ease_in_but_snap_on_a_new_world() {
        let mut night = texture_layer(6, 4);
        night.transition = 2.0;
        let mut still = texture_layer(6, 4);
        still.blend = SkyBlend::Alpha;
        still.rotate = false;
        let layers = vec![night, still];
        let clear = SkyTextureConditions { day_time: 6000, weather: SkyNow::Clear };
        let mut eased = Vec::new();
        // Arrival (snap): both layers at full brightness at once.
        let out = texture_draws(&layers, 5, &clear, 0.0, 0.05, true, &mut eased);
        assert_eq!(out.iter().map(|d| d.brightness).collect::<Vec<_>>(), [1.0, 1.0]);
        assert_eq!(out.iter().map(|d| d.image).collect::<Vec<_>>(), [5, 6]);
        assert_eq!(out[1].blend, SkyBlend::Alpha);
        assert_eq!(out[1].turns, 0.0, "a layer that does not rotate");
        // Rain refuses both (clear only): they ease out over their transitions.
        let rain = SkyTextureConditions { weather: SkyNow::Rain, ..clear };
        let out = texture_draws(&layers, 5, &rain, 0.0, 0.5, false, &mut eased);
        assert_eq!(out[0].brightness, 0.75);
        let out = texture_draws(&layers, 5, &rain, 0.0, 2.0, false, &mut eased);
        assert!(out.is_empty(), "eased-out layers are not drawn: {out:?}");
        // Back to clear in a new world (snap): no fade in.
        let out = texture_draws(&layers, 5, &clear, 0.0, 0.0, true, &mut eased);
        assert_eq!(out.len(), 2);
    }
}

