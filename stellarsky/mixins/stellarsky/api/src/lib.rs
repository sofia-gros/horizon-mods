//! Stellarsky for Horizon: a real night sky over the game. The bright end
//! cannot be a texture, because a texture cannot twinkle and cannot darken
//! what another layer drew, so this native half registers it as a dynamic
//! sky layer through PlutoLib's public sky surface: the 523 stars of
//! magnitude 4 and brighter, each with its spectral colour and its own
//! scintillation, the dark patch of Sagittarius A* over the galactic
//! centre, and no vanilla stars — this field replaces them. The faint end —
//! stars of magnitude 4 to 7 and the Milky Way — is a texture, and it
//! registers through the same surface as textured layers (static_sky), so
//! the whole sky is data this native half hands to PlutoLib and no resource
//! pack is needed for it. Nothing here injects, exactly as Tinkers'
//! Construct and Pipes hand PlutoLib their declarations.
mod stars;
mod stars_table;
mod static_sky;

pub fn hooks() -> pluto_lib::ModHooks {
    // The registries validate each layer whole before taking it, and the
    // crate's tests pin that these exact layers pass; a refusal would leave
    // that half of the sky to whatever else draws it.
    let _ = register();
    let _ = static_sky::register();
    pluto_lib::no_hooks()
}

/// Register the Overworld's dynamic sky layer (world 0, the world PlutoLib's
/// sky transport serves). Re-registering replaces the layer.
pub fn register() -> Result<(), String> {
    pluto_lib::SkyRegistry::register(0, stars::layer())
}
