// Transport for a thrown type's throw sound: the game plays the vanilla snowball throw for every thrown type
// (Game::throw_item), so this sink lets the API side answer the sound a type throws with instead. The game side
// (pluto-throw-sound-game, in the game's creatures module) asks through it from the throw-sound reach, in plain
// data; no sound is invented here - a type with no registered throw sound keeps the game's own sound. The sink
// also answers whether an event is a registered throw sound, which the throw plays' position and source reaches
// (pluto-throw-sound-position, pluto-throw-source) need: installing the sink forwards that answer into the render
// crate's own sink (pluto-throw-source-access), where the game's sound-source table lives.
use std::sync::OnceLock;

pub struct Sink {
    pub throw_sound: fn(&str) -> Option<&'static str>,
    pub is_throw_sound: fn(&str) -> bool,
}

static SINK: OnceLock<Sink> = OnceLock::new();

pub fn set_sink(sink: Sink) -> Result<(), &'static str> {
    horizon_client_render::settings::pluto_throw_source_access::set_sink(horizon_client_render::settings::pluto_throw_source_access::Sink {
        is_throw_sound: sink.is_throw_sound,
    })?;
    SINK.set(sink).map_err(|_| "throw sound sink already installed")
}

pub fn throw_sound(id: &str) -> Option<&'static str> {
    SINK.get().and_then(|sink| (sink.throw_sound)(id))
}
