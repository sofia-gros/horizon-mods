// Private native staging adapter. No world/progress mutation or cache owner.
use super::{Advancement, Tree};
use pluto_inventory_api::advancements::projection::{
    self, Criterion, Definition, Display, Error, Frame, InvalidDefinition, Replacement, Rewards,
};
use serde_json::{json, Value};

fn definition(goal: &Advancement) -> Definition {
    Definition {
        parent: goal.parent.clone(),
        display: goal.display.as_ref().map(|display| Display {
            icon: display.icon.clone(),
            title: display.title.clone(),
            description: display.description.clone(),
            frame: match display.frame {
                super::Frame::Task => Frame::Task,
                super::Frame::Goal => Frame::Goal,
                super::Frame::Challenge => Frame::Challenge,
            },
            background: display.background.clone(),
            show_toast: display.show_toast,
            announce: display.announce,
            hidden: display.hidden,
        }),
        criteria: goal
            .criteria
            .iter()
            .map(|criterion| Criterion {
                name: criterion.name.clone(),
                trigger: criterion.trigger.clone(),
                conditions: criterion.conditions.read().as_ref().clone(),
            })
            .collect(),
        requirements: goal.requirements.clone(),
        rewards: Rewards {
            recipes: goal.rewards.recipes.clone(),
            experience: goal.rewards.experience,
            loot: goal.rewards.loot.clone(),
            function: goal.rewards.function.clone(),
        },
    }
}

pub fn get(tree: &Tree, id: &str) -> Result<Option<Definition>, Error> {
    let Some(&at) = tree.by_id.get(id) else {
        return Ok(None);
    };
    let goal = tree
        .list
        .get(at)
        .filter(|goal| goal.id == id)
        .ok_or_else(|| Error::InconsistentTree(id.to_owned()))?;
    Ok(Some(definition(goal)))
}

fn document(definition: &Definition) -> Value {
    let criteria: serde_json::Map<String, Value> = definition
        .criteria
        .iter()
        .map(|criterion| {
            (
                criterion.name.clone(),
                json!({"trigger":criterion.trigger,"conditions":criterion.conditions}),
            )
        })
        .collect();
    let display=definition.display.as_ref().map(|d|json!({
        "icon":{"id":d.icon},"title":d.title,"description":d.description,
        "frame":match d.frame { Frame::Task=>"task", Frame::Goal=>"goal", Frame::Challenge=>"challenge" },
        "background":d.background,"show_toast":d.show_toast,"announce_to_chat":d.announce,"hidden":d.hidden,
    }));
    json!({"parent":definition.parent,"display":display,"criteria":criteria,"requirements":definition.requirements,
        "rewards":{"recipes":definition.rewards.recipes,"experience":definition.rewards.experience,"loot":definition.rewards.loot,"function":definition.rewards.function}})
}

/// Validate the complete batch before allocating/publishing a replacement tree.
/// None preserves the caller's Arc and both native/custom index caches.
pub fn stage(tree: &Tree, changes: &[Replacement]) -> Result<Option<Tree>, Error> {
    projection::validate(changes)?;
    let mut staged = Vec::new();
    for change in changes {
        let old = get(tree, &change.id)?.ok_or_else(|| Error::MissingGoal(change.id.clone()))?;
        if old != change.expected {
            return Err(Error::StaleGoal(change.id.clone()));
        }
        let proposed = change.definition();
        if proposed == old {
            continue;
        }
        let parsed =
            super::parse(&change.id, &document(&proposed)).ok_or_else(|| Error::InvalidGoal {
                id: change.id.clone(),
                reason: InvalidDefinition::NonCanonical,
            })?;
        // Reject definitions the native parser would silently normalize/drop.
        // Consumers receive normalized snapshots and must submit that same form.
        if definition(&parsed) != proposed {
            return Err(Error::InvalidGoal {
                id: change.id.clone(),
                reason: InvalidDefinition::NonCanonical,
            });
        }
        if proposed != old {
            staged.push((tree.by_id[&change.id], parsed));
        }
    }
    if staged.is_empty() {
        return Ok(None);
    }
    let mut next = Tree {
        list: tree.list.clone(),
        by_id: tree.by_id.clone(),
        ..Tree::default()
    };
    for (at, goal) in staged {
        next.list[at] = goal;
    }
    for (ai, goal) in next.list.iter().enumerate() {
        for (ci, criterion) in goal.criteria.iter().enumerate() {
            next.by_trigger
                .entry(criterion.trigger.clone())
                .or_default()
                .push((ai, ci));
        }
    }
    next.index_recipes();
    next.index_looking_at();
    next.index_nbt();
    next.index_player_facts();
    Ok(Some(next))
}
