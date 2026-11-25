// Ordered expansion of normalized tag graphs, independent of a file format.
//
// Adapted from TConstruct's tag resolvers at horizon-mods commit
// 545ebd3e510c868bc6bf84f2b6a2529a1ef21ff3. Parsing, namespacing, registry
// identity, pack precedence and format-specific policy belong to the caller.
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    Value(String),
    Group(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Add {
        required: bool,
    },
    /// Missing targets are a no-op. Cycles are still unresolved errors.
    Remove,
}
impl Operation {
    fn optional(self) -> bool {
        matches!(self, Self::Add { required: false } | Self::Remove)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub target: Target,
    pub operation: Operation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Group {
    Missing,
    Invalid,
    Entries(Vec<Entry>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissingValues {
    /// An absent required value fails its group.
    Reject,
    /// Project onto a partial registry, omitting all absent values.
    Ignore,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Cycle(String),
    MissingGroup(String),
    InvalidGroup(String),
    MissingValue(String),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (kind, id) = match self {
            Self::Cycle(id) => ("cyclic group", id),
            Self::MissingGroup(id) => ("missing group", id),
            Self::InvalidGroup(id) => ("invalid group", id),
            Self::MissingValue(id) => ("missing value", id),
        };
        write!(f, "{kind} {id}")
    }
}
impl std::error::Error for Error {}

struct Frame {
    id: String,
    entries: std::vec::IntoIter<Entry>,
    result: Vec<String>,
    incoming: Option<Operation>,
}

fn load(id: String, incoming: Option<Operation>, group: Group) -> Result<Frame, Error> {
    match group {
        Group::Missing => Err(Error::MissingGroup(id)),
        Group::Invalid => Err(Error::InvalidGroup(id)),
        Group::Entries(entries) => Ok(Frame {
            id,
            incoming,
            entries: entries.into_iter(),
            result: Vec::new(),
        }),
    }
}

fn apply(result: &mut Vec<String>, values: impl IntoIterator<Item = String>, operation: Operation) {
    for value in values {
        if operation == Operation::Remove {
            result.retain(|existing| existing != &value);
        } else if !result.contains(&value) {
            result.push(value);
        }
    }
}

// Discard incomplete groups up to the first optional incoming edge. In
// particular, a child's partial values never escape a failed child group.
fn fail(frames: &mut Vec<Frame>, active: &mut HashSet<String>, error: Error) -> Result<(), Error> {
    while let Some(frame) = frames.pop() {
        active.remove(&frame.id);
        match frame.incoming {
            Some(operation) if operation.optional() => return Ok(()),
            Some(_) => {}
            None => return Err(error),
        }
    }
    Err(error)
}

/// Expand one normalized group into an ordered set of registered value IDs.
/// The callbacks must describe a stable snapshot for this call. No namespace,
/// `#` syntax, byte format, aliases, or particular registry is assumed here.
///
/// First insertion determines order. Removing and re-adding a value moves it
/// to the end. Missing/invalid optional groups are skipped in their entirety;
/// required failures propagate. Cycles always fail, even on optional/removal
/// edges. An explicit stack avoids recursion-depth dependence. It does not
/// impose a resource budget or cache repeated graph expansions.
pub fn resolve(
    id: &str,
    mut group: impl FnMut(&str) -> Group,
    mut contains: impl FnMut(&str) -> bool,
    missing_values: MissingValues,
) -> Result<Vec<String>, Error> {
    let root = load(id.to_owned(), None, group(id))?;
    let mut active = HashSet::from([id.to_owned()]);
    let mut frames = vec![root];
    loop {
        let entry = frames
            .last_mut()
            .expect("root or unfinished parent")
            .entries
            .next();
        let Some(entry) = entry else {
            let frame = frames.pop().expect("completed frame");
            active.remove(&frame.id);
            if let Some(parent) = frames.last_mut() {
                apply(
                    &mut parent.result,
                    frame.result,
                    frame.incoming.expect("child edge"),
                );
                continue;
            }
            return Ok(frame.result);
        };
        match entry.target {
            Target::Value(value) => {
                if contains(&value) {
                    apply(
                        &mut frames.last_mut().unwrap().result,
                        [value],
                        entry.operation,
                    );
                } else if missing_values == MissingValues::Reject && !entry.operation.optional() {
                    fail(&mut frames, &mut active, Error::MissingValue(value))?;
                }
            }
            Target::Group(id) => {
                if active.contains(&id) {
                    return Err(Error::Cycle(id));
                }
                match load(id.clone(), Some(entry.operation), group(&id)) {
                    Ok(frame) => {
                        active.insert(id);
                        frames.push(frame);
                    }
                    Err(_) if entry.operation.optional() => {}
                    Err(error) => fail(&mut frames, &mut active, error)?,
                }
            }
        }
    }
}

/// Search caller-selected graph edges for one value, in depth-first order.
/// This is structural reachability, not resolved registry membership. The
/// caller decides which operations become edges and how missing or invalid
/// groups are represented; an empty edge list simply has no reachable values.
///
/// Returns as soon as the value is encountered. An active-path cycle visited
/// before that match returns `Error::Cycle`; later branches are not visited.
/// Repeated groups on different completed paths are not cycles. Callbacks must
/// describe a stable snapshot. Traversal is iterative, without a cache or a
/// resource budget, and this function never emits the other `Error` variants.
pub fn reaches_value(
    id: &str,
    value: &str,
    mut edges: impl FnMut(&str) -> Vec<Target>,
) -> Result<bool, Error> {
    struct SearchFrame {
        id: String,
        targets: std::vec::IntoIter<Target>,
    }
    let mut active = HashSet::from([id.to_owned()]);
    let mut frames = vec![SearchFrame { id: id.to_owned(), targets: edges(id).into_iter() }];
    while let Some(frame) = frames.last_mut() {
        match frame.targets.next() {
            None => {
                let frame = frames.pop().expect("completed frame");
                active.remove(&frame.id);
            }
            Some(Target::Value(target)) if target == value => return Ok(true),
            Some(Target::Value(_)) => {}
            Some(Target::Group(id)) => {
                if !active.insert(id.clone()) {
                    return Err(Error::Cycle(id));
                }
                let targets = edges(&id).into_iter();
                frames.push(SearchFrame { id, targets });
            }
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    fn value(id: &str) -> Entry {
        Entry {
            target: Target::Value(id.into()),
            operation: Operation::Add { required: true },
        }
    }
    fn group(id: &str, required: bool) -> Entry {
        Entry {
            target: Target::Group(id.into()),
            operation: Operation::Add { required },
        }
    }
    fn run(entries: BTreeMap<&str, Vec<Entry>>) -> Result<Vec<String>, Error> {
        resolve(
            "root",
            |id| {
                entries
                    .get(id)
                    .cloned()
                    .map_or(Group::Missing, Group::Entries)
            },
            |_| true,
            MissingValues::Reject,
        )
    }
    #[test]
    fn ordered_nested_dedup_remove_and_readd() {
        let remove = Entry {
            target: Target::Group("child".into()),
            operation: Operation::Remove,
        };
        let result = run(BTreeMap::from([
            (
                "root",
                vec![
                    value("a"),
                    group("child", true),
                    value("b"),
                    value("d"),
                    remove,
                    value("b"),
                ],
            ),
            ("child", vec![value("b"), value("c")]),
        ]))
        .unwrap();
        assert_eq!(result, ["a", "d", "b"]);
    }
    #[test]
    fn optional_failed_child_discards_partial_values() {
        let graph = BTreeMap::from([
            ("root", vec![value("a"), group("bad", false), value("c")]),
            ("bad", vec![value("partial"), value("absent")]),
        ]);
        let result = resolve(
            "root",
            |id| Group::Entries(graph[id].clone()),
            |id| id != "absent",
            MissingValues::Reject,
        )
        .unwrap();
        assert_eq!(result, ["a", "c"]);
    }
    #[test]
    fn required_failure_and_projection_are_distinct_policies() {
        let read = |_: &str| Group::Entries(vec![value("a"), value("missing")]);
        assert_eq!(
            resolve("root", read, |id| id == "a", MissingValues::Reject),
            Err(Error::MissingValue("missing".into()))
        );
        assert_eq!(
            resolve("root", read, |id| id == "a", MissingValues::Ignore),
            Ok(vec!["a".into()])
        );
    }
    #[test]
    fn invalid_missing_and_optional_groups() {
        assert_eq!(
            resolve("root", |_| Group::Missing, |_| true, MissingValues::Reject),
            Err(Error::MissingGroup("root".into()))
        );
        assert_eq!(
            resolve("root", |_| Group::Invalid, |_| true, MissingValues::Reject),
            Err(Error::InvalidGroup("root".into()))
        );
        let result = resolve(
            "root",
            |id| {
                if id == "root" {
                    Group::Entries(vec![group("bad", false), value("a")])
                } else {
                    Group::Invalid
                }
            },
            |_| true,
            MissingValues::Reject,
        );
        assert_eq!(result, Ok(vec!["a".into()]));
    }
    #[test]
    fn cycles_are_never_optional() {
        assert_eq!(
            run(BTreeMap::from([("root", vec![group("root", false)])])),
            Err(Error::Cycle("root".into()))
        );
        let remove = Entry {
            target: Target::Group("root".into()),
            operation: Operation::Remove,
        };
        assert_eq!(
            run(BTreeMap::from([("root", vec![remove])])),
            Err(Error::Cycle("root".into()))
        );
    }
    #[test]
    fn failure_unwinds_to_nearest_optional_ancestor() {
        let graph = BTreeMap::from([
            ("root", vec![value("a"), group("middle", false), value("c")]),
            ("middle", vec![value("partial"), group("bad", true)]),
            ("bad", vec![value("missing")]),
        ]);
        let result = resolve(
            "root",
            |id| Group::Entries(graph[id].clone()),
            |id| id != "missing",
            MissingValues::Reject,
        );
        assert_eq!(result, Ok(vec!["a".into(), "c".into()]));
    }
    #[test]
    fn failed_removal_group_does_not_partially_remove_values() {
        let remove = Entry {
            target: Target::Group("bad".into()),
            operation: Operation::Remove,
        };
        let graph = BTreeMap::from([
            ("root", vec![value("a"), remove, value("c")]),
            ("bad", vec![value("a"), value("missing")]),
        ]);
        let result = resolve(
            "root",
            |id| Group::Entries(graph[id].clone()),
            |id| id != "missing",
            MissingValues::Reject,
        );
        assert_eq!(result, Ok(vec!["a".into(), "c".into()]));
    }
    #[test]
    fn deep_groups_do_not_recurse_on_the_call_stack() {
        let result = resolve(
            "0",
            |id| {
                let n: usize = id.parse().unwrap();
                Group::Entries(if n == 3000 {
                    vec![value("leaf")]
                } else {
                    vec![group(&(n + 1).to_string(), true)]
                })
            },
            |_| true,
            MissingValues::Reject,
        );
        assert_eq!(result, Ok(vec!["leaf".into()]));
    }
    #[test]
    fn reachability_stops_before_later_groups_after_a_match() {
        let mut reads = Vec::new();
        let result = reaches_value("root", "needle", |id| {
            reads.push(id.to_owned());
            match id {
                "root" => vec![Target::Group("child".into()), Target::Group("unvisited".into())],
                "child" => vec![Target::Value("needle".into()), Target::Group("child".into())],
                _ => panic!("unnecessary group lookup"),
            }
        });
        assert_eq!(result, Ok(true));
        assert_eq!(reads, ["root", "child"]);
    }
    #[test]
    fn reachability_reports_a_cycle_before_a_later_match() {
        assert_eq!(reaches_value("root", "needle", |_| vec![
            Target::Group("root".into()), Target::Value("needle".into()),
        ]), Err(Error::Cycle("root".into())));
    }
    #[test]
    fn completed_shared_groups_are_not_active_path_cycles() {
        let mut reads = Vec::new();
        let result = reaches_value("root", "needle", |id| {
            reads.push(id.to_owned());
            match id {
                "root" => vec![Target::Group("shared".into()), Target::Group("shared".into())],
                "shared" => vec![Target::Group("empty".into()), Target::Value("other".into())],
                _ => vec![],
            }
        });
        assert_eq!(result, Ok(false));
        assert_eq!(reads, ["root", "shared", "empty", "shared", "empty"]);
    }
    #[test]
    fn deep_reachability_does_not_recurse_on_the_call_stack() {
        let result = reaches_value("0", "leaf", |id| {
            let n: usize = id.parse().unwrap();
            if n == 3000 { vec![Target::Value("leaf".into())] }
            else { vec![Target::Group((n + 1).to_string())] }
        });
        assert_eq!(result, Ok(true));
    }
}
