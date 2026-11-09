//! Channels (`ChannelBlockEntity` and `ChannelTank`): a small tank that passes its fluid on, down first, else out through its outputs in
//! equal shares. A fluid that has just come in is locked for the rest of that tick, so it moves one channel per tick and not further.
use super::alloy::Fluid;
use crate::resource::ResourceId;

/// One nugget of fluid: how much a channel or a faucet moves in a tick (`FaucetBlockEntity.MB_PER_TICK`).
pub const MB_PER_TICK: i32 = 10;

/// The sides a channel passes fluid to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Down,
    North,
    South,
    West,
    East,
}

impl Side {
    pub const HORIZONTAL: [Side; 4] = [Side::North, Side::South, Side::West, Side::East];

    /// The slot of this side among the five flow flags.
    fn flow_index(self) -> usize {
        match self {
            Side::Down => 0,
            Side::North => 1,
            Side::South => 2,
            Side::West => 3,
            Side::East => 4,
        }
    }
}

/// What is beside a channel.
pub trait Neighbors {
    /// Offers `amount` of `fluid` to the block on `side`: None when there is nothing there that takes fluid, else how much it takes
    /// (`execute` false only asks).
    fn fill(&mut self, side: Side, fluid: &ResourceId, amount: i32, execute: bool) -> Option<i32>;
}

/// `ChannelTank`: a tank of one fluid with a part of it locked until the end of the tick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelTank {
    pub fluid: Option<Fluid>,
    pub capacity: i32,
    locked: i32,
}

impl ChannelTank {
    pub fn new(capacity: i32) -> Self {
        Self { fluid: None, capacity, locked: 0 }
    }

    pub fn free_fluid(&mut self) {
        self.locked = 0;
    }

    /// What may leave this tick: all but what came in this tick.
    pub fn max_usable(&self) -> i32 {
        (self.fluid.as_ref().map_or(0, |f| f.amount) - self.locked).max(0)
    }

    /// Takes in as much of `amount` as fits (a tank holds one fluid at a time); what comes in is locked until the tick ends.
    pub fn fill(&mut self, fluid: &ResourceId, amount: i32, execute: bool) -> i32 {
        if amount <= 0 {
            return 0;
        }
        let held = self.fluid.as_ref();
        if held.is_some_and(|f| f.id != *fluid) {
            return 0;
        }
        let room = self.capacity - held.map_or(0, |f| f.amount);
        let taken = room.min(amount).max(0);
        if execute && taken > 0 {
            match self.fluid.as_mut() {
                Some(f) => f.amount += taken,
                None => self.fluid = Some(Fluid { id: fluid.clone(), amount: taken }),
            }
            self.locked += taken;
        }
        taken
    }

    pub fn drain(&mut self, max: i32, execute: bool) -> Option<Fluid> {
        let held = self.fluid.as_ref()?;
        let drained = held.amount.min(max);
        let out = Fluid { id: held.id.clone(), amount: drained };
        if execute {
            let left = held.amount - drained;
            self.fluid = (left > 0).then(|| Fluid { id: out.id.clone(), amount: left });
        }
        Some(out)
    }
}

/// What a channel's block says about its connections.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Connections {
    /// The down side is an output.
    pub down: bool,
    /// For each of north, south, west and east: that side is an output.
    pub out: [bool; 4],
}

impl Connections {
    fn is_output(&self, side: Side) -> bool {
        match side {
            Side::Down => self.down,
            other => self.out[other.flow_index() - 1],
        }
    }
}

/// A channel block entity's flow state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Channel {
    pub tank: ChannelTank,
    /// Per side, how many more ticks it is shown as flowing (two after the last flow).
    flowing: [u8; 5],
}

impl Channel {
    pub fn new() -> Self {
        Self { tank: ChannelTank::new(MB_PER_TICK * 4), flowing: [0; 5] }
    }

    pub fn is_flowing(&self, side: Side) -> bool {
        self.flowing[side.flow_index()] > 0
    }

    /// `setFlow`: returns true when the flag changed (the clients are told then).
    fn set_flow(&mut self, side: Side, flowing: bool) -> bool {
        let index = side.flow_index();
        let was = self.flowing[index] > 0;
        self.flowing[index] = if flowing { 2 } else { 0 };
        was != flowing
    }

    fn fill_side(&mut self, side: Side, neighbors: &mut dyn Neighbors, amount: i32, changes: &mut Vec<(Side, bool)>) -> bool {
        let usable = self.tank.max_usable().min(amount);
        let mut done = false;
        if usable > 0 {
            if let Some(fluid) = self.tank.drain(usable, false) {
                let filled = neighbors.fill(side, &fluid.id, fluid.amount, false).unwrap_or(0);
                if filled > 0 {
                    if let Some(out) = self.tank.drain(filled, true) {
                        neighbors.fill(side, &out.id, out.amount, true);
                    }
                    done = true;
                }
            }
        }
        if self.set_flow(side, done) {
            changes.push((side, done));
        }
        done
    }

    fn try_side(&mut self, side: Side, connections: &Connections, neighbors: &mut dyn Neighbors, amount: i32, changes: &mut Vec<(Side, bool)>) -> bool {
        if self.tank.fluid.is_none() || !connections.is_output(side) {
            return false;
        }
        self.fill_side(side, neighbors, amount, changes)
    }

    /// `tick`: pass fluid down if the down side is an output, else share it among the horizontal outputs; flow flags run out after two
    /// ticks; then the fluid that came in this tick is freed. Returns the flow flags that changed, for the clients.
    pub fn tick(&mut self, connections: &Connections, neighbors: &mut dyn Neighbors) -> Vec<(Side, bool)> {
        let mut changes = Vec::new();
        if self.tank.fluid.is_some() {
            let mut has_flown = false;
            if connections.down {
                has_flown = self.try_side(Side::Down, connections, neighbors, MB_PER_TICK, &mut changes);
            }
            let outputs = connections.out.iter().filter(|o| **o).count() as i32;
            if !has_flown && outputs > 0 {
                let rate = (self.tank.max_usable() / outputs).clamp(1, MB_PER_TICK);
                for side in Side::HORIZONTAL {
                    self.try_side(side, connections, neighbors, rate, &mut changes);
                }
            }
        }
        for side in [Side::Down, Side::North, Side::South, Side::West, Side::East] {
            let index = side.flow_index();
            if self.flowing[index] > 0 {
                self.flowing[index] -= 1;
                if self.flowing[index] == 0 {
                    changes.push((side, false));
                }
            }
        }
        self.tank.free_fluid();
        changes
    }
}

impl Default for Channel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn id(text: &str) -> ResourceId {
        ResourceId::try_parse(text).unwrap()
    }

    /// Neighbors that are tanks of a given room; a side without an entry has nothing there.
    struct Around(HashMap<usize, (i32, i32)>); // flow index -> (room, received)
    impl Neighbors for Around {
        fn fill(&mut self, side: Side, _: &ResourceId, amount: i32, execute: bool) -> Option<i32> {
            let (room, got) = self.0.get_mut(&side.flow_index())?;
            let taken = amount.min(*room);
            if execute {
                *room -= taken;
                *got += taken;
            }
            Some(taken)
        }
    }

    fn around(sides: &[(Side, i32)]) -> Around {
        Around(sides.iter().map(|(s, room)| (s.flow_index(), (*room, 0))).collect())
    }

    fn received(a: &Around, side: Side) -> i32 {
        a.0.get(&side.flow_index()).map_or(0, |v| v.1)
    }

    fn channel_with(amount: i32) -> Channel {
        let mut channel = Channel::new();
        channel.tank.fill(&id("a:iron"), amount, true);
        channel.tank.free_fluid();
        channel
    }

    #[test]
    fn fluid_just_poured_in_is_locked_until_the_tick_ends() {
        let mut tank = ChannelTank::new(40);
        assert_eq!(tank.fill(&id("a:iron"), 25, true), 25);
        assert_eq!(tank.max_usable(), 0);
        tank.free_fluid();
        assert_eq!(tank.max_usable(), 25);
        assert_eq!(tank.fill(&id("a:gold"), 5, true), 0, "one fluid at a time");
        assert_eq!(tank.fill(&id("a:iron"), 50, true), 15, "only the room left");
    }

    #[test]
    fn a_channel_with_a_down_output_pours_down_first_and_nothing_goes_sideways() {
        let mut channel = channel_with(40);
        let mut neighbors = around(&[(Side::Down, 1000), (Side::East, 1000)]);
        let connections = Connections { down: true, out: [false, false, false, true] };
        let changes = channel.tick(&connections, &mut neighbors);
        assert_eq!((received(&neighbors, Side::Down), received(&neighbors, Side::East)), (MB_PER_TICK, 0));
        assert!(changes.contains(&(Side::Down, true)) && channel.is_flowing(Side::Down));
    }

    #[test]
    fn without_a_way_down_the_fluid_is_shared_among_the_outputs() {
        let mut channel = channel_with(40);
        let mut neighbors = around(&[(Side::East, 1000), (Side::West, 1000)]);
        let connections = Connections { down: false, out: [false, false, true, true] };
        channel.tick(&connections, &mut neighbors);
        // 40 usable shared by 2 outputs is 20 each, clamped to 10 a tick
        assert_eq!((received(&neighbors, Side::East), received(&neighbors, Side::West)), (10, 10));
        assert_eq!(channel.tank.fluid.as_ref().map(|f| f.amount), Some(20));
    }

    #[test]
    fn a_full_neighbor_stops_the_flow_and_the_flag_runs_out_after_two_ticks() {
        let mut channel = channel_with(40);
        let connections = Connections { down: false, out: [false, false, false, true] };
        let mut open = around(&[(Side::East, 1000)]);
        channel.tick(&connections, &mut open);
        assert!(channel.is_flowing(Side::East));
        let mut full = around(&[(Side::East, 0)]);
        let changes = channel.tick(&connections, &mut full);
        assert!(!channel.is_flowing(Side::East), "nothing was taken, so the flag is cleared at once");
        assert_eq!(changes, vec![(Side::East, false)]);
        assert_eq!(channel.tank.fluid.as_ref().map(|f| f.amount), Some(30));
    }

    #[test]
    fn a_side_with_nothing_there_or_that_is_not_an_output_gets_nothing() {
        let mut channel = channel_with(40);
        let mut neighbors = around(&[(Side::North, 1000)]);
        let connections = Connections { down: false, out: [false, false, false, true] };
        channel.tick(&connections, &mut neighbors);
        assert_eq!(received(&neighbors, Side::North), 0, "north is not an output");
        assert_eq!(channel.tank.fluid.as_ref().map(|f| f.amount), Some(40), "east has no neighbor");
    }

    #[test]
    fn what_arrives_is_passed_on_one_tick_later() {
        let mut channel = Channel::new();
        let mut neighbors = around(&[(Side::East, 1000)]);
        let connections = Connections { down: false, out: [false, false, false, true] };
        channel.tank.fill(&id("a:iron"), 30, true);
        channel.tick(&connections, &mut neighbors);
        assert_eq!(received(&neighbors, Side::East), 0, "locked in the tick it came in");
        channel.tick(&connections, &mut neighbors);
        assert_eq!(received(&neighbors, Side::East), MB_PER_TICK);
    }
}
