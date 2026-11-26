//! The static layer's contents: the faint end of the sky as textured
//! layers — the real star sphere with the Milky Way (magnitudes 4.0–7.0),
//! the airglow band, and the zodiacal light.
//!
//! SUNRISE v1.0.0 LIMITATION: PlutoLib's Sunrise sky surface offers the
//! dynamic layer (`SkyRegistry` — the bright stars, drawn star by star,
//! which `stars.rs` registers) but no texture-layer registry: the
//! Horizon edition's `SkyTextureRegistry` / `SkyTextureLayer` / `SkyFade`
//! have no Sunrise counterpart in PlutoLib v1.0.0, and the native-mod
//! tool stages only `.rs` sources, so the `include_bytes!` textures
//! cannot even be carried into the build. Registration is therefore a
//! documented no-op on Sunrise for now; the three textures stay in
//! `src/data/` for the release that restores them. The dynamic layer
//! (twinkling bright stars, the Sagittarius A* dark patch, vanilla
//! stars suppressed) is unaffected.

/// Register the Overworld's static texture layers. On Sunrise v1.0.0
/// there is no texture-layer registry to register them with; see the
/// module note. Kept as the one call site the mod's hooks invoke.
pub fn register() -> Result<(), String> {
    Ok(())
}
