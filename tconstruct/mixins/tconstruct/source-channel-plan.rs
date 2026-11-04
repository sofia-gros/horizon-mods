//! Unconnected, bounded Channel amount planner for independently staged sinks.
//! This does not read or mutate a world, item, block entity or fluid handler.

pub(super) const CAPACITY: u32 = 40;
pub(super) const PER_TICK: u32 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Buffer { pub amount: u32, pub locked: u32 }

impl Buffer {
    /// Stage an already validated compatible fill; no fluid identity is inferred.
    pub(super) fn fill(self, requested: u32) -> Option<(Self, u32)> {
        if self.amount > CAPACITY || self.locked > self.amount { return None; }
        let accepted = requested.min(CAPACITY - self.amount);
        Some((Self { amount: self.amount + accepted, locked: self.locked + accepted }, accepted))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Plan { pub buffer: Buffer, pub down: u32, pub sides: [u32; 4] }

/// `down` is None for a closed side; Some(capacity) is an independent,
/// compatible simulated sink. `sides` are in the caller's source iteration
/// order, each containing (is_output, simulated acceptance capacity).
/// The caller must reject aliased sinks and atomically revalidate/commit the
/// complete plan. Unknown fluid data and external capability behavior are not
/// represented here. None means an invalid buffer; no external effect occurs.
pub(super) fn plan(buffer: Buffer, down: Option<u32>, sides: [(bool, u32); 4]) -> Option<Plan> {
    if buffer.amount > CAPACITY || buffer.locked > buffer.amount { return None; }
    let mut remaining = buffer.amount;
    let mut down_sent = 0;
    let mut sent = [0; 4];
    if remaining > 0 {
        let available = remaining.saturating_sub(buffer.locked);
        if let Some(capacity) = down {
            down_sent = available.min(PER_TICK).min(capacity);
            remaining -= down_sent;
        }
        if down_sent == 0 {
            let outputs = sides.iter().filter(|(out, _)| *out).count() as u32;
            if outputs > 0 {
                let rate = (remaining.saturating_sub(buffer.locked) / outputs).clamp(1, PER_TICK);
                for (i, (out, capacity)) in sides.into_iter().enumerate() {
                    if out {
                        sent[i] = remaining.saturating_sub(buffer.locked).min(rate).min(capacity);
                        remaining -= sent[i];
                    }
                }
            }
        }
    }
    // Source ChannelTank.freeFluid() runs after the tick, even when empty.
    Some(Plan { buffer: Buffer { amount: remaining, locked: 0 }, down: down_sent, sides: sent })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn fill_caps_at_forty_and_locks_new_fluid() {
        assert_eq!(Buffer{amount:25,locked:5}.fill(30),Some((Buffer{amount:40,locked:20},15)));
        assert_eq!(Buffer{amount:41,locked:0}.fill(1),None);
        assert_eq!(Buffer{amount:5,locked:6}.fill(1),None);
    }
    #[test] fn down_success_takes_priority_over_all_sides() {
        let p=plan(Buffer{amount:40,locked:0},Some(7),[(true,40);4]).unwrap();
        assert_eq!((p.down,p.sides,p.buffer),(7,[0;4],Buffer{amount:33,locked:0}));
    }
    #[test] fn blocked_down_splits_using_all_configured_outputs() {
        let p=plan(Buffer{amount:30,locked:0},Some(0),[(true,0),(true,40),(true,40),(false,40)]).unwrap();
        assert_eq!((p.down,p.sides,p.buffer.amount),(0,[0,10,10,0],10));
    }
    #[test] fn new_fill_waits_until_next_tick() {
        let (b,accepted)=Buffer{amount:0,locked:0}.fill(20).unwrap(); assert_eq!(accepted,20);
        let first=plan(b,Some(40),[(true,40);4]).unwrap();
        assert_eq!((first.down,first.sides,first.buffer),(0,[0;4],Buffer{amount:20,locked:0}));
        assert_eq!(plan(first.buffer,Some(40),[(true,40);4]).unwrap().down,10);
    }
    #[test] fn round_down_and_min_one_follow_source_order() {
        let p=plan(Buffer{amount:2,locked:0},None,[(true,40);4]).unwrap();
        assert_eq!((p.sides,p.buffer.amount),([1,1,0,0],0));
    }
    #[test] fn locked_tail_is_never_sent_and_is_released() {
        let p=plan(Buffer{amount:17,locked:12},None,[(true,40),(true,40),(false,0),(false,0)]).unwrap();
        assert_eq!((p.sides,p.buffer),([2,2,0,0],Buffer{amount:13,locked:0}));
    }
    #[test] fn invalid_buffer_is_rejected() {
        assert!(plan(Buffer{amount:41,locked:0},None,[(false,0);4]).is_none());
        assert!(plan(Buffer{amount:1,locked:2},None,[(false,0);4]).is_none());
    }
}
