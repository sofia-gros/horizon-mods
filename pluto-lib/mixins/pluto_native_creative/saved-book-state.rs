// Exact prepared-world ownership in the render layer; std only, no API dependency.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub type Saved = (BTreeSet<String>, BTreeSet<String>);
struct Pending {
    save_root: PathBuf,
    owners: BTreeMap<String, Saved>,
}
#[derive(Default)]
pub struct State {
    pending: Mutex<Option<Pending>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallError {
    Poisoned,
    AlreadyCaptured,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TakeError {
    Poisoned,
    WorldMismatch,
}
impl State {
    pub fn install(
        &self,
        save_root: PathBuf,
        owners: BTreeMap<String, Saved>,
    ) -> Result<(), InstallError> {
        let mut pending = self.pending.lock().map_err(|_| InstallError::Poisoned)?;
        if pending.is_some() {
            return Err(InstallError::AlreadyCaptured);
        }
        *pending = Some(Pending { save_root, owners });
        Ok(())
    }
    pub fn take(&self, save_root: &Path, owner: &str) -> Result<Option<Saved>, TakeError> {
        let mut pending = self.pending.lock().map_err(|_| TakeError::Poisoned)?;
        let Some(pending) = pending.as_mut() else {
            return Ok(None);
        };
        if pending.save_root != save_root {
            return Err(TakeError::WorldMismatch);
        }
        // Keep the empty marker: a second capture may not revive consumed IDs.
        Ok(pending.owners.remove(owner))
    }
    pub fn clear(&mut self) {
        self.pending = Mutex::new(None);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn owners() -> BTreeMap<String, Saved> {
        BTreeMap::from([(
            "test:a".into(),
            (BTreeSet::from(["a:1".into()]), BTreeSet::new()),
        )])
    }
    #[test]
    fn moves_with_exact_state_and_consumes_each_owner_once() {
        let first = State::default();
        let second = State::default();
        first.install("world-a".into(), owners()).unwrap();
        assert_eq!(second.take(Path::new("world-a"), "test:a"), Ok(None));
        let moved = first;
        assert_eq!(
            moved.take(Path::new("world-b"), "test:a"),
            Err(TakeError::WorldMismatch)
        );
        assert!(moved
            .take(Path::new("world-a"), "test:a")
            .unwrap()
            .is_some());
        assert_eq!(moved.take(Path::new("world-a"), "test:a"), Ok(None));
        assert_eq!(
            moved.install("world-a".into(), owners()),
            Err(InstallError::AlreadyCaptured)
        );
    }
    #[test]
    fn clear_discards_pending_and_does_not_touch_another_world() {
        let mut first = State::default();
        let second = State::default();
        first.install("world-a".into(), owners()).unwrap();
        second.install("world-b".into(), owners()).unwrap();
        first.clear();
        assert_eq!(first.take(Path::new("world-a"), "test:a"), Ok(None));
        assert!(second
            .take(Path::new("world-b"), "test:a")
            .unwrap()
            .is_some());
    }
}
