//! Pipes for Horizon: item and fluid transport pipes that work against Tinkers' Construct's
//! furnaces, tanks, casting blocks and chests (and the vanilla chest) over PlutoLib's public
//! neighbour reach. A clean-room design in the classic extraction-pipe style; no other mod's
//! code or assets are used, and nothing here injects — every reach into the game goes through
//! PlutoLib, exactly as Tinkers' Construct itself does.
mod common;
mod connections;
mod fluid_pipe;
mod item_pipe;

pub fn hooks() -> pluto_lib::ModHooks {
    item_pipe::register();
    fluid_pipe::register();
    connections::register();
    pluto_lib::no_hooks()
}
