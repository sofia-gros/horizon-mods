//! Synchronous projection of an already-loaded advancement tree.
//!
//! This API owns no registry, cache, lifecycle hook, goal IDs or reward policy.
//! Requests replace only criteria, requirements and recipe-reward IDs on existing
//! goals. Every other field is retained, and the entire batch commits or fails.
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Frame {
    Task,
    Goal,
    Challenge,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Display {
    pub icon: String,
    pub title: Value,
    pub description: Value,
    pub frame: Frame,
    pub background: Option<String>,
    pub show_toast: bool,
    pub announce: bool,
    pub hidden: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Criterion {
    pub name: String,
    pub trigger: String,
    pub conditions: Value,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Rewards {
    pub recipes: Vec<String>,
    pub experience: i32,
    pub loot: Vec<String>,
    pub function: Option<String>,
}
/// Complete normalized fields understood by the selected native adapter.
/// This is not the original source JSON: unknown native-ignored fields and JSON
/// formatting cannot be recovered from an already parsed tree.
#[derive(Clone, Debug, PartialEq)]
pub struct Definition {
    pub parent: Option<String>,
    pub display: Option<Display>,
    pub criteria: Vec<Criterion>,
    pub requirements: Vec<Vec<String>>,
    pub rewards: Rewards,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Replacement {
    pub id: String,
    /// Compare against the current complete normalized definition, not only ID.
    pub expected: Definition,
    pub criteria: Vec<Criterion>,
    pub requirements: Vec<Vec<String>>,
    pub recipe_rewards: Vec<String>,
}
impl Replacement {
    /// Proposed definition with nonprojectable fields retained exactly.
    pub fn definition(&self) -> Definition {
        let mut next = self.expected.clone();
        next.criteria = self.criteria.clone();
        next.requirements = self.requirements.clone();
        next.rewards.recipes = self.recipe_rewards.clone();
        next
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InvalidDefinition {
    DuplicateCriterion(String),
    UnknownRequirement(String),
    NonCanonical,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    RemoteClient,
    NotLoaded,
    DuplicateGoal(String),
    MissingGoal(String),
    StaleGoal(String),
    InvalidGoal {
        id: String,
        reason: InvalidDefinition,
    },
    InconsistentTree(String),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    Unchanged,
    Changed,
}

/// A borrowed capability for the current world, not an actor-progress handle.
/// No method loads a pending tree, emits an inventory event, grants rewards,
/// changes progress or schedules work. A remote client is never authoritative.
pub trait TreeSink {
    fn get(&self, id: &str) -> Result<Option<Definition>, Error>;
    fn replace(&mut self, changes: &[Replacement]) -> Result<Change, Error>;
}

/// Structural checks shared by adapters. No eligibility or completion policy.
/// Adapter validation/commit remains atomic even when this check succeeds.
pub fn validate(changes: &[Replacement]) -> Result<(), Error> {
    let mut ids = BTreeSet::new();
    for change in changes {
        if !ids.insert(change.id.as_str()) {
            return Err(Error::DuplicateGoal(change.id.clone()));
        }
        let mut names = BTreeSet::new();
        for criterion in &change.criteria {
            if !names.insert(criterion.name.as_str()) {
                return Err(Error::InvalidGoal {
                    id: change.id.clone(),
                    reason: InvalidDefinition::DuplicateCriterion(criterion.name.clone()),
                });
            }
        }
        for name in change.requirements.iter().flatten() {
            if !names.contains(name.as_str()) {
                return Err(Error::InvalidGoal {
                    id: change.id.clone(),
                    reason: InvalidDefinition::UnknownRequirement(name.clone()),
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn replacement() -> Replacement {
        let criteria = vec![Criterion {
            name: "a".into(),
            trigger: "example:trigger".into(),
            conditions: Value::Null,
        }];
        Replacement {
            id: "example:goal".into(),
            expected: Definition {
                parent: Some("example:parent".into()),
                display: None,
                criteria: criteria.clone(),
                requirements: vec![vec!["a".into()]],
                rewards: Rewards {
                    recipes: vec![],
                    experience: 3,
                    loot: vec!["example:loot".into()],
                    function: Some("example:function".into()),
                },
            },
            criteria,
            requirements: vec![vec!["a".into()]],
            recipe_rewards: vec!["example:recipe".into()],
        }
    }
    #[test]
    fn limited_projection_retains_nonrecipe_rewards_and_parent() {
        let r = replacement();
        let next = r.definition();
        assert_eq!(next.parent, r.expected.parent);
        assert_eq!(next.display, r.expected.display);
        assert_eq!(next.rewards.experience, 3);
        assert_eq!(next.rewards.function, r.expected.rewards.function);
        assert_eq!(next.rewards.loot, r.expected.rewards.loot);
        assert_eq!(next.rewards.recipes, r.recipe_rewards);
    }
    #[test]
    fn duplicates_and_dangling_requirements_fail_before_commit() {
        let mut r = replacement();
        assert!(validate(&[r.clone()]).is_ok());
        assert_eq!(
            validate(&[r.clone(), r.clone()]),
            Err(Error::DuplicateGoal(r.id.clone()))
        );
        r.criteria.push(r.criteria[0].clone());
        assert!(matches!(
            validate(&[r.clone()]),
            Err(Error::InvalidGoal {
                reason: InvalidDefinition::DuplicateCriterion(_),
                ..
            })
        ));
        r.criteria.pop();
        r.requirements.push(vec!["absent".into()]);
        assert!(matches!(
            validate(&[r]),
            Err(Error::InvalidGoal {
                reason: InvalidDefinition::UnknownRequirement(_),
                ..
            })
        ));
    }
    #[test]
    fn empty_batch_is_valid_and_no_namespace_or_recipe_policy_is_added() {
        assert_eq!(validate(&[]), Ok(()));
        let mut r = replacement();
        r.id = "another:goal".into();
        r.recipe_rewards = vec!["other:recipe".into()];
        assert_eq!(validate(&[r]), Ok(()));
    }
}
