// What the game asks the mods as it builds a frame's celestial bodies: the quads of the dynamic
// sky layers registered for the world (bright stars that twinkle, and the dark patches), drawn
// with the sun and moon, and whether the vanilla procedural stars may stay. The facts are plain
// numbers — corners on the celestial sphere and a colour — so this module names no rendering
// type; the reach that calls the sink (pluto-sky-celestial) turns the quads into vertices.
use std::sync::OnceLock;
#[derive(Clone, Copy, Debug)]
pub struct SkyQuad {
    pub corners: [[f32; 3]; 4],
    pub rgba: [f32; 4],
}
#[derive(Clone, Copy)]
pub struct Sink {
    // Fills `out` with the frame's quads and answers whether the vanilla star frame is dropped.
    pub frame: fn(star_angle: f32, star_brightness: f32, out: &mut Vec<SkyQuad>) -> bool,
}
static SINK: OnceLock<Sink> = OnceLock::new();
pub fn set_sink(sink: Sink) -> Result<(), &'static str> { SINK.set(sink).map_err(|_| "sky sink already installed") }
pub fn sink() -> Option<&'static Sink> { SINK.get() }
