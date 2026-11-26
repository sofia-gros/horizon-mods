// What the game hands the mods when a pack's custom sky is loaded: the pack's own layers,
// wrapped with the textured layers the mods registered for themselves (a mod's own skybox
// textures, drawn over the pack's layers and under the dynamic layers' quads). The facts that
// cross are plain data — decoded pixels and per-frame numbers — so this module's sink names no
// registry; the composite below is the only host-typed piece, and it speaks the engine's own
// sky-layer terms because it lives in the engine's custom-sky module.
use std::sync::{Arc, Mutex, OnceLock};
#[derive(Clone, Debug)]
pub struct TexImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}
#[derive(Clone, Copy, Debug)]
pub struct TexDraw {
    // The layer's texture over every registered layer's textures, in census order.
    pub image: u16,
    // The blend, in the engine's order (the pack format's `blend=` list).
    pub blend: u8,
    pub brightness: f32,
    pub axis: [f32; 3],
    pub turns: f32,
}
#[derive(Clone, Copy)]
pub struct Sink {
    // Whether any textured layers are registered at all.
    pub any: fn() -> bool,
    // Every registered layer's decoded texture, in census order.
    pub images: fn() -> Vec<TexImage>,
    // Whether `world` has registered layers.
    pub has: fn(world: i32) -> bool,
    // Fills `out` with the frame's draws for `world` (weather: 0 clear, 1 rain, 2 thunder);
    // `eased` is the caller's per-layer eased brightness, kept between frames, and `snap`
    // asks for every layer to land on its target at once (a world change).
    pub draws: fn(world: i32, day_time: i64, weather: u8, sun_turns: f32, seconds: f32, snap: bool, eased: &mut Vec<f32>, out: &mut Vec<TexDraw>),
}
static SINK: OnceLock<Sink> = OnceLock::new();
pub fn set_sink(sink: Sink) -> Result<(), &'static str> { SINK.set(sink).map_err(|_| "sky texture sink already installed") }
pub fn sink() -> Option<&'static Sink> { SINK.get() }
// The pack's layers and the mods' layers as one: the images are the pack's first (its draw
// indices do not move), then the mods'; a frame's draws are the pack's, then the mods' with
// their image indices shifted past the pack's. The pack keeps the game's own eased state;
// the mods' easing lives here, with the world it belongs to, so the two never mix.
struct Composite {
    pack: Option<Arc<dyn super::SkyLayers>>,
    images: Vec<super::Image>,
    pack_images: usize,
    sink: Sink,
    eased: Mutex<(Option<i32>, Vec<f32>)>,
}
fn blend_of(blend: u8) -> super::Blend {
    match blend {
        1 => super::Blend::Subtract,
        2 => super::Blend::Multiply,
        3 => super::Blend::Dodge,
        4 => super::Blend::Burn,
        5 => super::Blend::Screen,
        6 => super::Blend::Replace,
        7 => super::Blend::Overlay,
        8 => super::Blend::Alpha,
        _ => super::Blend::Add,
    }
}
fn weather_of(weather: super::Weather) -> u8 {
    match weather {
        super::Weather::Clear => 0,
        super::Weather::Rain => 1,
        super::Weather::Thunder => 2,
    }
}
impl super::SkyLayers for Composite {
    fn images(&self) -> &[super::Image] {
        &self.images
    }
    fn has(&self, world: i32) -> bool {
        self.pack.as_ref().is_some_and(|pack| pack.has(world)) || (self.sink.has)(world)
    }
    fn draws(
        &self,
        world: i32,
        conditions: &super::Conditions,
        sun_turns: f32,
        seconds: f32,
        state: &mut super::State,
        out: &mut Vec<super::Draw>,
    ) {
        if let Some(pack) = &self.pack {
            pack.draws(world, conditions, sun_turns, seconds, state, out);
        }
        let mut guard = self.eased.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let snap = guard.0 != Some(world);
        if snap {
            guard.0 = Some(world);
        }
        let mut tex = Vec::new();
        (self.sink.draws)(
            world,
            conditions.day_time,
            weather_of(conditions.weather),
            sun_turns,
            seconds,
            snap,
            &mut guard.1,
            &mut tex,
        );
        for draw in tex {
            let rotation = if draw.turns == 0.0 {
                glam::Mat4::IDENTITY
            } else {
                glam::Mat4::from_axis_angle(glam::Vec3::from_array(draw.axis), draw.turns * std::f32::consts::TAU)
            };
            out.push(super::Draw {
                image: self.pack_images as u16 + draw.image,
                blend: blend_of(draw.blend),
                brightness: draw.brightness,
                rotation,
            });
        }
    }
}
// The pack's layers (or none: the game's own empty answer when a pack has no sky) wrapped
// with the registered layers the sink serves.
pub fn wrap(pack: Option<Arc<dyn super::SkyLayers>>, sink: Sink) -> Arc<dyn super::SkyLayers> {
    let mut images: Vec<super::Image> = Vec::new();
    if let Some(pack) = &pack {
        images.extend(pack.images().iter().map(|image| super::Image {
            width: image.width,
            height: image.height,
            rgba: image.rgba.clone(),
        }));
    }
    let pack_images = images.len();
    images.extend((sink.images)().into_iter().map(|image| super::Image {
        width: image.width,
        height: image.height,
        rgba: image.rgba,
    }));
    Arc::new(Composite { pack, images, pack_images, sink, eased: Mutex::new((None, Vec::new())) })
}
