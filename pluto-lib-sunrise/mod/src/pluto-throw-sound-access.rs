// Transport for a thrown type's throw sound: the game plays the vanilla snowball throw for every thrown type
// (Game::throw_item), so this sink lets the API side answer the sound a type throws with instead. The game side
// (pluto-throw-sound-game, in the game's creatures module) asks through it from the throw-sound reach, in plain
// data; no sound is invented here - a type with no registered throw sound keeps the game's own sound.
use std::sync::OnceLock;

pub struct Sink {
    pub throw_sound: fn(&str) -> Option<&'static str>,
}

static SINK: OnceLock<Sink> = OnceLock::new();

pub fn set_sink(sink: Sink) -> Result<(), &'static str> {
    SINK.set(sink).map_err(|_| "throw sound sink already installed")
}

pub fn throw_sound(id: &str) -> Option<&'static str> {
    SINK.get().and_then(|sink| (sink.throw_sound)(id))
}
