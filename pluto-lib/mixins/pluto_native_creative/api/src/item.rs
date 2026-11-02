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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hand {
    Main,
    Off,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockTarget {
    pub position: [i32; 3],
    pub face: [i32; 3],
}
/// One candidate placement, committed only with a handled block use.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacementRequest {
    /// Namespaced block ID and registered state properties.
    pub block_id: String,
    pub properties: BTreeMap<String, String>,
    /// Direction from the destination to the adjacent supporting block.
    pub support_direction: [i32; 3],
    /// Permit replacing a liquid destination when the host marks it replaceable.
    pub replace_liquids: bool,
}
/// The host reports whether a requested placement took effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlacementOutcome {
    Rejected,
    Applied {
        item_committed: bool,
        events_ok: bool,
    },
}
/// A block-use result; requests on Pass or Cancel are discarded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockUse {
    pub outcome: UseOutcome,
    pub placement: Option<PlacementRequest>,
}
impl From<UseOutcome> for BlockUse {
    fn from(outcome: UseOutcome) -> Self {
        Self {
            outcome,
            placement: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Actor {
    pub id: u128,
    pub dimension: String,
    pub position: [f64; 3],
    pub permission: u8,
}
#[derive(Clone, Debug, PartialEq)]
pub struct UseContext {
    pub actor: Option<Actor>,
    pub hand: Hand,
    pub item_id: String,
    pub count: u32,
}
pub trait ItemBehavior: Send + Sync + 'static {
    fn on_use(&self, data: &mut ItemData) -> UseOutcome;
    fn on_use_with_context(&self, _context: &UseContext, data: &mut ItemData) -> UseOutcome {
        self.on_use(data)
    }
    fn on_use_on_block(
        &self,
        _context: &UseContext,
        _target: BlockTarget,
        _data: &mut ItemData,
    ) -> UseOutcome {
        UseOutcome::Pass
    }
    /// Return at most one placement request. No world changes happen here.
    fn on_use_on_block_request(
        &self,
        context: &UseContext,
        target: BlockTarget,
        data: &mut ItemData,
    ) -> BlockUse {
        self.on_use_on_block(context, target, data).into()
    }
    /// Called once after a requested placement; it cannot change staged data.
    fn on_placement_result(&self, _result: PlacementOutcome) {}
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
/// Invoke exactly one event. Only `Handled` commits staged integer fields.
pub fn dispatch_event(
    id: &str,
    context: &UseContext,
    target: Option<BlockTarget>,
    data: &mut ItemData,
) -> UseOutcome {
    let behavior = registry().read().ok().and_then(|r| r.get(id).cloned());
    let Some(behavior) = behavior else {
        return UseOutcome::Pass;
    };
    let mut staged = data.clone();
    let outcome = match target {
        Some(target) => behavior.on_use_on_block(context, target, &mut staged),
        None => behavior.on_use_with_context(context, &mut staged),
    };
    if outcome == UseOutcome::Handled {
        *data = staged;
    }
    outcome
}
/// Stage block-use data and keep a placement only for Handled.
pub fn dispatch_block(
    id: &str,
    context: &UseContext,
    target: BlockTarget,
    data: &mut ItemData,
) -> BlockUse {
    let behavior = registry().read().ok().and_then(|r| r.get(id).cloned());
    let Some(behavior) = behavior else {
        return UseOutcome::Pass.into();
    };
    let mut staged = data.clone();
    let mut result = behavior.on_use_on_block_request(context, target, &mut staged);
    if result.outcome == UseOutcome::Handled {
        *data = staged;
    } else {
        result.placement = None;
    }
    result
}
pub fn notify_placement(id: &str, result: PlacementOutcome) {
    if let Some(behavior) = registry().read().ok().and_then(|r| r.get(id).cloned()) {
        behavior.on_placement_result(result);
    }
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
    struct Events;
    impl ItemBehavior for Events {
        fn on_use(&self, data: &mut ItemData) -> UseOutcome {
            data.set("test:value", 8);
            UseOutcome::Handled
        }
        fn on_use_on_block(
            &self,
            _: &UseContext,
            _: BlockTarget,
            data: &mut ItemData,
        ) -> UseOutcome {
            data.set("test:value", 9);
            UseOutcome::Pass
        }
    }
    #[test]
    fn block_pass_discards_edits_then_item_event_can_commit() {
        register("test:events", Events).unwrap();
        let context = UseContext {
            actor: None,
            hand: Hand::Off,
            item_id: "test:events".into(),
            count: 1,
        };
        let mut data = ItemData::default();
        data.set("test:value", 1);
        let target = BlockTarget {
            position: [1, 2, 3],
            face: [0, 1, 0],
        };
        assert_eq!(
            dispatch_event("test:events", &context, Some(target), &mut data),
            UseOutcome::Pass
        );
        assert_eq!(data.get("test:value"), Some(1));
        assert_eq!(
            dispatch_event("test:events", &context, None, &mut data),
            UseOutcome::Handled
        );
        assert_eq!(data.get("test:value"), Some(8));
    }
    #[test]
    fn contextual_item_cancel_discards_edits() {
        register("test:event_cancel", Set(UseOutcome::Cancel)).unwrap();
        let context = UseContext {
            actor: None,
            hand: Hand::Main,
            item_id: "test:event_cancel".into(),
            count: 1,
        };
        let mut data = ItemData::default();
        data.set("test:value", 1);
        assert_eq!(
            dispatch_event("test:event_cancel", &context, None, &mut data),
            UseOutcome::Cancel
        );
        assert_eq!(data.get("test:value"), Some(1));
    }
}

#[cfg(test)]
mod placement_tests {
    use super::*;
    struct Request(UseOutcome);
    impl ItemBehavior for Request {
        fn on_use(&self, _: &mut ItemData) -> UseOutcome {
            UseOutcome::Pass
        }
        fn on_use_on_block_request(
            &self,
            _: &UseContext,
            _: BlockTarget,
            data: &mut ItemData,
        ) -> BlockUse {
            data.set("cost", 1);
            BlockUse {
                outcome: self.0,
                placement: Some(PlacementRequest {
                    block_id: "example:block".into(),
                    properties: BTreeMap::new(),
                    support_direction: [0, -1, 0],
                    replace_liquids: false,
                }),
            }
        }
    }
    #[test]
    fn nonhandled_block_use_discards_data_and_request() {
        let context = UseContext {
            actor: None,
            hand: Hand::Main,
            item_id: "test:placement".into(),
            count: 1,
        };
        let target = BlockTarget {
            position: [0, 1, 0],
            face: [0, 1, 0],
        };
        for (id, outcome) in [
            ("placement:pass", UseOutcome::Pass),
            ("placement:cancel", UseOutcome::Cancel),
            ("placement:handled", UseOutcome::Handled),
        ] {
            register(id, Request(outcome)).unwrap();
            let mut data = ItemData::default();
            let result = dispatch_block(id, &context, target, &mut data);
            assert_eq!(result.outcome, outcome);
            assert_eq!(result.placement.is_some(), outcome == UseOutcome::Handled);
            assert_eq!(
                data.get("cost"),
                if outcome == UseOutcome::Handled {
                    Some(1)
                } else {
                    None
                }
            );
        }
    }
}
