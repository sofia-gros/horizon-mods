//! ModifierEntry/IncrementalModifierEntry state from SlimeKnights (MIT; see LICENSE).
//! IDs remain unresolved here; this is not a substitute for ModifierManager readiness.
use pluto_lib::{Identifier, Nbt, NbtCompound};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Entry {
    pub id: Identifier,
    pub level: i32,
    // Only retain a fraction when 0 < amount < needed, as in IncrementalModifierEntry.of.
    progress: Option<(i32, i32)>,
}
impl Entry {
    pub fn new(id: Identifier, level: i32) -> Self { Self { id, level, progress: None } }
    pub fn empty() -> Self { Self::new(Identifier::new("tconstruct", "empty").unwrap(), 0) }
    pub fn incremental(id: Identifier, level: i32, amount: i32, needed: i32) -> Self {
        if needed == 0 || amount >= needed { return Self::new(id, level); }
        if amount <= 0 { return Self::new(id, level.wrapping_sub(1)); }
        Self { id, level, progress: Some((amount, needed)) }
    }
    pub fn effective_level(&self) -> f32 {
        match self.progress {
            Some((amount, needed)) => self.level as f32 + amount.wrapping_sub(needed) as f32 / needed as f32,
            None => self.level as f32,
        }
    }
    pub fn int_effective_level(&self) -> i32 {
        if self.progress.is_some() { self.level.wrapping_sub(1) } else { self.level }
    }
    pub fn with_level(&self, level: i32) -> Self { Self { level, ..self.clone() } }
    /// ModifierEntry.getAmount(fallback): the partial amount, or the fallback for a whole level.
    pub fn amount(&self, fallback: i32) -> i32 { self.progress.map_or(fallback, |(amount, _)| amount) }
    /// ModifierEntry.getNeeded: the amount a level needs, 0 when not incremental.
    pub fn needed(&self) -> i32 { self.progress.map_or(0, |(_, needed)| needed) }
    pub fn add_amount(&self, mut amount: i32, mut needed: i32) -> Self {
        if needed <= 0 || amount <= 0 { return self.clone(); }
        let Some((previous, previous_need)) = self.progress else {
            return Self::incremental(self.id.clone(), self.level.wrapping_add(1), amount, needed);
        };
        if amount >= needed { return Self::new(self.id.clone(), self.level); }
        if needed == previous_need { amount = amount.wrapping_add(previous); }
        else {
            let gcd = gcd(needed, previous_need);
            let multiplier = previous_need / gcd;
            // Preserve Java int overflow and operation order, including multiplication before division.
            amount = (previous.wrapping_mul(needed) / gcd).wrapping_add(amount.wrapping_mul(multiplier));
            needed = needed.wrapping_mul(multiplier);
        }
        Self::incremental(self.id.clone(), self.level, amount, needed)
    }
    pub fn merge(&self, other: &Self) -> Result<Self, String> {
        if self.id != other.id { return Err(format!("modifier IDs differ: {} and {}", self.id, other.id)); }
        if self.progress.is_none() { return Ok(other.with_level(self.level.wrapping_add(other.level))); }
        let result = self.with_level(self.level.wrapping_add(other.int_effective_level()));
        let (amount, needed) = other.progress.unwrap_or((0, 0));
        Ok(result.add_amount(amount, needed))
    }
    pub fn from_nbt(tag: &NbtCompound) -> Self { Self::decode_nbt(tag).unwrap_or_else(Self::empty) }
    fn decode_nbt(tag: &NbtCompound) -> Option<Self> {
        let Some(Nbt::String(name)) = tag.get("name") else { return None; };
        let id = Identifier::parse(name).ok()?;
        let level = nbt_int(tag.get("level"));
        if level <= 0 { return None; }
        // The serialized effective cache is deliberately ignored. A valid level 1
        // with zero partial progress yields a retained level 0, not the EMPTY sentinel.
        Some(Self::incremental(id, level, nbt_int(tag.get("amount")), nbt_int(tag.get("needed"))))
    }
    pub fn to_nbt(&self) -> NbtCompound {
        let mut tag = NbtCompound::from([
            ("name".into(), Nbt::String(self.id.to_string())), ("level".into(), Nbt::Int(self.level)),
        ]);
        if let Some((amount, needed)) = self.progress {
            tag.insert("amount".into(), Nbt::Int(amount));
            tag.insert("needed".into(), Nbt::Int(needed));
            tag.insert("effective".into(), Nbt::Float(self.effective_level()));
        }
        tag
    }
}
/// Unsorted saved upgrades (ModifierNBT). Rebuild sorting requires resolved
/// modifier priorities and is deliberately not guessed from these saved IDs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct List(Vec<Entry>);
impl List {
    pub fn entries(&self) -> &[Entry] { &self.0 }
    pub fn from_nbt(value: Option<&Nbt>) -> Self {
        let Some(Nbt::List(values)) = value else { return Self::default(); };
        if values.iter().any(|value| !matches!(value, Nbt::Compound(_))) { return Self::default(); }
        Self(values.iter().filter_map(|value| match value {
            Nbt::Compound(tag) => Entry::decode_nbt(tag), _ => None,
        }).collect())
    }
    pub fn to_nbt(&self) -> Nbt { Nbt::List(self.0.iter().map(|entry| Nbt::Compound(entry.to_nbt())).collect()) }
    /// The same entries with two positions exchanged (ModifierSortingRecipe's reorder).
    pub fn swapped(&self, a: usize, b: usize) -> Self {
        let mut entries = self.0.clone();
        if a < entries.len() && b < entries.len() { entries.swap(a, b); }
        Self(entries)
    }
    /// ModifierNBT.getEntry: the entry for this modifier, or the empty entry.
    pub fn entry(&self, id: &Identifier) -> Entry { self.0.iter().find(|entry| &entry.id == id).cloned().unwrap_or_else(Entry::empty) }
    pub fn level(&self, id: &Identifier) -> i32 {
        self.0.iter().find(|entry| &entry.id == id).map_or(0, |entry| entry.level)
    }
    pub fn with_modifier(&self, id: Identifier, level: i32) -> Result<Self, String> {
        if level <= 0 { return Err("modifier level addition must be positive".into()); }
        let mut entries = self.0.clone();
        if let Some(entry) = entries.iter_mut().find(|entry| entry.id == id) {
            *entry = entry.with_level(entry.level.wrapping_add(level));
        } else { entries.push(Entry::new(id, level)); }
        Ok(Self(entries))
    }
    pub fn add_amount(&self, id: Identifier, amount: i32, needed: i32) -> Self {
        if amount <= 0 || needed <= 0 { return self.clone(); }
        let mut entries = self.0.clone();
        if let Some(entry) = entries.iter_mut().find(|entry| entry.id == id) {
            *entry = entry.add_amount(amount, needed);
        } else { entries.push(Entry::incremental(id, 1, amount, needed)); }
        Self(entries)
    }
    pub fn without_modifier(&self, id: &Identifier, mut level: i32) -> Result<Self, String> {
        if level <= 0 { return Err("modifier level removal must be positive".into()); }
        let mut entries = Vec::new();
        for entry in &self.0 {
            if &entry.id == id && level > 0 {
                if entry.level > level { entries.push(entry.with_level(entry.level.wrapping_sub(level))); level = 0; }
                else { level = level.wrapping_sub(entry.level); }
            } else { entries.push(entry.clone()); }
        }
        Ok(Self(entries))
    }
}

pub(crate) fn gcd(mut a: i32, mut b: i32) -> i32 {
    while b != 0 { (a, b) = (b, a % b); }
    a
}
// CompoundTag.getInt accepts all numeric tag kinds. Float/Double tags use Mth.floor,
// not Rust's truncating cast; Long tags use Java's narrowing conversion.
pub(crate) fn nbt_int(value: Option<&Nbt>) -> i32 {
    match value {
        Some(Nbt::Float(value)) => java_floor(f64::from(*value)),
        Some(Nbt::Double(value)) => java_floor(*value),
        Some(value) => value.as_i64().unwrap_or(0) as i32,
        None => 0,
    }
}
fn java_floor(value: f64) -> i32 {
    let integer = value as i32;
    if value < f64::from(integer) { integer.wrapping_sub(1) } else { integer }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn entry(level: i32, amount: i32, needed: i32) -> Entry {
        Entry::incremental(Identifier::parse("tconstruct:reinforced").unwrap(), level, amount, needed)
    }
    #[test]
    fn saved_upgrade_lists_preserve_order_duplicates_and_partial_progress() {
        let first = entry(2, 1, 3);
        let id = first.id.clone();
        let mut zero = entry(1, 0, 0).to_nbt();
        zero.insert("needed".into(), Nbt::Int(5));
        let input = Nbt::List(vec![Nbt::Compound(first.to_nbt()), Nbt::Compound(NbtCompound::new()),
            Nbt::Compound(entry(3, 0, 0).to_nbt()), Nbt::Compound(zero)]);
        let list = List::from_nbt(Some(&input));
        assert_eq!(list.entries(), &[first.clone(), entry(3, 0, 0), entry(0, 0, 0)]);
        assert_eq!(list.level(&id), 2, "first duplicate wins the lookup");
        let increased = list.with_modifier(id.clone(), 2).unwrap();
        assert_eq!(increased.entries()[0], entry(4, 1, 3));
        assert_eq!(increased.entries()[1], entry(3, 0, 0));
        let removed = list.without_modifier(&id, 3).unwrap();
        assert_eq!(removed.entries(), &[entry(2, 0, 0), entry(0, 0, 0)]);
        let partial = List::default().add_amount(id.clone(), 1, 3).add_amount(id.clone(), 1, 6);
        assert_eq!(partial.entries(), &[entry(1, 3, 6)]);
        assert_eq!(List::from_nbt(Some(&partial.to_nbt())), partial);
        assert!(list.with_modifier(id.clone(), 0).is_err());
        assert!(list.without_modifier(&id, -1).is_err());
        assert_eq!(list.entries()[0], first, "copy operations preserve original state");
        assert!(List::from_nbt(Some(&Nbt::List(vec![Nbt::Compound(first.to_nbt()), Nbt::Int(1)]))).entries().is_empty());
    }
    #[test]
    fn partial_progress_merges_at_common_scale_and_normalizes_full_levels() {
        let a = entry(2, 1, 3);
        let b = entry(3, 1, 4);
        let sum = a.merge(&b).unwrap();
        assert_eq!(sum, entry(4, 7, 12));
        assert_eq!(sum.int_effective_level(), 3);
        assert!((sum.effective_level() - (3.0 + 7.0 / 12.0)).abs() < 0.000001);
        assert_eq!(a.merge(&entry(3, 2, 3)).unwrap(), entry(4, 0, 0));
        assert_eq!(a.merge(&entry(3, 0, 0)).unwrap(), entry(5, 1, 3));
        assert_eq!(entry(3, 0, 0).merge(&a).unwrap(), entry(5, 1, 3));
        assert_eq!(entry(2, 0, 0).add_amount(1, 4), entry(3, 1, 4));
        assert_eq!(a.add_amount(4, 4), entry(2, 0, 0));
        assert_eq!(a.add_amount(0, 4), a);
        assert!(a.merge(&Entry::empty()).is_err());
        assert_eq!(entry(1, 0, 5).level, 0);
        assert_eq!(entry(2, 9, 4), entry(2, 0, 0), "extra amount does not grant extra levels");
        assert_eq!(entry(i32::MAX, 0, 0).merge(&entry(1, 0, 0)).unwrap().level, i32::MIN);
        assert_eq!(entry(2, i32::MAX - 1, i32::MAX).add_amount(2, i32::MAX), entry(1, 0, 0),
            "overflowed Java amount normalizes to the lower level rather than panicking");
    }
    #[test]
    fn persistence_ignores_cache_preserves_numeric_tags_and_rejects_invalid_identity() {
        let original = entry(3, 2, 7);
        let mut tag = original.to_nbt();
        tag.insert("effective".into(), Nbt::Float(99.0));
        assert_eq!(Entry::from_nbt(&tag), original);
        tag.insert("level".into(), Nbt::Double(3.9));
        tag.insert("amount".into(), Nbt::Short(2));
        assert_eq!(Entry::from_nbt(&tag), original);
        tag.insert("level".into(), Nbt::Long(0x1_0000_0003));
        assert_eq!(Entry::from_nbt(&tag), original);
        tag.insert("level".into(), Nbt::String("3".into()));
        assert_eq!(Entry::from_nbt(&tag), Entry::empty());
        tag.insert("level".into(), Nbt::Int(3));
        tag.insert("name".into(), Nbt::String("invalid ID".into()));
        assert_eq!(Entry::from_nbt(&tag), Entry::empty());
        assert_eq!(nbt_int(Some(&Nbt::Double(-1.5))), -2);
        assert_eq!(nbt_int(Some(&Nbt::Double(f64::NAN))), 0);
        assert_eq!(Entry::from_nbt(&NbtCompound::new()), Entry::empty());
        assert!(!entry(3, 0, 0).to_nbt().contains_key("effective"));
    }
}
