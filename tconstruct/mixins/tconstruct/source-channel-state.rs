//! Unconnected Channel state codec for a bounded native representation.
//! It is not a Forge save importer and performs no world writes.
use horizon_world::nbt::Tag;
use super::tconstruct_source_melter_state::Tank;
use super::tconstruct_source_channel_plan::CAPACITY;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ChannelState { pub tank: Tank, pub locked: u32 }
impl ChannelState {
    pub(super) fn new() -> Self { Self { tank: Tank::new(CAPACITY), locked: 0 } }
    fn valid(&self) -> bool {
        self.tank.capacity == CAPACITY && self.locked <= self.tank.amount
            && Tank::from_nbt(&self.tank.to_nbt(), CAPACITY).as_ref() == Some(&self.tank)
    }
    pub(super) fn to_nbt(&self) -> Option<Tag> {
        if !self.valid() { return None; }
        let mut tank = match self.tank.to_nbt() { Tag::Compound(v) => v, _ => unreachable!() };
        tank.insert("locked".into(), Tag::Int(self.locked as i32));
        Some(Tag::compound([("tank", Tag::Compound(tank))]))
    }
    /// Strictly decode only the native lower-case tank format. Legacy Forge
    /// spellings may coexist with native fields, and must not be rewritten.
    pub(super) fn from_nbt(raw: &Tag) -> Option<Self> {
        let Tag::Compound(_) = raw else { return None; };
        let tank_raw = raw.get("tank")?;
        let Tag::Compound(fields) = tank_raw else { return None; };
        if ["FluidName", "Amount", "Tag"].iter().any(|key| fields.contains_key(&(*key).into())) { return None; }
        // Tank::from_nbt uses a lossy UTF-16 conversion; never rewrite an
        // identifier that cannot round-trip through Rust UTF-8 unchanged.
        if let Some(Tag::String(id)) = tank_raw.get("fluid") { id.to_string().ok()?; }
        let tank = Tank::from_nbt(tank_raw, CAPACITY)?;
        let locked = match tank_raw.get("locked")? { Tag::Int(v) => u32::try_from(*v).ok()?, _ => return None };
        if locked > tank.amount { return None; }
        Some(Self { tank, locked })
    }
    /// Update only the owned native fields, keeping unrelated outer and tank
    /// NBT as parsed Tags. Reject unknown or malformed legacy layouts.
    pub(super) fn to_nbt_preserving(&self, raw: &Tag) -> Option<Tag> {
        Self::from_nbt(raw)?;
        if !self.valid() { return None; }
        let Tag::Compound(mut outer) = raw.clone() else { return None; };
        let updated = self.tank.to_nbt_preserving(outer.get(&"tank".into())?)?;
        let Tag::Compound(mut tank) = updated else { return None; };
        tank.insert("locked".into(), Tag::Int(self.locked as i32));
        outer.insert("tank".into(), Tag::Compound(tank));
        Some(Tag::Compound(outer))
    }
}
