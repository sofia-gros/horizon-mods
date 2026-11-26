// Map faces a mod's screen or HUD drawing asked for (a HUD minimap's map), waiting for the
// flush at the end of the HUD's draw (pluto-map-face): the drawing itself runs through the
// mod API's canvas, which holds only the draw batch, while a map's face is rendered from its
// texture by the game.
use std::sync::Mutex;

use sunrise_items::items::ItemStack;

/// One queued map face: the filled map stack whose face is to be drawn into `rect`
/// (`[x, y, width, height]` in GUI pixels).
pub struct QueuedMapFace {
    pub rect: [f32; 4],
    pub stack: ItemStack,
}

static QUEUE: Mutex<Vec<QueuedMapFace>> = Mutex::new(Vec::new());

/// Queues a map face for the flush at the end of the current HUD draw.
pub fn queue(face: QueuedMapFace) {
    if let Ok(mut queue) = QUEUE.lock() {
        queue.push(face);
    }
}

/// Takes every queued map face, leaving the queue empty.
pub fn take_queued() -> Vec<QueuedMapFace> {
    QUEUE.lock().map(|mut queue| std::mem::take(&mut *queue)).unwrap_or_default()
}
