//! Faucets (`FaucetBlockEntity`): a faucet drains a packet (one ingot, 90 mB) from the tank behind it and pours it, a nugget (10 mB) a tick,
//! into the tank below. A click starts it and, while it pours, asks it to stop at the end of the packet; with a redstone signal it waits
//! (`Powered`) for something to pour and pours again and again.
use super::alloy::Fluid;
use crate::resource::ResourceId;

/// How much a faucet drains in one go (`PACKET_SIZE`, an ingot).
pub const PACKET_SIZE: i32 = 90;
/// How much it pours in a tick (`MB_PER_TICK`, a nugget).
pub const MB_PER_TICK: i32 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaucetState {
    Off,
    /// Waiting for a signal-driven pour: there is a signal but nothing to pour yet.
    Powered,
    Pouring,
}

/// The tanks behind and below the faucet.
pub trait Pipes {
    fn has_input(&self) -> bool;
    fn has_output(&self) -> bool;
    /// Drains up to `max` from the input; with `execute` false only says what would come.
    fn drain_input(&mut self, max: i32, execute: bool) -> Option<Fluid>;
    /// Fills the output; with `execute` false only says how much would go in.
    fn fill_output(&mut self, fluid: &ResourceId, amount: i32, execute: bool) -> i32;
}

/// What the clients must be told (`FaucetActivationPacket`): the fluid to draw in the stream and whether it pours.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sync {
    pub fluid: Option<Fluid>,
    pub pouring: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Faucet {
    pub state: FaucetState,
    stop_pouring: bool,
    drained: Option<Fluid>,
    render_fluid: Option<Fluid>,
    last_redstone: bool,
    /// The syncs since they were last taken.
    pub syncs: Vec<Sync>,
}

impl Default for Faucet {
    fn default() -> Self {
        Self { state: FaucetState::Off, stop_pouring: false, drained: None, render_fluid: None, last_redstone: false, syncs: Vec::new() }
    }
}

fn same(a: &Option<Fluid>, b: &Option<Fluid>) -> bool {
    a.as_ref().map(|f| &f.id) == b.as_ref().map(|f| &f.id)
}

impl Faucet {
    pub fn is_pouring(&self) -> bool {
        self.state != FaucetState::Off
    }

    /// How much of the packet is still to pour.
    pub fn drained(&self) -> i32 {
        self.drained.as_ref().map_or(0, |f| f.amount)
    }

    fn sync(&mut self, fluid: Option<Fluid>, pouring: bool) {
        self.render_fluid = fluid.clone();
        self.syncs.push(Sync { fluid, pouring });
    }

    fn reset(&mut self) {
        self.stop_pouring = false;
        self.drained = None;
        if self.state != FaucetState::Off || self.render_fluid.is_some() {
            self.state = FaucetState::Off;
            self.sync(None, false);
        }
    }

    fn do_transfer(&mut self, execute: bool, pipes: &mut dyn Pipes) -> bool {
        if pipes.has_input() && pipes.has_output() {
            if let Some(offered) = pipes.drain_input(PACKET_SIZE, false) {
                let filled = pipes.fill_output(&offered.id, offered.amount, false);
                if filled > 0 && (filled <= MB_PER_TICK || pipes.fill_output(&offered.id, MB_PER_TICK, false) > 0) {
                    if execute {
                        self.drained = pipes.drain_input(filled, true);
                        if self.state == FaucetState::Off || !same(&self.render_fluid, &self.drained) {
                            let fluid = self.drained.clone();
                            self.sync(fluid, true);
                        }
                        self.state = FaucetState::Pouring;
                        self.pour(pipes);
                    }
                    return true;
                }
            }
            if self.last_redstone {
                if execute && (self.state == FaucetState::Off || self.render_fluid.is_some()) {
                    self.sync(None, true);
                }
                self.state = FaucetState::Powered;
                return false;
            }
        }
        if execute {
            self.reset();
        }
        false
    }

    fn pour(&mut self, pipes: &mut dyn Pipes) {
        let Some(drained) = self.drained.clone() else { return };
        if !pipes.has_output() {
            self.reset();
            return;
        }
        let amount = drained.amount.min(MB_PER_TICK);
        let filled = pipes.fill_output(&drained.id, amount, false);
        if filled > 0 {
            if !same(&self.render_fluid, &self.drained) {
                self.sync(Some(drained.clone()), true);
            }
            let left = drained.amount - filled;
            self.drained = (left > 0).then(|| Fluid { id: drained.id.clone(), amount: left });
            pipes.fill_output(&drained.id, filled, true);
        }
    }

    /// A click on the faucet (`activate`): starts it, switches a waiting one off, or asks a pouring one to stop at the end of its packet.
    pub fn activate(&mut self, pipes: &mut dyn Pipes) {
        match self.state {
            FaucetState::Off => {
                self.stop_pouring = false;
                self.do_transfer(true, pipes);
            }
            FaucetState::Powered => {
                self.state = FaucetState::Off;
                self.sync(None, false);
            }
            FaucetState::Pouring => self.stop_pouring = true,
        }
    }

    /// `handleRedstone`: returns true when a tick should be scheduled (the signal just came); a signal that goes while the faucet waits
    /// switches it off.
    pub fn handle_redstone(&mut self, has_signal: bool) -> bool {
        if has_signal == self.last_redstone {
            return false;
        }
        self.last_redstone = has_signal;
        if has_signal {
            return true;
        }
        if self.state == FaucetState::Powered {
            self.state = FaucetState::Off;
            self.sync(None, false);
        }
        false
    }

    /// One tick (`tick`).
    pub fn tick(&mut self, pipes: &mut dyn Pipes) {
        if self.state == FaucetState::Off {
            return;
        }
        if self.state == FaucetState::Powered && self.do_transfer(false, pipes) {
            self.state = FaucetState::Pouring;
            return;
        }
        if self.drained.is_some() {
            self.pour(pipes);
        } else if self.stop_pouring {
            self.reset();
        } else {
            self.do_transfer(true, pipes);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(text: &str) -> ResourceId {
        ResourceId::try_parse(text).unwrap()
    }

    /// An input tank holding iron and an output with room.
    struct Tanks {
        input: i32,
        has_output: bool,
        room: i32,
        poured: i32,
    }
    impl Pipes for Tanks {
        fn has_input(&self) -> bool {
            true
        }
        fn has_output(&self) -> bool {
            self.has_output
        }
        fn drain_input(&mut self, max: i32, execute: bool) -> Option<Fluid> {
            let taken = self.input.min(max);
            if taken == 0 {
                return None;
            }
            if execute {
                self.input -= taken;
            }
            Some(Fluid { id: id("a:iron"), amount: taken })
        }
        fn fill_output(&mut self, _: &ResourceId, amount: i32, execute: bool) -> i32 {
            let taken = amount.min(self.room);
            if execute {
                self.room -= taken;
                self.poured += taken;
            }
            taken
        }
    }

    fn tanks(input: i32) -> Tanks {
        Tanks { input, has_output: true, room: 1000, poured: 0 }
    }

    #[test]
    fn a_click_drains_an_ingot_and_it_pours_a_nugget_a_tick_then_the_faucet_goes_off() {
        let mut faucet = Faucet::default();
        let mut pipes = tanks(500);
        faucet.activate(&mut pipes);
        assert_eq!((faucet.state, pipes.input, pipes.poured, faucet.drained()), (FaucetState::Pouring, 410, 10, 80), "the first nugget goes at once");
        // keeps pouring: the packet is drained, then it asks for another unless a stop was asked for
        faucet.activate(&mut pipes); // a click while pouring asks it to stop
        for _ in 0..8 {
            faucet.tick(&mut pipes);
        }
        assert_eq!((pipes.poured, faucet.drained()), (90, 0));
        faucet.tick(&mut pipes);
        assert_eq!(faucet.state, FaucetState::Off, "stopped at the end of the packet");
        assert_eq!(pipes.input, 410, "no second packet was taken");
    }

    #[test]
    fn without_a_stop_the_faucet_takes_the_next_packet() {
        let mut faucet = Faucet::default();
        let mut pipes = tanks(500);
        faucet.activate(&mut pipes);
        for _ in 0..9 {
            faucet.tick(&mut pipes);
        }
        assert_eq!(pipes.input, 320, "a second packet was drained");
        assert_eq!(faucet.state, FaucetState::Pouring);
    }

    #[test]
    fn a_faucet_with_nothing_to_pour_stays_off_and_a_signal_makes_it_wait_and_then_pour_when_something_comes() {
        let mut faucet = Faucet::default();
        let mut empty = tanks(0);
        faucet.activate(&mut empty);
        assert_eq!(faucet.state, FaucetState::Off);
        assert!(faucet.handle_redstone(true), "the signal asks for a tick");
        let mut still_empty = tanks(0);
        faucet.activate(&mut still_empty);
        assert_eq!(faucet.state, FaucetState::Powered);
        let mut filled = tanks(200);
        faucet.tick(&mut filled);
        assert_eq!(faucet.state, FaucetState::Pouring, "something arrived");
        // a signal that goes while it waits switches it off
        let mut waiting = Faucet::default();
        waiting.handle_redstone(true);
        waiting.activate(&mut tanks(0));
        waiting.handle_redstone(false);
        assert_eq!(waiting.state, FaucetState::Off);
    }

    #[test]
    fn losing_the_output_resets_the_faucet() {
        let mut faucet = Faucet::default();
        let mut pipes = tanks(500);
        faucet.activate(&mut pipes);
        pipes.has_output = false;
        faucet.tick(&mut pipes);
        assert_eq!((faucet.state, faucet.drained()), (FaucetState::Off, 0));
        assert_eq!(faucet.syncs.last(), Some(&Sync { fluid: None, pouring: false }));
    }

    #[test]
    fn an_output_with_little_room_only_gets_what_fits_and_the_faucet_then_goes_off() {
        let mut faucet = Faucet::default();
        let mut pipes = Tanks { input: 500, has_output: true, room: 10, poured: 0 };
        faucet.activate(&mut pipes);
        // only what the output can take is drained from the input, so nothing is lost
        assert_eq!((pipes.poured, pipes.input, faucet.drained()), (10, 490, 0));
        faucet.tick(&mut pipes);
        assert_eq!((faucet.state, pipes.input), (FaucetState::Off, 490), "the output is full: nothing more is taken");
    }
}
