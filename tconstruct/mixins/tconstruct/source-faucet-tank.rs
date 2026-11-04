// Built-in single-fluid tanks as faucet destinations. Only known saved
// machine states are touched; arbitrary external capabilities are not used.
use super::tconstruct_source_melter_state::{Faucet, Melter, Tank};
use super::tconstruct_source_melter_store as store;
type Pos = (i32,i32,i32);

use horizon_world::nbt::Tag;
/// Preserve extension fields from the original tank compound while replacing
/// only the fields the tank codec owns (including removing emptied fields).
fn patch_tank(raw: &Tag, tank: &Tank) -> Option<Tag> {
    let Tag::Compound(mut fields) = raw.clone() else { return None; };
    let Tag::Compound(next) = tank.to_nbt() else { return None; };
    for key in ["fluid", "fluid_tag", "amount"] {
        fields.remove(&key.into());
        if let Some(value) = next.get(&key.into()) { fields.insert(key.into(), value.clone()); }
    }
    Some(Tag::Compound(fields))
}
/// A faucet changes only the Melter's fluid tank. Preserve the entire raw
/// machine compound, including unrecognized slot/item fields, verbatim.
pub(super) fn patch_melter_tank(raw: &Tag, tank: &Tank) -> Option<Tag> {
    let Tag::Compound(mut fields) = raw.clone() else { return None; };
    let next = tank.to_nbt_preserving(fields.get(&"tank".into())?)?;
    fields.insert("tank".into(), next);
    Some(Tag::Compound(fields))
}
fn patch_faucet(raw: &Tag, faucet: &Faucet) -> Option<Tag> {
    let Tag::Compound(mut fields) = raw.clone() else { return None; };
    let pending = patch_tank(fields.get(&"pending".into())?, &faucet.pending)?;
    fields.insert("pending".into(), pending);
    fields.insert("active".into(), Tag::Byte(i8::from(faucet.active)));
    fields.insert("stop".into(), Tag::Byte(i8::from(faucet.stop_after_packet)));
    Some(Tag::Compound(fields))
}
enum Source { Melter(Melter, Tag), Tank(&'static str, Tank, Tag) }
impl Source {
    fn tank(&mut self) -> &mut Tank { match self { Self::Melter(m,_) => &mut m.tank, Self::Tank(_,t,_) => t } }
    fn data(&self) -> Option<(&'static str, Tag)> { match self {
        Self::Melter(m,raw) => {
            Some(("tconstruct:melter", patch_melter_tank(raw, &m.tank)?))
        }, Self::Tank(k,t,raw) => Some((*k,patch_tank(raw,t)?)) } }
}
fn name(game: &super::Game<'_>, at: Pos) -> String {
    game.res.facts().get(game.world.block(at.0,at.1,at.2)).name.to_string()
}
fn tank_kind(name: &str) -> Option<(&'static str,u32)> {
    super::tconstruct_source_smeltery_fuel::capacity(name).zip(super::tconstruct_source_machine_runtime::kind(name))
        .map(|(capacity,key)| (key,capacity))
}
fn load_source(game: &super::Game<'_>, at: Pos, block: &str) -> Option<Source> {
    if !matches!(game.world.block_entities.get(&at), Some(horizon_client::containers::BlockEntity::Chest(_))) { return None; }
    if block == "tconstruct:seared_melter" {
        let raw = store::read(&game.world,at,"tconstruct:melter")?;
        return Some(Source::Melter(Melter::from_nbt(&raw)?, raw));
    }
    let (key,capacity)=tank_kind(block)?;
    let raw=store::read(&game.world,at,key)?;
    Some(Source::Tank(key,Tank::from_nbt(&raw,capacity)?,raw))
}
fn is_source(name: &str) -> bool { name == "tconstruct:seared_melter" || tank_kind(name).is_some() }

/// Stop after the reserved packet, retaining unknown faucet and packet data.
pub(super) fn stop(game: &mut super::Game<'_>, pos: Pos, mut faucet: Faucet) -> bool {
    let Some(raw)=store::read(&game.world,pos,"tconstruct:faucet") else { return false; };
    faucet.stop_after_packet = true;
    let Some(data)=patch_faucet(&raw,&faucet) else { return false; };
    store::write_many(&mut game.world,&[(pos,"tconstruct:faucet",data)])
}

/// Returns None when this is not our bounded route; Some is handled.
pub(super) fn activate(game: &mut super::Game<'_>, pos: Pos, source: Pos, target: Pos, mut faucet: Faucet) -> Option<bool> {
    let (out_key,capacity)=tank_kind(&name(game,target))?;
    let block=name(game,source);
    if !is_source(&block) { return None; }
    if source == target || source == pos || target == pos { return Some(false); }
    let Some(mut input)=load_source(game,source,&block) else { return Some(false); };
    if !matches!(game.world.block_entities.get(&target), Some(horizon_client::containers::BlockEntity::Chest(_))) { return Some(false); }
    let Some(raw_output)=store::read(&game.world,target,out_key) else { return Some(false); };
    let Some(mut output)=Tank::from_nbt(&raw_output,capacity) else { return Some(false); };
    if !faucet.activate(input.tank(),&output) { return Some(true); }
    // Source doTransfer(true) pours its first increment immediately.
    faucet.tick(input.tank(),Some(&mut output));
    let Some((key,data))=input.data() else { return Some(false); };
    let Some(raw_faucet)=store::read(&game.world,pos,"tconstruct:faucet") else { return Some(false); };
    let Some(next_faucet)=patch_faucet(&raw_faucet,&faucet) else { return Some(false); };
    let Some(next_output)=patch_tank(&raw_output,&output) else { return Some(false); };
    Some(store::write_many(&mut game.world,&[(source,key,data),(target,out_key,next_output),(pos,"tconstruct:faucet",next_faucet)]))
}
/// Keep pending fluid on an unreadable known source/destination. A removed
/// source is not needed for a packet already reserved by the faucet.
pub(super) fn tick(game: &mut super::Game<'_>, pos: Pos, source: Pos, target: Pos, mut faucet: Faucet) -> bool {
    let Some((out_key,capacity))=tank_kind(&name(game,target)) else { return false; };
    if source == target || source == pos || target == pos { return true; }
    if !matches!(game.world.block_entities.get(&target), Some(horizon_client::containers::BlockEntity::Chest(_))) { return true; }
    let Some(raw_output)=store::read(&game.world,target,out_key) else { return true; };
    let Some(mut output)=Tank::from_nbt(&raw_output,capacity) else { return true; };
    let block=name(game,source);
    let mut input=if is_source(&block) {
        let Some(input)=load_source(game,source,&block) else { return true; }; Some(input)
    } else { None };
    let mut empty=Tank::new(0);
    let needs_packet = faucet.pending.amount == 0;
    faucet.tick(input.as_mut().map_or(&mut empty,Source::tank),Some(&mut output));
    // Source doTransfer(true) also pours immediately when starting a later packet.
    if needs_packet && faucet.pending.amount > 0 {
        faucet.tick(input.as_mut().map_or(&mut empty,Source::tank),Some(&mut output));
    }
    let Some(next_output)=patch_tank(&raw_output,&output) else { return true; };
    let Some(raw_faucet)=store::read(&game.world,pos,"tconstruct:faucet") else { return true; };
    let Some(next_faucet)=patch_faucet(&raw_faucet,&faucet) else { return true; };
    let mut changes=vec![(target,out_key,next_output),(pos,"tconstruct:faucet",next_faucet)];
    if let Some(input)=input { let Some((key,data))=input.data() else { return true; }; changes.push((source,key,data)); }
    let _=store::write_many(&mut game.world,&changes);
    true
}
