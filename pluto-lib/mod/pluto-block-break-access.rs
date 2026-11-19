// What the game asks the mods when a player's break of a block has ended: the API side installs one sink, and the end of the break calls it with
// the game as the host, so the callbacks can read and change the player the way every other native hook lets them.
use horizon_client::mod_api::Host;
use std::sync::OnceLock;
#[derive(Clone,Copy)]
pub struct Sink {pub after:fn(&mut dyn Host,[i32;3],bool)}
static SINK:OnceLock<Sink>=OnceLock::new();
pub fn set_sink(sink:Sink)->Result<(),&'static str> {SINK.set(sink).map_err(|_|"block break sink already installed")}
pub fn sink()->Option<&'static Sink> {SINK.get()}
