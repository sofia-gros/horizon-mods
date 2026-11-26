# Sky

A mod draws the bright end of its sky itself: stars too bright for a texture to do justice, each
with its own colour and its own twinkle, and dark patches — a black hole's shadow — laid over the
glow beneath. The faint end of the sky is a texture, and a mod carries that texture itself: a
textured layer (below) is a skybox in the host's custom-sky format — the OptiFine-format layer a
resource pack would ship — registered by the mod, so no pack is needed for a mod's sky. The
dynamic layer is the part a texture cannot do, because a texture cannot flicker and cannot darken
what another layer drew.

```rust
use pluto_lib::{SkyDarkening, SkyLayer, SkyRegistry, SkyStar, Twinkle, VanillaStars};

// Stellarsky's dynamic layer for the Overworld: the bright stars of the catalog
// (Sirius shown; the layer ships all of magnitude 4 and brighter), the galactic
// centre's dark patch, and no vanilla stars — this field replaces them. The
// meridian right ascension is the phase the mod's static sky texture is painted
// with, so the two halves of the sky agree.
SkyRegistry::register(0, SkyLayer {
    meridian_ra_hours: 5.75,
    stars: vec![SkyStar {
        ra_hours: 6.75248, dec_deg: -16.71612, magnitude: -1.44,
        rgb: [0.79, 0.84, 1.0],
        twinkle: Twinkle { amplitude: 0.22, rate: 1.02, phase: 0.57 },
    }],
    darkenings: vec![SkyDarkening {
        ra_hours: 17.75, dec_deg: -29.0, radius_deg: 3.2, depth: 0.85,
    }],
    vanilla_stars: VanillaStars::Hide,
}).unwrap();
assert!(SkyRegistry::registered_worlds().contains(&0));
```

Every frame the game builds the celestial bodies, it asks the registry for the frame's quads
([`frame_quads`](#frame_quads)) and draws them with the sun and moon; a layer that hides the
vanilla stars also drops the host's procedural star frame for that world. Positions are
equatorial, as the catalogs give them (J2000 right ascension in hours, declination in degrees).

> **Use with Caution.** The registry, the frame mathematics and the validation are pure-tested.
> The transport is source-declared: a cancellable head reach on the game's celestial-geometry
> builder hands the frame's star angle and brightness to the registered layer. (A return reach
> cannot weave here: the builder names the moon's sprite with a bare `format!`, which the weaver
> cannot re-emit — E_UNSUPPORTED_MACRO.) With no layer for the frame the reach continues and the
> game builds the sun, moon and stars itself, untouched; when a layer answers, the reach returns
> the bodies built the game's own way with the layer's quads joined to the vertex list, sampling
> the atlas's white texel — the same untextured-quad path the lightning bolts use — and drops the
> vanilla star frame when the layer says so. The reach's weave plan is verified; it has not been
> natively compiled yet: its first compile is the combined build, and the darkening rings'
> softness rides on the celestial pass's alpha blending, which the moon's own rain fade already
> exercises.

A textured layer is the static half of a mod's sky: the Milky Way and the faint stars, painted
once and turning with the sun. Stellarsky's star sphere, registered for the Overworld:

```rust
use pluto_lib::{SkyFade, SkyTextureLayer, SkyTextureRegistry, fade_ticks};

// A PNG header declaring 6x4 stands in for the layer's skybox texture (a 3x2
// grid of faces); the real layer ships the painted sphere.
let mut header = Vec::new();
header.extend_from_slice(b"\x89PNG\r\n\x1a\n");
header.extend_from_slice(&13u32.to_be_bytes());
header.extend_from_slice(b"IHDR");
header.extend_from_slice(&6u32.to_be_bytes());
header.extend_from_slice(&4u32.to_be_bytes());
header.extend_from_slice(&[8, 6, 0, 0, 0]);
let mut layer = SkyTextureLayer::new(header);
// In after sunset, out before sunrise, turning with the sun.
layer.fade = Some(SkyFade::new(fade_ticks(18, 20).unwrap(), fade_ticks(19, 20).unwrap(), fade_ticks(5, 40).unwrap()));
SkyTextureRegistry::register(0, layer).unwrap();
assert!(SkyTextureRegistry::registered_worlds().contains(&0));
assert_eq!(SkyTextureRegistry::layers(0).len(), 1);
```

The game reads the registered layers when it assembles a pack: their textures join the pack's
own custom-sky textures (the pack's first, so its indices do not move) and their draws follow
the pack's layers in the frame, under the dynamic layers' quads. A layer whose PNG the game's
decoder refuses is dropped at that point, as a pack layer is. Register at startup, from the
mod's hooks — a registration after a pack has assembled waits for the next assembly.

> **Use with Caution.** The registry, the validation and the frame evaluation are pure-tested
> against the host's own pack-layer cases. The transport is source-declared: a cancellable head
> reach on the renderer's custom-sky load wraps the pack's layers in a composite that appends
> the registered layers (a return reach cannot weave here: the load's own empty answer is built
> inside a closure, which the weaver cannot re-emit). With no layer registered the reach
> continues and the load runs untouched. Its first compile is the combined build.

## Reference

### `Twinkle`

How one star's light flickers: `amplitude` is how far the light dips (0 is a steady star, 1 goes
out at the trough), `rate` is roughly cycles per second, and `phase` is where in its cycle the
star starts, in turns. The flicker is the air's, not the star's — scintillation — and it
strengthens of itself low in the sky when the frame is built.

```rust
use pluto_lib::Twinkle;

let steady = Twinkle { amplitude: 0.0, rate: 1.0, phase: 0.0 };
let lively = Twinkle { amplitude: 0.45, rate: 1.8, phase: 0.25 };
assert!(lively.amplitude > steady.amplitude);
```

### `SkyStar`

One bright star of a layer: `ra_hours` and `dec_deg` are its equatorial position, `magnitude` its
visual magnitude (the brightness and size curves are keyed to it), `rgb` its linear colour (a
spectral class colour, as the catalogs give it), and `twinkle` its scintillation.

```rust
use pluto_lib::{SkyStar, Twinkle};

let sirius = SkyStar {
    ra_hours: 6.75248, dec_deg: -16.71612, magnitude: -1.44,
    rgb: [0.79, 0.84, 1.0],
    twinkle: Twinkle { amplitude: 0.22, rate: 1.02, phase: 0.57 },
};
assert!(sirius.magnitude < 0.0);
```

### `SkyDarkening`

A patch of sky darker than its surroundings: the centre is `ra_hours` / `dec_deg`, `radius_deg`
is its radius in degrees, and `depth` is how dark the centre gets (0 is nothing, 1 is black). It
is drawn as concentric quads of rising alpha, so the edge is soft: a black hole at the galactic
centre darkens the bright core around it this way.

```rust
use pluto_lib::SkyDarkening;

let sagittarius_a = SkyDarkening { ra_hours: 17.75, dec_deg: -29.0, radius_deg: 3.2, depth: 0.85 };
assert!(sagittarius_a.depth > 0.5);
```

### `VanillaStars`

Whether the host's vanilla procedural stars show in a world that has a registered layer. The
vanilla field is a fixed buffer with one scalar brightness — no per-star colour, no twinkle — and
every custom-sky layer draws before it, so no texture can cover it; a mod that brings its own
star field says `Hide`, and one that only adds to the sky says `Keep` (the default).

```rust
use pluto_lib::VanillaStars;

assert_eq!(VanillaStars::default(), VanillaStars::Keep);
assert_ne!(VanillaStars::Hide, VanillaStars::Keep);
```

### `SkyLayer`

One mod's dynamic sky for one world: `meridian_ra_hours` is the right ascension standing on the
meridian when the host's star angle is 0 (noon) — the phase the layer's positions are read
against, the same number a static sky texture is painted with; `stars` are its bright stars;
`darkenings` its dark patches, drawn after the stars; and `vanilla_stars` its say over the
host's own star field.

```rust
use pluto_lib::{SkyLayer, VanillaStars};

let layer = SkyLayer { meridian_ra_hours: 5.75, stars: vec![], darkenings: vec![], vanilla_stars: VanillaStars::Hide };
assert_eq!(layer.meridian_ra_hours, 5.75);
assert!(layer.stars.is_empty());
```

### `SkyRegistry`

The registered dynamic sky layers, by world number (the host's custom-sky numbering: 0 the
Overworld, 1 the End). Registration is process-lifetime and re-registering a world replaces its
layer.

```rust
use pluto_lib::{SkyLayer, SkyRegistry, VanillaStars};

SkyRegistry::register(71, SkyLayer {
    meridian_ra_hours: 0.0, stars: vec![], darkenings: vec![], vanilla_stars: VanillaStars::Keep,
}).unwrap();
assert_eq!(SkyRegistry::layer(71).map(|layer| layer.meridian_ra_hours), Some(0.0));
```

### `SkyRegistry::register`

Registers a layer for a world, replacing any layer that world had. The layer is validated whole
first — positions in range, colours finite and non-negative, a twinkle's amplitude at most 1 and
rate at most 10, a darkening's radius in (0, 45] and depth at most 1, at most 4096 stars — so a
refused layer leaves the previous registration intact. Right ascensions are taken modulo 24
hours.

```rust
use pluto_lib::{SkyDarkening, SkyLayer, SkyRegistry, VanillaStars};

let good = SkyLayer { meridian_ra_hours: 5.75, stars: vec![], darkenings: vec![], vanilla_stars: VanillaStars::Hide };
SkyRegistry::register(72, good).unwrap();
let mut bad = SkyLayer { meridian_ra_hours: 5.75, stars: vec![], darkenings: vec![], vanilla_stars: VanillaStars::Hide };
bad.darkenings.push(SkyDarkening { ra_hours: 17.75, dec_deg: -29.0, radius_deg: 0.0, depth: 0.5 });
assert!(SkyRegistry::register(72, bad).is_err());
assert_eq!(SkyRegistry::layer(72).map(|layer| layer.darkenings.len()), Some(0));
```

### `SkyRegistry::layer`

The layer registered for a world, or None when no mod registered one.

```rust
use pluto_lib::SkyRegistry;

assert_eq!(SkyRegistry::layer(73), None);
```

### `SkyRegistry::registered_worlds`

Every world that has a layer, sorted: the registry's own census, for tests and tools.

```rust
use pluto_lib::{SkyLayer, SkyRegistry, VanillaStars};

SkyRegistry::register(74, SkyLayer {
    meridian_ra_hours: 0.0, stars: vec![], darkenings: vec![], vanilla_stars: VanillaStars::Keep,
}).unwrap();
assert!(SkyRegistry::registered_worlds().contains(&74));
```

### `SkyFrame`

What one frame asks a layer for: `star_angle_deg` is the host's star angle for the frame in
degrees (0 at noon, 180 at midnight), `star_brightness` is the host's own star brightness (its
twilight and weather fade, 0 to 1), and `seconds` is the time the twinkles are read at — seconds
since an arbitrary fixed point; only differences matter.

```rust
use pluto_lib::SkyFrame;

let midnight = SkyFrame { star_angle_deg: 180.0, star_brightness: 1.0, seconds: 42.0 };
assert_eq!(midnight.star_angle_deg, 180.0);
```

### `SkyQuad`

One quad of a frame's layer, ready to draw: `corners` are its four corners on the celestial
sphere, at the celestial bodies' own distance (100), in the sun quad's corner order, and `rgba`
is its colour — a star's light in RGB with its brightness in A, a darkening's black with its
alpha.

```rust
use pluto_lib::{SkyFrame, SkyLayer, Twinkle, VanillaStars, frame_quads, SkyStar};

let layer = SkyLayer {
    meridian_ra_hours: 5.75,
    stars: vec![SkyStar {
        ra_hours: 17.75, dec_deg: -29.0, magnitude: 1.0, rgb: [1.0, 1.0, 1.0],
        twinkle: Twinkle { amplitude: 0.0, rate: 1.0, phase: 0.0 },
    }],
    darkenings: vec![], vanilla_stars: VanillaStars::Keep,
};
let quads = frame_quads(&layer, &SkyFrame { star_angle_deg: 180.0, star_brightness: 1.0, seconds: 0.0 });
assert_eq!(quads.len(), 1);
assert!(quads[0].rgba[3] > 0.5);
assert!(quads[0].corners.iter().all(|corner| (corner[1] - 87.46).abs() < 2.0));
```

### `celestial_direction`

The world direction of an equatorial position at a star angle, on the game's celestial sphere
(an equatorial observer's: the poles stand at the north and south horizon points). The result is
a unit vector, x east, y up, z south. At midnight with a meridian right ascension of 5.75 h, the
galactic centre (17.75 h, −29°) stands high in the south:

```rust
use pluto_lib::celestial_direction;

let d = celestial_direction(17.75, -29.0, 5.75, 180.0);
assert!((d[1] - 0.8746).abs() < 0.001);
assert!((d[2] - 0.4848).abs() < 0.001);
assert!(d[0].abs() < 0.001);
```

### `twinkle_factor`

One star's twinkle factor at a time: 1 at the crest, down to `1 - amplitude` at the trough. Two
detuned sine waves are summed so the flicker does not read as a mechanism; a star with no
amplitude (or no rate) is steady at 1.

```rust
use pluto_lib::{Twinkle, twinkle_factor};

let twinkle = Twinkle { amplitude: 0.4, rate: 1.3, phase: 0.2 };
for i in 0..50 {
    let f = twinkle_factor(&twinkle, i as f32 * 0.1);
    assert!((0.6..=1.0).contains(&f));
}
assert_eq!(twinkle_factor(&Twinkle { amplitude: 0.0, rate: 1.0, phase: 0.0 }, 3.0), 1.0);
```

### `frame_quads`

The quads of a layer for one frame: its stars first — each a billboard on the celestial sphere,
sized and lit by its magnitude, dimmed by the frame's star brightness, by the air near the
horizon (extinction), and by its twinkle, which strengthens low in the sky — then its
darkenings' rings, so a dark patch falls over the layer's own stars as well as the sky beneath.
Stars below the horizon and quads too faint to see are not emitted; a darkening's alpha follows
the frame's star brightness, so the patch stays out of the day sky.

```rust
use pluto_lib::{SkyDarkening, SkyFrame, SkyLayer, SkyStar, Twinkle, VanillaStars, frame_quads};

let layer = SkyLayer {
    meridian_ra_hours: 5.75,
    stars: vec![SkyStar {
        ra_hours: 17.75, dec_deg: -29.0, magnitude: 1.0, rgb: [1.0, 1.0, 1.0],
        twinkle: Twinkle { amplitude: 0.0, rate: 1.0, phase: 0.0 },
    }],
    darkenings: vec![SkyDarkening { ra_hours: 17.75, dec_deg: -29.0, radius_deg: 3.2, depth: 0.85 }],
    vanilla_stars: VanillaStars::Hide,
};
let midnight = SkyFrame { star_angle_deg: 180.0, star_brightness: 1.0, seconds: 0.0 };
let quads = frame_quads(&layer, &midnight);
assert_eq!(quads.len(), 1 + 4); // the star, then the darkening's four rings
assert!(quads[1].rgba[3] < quads[4].rgba[3]); // rings deepen toward the centre
let day = SkyFrame { star_brightness: 0.0, ..midnight };
assert!(frame_quads(&layer, &day).is_empty());
```

### `sky_darken`

`sky_darken(rain, thunder, day_time)` is `Level.getSkyDarken`: how much sky light the weather and time take off, from 0 (clear noon) to 11 (midnight in a thunderstorm). `rain` and `thunder` are the levels, 0.0 to 1.0, and `day_time` the world's time of day in ticks (it may run past 24000).

```rust
use pluto_lib::sky_darken;

assert_eq!(sky_darken(0.0, 0.0, 6000), 0);
assert_eq!(sky_darken(0.0, 0.0, 18000), 11);
assert!(sky_darken(1.0, 1.0, 6000) > sky_darken(1.0, 0.0, 6000));
```

### `SkyBlend`

How a textured layer mixes with what is drawn under it: the host's own custom-sky blends
(OptiFine's `blend=`). `Add` is the format's default.

```rust
use pluto_lib::SkyBlend;

assert_eq!(SkyBlend::default(), SkyBlend::Add);
assert_ne!(SkyBlend::Replace, SkyBlend::Alpha);
```

### `SkyNow`

The weather of a frame, as a textured layer's admission reads it: thunder is a storm's rain,
so it is only `Thunder`.

```rust
use pluto_lib::SkyNow;

let now = SkyNow::Clear;
assert_ne!(now, SkyNow::Thunder);
```

### `SkyWeather`

The weathers a textured layer shows in: `clear`, `rain` and `thunder`. Clear alone by
default, so a layer without a say hides in the rain, like the vanilla stars.

```rust
use pluto_lib::{SkyNow, SkyWeather};

let weather = SkyWeather::default();
assert!(weather.clear && !weather.rain && !weather.thunder);
assert!(weather.admits(SkyNow::Clear));
```

### `SkyWeather::admits`

Whether the layer shows in the frame's weather.

```rust
use pluto_lib::{SkyNow, SkyWeather};

let wet = SkyWeather { clear: false, rain: true, thunder: true };
assert!(!wet.admits(SkyNow::Clear));
assert!(wet.admits(SkyNow::Rain));
assert!(wet.admits(SkyNow::Thunder));
```

### `fade_ticks`

A clock time as ticks of the day, as the pack format's fade times are written: 6:00 is tick 0
(sunrise), 12:00 is 6000, 18:00 is 12000, 0:00 is 18000. `None` off the clock.

```rust
use pluto_lib::fade_ticks;

assert_eq!(fade_ticks(6, 0), Some(0));
assert_eq!(fade_ticks(18, 20), Some(12333));
assert_eq!(fade_ticks(24, 0), None);
```

### `SkyFade`

The fade times of a textured layer, in ticks of the day: `start_in` and `end_in` bound the
fade in, `start_out` and `end_out` the fade out; each span may cross midnight.

```rust
use pluto_lib::SkyFade;

let fade = SkyFade { start_in: 12000, end_in: 14000, start_out: 22000, end_out: 0 };
assert_eq!(fade.brightness(13000), 0.5);
```

### `SkyFade::new`

A fade from its in-times and its out end: the fade out starts as long before the end as the
fade in takes, as the pack format derives it when `startFadeOut` is absent.

```rust
use pluto_lib::SkyFade;

// In from 18:00 to 20:00, out by 6:00: the fade out starts at 4:00.
let fade = SkyFade::new(12000, 14000, 0);
assert_eq!(fade.start_out, 22000);
```

### `SkyFade::brightness`

The brightness at a day time (ticks; any day): 0 before the fade in, 1 between the fades, 0
after the fade out.

```rust
use pluto_lib::SkyFade;

let fade = SkyFade::new(12000, 14000, 0);
assert_eq!(fade.brightness(12000), 0.0);
assert_eq!(fade.brightness(18000), 1.0);
assert_eq!(fade.brightness(6000), 0.0);
```

### `SkyTextureLayer`

One textured sky layer a mod draws itself: its `png` skybox texture (six faces in a 3x2
grid), its `blend`, its `fade` times, whether it `rotate`s and at what `speed` around its
`axis`, the `weather` it shows in, and the `transition` its brightness eases over. Layers of
a world draw in registration order, over the pack's layers and under the dynamic layers'
quads.

```rust
use pluto_lib::{SkyBlend, SkyTextureLayer, SkyWeather};

let mut header = Vec::new();
header.extend_from_slice(b"\x89PNG\r\n\x1a\n");
header.extend_from_slice(&13u32.to_be_bytes());
header.extend_from_slice(b"IHDR");
header.extend_from_slice(&6u32.to_be_bytes());
header.extend_from_slice(&4u32.to_be_bytes());
header.extend_from_slice(&[8, 6, 0, 0, 0]);
let layer = SkyTextureLayer {
    png: header, blend: SkyBlend::Add, fade: None, rotate: true, speed: 1.0,
    axis: [0.0, 0.0, 1.0], weather: SkyWeather::default(), transition: 1.0,
};
assert_eq!(layer.speed, 1.0);
```

### `SkyTextureLayer::new`

A layer of just a texture, with the pack format's defaults: `add`, no fade, turning with the
sun about the south-north axis, clear weather only, a second's transition.

```rust
use pluto_lib::{SkyBlend, SkyTextureLayer};

let layer = SkyTextureLayer::new(vec![0x89, b'P', b'N', b'G']);
assert_eq!(layer.blend, SkyBlend::Add);
assert!(layer.rotate);
assert_eq!(layer.axis, [0.0, 0.0, 1.0]);
```

### `SkyTextureLayer::shows`

Whether the frame's weather admits the layer.

```rust
use pluto_lib::{SkyNow, SkyTextureLayer};

let layer = SkyTextureLayer::new(Vec::new());
assert!(layer.shows(SkyNow::Clear));
assert!(!layer.shows(SkyNow::Rain));
```

### `SkyTextureLayer::brightness`

The brightness of the fade times at a day time; a `replace` layer is all or nothing.

```rust
use pluto_lib::{SkyBlend, SkyFade, SkyTextureLayer};

let mut layer = SkyTextureLayer::new(Vec::new());
layer.blend = SkyBlend::Replace;
layer.fade = Some(SkyFade::new(0, 2000, 14000));
assert_eq!(layer.brightness(500), 1.0, "half way in, yet full");
assert_eq!(layer.brightness(20000), 0.0);
```

### `SkyTextureLayer::turns`

How far the layer has turned around its axis, in turns, when the sun stands at `sun_turns`:
a turn a day at speed 1, counted from noon of day 0, so a slower speed carries on across
days.

```rust
use pluto_lib::SkyTextureLayer;

let layer = SkyTextureLayer::new(Vec::new());
assert_eq!(layer.turns(12000, 0.25), 0.25);
let mut still = SkyTextureLayer::new(Vec::new());
still.rotate = false;
assert_eq!(still.turns(12000, 0.25), 0.0);
```

### `png_size`

The size a PNG declares in its header, without decoding it: the registry reads sizes to hold
layers to the host's texture budget before any decode is paid for.

```rust
use pluto_lib::png_size;

let mut header = Vec::new();
header.extend_from_slice(b"\x89PNG\r\n\x1a\n");
header.extend_from_slice(&13u32.to_be_bytes());
header.extend_from_slice(b"IHDR");
header.extend_from_slice(&3072u32.to_be_bytes());
header.extend_from_slice(&2048u32.to_be_bytes());
header.extend_from_slice(&[8, 6, 0, 0, 0]);
assert_eq!(png_size(&header), Some((3072, 2048)));
assert_eq!(png_size(&[]), None);
```

### `SkyTextureRegistry`

The registered textured sky layers, by world number (the host's custom-sky numbering: 0 the
Overworld, 1 the End). Registration is process-lifetime; a world's layers draw in
registration order. Layers are read when the game assembles a pack, so a mod registers its
layers at startup, from its hooks.

```rust
use pluto_lib::{SkyTextureLayer, SkyTextureRegistry};

let mut header = Vec::new();
header.extend_from_slice(b"\x89PNG\r\n\x1a\n");
header.extend_from_slice(&13u32.to_be_bytes());
header.extend_from_slice(b"IHDR");
header.extend_from_slice(&6u32.to_be_bytes());
header.extend_from_slice(&4u32.to_be_bytes());
header.extend_from_slice(&[8, 6, 0, 0, 0]);
SkyTextureRegistry::register(81, SkyTextureLayer::new(header)).unwrap();
assert_eq!(SkyTextureRegistry::layers(81).len(), 1);
```

### `SkyTextureRegistry::register`

Appends a layer to a world's textured layers. The layer is validated whole first — its PNG
header must declare a size, the registry's whole census must stay within the host's
custom-sky texture budget (64M texels), fade ticks on the day, a finite speed, an axis that
is not the zero vector (it is normalised), a transition that is not negative — so a refused
layer changes nothing.

```rust
use pluto_lib::{SkyTextureLayer, SkyTextureRegistry};

let mut header = Vec::new();
header.extend_from_slice(b"\x89PNG\r\n\x1a\n");
header.extend_from_slice(&13u32.to_be_bytes());
header.extend_from_slice(b"IHDR");
header.extend_from_slice(&6u32.to_be_bytes());
header.extend_from_slice(&4u32.to_be_bytes());
header.extend_from_slice(&[8, 6, 0, 0, 0]);
SkyTextureRegistry::register(82, SkyTextureLayer::new(header)).unwrap();
let mut bad = SkyTextureLayer::new(Vec::new());
bad.axis = [0.0, 0.0, 0.0];
assert!(SkyTextureRegistry::register(82, bad).is_err());
assert_eq!(SkyTextureRegistry::layers(82).len(), 1, "the refusal added nothing");
```

### `SkyTextureRegistry::layers`

The textured layers registered for a world, in drawing order.

```rust
use pluto_lib::SkyTextureRegistry;

assert!(SkyTextureRegistry::layers(83).is_empty());
```

### `SkyTextureRegistry::registered_worlds`

Every world that has textured layers, sorted: the registry's own census, for tests and
tools.

```rust
use pluto_lib::{SkyTextureLayer, SkyTextureRegistry};

let mut header = Vec::new();
header.extend_from_slice(b"\x89PNG\r\n\x1a\n");
header.extend_from_slice(&13u32.to_be_bytes());
header.extend_from_slice(b"IHDR");
header.extend_from_slice(&6u32.to_be_bytes());
header.extend_from_slice(&4u32.to_be_bytes());
header.extend_from_slice(&[8, 6, 0, 0, 0]);
SkyTextureRegistry::register(84, SkyTextureLayer::new(header)).unwrap();
assert!(SkyTextureRegistry::registered_worlds().contains(&84));
```

### `SkyTextureConditions`

What one frame asks the textured layers for: `day_time` (ticks of the day, counting the
days) and the frame's `weather`. (Biome and height gating stay with pack layers; a mod layer
that needs them is a pack layer.)

```rust
use pluto_lib::{SkyNow, SkyTextureConditions};

let noon = SkyTextureConditions { day_time: 6000, weather: SkyNow::Clear };
assert_eq!(noon.day_time, 6000);
```

### `SkyTextureDraw`

One textured layer of a frame, ready to draw: its `image` (a texture index over every
registered layer's texture, in census order), its `blend`, its `brightness`, and its turning
— the `axis` it turns around and the `turns` it has turned.

```rust
use pluto_lib::{SkyNow, SkyTextureConditions, SkyTextureLayer, texture_draws};

let layer = SkyTextureLayer::new(Vec::new());
let conditions = SkyTextureConditions { day_time: 6000, weather: SkyNow::Clear };
let mut eased = Vec::new();
let draws = texture_draws(&[layer], 3, &conditions, 0.0, 0.0, true, &mut eased);
assert_eq!(draws.len(), 1);
assert_eq!(draws[0].image, 3);
assert_eq!(draws[0].brightness, 1.0);
assert_eq!(draws[0].turns, 0.0);
```

### `texture_draws`

The textured draws of one world's layers for one frame: each layer the weather admits eases
toward full brightness over its transition (a layer the weather refuses eases out), is lit by
its fade times, and turns as its rotation says. `first_image` is the texture index of the
world's first layer over the whole census; `eased` is the caller's per-layer eased
brightness, kept between frames — a length mismatch, or `snap`, lands every layer on its
target at once, as the host's own evaluation does on a world change.

```rust
use pluto_lib::{SkyNow, SkyTextureConditions, SkyTextureLayer, texture_draws};

let mut layer = SkyTextureLayer::new(Vec::new());
layer.transition = 2.0;
let layers = [layer];
let clear = SkyTextureConditions { day_time: 6000, weather: SkyNow::Clear };
let mut eased = Vec::new();
let draws = texture_draws(&layers, 0, &clear, 0.0, 0.05, true, &mut eased);
assert_eq!(draws[0].brightness, 1.0, "arrival snaps");
// Rain refuses the layer (clear only): it eases out over its two seconds.
let rain = SkyTextureConditions { weather: SkyNow::Rain, ..clear };
let draws = texture_draws(&layers, 0, &rain, 0.0, 0.5, false, &mut eased);
assert_eq!(draws[0].brightness, 0.75);
```
