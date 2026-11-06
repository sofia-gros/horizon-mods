//! UNAPPLIED borrowed inspection, not a migration, save hook or actor authority.
//! Mirrors only the selected Level::read_player tuple decoding stage.
use horizon_client_core::nbt_value::{number, text};
use horizon_world::nbt::{JavaString, Tag};

pub type NativeTuple = (String, i32, i32, bool, bool);

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_entries: usize,
    pub max_fields_per_entry: usize,
    pub max_id_utf16_units: usize,
    pub max_total_id_utf16_units: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error { TooManyEntries, TooManyFields, IdTooLong, TotalIdsTooLong }

/// Observations identify data the five-field/native-record boundary cannot
/// silently discard. They are not a permission or a decision to migrate.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Notes {
    pub omitted_by_native: bool,
    pub lossy_id: bool,
    pub hidden_present: bool,
    pub icon_present: bool,
    pub unknown_fields: bool,
    pub amplifier_outside_record_range: bool,
}
#[derive(Debug)]
pub struct Entry<'a> {
    /// Exact source Tag remains borrowed, including unsupported fields/types.
    pub original: &'a Tag,
    /// The native tuple before registry resolution, constructor clamps or merge.
    pub native_tuple: Option<NativeTuple>,
    pub notes: Notes,
}
#[derive(Debug)]
pub struct Inspection<'a> {
    /// None means no original active_effects key, not an empty replacement key.
    pub original: Option<&'a Tag>,
    /// The existing reader treats non-List values as empty; migration must not
    /// reinterpret that observation as permission to delete the original value.
    pub wrong_root_type: bool,
    pub entries: Vec<Entry<'a>>,
}
impl Inspection<'_> {
    pub fn native_tuples(&self) -> impl Iterator<Item = &NativeTuple> {
        self.entries.iter().filter_map(|entry| entry.native_tuple.as_ref())
    }
}

/// Inspect the already-parsed, native-loader-stage active_effects Tag. Bounds
/// precede any ID conversion or result allocation. Unknown subtrees are only
/// borrowed; this does not parse wire bytes, recursively clone them or validate
/// their encoded size. The owner must retain its normal native parser limits.
pub fn inspect(original: Option<&Tag>, limits: Limits) -> Result<Inspection<'_>, Error> {
    let Some(Tag::List(list)) = original else {
        return Ok(Inspection { original, wrong_root_type: original.is_some(), entries: Vec::new() });
    };
    if list.len() > limits.max_entries { return Err(Error::TooManyEntries); }
    let mut total = 0usize;
    for entry in list {
        if let Tag::Compound(fields) = entry {
            if fields.len() > limits.max_fields_per_entry { return Err(Error::TooManyFields); }
        }
        if let Some(Tag::String(id)) = entry.get("id") {
            if id.0.len() > limits.max_id_utf16_units { return Err(Error::IdTooLong); }
            total = total.checked_add(id.0.len()).ok_or(Error::TotalIdsTooLong)?;
            if total > limits.max_total_id_utf16_units { return Err(Error::TotalIdsTooLong); }
        }
    }
    let known: [JavaString; 7] = ["id", "duration", "amplifier", "ambient", "show_particles", "show_icon", "hidden_effect"].map(Into::into);
    let entries = list.iter().map(|entry| {
        let native_tuple = text(entry.get("id")).map(|id| {
            let amplifier = match entry.get("amplifier") {
                Some(Tag::Byte(value)) => i32::from(*value as u8),
                other => number(other).unwrap_or(0) as i32,
            };
            (id, number(entry.get("duration")).unwrap_or(0) as i32,
             amplifier, number(entry.get("ambient")).unwrap_or(0) != 0,
             number(entry.get("show_particles")).is_none_or(|value| value != 0))
        });
        let notes = Notes {
            omitted_by_native: native_tuple.is_none(),
            lossy_id: matches!(entry.get("id"), Some(Tag::String(id)) if id.to_string().is_err()),
            hidden_present: entry.get("hidden_effect").is_some(),
            icon_present: entry.get("show_icon").is_some(),
            unknown_fields: matches!(entry, Tag::Compound(fields) if fields.keys().any(|key| !known.contains(key))),
            amplifier_outside_record_range: native_tuple.as_ref().is_some_and(|tuple| !(0..=255).contains(&tuple.2)),
        };
        Entry { original: entry, native_tuple, notes }
    }).collect();
    Ok(Inspection { original, wrong_root_type: false, entries })
}
