// What the game asks the mods when a mob works out how far it sees a player: the factor the player's worn gear puts on that range for
// that kind of mob (a disguise halves it). The answer is 1 when nothing applies.
use std::sync::OnceLock;
#[derive(Clone,Copy)]
pub struct Sink {pub factor:fn(u128,&str)->f64}
static SINK:OnceLock<Sink>=OnceLock::new();
pub fn set_sink(sink:Sink)->Result<(),&'static str> {SINK.set(sink).map_err(|_|"player visibility sink already installed")}
pub fn sink()->Option<&'static Sink> {SINK.get()}
