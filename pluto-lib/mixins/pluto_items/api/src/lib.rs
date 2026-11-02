use std::{
    collections::BTreeMap,
    sync::{Arc, OnceLock, RwLock},
};

/// Mod-owned signed integer fields for one item stack.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ItemData(BTreeMap<String, i64>);
impl ItemData {
    pub fn get(&self, key: &str) -> Option<i64> {
        self.0.get(key).copied()
    }
    pub fn set(&mut self, key: impl Into<String>, value: i64) {
        self.0.insert(key.into(), value);
    }
    pub fn iter(&self) -> impl Iterator<Item = (&str, i64)> {
        self.0.iter().map(|(k, v)| (k.as_str(), *v))
    }
}
/// Pass keeps standard behavior; Handled consumes the use and commits data;
/// Cancel consumes the use and discards data changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UseOutcome {
    Pass,
    Handled,
    Cancel,
}
pub trait ItemBehavior: Send + Sync + 'static {
    fn on_use(&self, data: &mut ItemData) -> UseOutcome;
    fn tooltip(&self, _data: &ItemData) -> Vec<String> {
        Vec::new()
    }
}
static REGISTRY: OnceLock<RwLock<BTreeMap<String, Arc<dyn ItemBehavior>>>> = OnceLock::new();
fn registry() -> &'static RwLock<BTreeMap<String, Arc<dyn ItemBehavior>>> {
    REGISTRY.get_or_init(Default::default)
}
/// Register exactly one behavior per namespaced item ID.
pub fn register(id: &str, behavior: impl ItemBehavior) -> Result<(), &'static str> {
    let Some((namespace, path)) = id.split_once(':') else {
        return Err("expected a namespaced item ID");
    };
    if namespace.is_empty() || path.is_empty() || id.matches(':').count() != 1 {
        return Err("invalid item ID");
    }
    let mut entries = registry()
        .write()
        .map_err(|_| "behavior registry poisoned")?;
    if entries.contains_key(id) {
        return Err("item behavior already registered");
    }
    entries.insert(id.to_owned(), Arc::new(behavior));
    Ok(())
}
pub fn contains(id: &str) -> bool {
    registry().read().is_ok_and(|r| r.contains_key(id))
}
/// Run on a staged copy: only Handled commits changes to `data`.
pub fn dispatch(id: &str, data: &mut ItemData) -> UseOutcome {
    let behavior = registry().read().ok().and_then(|r| r.get(id).cloned());
    let Some(behavior) = behavior else {
        return UseOutcome::Pass;
    };
    let mut staged = data.clone();
    let result = behavior.on_use(&mut staged);
    if result == UseOutcome::Handled {
        *data = staged;
    }
    result
}
pub fn tooltip(id: &str, data: &ItemData) -> Vec<String> {
    registry()
        .read()
        .ok()
        .and_then(|r| r.get(id).cloned())
        .map_or_else(Vec::new, |b| b.tooltip(data))
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Set(UseOutcome);
    impl ItemBehavior for Set {
        fn on_use(&self, data: &mut ItemData) -> UseOutcome {
            data.set("test:value", 9);
            self.0
        }
    }
    #[test]
    fn commits_only_handled_and_rejects_duplicate_registration() {
        register("test:handled", Set(UseOutcome::Handled)).unwrap();
        register("test:pass", Set(UseOutcome::Pass)).unwrap();
        register("test:cancel", Set(UseOutcome::Cancel)).unwrap();
        assert!(register("test:handled", Set(UseOutcome::Pass)).is_err());
        let mut data = ItemData::default();
        data.set("test:value", 1);
        assert_eq!(dispatch("test:pass", &mut data), UseOutcome::Pass);
        assert_eq!(data.get("test:value"), Some(1));
        assert_eq!(dispatch("test:cancel", &mut data), UseOutcome::Cancel);
        assert_eq!(data.get("test:value"), Some(1));
        assert_eq!(dispatch("test:handled", &mut data), UseOutcome::Handled);
        assert_eq!(data.get("test:value"), Some(9));
        assert_eq!(dispatch("test:unknown", &mut data), UseOutcome::Pass);
    }
}
