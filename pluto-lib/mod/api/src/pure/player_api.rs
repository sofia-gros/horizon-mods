//! A handle to a connected player, the counterpart of Fabric's `ServerPlayerEntity` for what mods do most: tell the player something and
//! give them items.
//!
//! The handle is plain data (uuid and name). Its methods reach the game through a *scope* that PlutoLib opens around every callback
//! that receives a player ([`with_ops`]); called outside such a scope they do nothing and report failure, so a stored handle can never touch
//! the game from the wrong place.

use std::cell::Cell;
use std::marker::PhantomData;

/// What the game can do for a player. Implemented once, in `hooks.rs`, over Horizon's `Host`.
pub trait PlayerOps {
    /// Sends a system chat line to the player; false when the player is not online.
    fn tell(&mut self, player: u128, line: &str) -> bool;
    /// Gives items to the player; returns how many did not fit.
    fn give(&mut self, player: u128, item: &str, count: u32) -> u32;
}

thread_local! {
    static OPS: Cell<Option<*mut dyn PlayerOps>> = const { Cell::new(None) };
}

/// Restores the previous scope when dropped, also on panic.
struct Scope<'a> {
    previous: Option<*mut dyn PlayerOps>,
    _borrow: PhantomData<&'a mut dyn PlayerOps>,
}

impl Drop for Scope<'_> {
    fn drop(&mut self) {
        OPS.with(|slot| slot.set(self.previous));
    }
}

/// Runs `f` with `ops` available to every [`ServerPlayer`] method called (on this thread) during `f`.
pub fn with_ops<R>(ops: &mut dyn PlayerOps, f: impl FnOnce() -> R) -> R {
    // SAFETY: the pointer is only dereferenced while `f` runs, strictly inside the lifetime of `ops`; `Scope` clears it on every exit.
    let raw: *mut dyn PlayerOps = unsafe { std::mem::transmute::<&mut dyn PlayerOps, *mut dyn PlayerOps>(ops) };
    let previous = OPS.with(|slot| slot.replace(Some(raw)));
    let _scope = Scope { previous, _borrow: PhantomData };
    f()
}

fn with_current<R>(default: R, f: impl FnOnce(&mut dyn PlayerOps) -> R) -> R {
    match OPS.with(Cell::get) {
        // SAFETY: see `with_ops`: the pointer is valid for as long as it is stored.
        Some(raw) => f(unsafe { &mut *raw }),
        None => default,
    }
}

/// A connected player.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerPlayer {
    pub uuid: u128,
    pub name: String,
}

impl ServerPlayer {
    /// Fabric: `ServerPlayerEntity.sendMessage(Text)`. False if the player is gone or this is called outside a callback.
    pub fn send_message(&self, text: &str) -> bool {
        with_current(false, |ops| ops.tell(self.uuid, text))
    }

    /// Fabric: `PlayerInventory.offerOrDrop`, roughly: gives `count` of `item`; returns how many did not fit (`count` if called outside a callback).
    pub fn give_item(&self, item: &str, count: u32) -> u32 {
        with_current(count, |ops| ops.give(self.uuid, item, count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Recorder {
        told: Vec<(u128, String)>,
        given: Vec<(u128, String, u32)>,
    }

    impl PlayerOps for Recorder {
        fn tell(&mut self, player: u128, line: &str) -> bool {
            self.told.push((player, line.to_owned()));
            true
        }
        fn give(&mut self, player: u128, item: &str, count: u32) -> u32 {
            self.given.push((player, item.to_owned(), count));
            count.saturating_sub(2) // pretend only two fit
        }
    }

    fn player() -> ServerPlayer {
        ServerPlayer { uuid: 7, name: "Alex".into() }
    }

    #[test]
    fn methods_reach_the_game_inside_a_scope() {
        let mut ops = Recorder::default();
        let (said, left) = with_ops(&mut ops, || (player().send_message("hi"), player().give_item("minecraft:stick", 5)));
        assert!(said);
        assert_eq!(left, 3);
        assert_eq!(ops.told, [(7, "hi".to_owned())]);
        assert_eq!(ops.given, [(7, "minecraft:stick".to_owned(), 5)]);
    }

    #[test]
    fn a_stored_handle_does_nothing_outside_a_scope() {
        let stored = player();
        assert!(!stored.send_message("late"));
        assert_eq!(stored.give_item("minecraft:stick", 4), 4);
    }

    #[test]
    fn the_scope_ends_even_when_the_callback_panics() {
        let mut ops = Recorder::default();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| with_ops(&mut ops, || panic!("boom"))));
        assert!(caught.is_err());
        assert!(!player().send_message("after"), "the scope must be closed");
    }
}
