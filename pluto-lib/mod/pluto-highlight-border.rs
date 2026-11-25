// The world border of the level in place, as the game's own border last stood when the game ticked: the highlight transport reads it where the host's border read states none — off a server, where a remote client's border is the one its server syncs to it, and in dimensions the host read does not name. The bounds are the game border's own (`WorldBorder::bounds`): [min_x, min_z, max_x, max_z], the previous size while the border moves. None before the game first ticks.
use std::sync::{LazyLock, Mutex};

static BORDER: LazyLock<Mutex<Option<[f64; 4]>>> = LazyLock::new(|| Mutex::new(None));

pub fn set_border(bounds: [f64; 4]) {
    if let Ok(mut border) = BORDER.lock() {
        *border = Some(bounds);
    }
}

pub fn border() -> Option<[f64; 4]> {
    BORDER.lock().ok().and_then(|border| *border)
}
