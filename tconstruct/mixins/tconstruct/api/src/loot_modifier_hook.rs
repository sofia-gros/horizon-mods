//! Connects the global loot modifiers to PlutoLib's loot event. The event names the broken block's position but not its
//! state, so the block tag conditions are answered from the break this port saw just before: the block break event reports
//! the state, and a break's drops follow it at once. Breaks no player break event precedes (an explosion, a piston) report
//! no tool either, and every shipped modifier that tests a block tag also tests the tool, so they cannot fire there anyway.
//! A mob's death likewise reports no tool: the weapon-hit event, heard just before the killing damage, leaves the weapon
//! for the death's loot event, the same handoff the severing hook makes. The chrysophilite attribute is computed from the
//! breaker's worn armor, as GoldenAttributeModule maintains it: one flat point plus one per gold piece, present only while
//! some worn piece carries the modifier.
use crate::{break_speed_hook::with_held_tool, common, loot_modifiers::Context, melee_hook, tool_damage_hook, tool_stack};
use pluto_lib::{BlockState, Identifier, LootTableEvents, PlayerBlockBreakEvents, ServerLivingEntityEvents, ServerPlayer, StackData};
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

/// How long a hit's weapon stays waiting for the death it may cause (the severing hook's window).
const HIT_WINDOW: Duration = Duration::from_millis(500);
/// How long a seen break's state stays waiting for its drops.
const BREAK_WINDOW: Duration = Duration::from_secs(2);

struct Broken {
    dimension: String,
    pos: [i32; 3],
    state: BlockState,
    at: Instant,
}
static BROKEN: Mutex<Vec<Broken>> = Mutex::new(Vec::new());

struct Hit {
    entity: u32,
    weapon: StackData,
    at: Instant,
}
static HITS: Mutex<Vec<Hit>> = Mutex::new(Vec::new());

fn leave_broken(dimension: &str, pos: [i32; 3], state: BlockState) {
    let mut broken = BROKEN.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    broken.retain(|seen| seen.at.elapsed() < BREAK_WINDOW && (seen.dimension != dimension || seen.pos != pos));
    broken.push(Broken { dimension: dimension.to_owned(), pos, state, at: Instant::now() });
}

fn take_broken(dimension: &str, pos: [i32; 3]) -> Option<BlockState> {
    let mut broken = BROKEN.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    broken.retain(|seen| seen.at.elapsed() < BREAK_WINDOW);
    let at = broken.iter().position(|seen| seen.dimension == dimension && seen.pos == pos)?;
    Some(broken.remove(at).state)
}

fn leave_hit(entity: u32, weapon: StackData) {
    let mut hits = HITS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    hits.retain(|hit| hit.at.elapsed() < HIT_WINDOW && hit.entity != entity);
    hits.push(Hit { entity, weapon, at: Instant::now() });
}

fn take_hit(entity: u32) -> Option<StackData> {
    let mut hits = HITS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    hits.retain(|hit| hit.at.elapsed() < HIT_WINDOW);
    let at = hits.iter().position(|hit| hit.entity == entity)?;
    Some(hits.remove(at).weapon)
}

/// Whether the block is in the tag under the convention tag table, whose ore entries name the blocks of the original's
/// `forge:ores` tags (the game itself defines no `forge:` block tags).
fn convention_member(block: &Identifier, tag: &Identifier) -> bool {
    crate::convention_tags::TAGS
        .iter()
        .find(|(name, _)| *name == tag.to_string())
        .is_some_and(|(_, members)| members.iter().any(|member| *member == block.to_string()))
}

/// The breaker's chrysophilite attribute (GoldenAttributeModule): while a worn piece carries the modifier, one flat point
/// plus one per worn piece that has gold — a built piece with the piglin-neutral volatile flag (the golden modifier, or
/// chrysophilite itself at level 2), or one of the game's own golden armor pieces.
fn chrysophilite(player: Option<u128>) -> f32 {
    let Some(player) = player else { return 0.0 };
    let player = ServerPlayer { uuid: player, name: String::new() };
    let pieces: Vec<StackData> = (100..=103).filter_map(|slot| player.stack(slot)).collect();
    if pieces.is_empty() {
        return 0.0;
    }
    let Some(registry) = common::server_modifiers() else { return 0.0 };
    let Ok(registry) = registry.read() else { return 0.0 };
    let target = Identifier::new("tconstruct", "chrysophilite").expect("a written modifier name");
    let active = pieces.iter().any(|piece| {
        tool_stack::saved_modifiers(&piece.custom_data, &registry)
            .iter()
            .any(|(entry, _)| entry.id == target && entry.level >= 1)
    });
    if !active {
        return 0.0;
    }
    let gold = pieces
        .iter()
        .filter(|piece| {
            tool_stack::volatile_flag(&piece.custom_data, "tconstruct:piglin_neutral")
                || matches!(piece.item.as_str(), "minecraft:golden_helmet" | "minecraft:golden_chestplate" | "minecraft:golden_leggings" | "minecraft:golden_boots")
        })
        .count();
    1.0 + gold as f32
}

/// Registers the callbacks. Called once from the product's hooks, after the severing hook, whose callbacks the original's
/// list interleaves with these: the drops the two write never interact (heads and capacity against bones, bacon and
/// nuggets), so only the order of stacks in the drop list could ever differ.
pub(crate) fn register() {
    PlayerBlockBreakEvents::BEFORE.register(|broken| {
        if let Some(state) = broken.state.clone() {
            leave_broken(&broken.dimension, broken.pos, state);
        }
        true
    });
    ServerLivingEntityEvents::WEAPON_HIT.register(|hit| {
        let Ok(entity) = u32::try_from(hit.entity) else { return };
        if let Some(weapon) = hit.weapon.as_ref() {
            leave_hit(entity, weapon.clone());
        }
    });
    LootTableEvents::MODIFY_DROPS.register(|drops| {
        let Some(modifiers) = common::loot_modifiers() else { return };
        let Ok(modifiers) = modifiers.read() else { return };
        if !modifiers.loaded {
            return;
        }
        // The tool that made the drops: reported for breaks and uses; for a death, the weapon left by the killing hit.
        let mut tool = drops.tool.clone();
        if tool.is_none() && !drops.used && drops.block.is_none() {
            if let Some((entity, _)) = &drops.entity {
                tool = take_hit(*entity);
            }
        }
        let items = common::item_snapshot();
        let entities = common::entity_type_snapshot();
        let broken_state = drops.block.and_then(|pos| take_broken(&drops.dimension, pos));
        let block_in_tag = |tag: &Identifier| -> bool {
            let Some(state) = &broken_state else { return false };
            if state.is_in_tag(&tag.to_string()) {
                return true;
            }
            if convention_member(&state.block, tag) {
                return true;
            }
            // The chrysophilite ore tag names the gold ore tag, which the game does not define; the convention table
            // answers for it (the tag file itself is imported with the content).
            tag.to_string() == "tconstruct:chrysophilite_ores"
                && Identifier::parse("forge:ores/gold").is_ok_and(|gold| convention_member(&state.block, &gold))
        };
        let entity_kind = drops.entity.as_ref().and_then(|(_, kind)| Identifier::parse(kind).ok());
        let entity_in_tag = |tag: &Identifier| melee_hook::target_in_tag(entities.as_deref(), entity_kind.as_ref(), tag);
        let item_in_tag = |item: &Identifier, tag: &Identifier| {
            items.as_ref().is_some_and(|items| items.get_tag(tag).is_some_and(|members| members.contains(item)))
        };
        let tag_filled = |tag: &Identifier| items.as_ref().is_some_and(|items| items.get_tag(tag).is_some_and(|members| !members.is_empty()));
        let tag_preference = |tag: &Identifier| {
            let items = items.as_ref()?;
            items.get_tag(tag)?.iter().find(|item| items.contains(*item)).cloned()
        };
        let tool_level = |modifier: &Identifier| -> i32 {
            let (Some(tool), Some(items)) = (&tool, &items) else { return 0 };
            let (Ok(item), Ok(modifiable)) = (Identifier::parse(&tool.item), Identifier::new("tconstruct", "modifiable")) else { return 0 };
            if !items.get_tag(&modifiable).is_some_and(|members| members.contains(&item)) {
                return 0;
            }
            let Some(registry) = common::server_modifiers() else { return 0 };
            let Ok(registry) = registry.read() else { return 0 };
            tool_stack::saved_modifiers(&tool.custom_data, &registry)
                .iter()
                .filter(|(entry, _)| entry.id == *modifier)
                .map(|(entry, _)| entry.level)
                .sum()
        };
        let tool_looting = tool.as_ref().map_or(0, |tool| {
            with_held_tool(tool, |env, stack| tool_stack::looting(&stack.custom_data, env)).unwrap_or(0)
        });
        let config = |prop: &str| -> Option<bool> {
            let state = common::config_values()?;
            let state = state.read().ok()?;
            state.as_ref().ok()?.get(prop).copied()
        };
        let context = Context {
            table: &drops.table,
            has_block: drops.block.is_some(),
            has_entity: drops.entity.is_some(),
            entity: entity_kind.as_ref(),
            block_in_tag: &block_in_tag,
            entity_in_tag: &entity_in_tag,
            item_in_tag: &item_in_tag,
            tag_filled: &tag_filled,
            tag_preference: &tag_preference,
            tool_level: &tool_level,
            tool_looting,
            chrysophilite: chrysophilite(drops.player),
            config: &config,
            explosion: drops.explosion,
        };
        let mut loot = drops.drops.borrow_mut();
        modifiers.apply(&context, &mut loot, &mut tool_damage_hook::random);
    });
}
