// Transport for a thrown type's throw source: the game files every played sound under a source by its
// event (settings::sound_source), and a modded throw event matches none of the vanilla families, so it
// would play on MASTER where Java plays a gadget throw on NEUTRAL (SoundSource.NEUTRAL in the items'
// use). This sink lets the API side say whether an event is a registered thrown type's throw sound; the
// throw-source reach (pluto-throw-source, a RETURN on sound_source) files those under NEUTRAL. It is
// installed through the entry throw-sound sink (pluto-throw-sound-access), which forwards here - the
// API side installs one sink, and this crate is where the sound-source table lives.
use std::sync::OnceLock;

pub struct Sink {
    pub is_throw_sound: fn(&str) -> bool,
}

static SINK: OnceLock<Sink> = OnceLock::new();

pub fn set_sink(sink: Sink) -> Result<(), &'static str> {
    SINK.set(sink).map_err(|_| "throw source sink already installed")
}

pub fn is_throw_sound(event: &str) -> bool {
    SINK.get().is_some_and(|sink| (sink.is_throw_sound)(event))
}

/// The source a played sound belongs to: NEUTRAL for a registered throw sound, the game's own
/// filing otherwise. The index is looked up in the table by name, never hardcoded.
pub(crate) fn throw_source(value: usize, event: &str) -> usize {
    if is_throw_sound(event) {
        crate::settings::SOUND_SOURCES.iter().position(|name| *name == "neutral").unwrap_or(value)
    } else {
        value
    }
}
