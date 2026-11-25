//! Stellarsky for Horizon: a real night sky over the game. The faint end —
//! stars of magnitude 4 to 7 and the Milky Way — is a content package of
//! custom-sky textures; the bright end cannot be a texture, because a texture
//! cannot twinkle and cannot darken what another layer drew. So this native
//! half registers the bright end as a dynamic sky layer through PlutoLib's
//! public sky surface: the 523 stars of magnitude 4 and brighter, each with
//! its spectral colour and its own scintillation, the dark patch of
//! Sagittarius A* over the galactic centre, and no vanilla stars — this field
//! replaces them. Nothing here injects; the layer is data handed to PlutoLib,
//! exactly as Tinkers' Construct and Pipes hand it their declarations.
mod stars;
mod stars_table;

pub fn hooks() -> pluto_lib::ModHooks {
    // The registry validates the layer whole before taking it, and the
    // crate's tests pin that this exact layer passes; a refusal would leave
    // the sky as the content package alone draws it.
    let _ = register();
    pluto_lib::no_hooks()
}

/// Register the Overworld's dynamic sky layer (world 0, the world PlutoLib's
/// sky transport serves). Re-registering replaces the layer.
pub fn register() -> Result<(), String> {
    pluto_lib::SkyRegistry::register(0, stars::layer())
}
