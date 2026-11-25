//! Explicit compare-and-apply for one currently borrowed recipe index.
//!
//! Snapshots describe values, not ownership or reload identity. A byte-identical
//! replacement passes comparison. No registry, owner token or allocation lease
//! survives this operation. Callers own recipe policy and publication ordering.

use std::collections::{BTreeMap, BTreeSet};

pub const MAX_RECORDS: usize = 65_536;
pub const MAX_CHANGES: usize = 16_384;
pub const MAX_TEXT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_GAP_SCANS: usize = 1_000_000;

/// All index fields, including notification-suppressed station information.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub special: bool,
    pub displays: usize,
    pub first_display: i32,
    pub station: String,
    pub notify: bool,
    pub result: Option<String>,
}

pub type Snapshot = BTreeMap<String, Record>;

/// A new record. Its display range is assigned by the explicit allocation rule.
/// Zero-display records receive first_display=0 and consume no display range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Definition {
    pub special: bool,
    pub displays: usize,
    pub station: String,
    pub notify: bool,
    pub result: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Allocation {
    /// Start after the greatest retained range end, including empty ranges.
    Append,
    /// Use the lowest nonnegative contiguous gap large enough for each record.
    FirstGap,
}

/// Exact removals and replacements; no prefix or inferred ownership rules.
/// Replacing an existing ID requires explicitly listing it in `remove`.
#[derive(Clone, Debug)]
pub struct Patch {
    pub expected: Snapshot,
    pub remove: BTreeSet<String>,
    pub insert: BTreeMap<String, Definition>,
    pub allocation: Allocation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    StaleSnapshot,
    LimitExceeded,
    InvalidRecord(String),
    RangeOverflow(String),
    OverlappingDisplays,
    MissingRemoval(String),
    ExistingId(String),
    UnsupportedStation(String),
}

/// This synchronous capability changes only index records. It does not edit
/// recipe definitions, clear UI caches, grant knowledge or send network data.
/// When those must change together, operate on a staged index clone and publish
/// all validated candidates in a no-fail tail before any observer/callback.
pub trait Index {
    fn snapshot(&self) -> Result<Snapshot, Error>;
    fn compare_and_apply(&mut self, patch: &Patch) -> Result<Snapshot, Error>;
}

fn end(id: &str, record: &Record) -> Result<i32, Error> {
    if id.is_empty() || record.station.is_empty() || record.first_display < 0 {
        return Err(Error::InvalidRecord(id.into()));
    }
    let count = i32::try_from(record.displays).map_err(|_| Error::RangeOverflow(id.into()))?;
    record
        .first_display
        .checked_add(count)
        .ok_or_else(|| Error::RangeOverflow(id.into()))
}

/// Validate a complete current snapshot without changing it. Existing unusual
/// special/display combinations are preserved; newly proposed special records
/// must have no displays. Malformed or overlapping native ranges fail closed.
pub fn validate(snapshot: &Snapshot) -> Result<(), Error> {
    if snapshot.len() > MAX_RECORDS {
        return Err(Error::LimitExceeded);
    }
    let mut bytes = 0usize;
    let mut ranges = Vec::new();
    for (id, record) in snapshot {
        for size in [
            id.len(),
            record.station.len(),
            record.result.as_ref().map_or(0, String::len),
        ] {
            bytes = bytes.checked_add(size).ok_or(Error::LimitExceeded)?;
            if bytes > MAX_TEXT_BYTES {
                return Err(Error::LimitExceeded);
            }
        }
        let end = end(id, record)?;
        if record.displays != 0 {
            ranges.push((record.first_display, end));
        }
    }
    ranges.sort_unstable();
    if ranges.windows(2).any(|pair| pair[0].1 > pair[1].0) {
        return Err(Error::OverlappingDisplays);
    }
    Ok(())
}

/// Build the complete replacement in isolation. IDs are allocated in BTreeMap
/// order; retained records and ranges are byte-for-byte value preserving.
pub fn plan(current: &Snapshot, patch: &Patch) -> Result<Snapshot, Error> {
    validate(current)?;
    if current != &patch.expected {
        return Err(Error::StaleSnapshot);
    }
    if patch.remove.len() > MAX_CHANGES || patch.insert.len() > MAX_CHANGES {
        return Err(Error::LimitExceeded);
    }
    for id in &patch.remove {
        if !current.contains_key(id) {
            return Err(Error::MissingRemoval(id.clone()));
        }
    }
    for id in patch.insert.keys() {
        if current.contains_key(id) && !patch.remove.contains(id) {
            return Err(Error::ExistingId(id.clone()));
        }
    }
    // Bound the proposed text before cloning any consumer-supplied strings.
    let mut text = 0usize;
    for (id, record) in current.iter().filter(|(id, _)| !patch.remove.contains(*id)) {
        for size in [
            id.len(),
            record.station.len(),
            record.result.as_ref().map_or(0, String::len),
        ] {
            text = text.checked_add(size).ok_or(Error::LimitExceeded)?;
            if text > MAX_TEXT_BYTES {
                return Err(Error::LimitExceeded);
            }
        }
    }
    for (id, definition) in &patch.insert {
        for size in [
            id.len(),
            definition.station.len(),
            definition.result.as_ref().map_or(0, String::len),
        ] {
            text = text.checked_add(size).ok_or(Error::LimitExceeded)?;
            if text > MAX_TEXT_BYTES {
                return Err(Error::LimitExceeded);
            }
        }
    }
    let mut next = current.clone();
    for id in &patch.remove {
        next.remove(id);
    }
    if next
        .len()
        .checked_add(patch.insert.len())
        .is_none_or(|len| len > MAX_RECORDS)
    {
        return Err(Error::LimitExceeded);
    }
    let mut occupied = Vec::new();
    let mut append = 0;
    for (id, record) in &next {
        let finish = end(id, record)?;
        append = append.max(finish);
        if record.displays != 0 {
            occupied.push((record.first_display, finish));
        }
    }
    occupied.sort_unstable();
    let mut gaps = BTreeMap::new();
    let mut start = 0;
    for (from, to) in occupied {
        if start < from {
            gaps.insert(start, from);
        }
        start = to;
    }
    if start < i32::MAX {
        gaps.insert(start, i32::MAX);
    }
    let mut scans = 0usize;
    for (id, definition) in &patch.insert {
        if definition.special && definition.displays != 0 {
            return Err(Error::InvalidRecord(id.clone()));
        }
        let count =
            i32::try_from(definition.displays).map_err(|_| Error::RangeOverflow(id.clone()))?;
        let first = if count == 0 {
            0
        } else {
            match patch.allocation {
                Allocation::Append => append,
                Allocation::FirstGap => {
                    // Retain free ranges rather than rescanning every assigned
                    // record. Different sizes still search from the first gap.
                    let mut selected = None;
                    for (&from, &to) in &gaps {
                        scans += 1;
                        if scans > MAX_GAP_SCANS {
                            return Err(Error::LimitExceeded);
                        }
                        if to - from >= count {
                            selected = Some((from, to));
                            break;
                        }
                    }
                    let (from, to) = selected.ok_or_else(|| Error::RangeOverflow(id.clone()))?;
                    gaps.remove(&from);
                    let after = from + count;
                    if after < to {
                        gaps.insert(after, to);
                    }
                    from
                }
            }
        };
        let finish = first
            .checked_add(count)
            .ok_or_else(|| Error::RangeOverflow(id.clone()))?;
        let record = Record {
            special: definition.special,
            displays: definition.displays,
            first_display: first,
            station: definition.station.clone(),
            notify: definition.notify,
            result: definition.result.clone(),
        };
        end(id, &record)?;
        if count != 0 {
            append = finish;
        }
        next.insert(id.clone(), record);
    }
    validate(&next)?;
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn record(first: i32, count: usize) -> Record {
        Record {
            special: false,
            displays: count,
            first_display: first,
            station: "example:station".into(),
            notify: false,
            result: Some("example:item".into()),
        }
    }
    fn definition(count: usize) -> Definition {
        Definition {
            special: false,
            displays: count,
            station: "example:station".into(),
            notify: true,
            result: None,
        }
    }
    fn patch(current: &Snapshot, allocation: Allocation) -> Patch {
        Patch {
            expected: current.clone(),
            remove: BTreeSet::new(),
            insert: BTreeMap::new(),
            allocation,
        }
    }
    #[test]
    fn allocation_modes_remain_distinct_and_preserve_unrelated_ranges() {
        let current = Snapshot::from([
            ("other:a".into(), record(1, 2)),
            ("other:b".into(), record(5, 1)),
        ]);
        let mut p = patch(&current, Allocation::Append);
        p.insert.insert("example:a".into(), definition(1));
        p.insert.insert("example:b".into(), definition(1));
        let append = plan(&current, &p).unwrap();
        assert_eq!(
            (
                append["example:a"].first_display,
                append["example:b"].first_display
            ),
            (6, 7)
        );
        p.allocation = Allocation::FirstGap;
        let gaps = plan(&current, &p).unwrap();
        assert_eq!(
            (
                gaps["example:a"].first_display,
                gaps["example:b"].first_display
            ),
            (0, 3)
        );
        for (id, old) in current {
            assert_eq!(append[&id], old);
            assert_eq!(gaps[&id], old);
        }
    }
    #[test]
    fn first_gap_restarts_for_each_different_range_size() {
        let current = Snapshot::from([("other:a".into(), record(1, 2))]);
        let mut p = patch(&current, Allocation::FirstGap);
        p.insert.insert("example:a".into(), definition(2));
        p.insert.insert("example:b".into(), definition(1));
        let after = plan(&current, &p).unwrap();
        assert_eq!(after["example:a"].first_display, 3);
        assert_eq!(after["example:b"].first_display, 0);
    }
    #[test]
    fn explicit_replacement_reuses_range_and_never_infers_prefix_ownership() {
        let current = Snapshot::from([
            ("same:a".into(), record(0, 1)),
            ("same:b".into(), record(1, 1)),
        ]);
        let mut p = patch(&current, Allocation::FirstGap);
        p.insert.insert("same:a".into(), definition(1));
        assert_eq!(plan(&current, &p), Err(Error::ExistingId("same:a".into())));
        p.remove.insert("same:a".into());
        let after = plan(&current, &p).unwrap();
        assert_eq!(after["same:a"].first_display, 0);
        assert_eq!(after["same:b"], current["same:b"]);
        p.remove.insert("missing".into());
        assert_eq!(
            plan(&current, &p),
            Err(Error::MissingRemoval("missing".into()))
        );
    }
    #[test]
    fn every_current_field_and_unrelated_additions_participate_in_compare() {
        let current = Snapshot::from([("other:a".into(), record(2, 1))]);
        let p = patch(&current, Allocation::Append);
        let variants = [
            Record {
                special: true,
                ..record(2, 1)
            },
            record(3, 1),
            record(2, 2),
            Record {
                station: "example:other".into(),
                ..record(2, 1)
            },
            Record {
                notify: true,
                ..record(2, 1)
            },
            Record {
                result: None,
                ..record(2, 1)
            },
        ];
        for changed in variants {
            assert_eq!(
                plan(&Snapshot::from([("other:a".into(), changed)]), &p),
                Err(Error::StaleSnapshot)
            );
        }
        let mut changed = current.clone();
        changed.insert("other:b".into(), record(4, 1));
        assert_eq!(plan(&changed, &p), Err(Error::StaleSnapshot));
    }
    #[test]
    fn overlapping_negative_and_overflowing_ranges_fail_closed() {
        let overlap = Snapshot::from([("a".into(), record(0, 2)), ("b".into(), record(1, 1))]);
        assert_eq!(validate(&overlap), Err(Error::OverlappingDisplays));
        assert_eq!(
            validate(&Snapshot::from([("a".into(), record(-1, 1))])),
            Err(Error::InvalidRecord("a".into()))
        );
        let current = Snapshot::from([("a".into(), record(i32::MAX, 0))]);
        let mut p = patch(&current, Allocation::Append);
        p.insert.insert("new".into(), definition(1));
        assert_eq!(plan(&current, &p), Err(Error::RangeOverflow("new".into())));
        p.allocation = Allocation::FirstGap;
        assert_eq!(plan(&current, &p).unwrap()["new"].first_display, 0);
    }
    #[test]
    fn late_batch_failure_returns_no_partial_replacement() {
        let current = Snapshot::from([("a".into(), record(0, 1))]);
        let mut p = patch(&current, Allocation::Append);
        p.insert.insert("new:a".into(), definition(1));
        p.insert.insert("new:z".into(), definition(usize::MAX));
        assert_eq!(
            plan(&current, &p),
            Err(Error::RangeOverflow("new:z".into()))
        );
        assert_eq!(current.len(), 1);
    }
    #[test]
    fn zero_display_specials_need_no_range_and_invalid_new_specials_are_rejected() {
        let current = Snapshot::new();
        let mut p = patch(&current, Allocation::Append);
        p.insert.insert(
            "a".into(),
            Definition {
                special: true,
                ..definition(0)
            },
        );
        assert_eq!(plan(&current, &p).unwrap()["a"].first_display, 0);
        p.insert.get_mut("a").unwrap().displays = 1;
        assert_eq!(plan(&current, &p), Err(Error::InvalidRecord("a".into())));
    }
    #[test]
    fn empty_batch_is_an_exact_value_noop_and_duplicate_reload_is_not_detected() {
        let current = Snapshot::from([("a".into(), record(4, 1))]);
        let p = patch(&current, Allocation::Append);
        assert_eq!(plan(&current.clone(), &p), Ok(current));
    }
    #[test]
    fn proposed_text_is_bounded_before_cloning() {
        let current = Snapshot::new();
        let mut p = patch(&current, Allocation::Append);
        let mut d = definition(1);
        d.result = Some("x".repeat(MAX_TEXT_BYTES));
        p.insert.insert("a".into(), d);
        assert_eq!(plan(&current, &p), Err(Error::LimitExceeded));
    }

    #[test]
    fn first_gap_handles_large_simple_batches_without_quadratic_assigned_scan() {
        let current = Snapshot::new();
        let mut p = patch(&current, Allocation::FirstGap);
        for n in 0..8192 {
            p.insert.insert(format!("example:{n:05}"), definition(1));
        }
        let after = plan(&current, &p).unwrap();
        assert_eq!(after["example:08191"].first_display, 8191);
    }

    #[test]
    fn fragmented_search_has_a_bounded_work_budget() {
        let current = (0..1000)
            .map(|n| (format!("other:{n}"), record(2 * n + 1, 1)))
            .collect();
        let mut p = patch(&current, Allocation::FirstGap);
        for n in 0..1000 {
            p.insert.insert(format!("example:{n:04}"), definition(2));
        }
        assert_eq!(plan(&current, &p), Err(Error::LimitExceeded));
    }
}
