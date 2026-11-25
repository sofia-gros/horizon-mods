// Transport for the stack a mod's item-wear callback puts in place of a worn armor piece: the API side sets it while the
// callbacks run, and the wear step in the game takes it right after.
use std::sync::Mutex;
static PENDING:Mutex<Option<sunrise_client::items::ItemStack>>=Mutex::new(None);
pub fn set(stack:sunrise_client::items::ItemStack) {if let Ok(mut pending)=PENDING.lock() {*pending=Some(stack);}}
pub fn take()->Option<sunrise_client::items::ItemStack> {PENDING.lock().ok().and_then(|mut pending|pending.take())}
