//! The static layer's contents: the faint end of the sky as textured
//! layers — the real star sphere with the Milky Way (magnitudes 4.0–7.0),
//! the airglow band, and the zodiacal light — painted by the Stellarsky
//! pipeline (`pipeline/make_skybox.py`) in the host's custom-sky layout
//! (one PNG, six faces in a 3x2 grid). These are the companion pack's three
//! layers with their own semantics, registered by the mod itself through
//! PlutoLib's sky surface: the pack existed because the host's scanner
//! reads custom skies only from a pack's `minecraft` namespace, which a
//! mod package may not claim; the surface carries the textures in the
//! native half instead, so the pack is no longer needed for the sky.

use pluto_lib::{SkyFade, SkyTextureLayer, SkyTextureRegistry, fade_ticks};

/// The star sphere: magnitudes 4.0–7.0 and the Milky Way with its dark
/// lanes, painted at the dynamic layer's phase (`stars::MERIDIAN_RA_HOURS`).
const STAR_SPHERE: &[u8] = include_bytes!("../data/sky1.png");
/// The airglow: the upper atmosphere's own glow, a green band standing
/// some ten degrees over the horizon all night.
const AIRGLOW: &[u8] = include_bytes!("../data/sky2.png");
/// The zodiacal light: sunlight on interplanetary dust, a cone standing on
/// the sun along the ecliptic, with its faint band and the gegenschein.
const ZODIACAL: &[u8] = include_bytes!("../data/sky3.png");

/// The night fade the star sphere and the airglow share: in after sunset,
/// out before sunrise (the pack files' `startFadeIn` 18:20, `endFadeIn`
/// 19:20, `endFadeOut` 5:40; no weather key — clear only, like the vanilla
/// stars, so rain hides the layers).
fn night_fade() -> SkyFade {
    SkyFade::new(
        fade_ticks(18, 20).unwrap(),
        fade_ticks(19, 20).unwrap(),
        fade_ticks(5, 40).unwrap(),
    )
}

/// Register the Overworld's three static layers, in the pack's drawing
/// order (sky1, sky2, sky3). Registration appends, and the mod registers
/// once, from its hooks.
pub fn register() -> Result<(), String> {
    // The star sphere turns with the sun about the south-north axis, one
    // turn a day (the defaults): the painted sphere is the celestial
    // sphere, so it keeps pace with the game's own sky.
    let mut sphere = SkyTextureLayer::new(STAR_SPHERE.to_vec());
    sphere.fade = Some(night_fade());
    SkyTextureRegistry::register(0, sphere)?;
    // The airglow belongs to the horizon, not to the stars, so the layer
    // does not rotate: it stands world-fixed over the horizon.
    let mut airglow = SkyTextureLayer::new(AIRGLOW.to_vec());
    airglow.rotate = false;
    airglow.fade = Some(night_fade());
    SkyTextureRegistry::register(0, airglow)?;
    // The zodiacal cone is painted around the sun's fixed place in the
    // rotating frame, so this layer turns with the sun like the sphere and
    // the cone follows it; its window is the twilight (in 17:50–18:50, out
    // 5:10–6:10).
    let mut zodiacal = SkyTextureLayer::new(ZODIACAL.to_vec());
    zodiacal.fade = Some(SkyFade::new(
        fade_ticks(17, 50).unwrap(),
        fade_ticks(18, 50).unwrap(),
        fade_ticks(6, 10).unwrap(),
    ));
    SkyTextureRegistry::register(0, zodiacal)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::png_size;

    #[test]
    fn the_static_layers_carry_the_painted_textures() {
        assert_eq!(png_size(STAR_SPHERE), Some((3072, 2048)));
        assert_eq!(png_size(AIRGLOW), Some((1536, 1024)));
        assert_eq!(png_size(ZODIACAL), Some((3072, 2048)));
    }

    #[test]
    fn the_static_layers_register_whole_for_the_overworld() {
        // The exact call lib.rs's hooks make on the mod's behalf.
        register().unwrap();
        let layers = SkyTextureRegistry::layers(0);
        assert_eq!(layers.len(), 3);
        // The sphere and the zodiacal light turn with the sun; the airglow stands.
        assert!(layers[0].rotate && !layers[1].rotate && layers[2].rotate);
        assert_eq!(layers[0].axis, [0.0, 0.0, 1.0]);
        assert_eq!(layers[0].fade, Some(night_fade()));
        assert_eq!(layers[1].fade, Some(night_fade()));
        assert_eq!(layers[2].fade.unwrap().start_in, fade_ticks(17, 50).unwrap());
        assert_eq!(layers[2].fade.unwrap().end_out, fade_ticks(6, 10).unwrap());
    }
}
