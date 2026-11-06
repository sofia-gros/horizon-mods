// Ordered source conditional block effects for the bundled herbicide form.
use super::Game;
use serde_json::Value;

enum Predicate { Tag(String), Bush, Set(Vec<String>), Or(Vec<Predicate>) }
struct Action { offset: Option<[i32; 3]>, predicate: Predicate, replacement: Option<String> }

fn predicate(value: &Value, tags: &std::collections::HashMap<String, std::collections::HashSet<String>>) -> Option<Predicate> {
    if value.as_str() == Some("tconstruct:bush") { return Some(Predicate::Bush); }
    let object = value.as_object()?;
    match object.get("type")?.as_str()? {
        "mantle:tag" if object.len() == 2 => {
            let id = object.get("tag")?.as_str()?;
            tags.get(id)?;
            Some(Predicate::Tag(id.to_owned()))
        }
        "mantle:set" if object.len() == 2 => {
            let ids = object.get("blocks")?.as_array()?.iter().map(|v| v.as_str().map(str::to_owned)).collect::<Option<Vec<_>>>()?;
            Some(Predicate::Set(ids))
        }
        "mantle:or" if object.len() == 2 => {
            let entries = object.get("predicates")?.as_array()?.iter().map(|v| predicate(v, tags)).collect::<Option<Vec<_>>>()?;
            Some(Predicate::Or(entries))
        }
        _ => None,
    }
}

fn matches(predicate: &Predicate, name: &str, bush: bool, tags: &std::collections::HashMap<String, std::collections::HashSet<String>>) -> bool {
    match predicate {
        Predicate::Tag(id) => tags.get(id).is_some_and(|set| set.contains(name)),
        Predicate::Bush => bush,
        Predicate::Set(ids) => ids.iter().any(|id| id == name),
        Predicate::Or(values) => values.iter().any(|entry| matches(entry, name, bush, tags)),
    }
}

pub(super) fn supported(game: &Game<'_>, effects: &[Value]) -> bool {
    parse(game, effects).is_some()
}

fn parse(game: &Game<'_>, effects: &[Value]) -> Option<Vec<Action>> {
    let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_block_tags().as_ref()?.as_ref().ok()?;
    let mut result = Vec::new();
    for value in effects {
        let obj = value.as_object()?;
        let offset = match obj.get("type")?.as_str()? {
            "tconstruct:conditional" if obj.keys().all(|key| matches!(key.as_str(), "type" | "block" | "effect")) => None,
            "tconstruct:offset" if obj.get("offset_type")?.as_str()? == "tconstruct:conditional"
                && obj.keys().all(|key| matches!(key.as_str(), "type" | "offset_type" | "offset_direction" | "block" | "effect")) => {
                match obj.get("offset_direction") {
                    None => Some([0, 0, 0]), // resolved to the hit face at execution
                    Some(value) if value.as_str() == Some("up") => Some([0, 1, 0]),
                    _ => return None,
                }
            }
            _ => return None,
        };
        let predicate = predicate(obj.get("block")?, tags)?;
        let effect = obj.get("effect")?.as_object()?;
        let replacement = match effect.get("type")?.as_str()? {
            "tconstruct:remove_block" if effect.len() == 1 => None,
            "tconstruct:set_block" if effect.len() == 2 => {
                let id = effect.get("block")?.as_str()?;
                game.pack.assets.registry.default_state(id)?;
                Some(id.to_owned())
            }
            _ => return None,
        };
        result.push(Action { offset, predicate, replacement });
    }
    (!result.is_empty()).then_some(result)
}

/// Each source effect returns at most one level; `applied` survives a later
/// world-update error so the caller can still account for earlier effects.
pub(super) fn apply(game: &mut Game<'_>, effects: &[Value], position: [i32; 3], face: [i32; 3], applied: &mut bool) -> horizon_client::Result<()> {
    let Some(actions) = parse(game, effects) else { return Ok(()); };
    for action in actions {
        let offset = action.offset.map(|offset| if offset == [0, 0, 0] { face } else { offset }).unwrap_or([0, 0, 0]);
        let Some(x) = position[0].checked_add(offset[0]) else { continue; };
        let Some(y) = position[1].checked_add(offset[1]) else { continue; };
        let Some(z) = position[2].checked_add(offset[2]) else { continue; };
        let at = [x, y, z];
        if !game.world.loaded(x, z) { continue; }
        let state = game.world.block(x, y, z);
        let facts = game.res.facts().get(state);
        let name = facts.name.to_string();
        let bush = facts.has_class("BushBlock");
        let Some(tags) = game.world.chunks.__horizon_mixins_tconstruct_resolved_block_tags().as_ref().and_then(|r| r.as_ref().ok()) else { return Ok(()); };
        if !matches(&action.predicate, &name, bush, tags) { continue; }
        let replacement = if let Some(id) = action.replacement.as_deref() {
            if !game.may_build() { continue; }
            let Some(state) = game.pack.assets.registry.default_state(id) else { continue; };
            state
        } else { game.with_level(|blocks, _, _| horizon_client::pluto_fluid_facts::replacement_after_removal(blocks, state)) };
        if state == replacement { continue; }
        // Source reports one level for a valid change even if the world
        // operation reports an error after beginning the update.
        *applied = true;
        game.level_set(at, replacement)?;
    }
    Ok(())
}
