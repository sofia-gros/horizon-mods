//! Unconnected staging for known native Channel-to-Channel transfers only.
//! No live world is read or written by this module.
use horizon_world::nbt::Tag;
use super::tconstruct_source_channel_plan::{self as plan, Buffer, CAPACITY};
use super::tconstruct_source_channel_state::ChannelState;
use std::collections::HashSet;

pub(super) type Pos = (i32, i32, i32);
#[derive(Clone, Debug)]
pub(super) struct Snapshot { pub pos: Pos, pub raw: Tag }
#[derive(Debug)]
pub(super) struct Change { pub pos: Pos, pub before: Tag, pub after: Tag }

/// All supplied sinks are known native Channels with already-validated input
/// sides. A connected output with no eligible sink is `(true, None)`. Sides
/// must be in the original iteration order. None means reject the entire
/// stage, including malformed data, aliasing, or inconsistent side inputs.
/// The caller must validate block identity, same-name extension collisions,
/// fluid registry/resource ID, side rules and loaded chunks, then compare ALL
/// `before` snapshots, including side states and absent-sink positions, and
/// commit ALL `after` snapshots atomically on authority. The caller must run
/// at most one successful stage per Channel per game tick; this function does
/// not track tick identity and clears the source lock on every call.
pub(super) fn stage(source: Snapshot, down: Option<Snapshot>, sides: [(bool, Option<Snapshot>); 4]) -> Option<Vec<Change>> {
    let mut seen=HashSet::new(); seen.insert(source.pos);
    let mut input=ChannelState::from_nbt(&source.raw)?;
    let mut sinks: Vec<(Snapshot, ChannelState)> = Vec::new();
    let mut accept = |snapshot: Snapshot| -> Option<usize> {
        if !seen.insert(snapshot.pos) { return None; }
        let state=ChannelState::from_nbt(&snapshot.raw)?;
        let index=sinks.len(); sinks.push((snapshot,state)); Some(index)
    };
    let down_index=match down { Some(s) => Some(accept(s)?), None => None };
    let mut side_indices=[None;4];
    let mut flags=[false;4];
    for (i,(out,snapshot)) in sides.into_iter().enumerate() {
        flags[i]=out;
        if !out && snapshot.is_some() { return None; }
        if let Some(s)=snapshot { side_indices[i]=Some(accept(s)?); }
    }
    let mut capacity = |i: usize| -> u32 {
        let dest=&sinks[i].1.tank;
        match (&input.tank.fluid, &dest.fluid) {
            (Some(_),None) => CAPACITY - dest.amount,
            (Some(a),Some(b)) if a==b && input.tank.tag==dest.tag => CAPACITY - dest.amount,
            _ => 0,
        }
    };
    let down_capacity=down_index.map(&mut capacity);
    let mut side_caps=[(false,0);4];
    for i in 0..4 {
        // A connected output with no sink still counts in the source split.
        side_caps[i]=(flags[i], side_indices[i].map(&mut capacity).unwrap_or(0));
    }
    drop(capacity);
    let decision=plan::plan(Buffer{amount:input.tank.amount,locked:input.locked},down_capacity,side_caps)?;
    let id=input.tank.fluid.clone(); let tag=input.tank.tag.clone();
    let mut transfers=Vec::new();
    if let Some(i)=down_index { transfers.push((i,decision.down)); }
    for i in 0..4 { if let Some(index)=side_indices[i] { transfers.push((index,decision.sides[i])); } }
    for (index,amount) in transfers {
        if amount==0 { continue; }
        let fluid=id.as_deref()?;
        input.tank.take_tagged(tag.as_ref(),amount)?;
        let dest=&mut sinks[index].1;
        if !dest.tank.insert_tagged(fluid,tag.as_ref(),amount) { return None; }
        dest.locked=dest.locked.checked_add(amount)?;
    }
    input.locked=decision.buffer.locked;
    if input.tank.amount != decision.buffer.amount { return None; }
    let mut changes=vec![Change{pos:source.pos, before:source.raw.clone(), after:input.to_nbt_preserving(&source.raw)?}];
    for (snapshot,state) in sinks {
        changes.push(Change{pos:snapshot.pos, before:snapshot.raw.clone(), after:state.to_nbt_preserving(&snapshot.raw)?});
    }
    Some(changes)
}
