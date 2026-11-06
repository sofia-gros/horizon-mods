// Explicit current-index compare-and-apply; no owner registry or world hook.
use super::{Known, RecipeIndex};
use pluto_inventory_api::recipe_book::index::{self, Error, Index, Patch, Record, Snapshot};

fn snapshot(index: &RecipeIndex) -> Result<Snapshot, Error> {
    if index.recipes.len() > index::MAX_RECORDS {
        return Err(Error::LimitExceeded);
    }
    let mut result = Snapshot::new();
    let mut bytes = 0usize;
    for (id, known) in &index.recipes {
        for size in [
            id.len(),
            known.station.len(),
            known.result.as_ref().map_or(0, String::len),
        ] {
            bytes = bytes.checked_add(size).ok_or(Error::LimitExceeded)?;
            if bytes > index::MAX_TEXT_BYTES {
                return Err(Error::LimitExceeded);
            }
        }
        result.insert(
            id.clone(),
            Record {
                special: known.special,
                displays: known.displays,
                first_display: known.first_display,
                station: known.station.into(),
                notify: known.notify,
                result: known.result.clone(),
            },
        );
    }
    index::validate(&result)?;
    Ok(result)
}

// Known stores &'static str. Do not leak/intern arbitrary consumer strings.
// These are exactly the outputs of this official target's crafting_station.
fn station(value: &str) -> Result<&'static str, Error> {
    match value {
        "minecraft:crafting_table" => Ok("minecraft:crafting_table"),
        "minecraft:furnace" => Ok("minecraft:furnace"),
        "minecraft:blast_furnace" => Ok("minecraft:blast_furnace"),
        "minecraft:smoker" => Ok("minecraft:smoker"),
        "minecraft:campfire" => Ok("minecraft:campfire"),
        "minecraft:stonecutter" => Ok("minecraft:stonecutter"),
        "minecraft:brewing_stand" => Ok("minecraft:brewing_stand"),
        "minecraft:smithing_table" => Ok("minecraft:smithing_table"),
        _ => Err(Error::UnsupportedStation(value.into())),
    }
}

struct Borrowed<'a>(&'a mut RecipeIndex);
impl Index for Borrowed<'_> {
    fn snapshot(&self) -> Result<Snapshot, Error> {
        snapshot(self.0)
    }
    fn compare_and_apply(&mut self, patch: &Patch) -> Result<Snapshot, Error> {
        let current = snapshot(self.0)?;
        let next = index::plan(&current, patch)?;
        // All fallible validation/conversion precedes mutation. Clone retained
        // Known values instead of round-tripping arbitrary existing stations.
        let mut inserts = Vec::with_capacity(patch.insert.len());
        for id in patch.insert.keys() {
            let record = &next[id];
            inserts.push((
                id.clone(),
                Known {
                    special: record.special,
                    displays: record.displays,
                    first_display: record.first_display,
                    station: station(&record.station)?,
                    notify: record.notify,
                    result: record.result.clone(),
                },
            ));
        }
        let mut candidate = self.0.recipes.clone();
        for id in &patch.remove {
            candidate.remove(id);
        }
        candidate.extend(inserts);
        self.0.recipes = candidate;
        Ok(next)
    }
}

/// Borrow only this index for the call. This neither locates an authoritative
/// world nor publishes corresponding recipe definitions/knowledge to clients.
pub fn with_index<R>(index: &mut RecipeIndex, callback: impl FnOnce(&mut dyn Index) -> R) -> R {
    callback(&mut Borrowed(index))
}
