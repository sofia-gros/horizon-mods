# Sky

A mod draws the bright end of its sky itself: stars too bright for a texture to do justice, each
with its own colour and its own twinkle, and dark patches — a black hole's shadow — laid over the
glow beneath. The faint end of the sky stays a pack custom-sky texture (the host's OptiFine-format
layers); this surface is the part a texture cannot do, because a texture cannot flicker and cannot
darken what another layer drew.

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

## Reference

### `Twinkle`

![Horizon: inert](badges/horizon-inert.svg) ![Sunrise: works](badges/sunrise-works.svg)

How one star's light flickers: `amplitude` is how far the light dips (0 is a steady star, 1 goes out at the trough), `rate` is roughly cycles per second, and `phase` is where in its cycle the star starts, in turns.

> The flicker is the air's, not the star's — scintillation — and it strengthens of itself
> low in the sky when the frame is built.

```rust
use pluto_lib::Twinkle;

let steady = Twinkle { amplitude: 0.0, rate: 1.0, phase: 0.0 };
let lively = Twinkle { amplitude: 0.45, rate: 1.8, phase: 0.25 };
assert!(lively.amplitude > steady.amplitude);
```

### `SkyStar`

![Horizon: inert](badges/horizon-inert.svg) ![Sunrise: works](badges/sunrise-works.svg)

One bright star of a layer.

> `ra_hours` and `dec_deg` are its equatorial position, `magnitude` its visual magnitude
> (the brightness and size curves are keyed to it), `rgb` its linear colour (a spectral
> class colour, as the catalogs give it), and `twinkle` its scintillation.

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

![Horizon: inert](badges/horizon-inert.svg) ![Sunrise: works](badges/sunrise-works.svg)

A patch of sky darker than its surroundings: the centre is `ra_hours` / `dec_deg`, `radius_deg` is its radius in degrees, and `depth` is how dark the centre gets (0 is nothing, 1 is black).

> It is drawn as concentric quads of rising alpha, so the edge is soft: a black hole at
> the galactic centre darkens the bright core around it this way.

```rust
use pluto_lib::SkyDarkening;

let sagittarius_a = SkyDarkening { ra_hours: 17.75, dec_deg: -29.0, radius_deg: 3.2, depth: 0.85 };
assert!(sagittarius_a.depth > 0.5);
```

### `VanillaStars`

![Horizon: inert](badges/horizon-inert.svg) ![Sunrise: works](badges/sunrise-works.svg)

Whether the host's vanilla procedural stars show in a world that has a registered layer.

> The vanilla field is a fixed buffer with one scalar brightness — no per-star colour, no
> twinkle — and every custom-sky layer draws before it, so no texture can cover it; a mod
> that brings its own star field says `Hide`, and one that only adds to the sky says
> `Keep` (the default).

```rust
use pluto_lib::VanillaStars;

assert_eq!(VanillaStars::default(), VanillaStars::Keep);
assert_ne!(VanillaStars::Hide, VanillaStars::Keep);
```

### `SkyLayer`

![Horizon: inert](badges/horizon-inert.svg) ![Sunrise: works](badges/sunrise-works.svg)

One mod's dynamic sky for one world.

> `meridian_ra_hours` is the right ascension standing on the meridian when the host's star
> angle is 0 (noon) — the phase the layer's positions are read against, the same number a
> static sky texture is painted with; `stars` are its bright stars; `darkenings` its dark
> patches, drawn after the stars; and `vanilla_stars` its say over the host's own star
> field.

```rust
use pluto_lib::{SkyLayer, VanillaStars};

let layer = SkyLayer { meridian_ra_hours: 5.75, stars: vec![], darkenings: vec![], vanilla_stars: VanillaStars::Hide };
assert_eq!(layer.meridian_ra_hours, 5.75);
assert!(layer.stars.is_empty());
```

### `SkyRegistry`

![Horizon: inert](badges/horizon-inert.svg) ![Sunrise: works](badges/sunrise-works.svg)

The registered dynamic sky layers, by world number (the host's custom-sky numbering: 0 the Overworld, 1 the End).

> Registration is process-lifetime: a layer stays until the process ends or another
> registration for the same world replaces it. There is no unregister — replace a world's
> layer by registering a new one for its number.

```rust
use pluto_lib::{SkyLayer, SkyRegistry, VanillaStars};

SkyRegistry::register(71, SkyLayer {
    meridian_ra_hours: 0.0, stars: vec![], darkenings: vec![], vanilla_stars: VanillaStars::Keep,
}).unwrap();
assert_eq!(SkyRegistry::layer(71).map(|layer| layer.meridian_ra_hours), Some(0.0));
```

### `SkyRegistry::register`

![Horizon: inert](badges/horizon-inert.svg) ![Sunrise: works](badges/sunrise-works.svg)

Registers a layer for a world, replacing any layer that world had.

> The layer is validated whole first — positions in range, colours finite and
> non-negative, a twinkle's amplitude at most 1 and rate at most 10, a darkening's radius
> in (0, 45] and depth at most 1, at most 4096 stars — so a refused layer leaves the
> previous registration intact. Right ascensions are taken modulo 24 hours.

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

![Horizon: inert](badges/horizon-inert.svg) ![Sunrise: works](badges/sunrise-works.svg)

The layer registered for a world, or None when no mod registered one.

> World numbers follow the registry's own numbering (0 the Overworld, 1 the End), the same
> numbers `register` takes. A world nobody registered a layer for answers `None`.

```rust
use pluto_lib::SkyRegistry;

assert_eq!(SkyRegistry::layer(73), None);
```

### `SkyRegistry::registered_worlds`

![Horizon: inert](badges/horizon-inert.svg) ![Sunrise: works](badges/sunrise-works.svg)

Every world that has a layer, sorted.

> The numbers come back sorted, one per world with a registered layer. It is the
> registry's own census — for tests and tools that need to know what is registered without
> probing world by world.

```rust
use pluto_lib::{SkyLayer, SkyRegistry, VanillaStars};

SkyRegistry::register(74, SkyLayer {
    meridian_ra_hours: 0.0, stars: vec![], darkenings: vec![], vanilla_stars: VanillaStars::Keep,
}).unwrap();
assert!(SkyRegistry::registered_worlds().contains(&74));
```

### `SkyFrame`

![Horizon: inert](badges/horizon-inert.svg) ![Sunrise: works](badges/sunrise-works.svg)

What one frame asks a layer for.

> `star_angle_deg` is the host's star angle for the frame in degrees (0 at noon, 180 at
> midnight), `star_brightness` is the host's own star brightness (its twilight and weather
> fade, 0 to 1), and `seconds` is the time the twinkles are read at — seconds since an
> arbitrary fixed point; only differences matter.

```rust
use pluto_lib::SkyFrame;

let midnight = SkyFrame { star_angle_deg: 180.0, star_brightness: 1.0, seconds: 42.0 };
assert_eq!(midnight.star_angle_deg, 180.0);
```

### `SkyQuad`

![Horizon: inert](badges/horizon-inert.svg) ![Sunrise: works](badges/sunrise-works.svg)

One quad of a frame's layer, ready to draw.

> `corners` are its four corners on the celestial sphere, at the celestial bodies' own
> distance (100), in the sun quad's corner order, and `rgba` is its colour — a star's
> light in RGB with its brightness in A, a darkening's black with its alpha.

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

![Horizon: inert](badges/horizon-inert.svg) ![Sunrise: works](badges/sunrise-works.svg)

The world direction of an equatorial position at a star angle, on the game's celestial sphere (an equatorial observer's: the poles stand at the north and south horizon points).

> The result is a unit vector, x east, y up, z south. At midnight with a meridian right
> ascension of 5.75 h, the galactic centre (17.75 h, −29°) stands high in the south:

```rust
use pluto_lib::celestial_direction;

let d = celestial_direction(17.75, -29.0, 5.75, 180.0);
assert!((d[1] - 0.8746).abs() < 0.001);
assert!((d[2] - 0.4848).abs() < 0.001);
assert!(d[0].abs() < 0.001);
```

### `twinkle_factor`

![Horizon: inert](badges/horizon-inert.svg) ![Sunrise: works](badges/sunrise-works.svg)

One star's twinkle factor at a time: 1 at the crest, down to `1 - amplitude` at the trough.

> Two detuned sine waves are summed so the flicker does not read as a mechanism; a star
> with no amplitude (or no rate) is steady at 1.

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

![Horizon: inert](badges/horizon-inert.svg) ![Sunrise: works](badges/sunrise-works.svg)

The quads of a layer for one frame: its stars first — each a billboard on the celestial sphere, sized and lit by its magnitude, dimmed by the frame's star brightness, by the air near the horizon (extinction), and by its twinkle, which strengthens low in the sky — then its darkenings' rings, so a dark patch falls over the layer's own stars as well as the sky beneath.

> Stars below the horizon and quads too faint to see are not emitted; a darkening's alpha
> follows the frame's star brightness, so the patch stays out of the day sky.

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

![Horizon: absent](badges/horizon-absent.svg) ![Sunrise: works](badges/sunrise-works.svg)

How much sky light the weather and time take off, from 0 (clear noon) to 11 (midnight in a thunderstorm).

`sky_darken(rain, thunder, day_time)`

> It is the game's own `Level.getSkyDarken`. `rain` and `thunder` are the levels, 0.0 to
> 1.0, and `day_time` the world's time of day in ticks (it may run past 24000).

```rust
use pluto_lib::sky_darken;

assert_eq!(sky_darken(0.0, 0.0, 6000), 0);
assert_eq!(sky_darken(0.0, 0.0, 18000), 11);
assert!(sky_darken(1.0, 1.0, 6000) > sky_darken(1.0, 0.0, 6000));
```
